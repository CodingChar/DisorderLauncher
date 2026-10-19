# Rendimiento de la interfaz y splash — 2026-10-18

## Cambios

- **Splash amarillo**: conserva fuente bitmap, sombra, amarillo `#ffff00`, giro de -20 grados y escala `(1.8 - abs(sin(t * 2pi) * .1)) * 100 / (ancho + 32)`, con dos pulsos por segundo. El raster de la frase se reutiliza y solo se repinta la zona que ocupa durante el pulso. Los limites incluyen giro y sombra, y evitan recortes junto al borde derecho.
- **Animaciones decorativas**: helper compartido con limite de dibujo de 60 Hz para el splash y 30 Hz para el panorama. Se detienen al ocultar la ventana, siguen la preferencia de movimiento reducido y eliminan sus suscripciones al desmontarse.
- **Panorama**: calcula la distancia de la camara al redimensionar, en lugar de consultar las dimensiones en cada fotograma. Se ocultan las caras posteriores del cubo.
- **Texto de controles**: cambiar de color al pasar el puntero ya no recrea el buffer del canvas cuando sus dimensiones no cambian.
- **Instancias**: crear, editar, duplicar, importar y borrar actualizan la lista con la respuesta confirmada del backend. Se elimina una llamada IPC y el recorrido completo de las carpetas por accion cuando la biblioteca ya esta cargada. Una revision impide que una lectura antigua sobrescriba una mutacion reciente; la carga inicial y la recuperacion de errores siguen leyendo la biblioteca completa.
- **Consola**: sin filtros reutiliza la lista existente. El texto de busqueda se normaliza una vez por cambio, no una vez por cada linea recibida.
- **Arranque**: el editor de skins se importa al abrirlo, con estados de carga y error. Su chunk de JavaScript de 12.77 kB no se descarga ni evalua al abrir el menu.

## Evidencia

En la ventana de prueba de 663 x 579 CSS px, con GUI de 332 x 290 y factor 2, el splash ocupaba antes un buffer de 664 x 580 px. Con la frase mostrada durante la comprobacion, el buffer nuevo mide 200 x 122 px: 24400 frente a 385120 pixeles, un **93.7 % menos de superficie de repintado**. Es una medida de superficie, no una afirmacion de reduccion equivalente del tiempo total de CPU o de arranque. La cifra varia con la frase y el tamano de ventana.

## Validacion

- `npm run check`: sin errores ni advertencias.
- `npm test`: 89 pruebas correctas. Incluye geometria del splash a lo largo de todo el pulso, temporizacion, pausa, movimiento reducido, limpieza de animaciones y actualizaciones de instancias sin releer la biblioteca.
- `npm run build`: correcto, editor en un chunk separado.
- Matriz visual de pantallas y dialogos: 1100 x 720 con mas de 500 instancias, 820 x 560, 1920 x 1080 y escenario minimo de 320 x 240 px de GUI (ventana de 640 x 480), sin incidencias. Incluye la apertura del editor cargado bajo demanda. Se retiro la pagina temporal usada para accionar la matriz.
- Comprobacion visual en navegador: fuente pixelada y splash amarillo inclinado sobre el logo, sin errores en consola.

La matriz usa el backend simulado de desarrollo. No sustituye una medicion de CPU/GPU dentro de WebView2/Tauri ni un benchmark en distintos equipos.
