# 10 — Skins

## Separación conceptual

Tres conceptos:

1. `LocalSkinAsset`
2. skin asociada a perfil local
3. skin oficial de perfil Microsoft

Nunca fusionarlos.

## Fase 1

- importar PNG;
- validar;
- biblioteca;
- preview;
- asignar a perfil local.

## Fase 2

- preview 3D;
- modelos classic/slim;
- historial.

## Fase 3

Editor integrado:
- pixel pencil;
- eraser;
- fill;
- picker;
- undo/redo;
- layers;
- mirror;
- zoom;
- 3D preview;
- save/export.

## Storage

```text
skins/
├── index.json
└── assets/
```

## Modelo

```rust
pub struct SkinAsset {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub model: SkinModel,
    pub source: SkinSource,
}
```

```rust
pub enum SkinSource {
    Local,
    MicrosoftProfile,
}
```

## Offline profile

`LocalAccountData.skin_id`.

No almacenar binario dentro de account JSON.

## Microsoft futuro

El provider Microsoft puede exponer metadata visual del perfil por una interfaz separada.

No hacer que `IdentityProvider` se convierta en editor de skins.

## Render

Para 3D:
- renderer pequeño;
- no engine masivo.

## Skin del perfil Offline dentro del juego

El juego sin conexion no pide la skin a Mojang: `launcher-core/src/skin_pack.rs` genera
`DisorderLauncher-skin.zip` (resource pack que sustituye las texturas de Steve/Alex y de los
siete skins por defecto de 1.21.2+) y lo activa en `options.txt` en cada lanzamiento.

Por que una skin deja de cargar (todo cubierto y con test):

- **`pack.mcmeta` rechazado.** Hasta 1.21.8 el pack lleva `pack_format`. Desde 1.21.9 (25w31a) y
  en 26.x el juego exige `min_format`/`max_format` y descarta el pack si declara
  `supported_formats` por encima del formato 64. Un pack descartado se borra de `options.txt`.
  Cada epoca declara solo lo suyo (`PackFormat::Single` / `PackFormat::Ranged`).
- **Pack en `incompatibleResourcePacks`.** Si `options.txt` lista ahi el pack y el juego lo ve
  compatible, lo saca de la lista y NO lo activa en ese arranque (visto en 26.3). El launcher no lo
  anota y limpia la entrada si una version anterior la dejo.
- **Version no reconocida** (`26.x`, snapshots, ids propios): se resuelve por la fecha de publicacion
  y, sin ella, se trata como la version mas reciente.
- **Carpeta, prefijo y textura por epoca**: ver la tabla del modulo (`texturepacks/` + `skin:` hasta
  1.5, `resourcepacks/` sin prefijo hasta 1.12, `file/` desde 1.13, textura 64x32 antes de 1.8).

Relacionado, en el arranque de Fabric 26.x: el classpath se pasa con rutas canonicas
(`minecraft-launch/src/plan.rs::normalized`). Si `AppData` esta tras una redireccion (apps MSIX,
enlaces, `subst`), Fabric Loader no reconoce sus propios jars y todos los mods caen con
`ClassCastException` entre `knot` y `app`.

## Seguridad

Validar:
- MIME real;
- PNG decodificable;
- dimensiones;
- tamaño;
- alpha;
- rutas.
