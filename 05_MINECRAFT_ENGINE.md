# 05 — Minecraft Engine

## 1. Responsabilidad

Instalar, verificar y lanzar Minecraft independientemente del proveedor de identidad.

## 2. Version Resolver

```text
manifest
 -> VersionEntry
 -> version JSON
 -> normalized VersionDescriptor
```

No almacenar reglas por versión salvo excepciones justificadas.

## 3. InstallPlan

```rust
pub struct InstallPlan {
    pub version: VersionDescriptor,
    pub client: Artifact,
    pub libraries: Vec<LibraryArtifact>,
    pub assets: Vec<AssetArtifact>,
    pub natives: Vec<NativeArtifact>,
    pub java: JavaRequirement,
}
```

Construir plan completo antes de comenzar la escritura principal.

## 4. Artifact

```rust
pub struct Artifact {
    pub url: Url,
    pub path: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
}
```

## 5. Instalación

Fases:
1. fetch metadata;
2. validate;
3. allocate directories;
4. client;
5. libraries;
6. asset index;
7. assets;
8. natives;
9. Java;
10. persist descriptor;
11. final verification.

## 6. Caché

Compartir:
- assets;
- libraries;
- runtimes.

No compartir:
- mods;
- configs;
- saves;
- options de instancia.

## 7. LaunchPlan

```rust
pub struct LaunchPlan {
    pub java: PathBuf,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_directory: PathBuf,
}
```

`LaunchPlanBuilder` recibe `ResolvedIdentity`.

## 8. Placeholder mapping

La sustitución de variables de autenticación debe estar centralizada.

No hacer sustituciones ad hoc en UI o loaders.

## 9. Java

`JavaManager`:
- lee requerimiento;
- detecta instalado;
- selecciona compatible;
- puede descargar runtime administrado;
- permite override avanzado.

## 10. Repair

Recalcular integridad y redescargar solo faltantes/corruptos.

## 11. Offline network state

Antes de Play:
- si identidad local + instalación completa: permitir launch sin red;
- si faltan artefactos: explicar qué debe descargarse;
- no intentar descargas infinitamente.

## 12. ProcessRunner

- spawn directo;
- nada de concatenar shell;
- capturar stdout/stderr;
- exit code;
- kill controlado;
- no heredar variables sensibles innecesarias.
