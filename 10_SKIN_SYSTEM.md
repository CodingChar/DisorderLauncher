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

## Seguridad

Validar:
- MIME real;
- PNG decodificable;
- dimensiones;
- tamaño;
- alpha;
- rutas.
