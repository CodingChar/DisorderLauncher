# Optimizacion de instancias

Tres palancas, todas opcionales y reversibles. Codigo: `crates/jvm-tuning` (mecanismo), `crates/launcher-core/src/optimization.rs` (politica: niveles, opciones graficas, mods) y `apps/desktop/src/routes/parts/JavaTweaks.svelte` (pantalla).

## 1. JVM

Cada instancia tiene:

- **Nivel** (`optimization`): Normal, Equilibrado, Alto rendimiento, Modesto. Cada nivel es una lista de ajustes del catalogo (`profile_tweaks`).
- **Ajustes** (`jvmTweaks`): ids del catalogo que la persona activa ademas del nivel.
- **Argumentos propios** (`jvmArgs`): texto libre, validado.

Al lanzar, `apply_optimization` hace: nivel + ajustes + argumentos → `jvm_tuning::resolve` (descarta lo que no aplica a ese Java/equipo y resuelve conflictos) → `jvm_tuning::keep_working` (arranca el Java real con `-version`; si lo rechaza, va bloque a bloque y aparta solo el que falla). Lo omitido por la JVM se anota en la consola de la instancia. Con `Off` y sin ajustes ni argumentos no se anade ninguna bandera.

Los argumentos propios van **al final** (en la JVM manda la ultima bandera) y mandan sobre las banderas del catalogo con el mismo nombre; si fijan un recolector, los ajustes que eligen otro se descartan.

### Seguridad de los argumentos propios

Nunca pasan por un shell (el juego se lanza con argv directo). Solo se aceptan `-XX:`, `-X` y `-D`; ASCII, hasta 40 argumentos de 200 caracteres. Se rechazan (`crates/jvm-tuning/src/args.rs`): `-javaagent`, `-agentlib/-agentpath`, `-Xbootclasspath`, `-Xrun`, `-Xdebug`, `-cp/-jar`, `@archivo`, `-XX:OnError`, `-XX:OnOutOfMemoryError`, `-XX:Flags`, `-XX:VMOptionsFile`, `-XX:ErrorFile`, `-XX:HeapDumpPath`, `-XX:SharedArchiveFile`, `-Xlog:`, propiedades de seguridad/SSL/JMX/RMI/`java.library.path`/`user.home`... y `-Xmx/-Xms/-Xmn` (la memoria se elige aparte). Se valida al guardar (`instance-manager`), al comprobar (`instances_jvm_check`) y otra vez al lanzar (por si `instance.json` se edito a mano).

### Catalogo y fuentes

| Id | Banderas | Respaldo |
|---|---|---|
| `g1` | `-XX:+UseG1GC` | G1 es el recolector por defecto desde Java 9 (en Java 8 es Parallel); [Oracle `java` (JDK 21)](https://docs.oracle.com/en/java/javase/21/docs/specs/man/java.html) |
| `serial-gc` | `-XX:+UseSerialGC` | Recomendado para aplicaciones pequenas; es lo que la JVM elige sola con <2 CPU o <2 GB (ergonomia de HotSpot). Oracle `java` |
| `zgc` | `-XX:+UseZGC` (+ `-XX:+ZGenerational` en Java 21-22) | [JEP 439](https://openjdk.org/jeps/439): en JDK 21 `-XX:+UseZGC` elige el modo no generacional, hace falta `ZGenerational`; JEP 474/490 lo hacen defecto y retiran la bandera. Solo Java 21+ y 8 GB; experimental |
| `parallel-ref` | `-XX:+ParallelRefProcEnabled` | Oracle `java`; [Aikar/PaperMC](https://docs.papermc.io/paper/aikars-flags) |
| `no-explicit-gc` | `-XX:+DisableExplicitGC` | Oracle `java` ("la JVM sigue haciendo GC cuando hace falta"); Aikar/PaperMC |
| `no-perfdata` | `-XX:-UsePerfData` | Oracle `java`: suprime los directorios `hsperfdata_*` (menos E/S al arrancar) |
| `g1-young` | `-XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1ReservePercent=20` (+ `-XX:G1HeapRegionSize=8M` con heap >= 3 GB) | Aikar/PaperMC: Minecraft asigna muchisima memoria temporal, asi que se agranda la generacion joven (por defecto 5 %). `G1NewSizePercent` es experimental segun Oracle |
| `string-dedup` | `-XX:+UseStringDeduplication` | Oracle `java`: solo con G1; ahorra RAM compartiendo cadenas |
| `pretouch` | `-XX:+AlwaysPreTouch` | Oracle `java` / Aikar: toca todo el heap al arrancar (arranque mas lento, usa toda la RAM asignada) |
| `code-cache` | `-XX:ReservedCodeCacheSize=512m` | Oracle `java`: 240 MB por defecto; util en modpacks grandes |

Notas de criterio:

- `MaxGCPauseMillis` ya no se fuerza en los niveles: Oracle documenta 200 ms por defecto y Aikar avisa de que valores mas bajos pueden hacer que G1 se retrase y caiga en recolecciones completas. Se puede poner como argumento propio.
- Las guias comunitarias (p. ej. modpack wikis) piden no copiar banderas a ciegas: por eso el nivel «Normal» no toca nada y los perfiles son cortos.
- `-Xshare:auto` (se quito del nivel Modesto) es ya el valor por defecto.

### Comprobacion real

`crates/jvm-tuning/src/java_tests.rs` ejecuta **cada ajuste contra Javas reales** (`DISORDER_TEST_JAVAS`, `;`-separadas; si no, `JAVA_HOME`) y lee el valor final con `-XX:+PrintFlagsFinal` (p. ej. `UseG1GC=true`, `G1NewSizePercent=30`, `ReservedCodeCacheSize=536870912`), demostrando que la bandera surte efecto y no solo que se acepta. Se verifico con Java 8, 17, 21 y 25.

## 2. Opciones graficas iniciales (`options.txt`)

Solo Equilibrado y Modesto. Se aplican una vez por nivel (marca `.disorder-optimization`); luego la persona puede cambiarlas en el juego.

## 3. Mods de rendimiento

Se instalan a peticion (`instances_optimize_mods`) desde Modrinth segun el loader: Sodium/Embeddium, Lithium, FerriteCore, Entity Culling, ModernFix, ImmediatelyFast; en Modesto tambien Dynamic FPS y MemoryLeakFix. Todos los slugs se comprobaron contra la API de Modrinth el 2026-10-20 (activos, 37-250 M de descargas; MemoryLeakFix solo se actualiza para versiones antiguas).

## Anadir un ajuste

1. Entrada nueva en `CATALOG` (`crates/jvm-tuning/src/catalog.rs`) con version minima/maxima, RAM minima, recolector que elige o exige y fuente.
2. Si procede, incluirla en un nivel (`profile_tweaks`).
3. Anadirla a `every_applicable_tweak_is_accepted_by_each_real_java` / `flags_take_effect_in_the_real_jvm` y ejecutar las pruebas con los Java que se quieran soportar. La interfaz la muestra sola (la lista sale de `instances_jvm_check`).

## Medicion en juego real (2026-10-20)

Equipo: i3-8130U (4 hilos), ~10 GB de RAM, Java 21, Fabulously Optimized 1.21.1 (heap 2605 MB), arranque en frio de cada partida, 2 rondas alternadas, 25 s de espera tras el menu. Se pidieron las banderas por la app real y se comprobo la linea de comandos del proceso Java.

| Nivel | Lanzar → motor de sonido | Memoria (working set) |
|---|---|---|
| Alto rendimiento + `string-dedup` | 35 s / 32 s | 1447 / 1444 MB |
| Normal (sin banderas) | 38 s / 34 s | 1485 / 1480 MB |
| Equilibrado | 31 s | 1472 MB |

Lectura honesta: en este equipo y este modpack la diferencia es pequena (unos 2-4 s de arranque y ~40 MB menos de memoria con `string-dedup`); el CPU acumulado es igual. La prueba mide arranque y memoria, **no** fotogramas por segundo; los mayores beneficios en juego vienen de los mods de rendimiento (apartado 3) y de que las banderas se apliquen con garantia (apartado 1), no de la JVM sola. Las repeticiones son pocas: tomar los numeros como indicativos.

Cambio de comportamiento a tener en cuenta: antes «Modesto» usaba siempre el recolector simple; ahora lo usa solo con 2 hilos o menos o menos de 4 GB de RAM, y en el resto G1 (el simple recoge en un solo hilo y en Minecraft, con mucha asignacion de memoria, alarga las pausas en equipos de 4 hilos o mas).
