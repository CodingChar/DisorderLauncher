# Tutorial inicial y skins — 19 de octubre de 2026

## Implementación
- Tutorial de seis pasos, foco modal, siguiente/anterior, saltar, Escape y repetición desde Ayuda. Se recuerda por instalación/origen con localStorage; si no está disponible se recuerda durante la sesión.
- Resaltado estático con CSS y coordenadas reactivas de la misma rejilla del menú. Sin temporizadores, consultas de geometría ni bucles de animación nuevos.
- Personaje a la derecha, nombre encima y acceso Skins debajo. Sin skin local asignada se muestra un maniquí identificado; no se consulta la skin remota de Microsoft.
- Reutilización de SkinPreview3D y caché de PNG existente. Editor cargado bajo demanda.
- Creación guiada por nombre/modelo, selector visible de parte corporal y ayuda de pintura. Mensajes de guardado indican cómo asignar la skin.
- Apariencia Minecraft fija; se ignoran selecciones antiguas de temas y se normalizan al guardar, conservando Java y descargas. El backend de temas se mantiene por compatibilidad, sin controles de selección ni carga de catálogos en el frontend.

## Verificación
- Svelte/TypeScript: sin errores ni advertencias.
- Vitest: 107 pruebas en 18 archivos; incluye persistencia del tutorial, almacenamiento no disponible y ajustes antiguos con tema personalizado.
- Build Vite de producción correcto.
- Navegador con backend simulado: crear perfil, avanzar los seis pasos, terminar, repetir desde Ayuda, saltar, recuperar controles, crear skin, pintar con teclado, guardar, asignar al perfil y verla en el menú. Ajustes sin selección de temas.
- Captura: review-assets/tutorial-2026-10-19.png.

## Límites
No se ejecutó el binario Tauri ni se midieron FPS, GPU o memoria en hardware de gama baja. Las pruebas interactivas usan datos simulados; no verifican la aplicación de la skin dentro de Minecraft ni autenticación Microsoft. El tutorial se muestra después de crear el primer perfil; instalaciones existentes sin marcador también reciben el paseo una vez.

## Segunda revisión visual completa
Se revisaron capturas a 1280 × 720 y con rejilla virtual forzada temporalmente a 320 × 240. El cambio temporal de tamaño fue retirado después de las pruebas.

| Flujo | Resultado observado |
| --- | --- |
| Primer inicio | El paseo aparece después de crear el perfil. |
| Seis pasos | Objetivos visibles; Siguiente, Anterior y Terminar funcionan. |
| Repetir / saltar / Escape | Se recuperan los controles; el marcador persiste tras recargar el navegador y recrear el perfil simulado. |
| Foco modal | Tab desde Siguiente vuelve a Saltar; el fondo queda inerte. |
| Menú | Nombre encima, personaje a la derecha, Skins debajo; skin asignada visible con el píxel pintado. |
| Asistente | Nombre vacío desactiva continuar; modelos clásico y fino visibles; Cancelar y Escape permiten volver. Escape desde un campo primero retira su foco. |
| Editor | Selección de Cabeza atenúa otras partes, lápiz con ratón y teclado, Deshacer/Rehacer, controles de capa/modelo/espejo/cuadrícula y giro del personaje revisados. |
| Ayuda del editor | Texto legible, cierre por botón y Escape, sin abrir otro diálogo de salida. |
| Guardado | Aviso modal; volver, asignar y regresar al menú muestra el resultado. La exportación se revisó con respuesta simulada. |
| Biblioteca | Importación simulada, renombrado, quitar asignación y cancelar la eliminación por Escape funcionan; el nombre actualizado aparece en el diálogo. |
| Ajustes | Apariencia fija y guardado con confirmación visible; sin selector de temas. |

Correcciones encontradas en esta revisión:
- Escape se consume en el propio diálogo antes de alcanzar la navegación global, evitando que una transición de subpantalla intercepte el cierre de la ayuda.
- El paso Ajustes resalta solamente ese botón, sin incluir Salir.
- Escape desde Skins vuelve al menú, igual que su botón Atrás.
- Al cambiar capa o modelo se reposiciona el cursor del teclado en una cara válida. Reprobado desde una carga completa, pintando la capa exterior con Espacio.
- El primer paso del asistente cancela correctamente por Escape aunque se haya cambiado el nombre.
- Los avisos de error/guardado se renderizan también fuera de la vista principal del editor.

El resaltado continúa siendo CSS estático. No se añadieron sondeos ni ciclos de animación; el editor conserva su carga bajo demanda. Evidencias: `review-assets/menu-verified-2026-10-19.png`, `review-assets/editor-min-verified-2026-10-19.png` y `review-assets/tutorial-settings-verified-2026-10-19.png`.

Validación final del frontend: `npm run check` sin errores ni advertencias, 107 pruebas correctas en 18 archivos y compilación Vite correcta. Paquete principal: 209,04 kB (73,39 kB gzip); editor separado: 15,18 kB (6,24 kB gzip). Estos tamaños no constituyen una medición de FPS o memoria.

Validación nativa: `cargo test --workspace` completado correctamente en Windows, incluyendo la compilación de Tauri y las pruebas del backend de skins. Se respetaron las pruebas marcadas `ignored` (integraciones con internet/Minecraft y mediciones de rendimiento). Compilar y ejecutar pruebas no equivale a probar visualmente la ventana Tauri.

## Optimización adicional de texturas
- Las vistas proporcionan el alto validado del PNG a la vista 3D. Para 64 × 64 se usa directamente la textura y se evita crear/decodificar un `Image` adicional en cada actualización del editor. Para 64 × 32 y dimensiones desconocidas permanece la conversión existente; las pruebas de conversión de skins siguen pasando.
- Guardar reutiliza el PNG de la vista previa. Antes de capturarlo se termina el trazo pendiente, que actualiza esa vista; se evita una segunda codificación idéntica.
- Una lectura de miniatura invalidada mientras espera turno se descarta antes de llamar al backend, incluso cuando la versión de sus metadatos no cambió. Una prueba de regresión cubre la cola saturada y su reemplazo.
- La captura del giro 3D admite solamente el puntero primario con botón izquierdo y se limpia al perder la captura.
- Validación: 108 pruebas frontend correctas, comprobación Svelte/TypeScript sin errores ni advertencias, build correcto y revisión visual de rellenar la cara, guardar, reabrir y girar el personaje con el ratón. Evidencia: `review-assets/skin-optimized-2026-10-19.png`. No se repitió Rust porque estos cambios afectan exclusivamente al frontend. No se midieron FPS.
