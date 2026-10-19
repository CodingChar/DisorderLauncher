# Apertura de apartados — 2026-10-18

## Problemas y correcciones

1. **Trabajo sincronico al construir cada pantalla.** Cada boton montaba entre dos y cuatro lienzos de texto para sus estados, incluso los invisibles. Ahora monta uno y actualiza su color al pasar el puntero, enfocar con teclado, activar o deshabilitar. Se conservan los eventos extra de cada control.
2. **Fuente bitmap costosa en frio.** Por cada etiqueta nueva se creaban cuatro lienzos y se componian mascaras y tintes. Ahora se comparte un atlas por color y sombra, con un maximo de 32 atlas y el cache de 600 etiquetas existente. No se cambia la textura, las medidas ni la sombra.
3. **Logo calculado de nuevo al volver al menu.** Se reutiliza el raster para cada tamano de bloque, hasta cuatro tamanos, manteniendo el ajuste a la ventana.
4. **Versiones consultadas al abrir y reabrir formularios.** Cache independiente por loader de 60 segundos, peticion compartida cuando coinciden precarga y apertura, y renovacion en segundo plano con datos anteriores visibles. Las acciones explicitas de actualizar fuerzan una consulta. Una respuesta tardia de otro loader no sustituye el formulario actual.
5. **Skins esperaba todos los PNG.** La lista se considera cargada al recibir sus metadatos. Solo se leen las caras de perfiles y miniaturas visibles/seleccionadas, con cuatro lecturas simultaneas. Las peticiones del mismo PNG se comparten. Los cambios de version o borrados invalidan miniaturas y respuestas antiguas. Metadatos compartidos durante cinco segundos, con renovacion forzada tras mutaciones. No se sustituyen las lecturas de documentos al abrir el editor por un PNG potencialmente desactualizado.
6. **Mods se vaciaba cada vez que se salia.** Cache acotado a 16 instancias, mostrado inmediatamente al volver y siempre revalidado contra disco. Las mutaciones invalidan el cache; las respuestas de una pantalla cerrada no sustituyen la instancia actual.
7. **Recursos preparados en el clic.** Texturas y modulo del editor se preparan en tiempo ocioso despues del arranque. Versiones y skins se anticipan tambien al enfocar/acercarse a controles relevantes. La navegacion nunca espera estas precargas, y sus fallos no impiden abrir la pantalla.
8. **Configuracion nativa.** Se retiran `--renderer-process-limit=1` y `--in-process-gpu`, para no forzar ese modelo de procesos. Se conservan el resto de ajustes y los contratos IPC. Microsoft documenta el [modelo normal de procesos de WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model) y desaconseja distribuir [flags experimentales de WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags). No se atribuye a este cambio una mejora cuantificada por separado.

## Medicion

Prueba en navegador local, alternando el orden de los renderizadores antiguo y nuevo, con 400 etiquetas diferentes por tanda y cuatro tandas por implementacion:

| Medida | Antes | Despues |
|---|---:|---:|
| Mediana del dibujo de 400 etiquetas | 543.9 ms | 168.85 ms |
| Lienzos creados por tanda | 1600 | 400 (402 con dos atlas nuevos) |
| Lienzos en Crear instancia | 26 | 14 |
| Lienzos en Mods | 28 | 12 |
| Lienzos en el editor | 32 | 19 |

La mediana del trabajo de texto baja un **69 %** (aproximadamente **3.2 veces** mas rapido en esta prueba). No representa una mejora equivalente de toda la aplicacion. Los tiempos de navegacion se muestrearon hasta dos fotogramas despues de montar la pantalla, con espera adicional al modulo del editor; dependen del planificador, Vite, compilaciones simultaneas y del equipo, por lo que no se publica un porcentaje global a partir de ellos.

Datos: [texto](review-assets/text-render-benchmark.json), [navegacion anterior](review-assets/navigation-before.json), [navegacion posterior](review-assets/navigation-after.json). Los pixeles de 16 combinaciones de texto/color y del logo coinciden con la referencia anterior (ancho, alto y hash de RGBA).

## Validacion

- `npm run check`: sin errores ni advertencias.
- `npm test`: 102 pruebas correctas, incluidas respuestas fuera de orden, errores de red, invalidacion tras editar/borrar y limites de concurrencia.
- `npm run build`: correcto.
- Matriz de pantallas y dialogos en navegador: sin incidencias; carga diferida del editor y miniaturas incluidas.
- `npm run tauri -- build --no-bundle`: correcto. El `.exe` de `target/release` era de las 15:27 del mismo dia, anterior a las dos revisiones anteriores; el nuevo se genero a las 16:38. Se comprobaron la ausencia de los dos flags retirados en el binario y su arranque hasta el menu nativo accesible de WebView2. La comprobacion adicional mediante `computer-use` se detuvo cuando el usuario pulso Escape; no se afirma una medicion completa de navegacion nativa.
- Binario: `target/release/disorderlauncher-desktop.exe`, 21772800 bytes. SHA-256: `E63C32110BA2CEC5F0BC2DD552A6ED367922CE6D37446F320D37113986EEB99A`.
- Se eliminaron las paginas y el renderizador de referencia temporales utilizados en las mediciones.

Las pruebas de navegador usan el backend simulado. La carga de datos remotos ausentes sigue necesitando una conexion y no se reemplaza por datos inventados. Las lecturas, validaciones, instalacion y lanzamiento del juego siguen delegadas al backend.
