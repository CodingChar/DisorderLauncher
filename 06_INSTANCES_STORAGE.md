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

Futuro:
- manifest;
- mods/configs;
- saves opcionales.

Nunca empacar binarios de Minecraft como si fueran contenido propio.
