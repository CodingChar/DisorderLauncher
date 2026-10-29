# 06 — Instancias y almacenamiento

## 1. Directorio

```text
instances/<instance-id>/
├── instance.json
├── game/
│   ├── mods/
│   ├── config/
│   ├── saves/
│   ├── resourcepacks/
│   └── shaderpacks/
└── logs/
```

## 2. instance.json

```json
{
  "schemaVersion": 1,
  "id": "inst_...",
  "name": "Fabric Performance",
  "minecraftVersion": "1.x",
  "loader": {
    "kind": "fabric",
    "version": "..."
  },
  "profileId": "acc_local_...",
  "java": {"mode": "auto"},
  "memory": {"mode": "auto"}
}
```

## 3. Separación identidad/instancia

Implicación crítica:

Una instancia no debe cambiar internamente cuando el usuario pase de perfil local a Microsoft.

Solo cambia:

```text
instance.profile_id
```

o la selección temporal de perfil para ese launch.

## 4. Export

No exportar:
- tokens;
- secure store keys reutilizables;
- información privada no necesaria.

## 5. Migraciones

Cada archivo tiene `schemaVersion`.

Crear migradores deterministas.

Nunca mutar datos sin backup temporal.

## 6. Default profile

Guardar a nivel de launcher:

```json
{
  "defaultProfileId": "acc_local_..."
}
```

Una instancia puede sobrescribirlo.

## 7. Portable instances

Implementado (ver `docs/adr/ADR-003_artifact_store_and_dpack.md`):
- `.7z` estandar con `dpack-manifest.json` (version de formato, Minecraft, loader, archivos con SHA-512 y origen);
- mods/configs/resourcepacks/shaderpacks/kubejs; mundos solo si el usuario lo pide;
- sin cuentas, tokens, logs ni rutas absolutas.

Nunca empacar binarios de Minecraft como si fueran contenido propio.

## 8. Almacen comun de mods

`<datos>/store/sha512/<2 hex>/<hash>` guarda una sola vez los `.jar`/`.zip` inmutables de `mods/`, `resourcepacks/` y `shaderpacks/`. Cada instancia los ve como archivos normales (hardlink, o copia si el volumen no lo permite) y anota lo que usa en `game/.disorder/artifacts.json`. El estado mutable (`config/`, `saves/`, `options.txt`, logs) nunca se comparte. La limpieza solo borra objetos que ninguna instancia usa, y solo con todas las instancias reservadas (sin juego, instalaciones ni importaciones en curso).
