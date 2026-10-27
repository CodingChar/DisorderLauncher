# ADR-003 — Almacen comun de artefactos y paquetes de instancia `.7z`

## Contexto

Cada instancia guardaba copias de sus mods; duplicar una instancia las copiaba otra vez, el mismo mod se descargaba una vez por instancia y la unica exportacion (`.mrpack`) no podia llevar mods privados o manuales con garantias. El documento de diseno original esta en `ideas/DisorderLauncher_Arquitectura_Integracion_Mods_y_Exportacion.md`.

## Decision

1. **Almacen por contenido** (`crates/artifact-store`). Los archivos inmutables (`.jar`/`.zip` de `mods/`, `resourcepacks/`, `shaderpacks/`) viven una vez en `<datos>/store/sha512/<2 hex>/<128 hex>`. Identidad = SHA-512 del contenido; el origen (Modrinth, local) es metadata aparte. `config/`, `saves/`, `options.txt` y logs son estado de la instancia y nunca se comparten.
2. **La instancia sigue siendo una carpeta normal.** Minecraft y los loaders ven `mods/x.jar` como un archivo corriente: se *materializa* como hardlink al objeto y, si el volumen o el sistema de archivos no lo permiten, como copia. Toda escritura es temporal + rename, de modo que un hardlink nunca se edita en sitio.
3. **Quien usa que** se guarda por instancia en `game/.disorder/artifacts.json` (ruta, SHA-512, tamano, fecha, origen). No hay base de datos ni contador de referencias central: la limpieza es *mark & sweep* leyendo esos registros, reservando todas las instancias mientras limpia (sin instalaciones ni importaciones en curso) y borrando solo temporales abandonados de mas de 24 h; se niega a borrar nada si algun registro es ilegible.
4. **Adopcion perezosa** de lo ya existente: al lanzar, al anadir un jar local y al exportar se incorporan los archivos nuevos o cambiados (cache por ruta+tamano+fecha, hash en streaming con hasta 4 hilos). No hay migracion masiva al arrancar.
5. **Instalar un mod** (`ModsService::fetch`): si Modrinth declara un SHA-512 y el almacen lo tiene, no se descarga; si no, se descarga al almacen, se comprueba SHA-1 (como antes) y SHA-512 y se enlaza. Un SHA-512 que no coincide se rechaza sin dejar rastro.
6. **Paquete portable** (`crates/dpack`): `.7z` estandar con `dpack-manifest.json` (version de formato, pack, Minecraft, loader, y por archivo ruta/tamano/SHA-512/origen). Autocontenido: los mods viajan dentro. No incluye cuentas, tokens, logs, capturas, `options.txt` ni `servers.dat`; los mundos solo si el usuario lo pide.
7. **Importacion segura y transaccional.** El manifiesto se valida antes de extraer (rutas relativas sin `..`, raiz, unidades, `\`, `:`, nombres reservados de Windows ni `.disorder`; sin duplicados insensibles a mayusculas; topes de entradas y bytes). La extraccion cuenta los bytes reales, verifica tamano y SHA-512 de cada archivo contra el manifiesto, rechaza enlaces simbolicos e ignora lo que el manifiesto no lista. Lo que el almacen ya tiene no se escribe. Mismo esqueleto que la importacion `.mrpack` (instancia reservada, `import.pending`, rollback).

## Desviaciones respecto al documento de ideas (§51)

| Documento | Aqui | Por que |
|---|---|---|
| Registro logico de artefactos | Sidecar por instancia, sin BD | Menos estado, a prueba de cierres inesperados, sin dependencias nuevas |
| 7z con LZMA2 solido | Manifiesto y texto en bloques solidos LZMA2; `.jar`/`.zip`/imagenes con el coder COPY | Los jars ya estan comprimidos: recomprimirlos gasta CPU sin reducir tamano |
| hardlink / reflink / copy | hardlink / copy | Reflink anade una dependencia y rutas imposibles de probar en NTFS; queda como mejora futura |
| Refcount (§24) | Lectura de registros bajo reserva exclusiva | Evita carreras con instalaciones en curso sin un contador que pueda desincronizarse. Una gracia por fecha no sirve: la fecha de un objeto es la de su archivo, que puede ser antigua |

## Consecuencias

- Ahorro real de disco y de descargas cuando varias instancias comparten mods; duplicar una instancia es casi instantaneo.
- Un hardlink comparte inodo: editar un jar *en sitio* alteraria el almacen. Los jar no se editan; ante la duda, `ArtifactStore::verify` recalcula el hash.
- `sevenz-rust2` (Apache-2.0, Rust puro, sin caracteristicas de cifrado ni bzip2) es una dependencia nueva; la extraccion no confia en ella para la seguridad: las defensas de ruta, tamano, enlaces y hash son propias.
- "Ahorrado" en la pantalla de almacenamiento es lo que ocuparian las copias menos lo que ocupan los objetos distintos; si el almacen y la instancia estuvieran en volumenes distintos (copia en vez de hardlink) el ahorro real seria menor.
