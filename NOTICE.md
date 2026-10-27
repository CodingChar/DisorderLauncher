# Avisos y atribuciones

El **código** de DisorderLauncher se distribuye bajo la licencia MIT (ver [`LICENSE`](LICENSE)).

La licencia MIT **no cubre** los materiales de terceros de esta lista. Siguen siendo propiedad de sus titulares y se usan solo para dar a la interfaz el aspecto de Minecraft.

## Material de Mojang Studios / Microsoft

DisorderLauncher es un proyecto independiente. **No está afiliado, respaldado ni patrocinado por Mojang Studios ni por Microsoft.** «Minecraft» es una marca de Mojang Studios.

| Ruta | Contenido |
|---|---|
| `apps/desktop/public/assets/` (`ascii.png`, `button*.png`, `widgets.png`, `options_background.png`, `world_selection.png`, `unknown_server.png`, `splashes.txt`) | Texturas, fuente bitmap y textos de la interfaz de Minecraft |
| `apps/desktop/public/assets/panorama/` | Panorama del menú principal |
| `apps/desktop/public/assets/sounds/`, `launcher_music/` | Sonidos de clic y de volver |
| `minecraft_iconos_emblematicos/` | Iconos de objetos de Minecraft (origen: [destruc7i0n/minecraft-textures](https://github.com/destruc7i0n/minecraft-textures), texturas de la 1.16) |
| `landing/assets/` | Copias de varios de los archivos anteriores y capturas de pantalla del launcher |

Si eres titular de alguno de estos derechos y quieres que se retire, abre una incidencia en el repositorio y se eliminará.

## Dependencias

Entre ellas, [`sevenz-rust2`](https://github.com/hasenbanck/sevenz-rust2) (Apache-2.0) para leer y escribir paquetes `.7z`. Las dependencias de Rust (`Cargo.lock`) y de npm (`apps/desktop/package-lock.json`) conservan sus propias licencias. Los juegos y loaders que descarga el launcher (Minecraft, Fabric, Quilt, Forge, NeoForge, Modrinth, Adoptium) pertenecen a sus autores y se obtienen desde sus servidores oficiales.
