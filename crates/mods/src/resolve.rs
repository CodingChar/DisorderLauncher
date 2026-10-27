use crate::{ModError, ModVersion, ModrinthClient};
use std::collections::HashSet;

/// Fuente de versiones compatibles con la instancia (version de Minecraft + loader).
#[async_trait::async_trait]
pub trait VersionSource: Send + Sync {
    async fn compatible_versions(&self, project_id: &str) -> Result<Vec<ModVersion>, ModError>;
    async fn version_by_id(&self, version_id: &str) -> Result<ModVersion, ModError>;
    async fn project_title(&self, project_id: &str) -> Result<String, ModError>;
}

/// `ModrinthClient` ya filtrado por la version y el loader de una instancia.
pub struct ModrinthSource {
    pub client: ModrinthClient,
    pub minecraft_version: String,
    pub loader: String,
}

#[async_trait::async_trait]
impl VersionSource for ModrinthSource {
    async fn compatible_versions(&self, project_id: &str) -> Result<Vec<ModVersion>, ModError> {
        self.client.project_versions(project_id, &self.minecraft_version, &self.loader).await
    }

    async fn version_by_id(&self, version_id: &str) -> Result<ModVersion, ModError> {
        self.client.version(version_id).await
    }

    async fn project_title(&self, project_id: &str) -> Result<String, ModError> {
        self.client.project_title(project_id).await
    }
}

#[derive(Debug, Clone)]
pub struct PlannedMod {
    pub project_id: String,
    pub title: String,
    pub version: ModVersion,
    /// Proyecto que lo pidio como dependencia; `None` para el mod elegido por el usuario.
    pub dependency_of: Option<String>,
}

/// Prefiere la ultima version estable; solo recurre a beta y luego alpha si no hay otra.
pub fn pick_best_version(versions: &[ModVersion]) -> Option<&ModVersion> {
    ["release", "beta", "alpha"].iter().find_map(|kind| {
        versions
            .iter()
            .filter(|v| v.version_type == *kind && v.primary_file().is_some())
            .max_by_key(|v| v.date_published)
    })
}

/// Nombres con los que Modrinth publica un mod que otros piden por su id interno. Primero los alias
/// conocidos (el id de un mod no siempre coincide con su proyecto), despues el id tal cual y con `_` -> `-`.
pub fn modrinth_candidates(mod_id: &str) -> Vec<String> {
    let alias = match mod_id {
        "fabric" | "fabric-api" | "fabric_api" => Some("fabric-api"),
        "cloth-config" | "cloth-config2" | "cloth_config" | "cloth_config2" => Some("cloth-config"),
        "yet_another_config_lib_v3" => Some("yacl"),
        "modmenu" => Some("modmenu"),
        "quilted_fabric_api" => Some("qsl"),
        _ => None,
    };
    let mut out: Vec<String> = alias.into_iter().map(str::to_string).collect();
    for candidate in [mod_id.to_string(), mod_id.replace('_', "-")] {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

/// Resultado de buscar en Modrinth lo que a una instancia le falta: que instalar y que no se encontro.
#[derive(Debug, Default)]
pub struct MissingPlan {
    pub plan: Vec<PlannedMod>,
    /// Ids que no existen en Modrinth para esta version de Minecraft y loader.
    pub unresolved: Vec<String>,
}

/// Planifica la instalacion de varios mods que faltan (por su id) compartiendo lo ya decidido: una dependencia
/// comun a dos de ellos se instala una sola vez. No escribe nada.
pub async fn plan_missing(
    source: &dyn VersionSource,
    mod_ids: &[String],
    installed_projects: &HashSet<String>,
) -> MissingPlan {
    let mut out = MissingPlan::default();
    let mut tracked = installed_projects.clone();
    'ids: for mod_id in mod_ids {
        for candidate in modrinth_candidates(mod_id) {
            if let Ok(found) = plan_install(source, &candidate, &tracked).await {
                for item in found {
                    tracked.insert(item.project_id.clone());
                    out.plan.push(item);
                }
                continue 'ids;
            }
        }
        out.unresolved.push(mod_id.clone());
    }
    out
}

/// Calcula que instalar para `project_id` y sus dependencias obligatorias, sin escribir nada.
/// El resultado va en orden de instalacion: dependencias primero, el mod pedido al final.
pub async fn plan_install(
    source: &dyn VersionSource,
    project_id: &str,
    installed_projects: &HashSet<String>,
) -> Result<Vec<PlannedMod>, ModError> {
    let mut plan = Vec::new();
    let mut seen: HashSet<String> = installed_projects.clone();
    visit(source, project_id, None, &mut seen, &mut plan, true).await?;
    Ok(plan)
}

fn visit<'a>(
    source: &'a dyn VersionSource,
    project_id: &'a str,
    parent: Option<&'a str>,
    seen: &'a mut HashSet<String>,
    plan: &'a mut Vec<PlannedMod>,
    is_root: bool,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ModError>> + Send + 'a>> {
    Box::pin(async move {
        if !seen.insert(project_id.to_string()) {
            return Ok(());
        }
        let versions = source.compatible_versions(project_id).await?;
        let Some(best) = pick_best_version(&versions).cloned() else {
            let title = source.project_title(project_id).await.unwrap_or_else(|_| project_id.to_string());
            return Err(if is_root { ModError::NotCompatible(title) } else { ModError::MissingDependency(title) });
        };
        let title = source.project_title(project_id).await.unwrap_or_else(|_| project_id.to_string());

        resolve_dependencies(source, &best, project_id, seen, plan).await?;
        plan.push(PlannedMod {
            project_id: project_id.to_string(),
            title,
            version: best,
            dependency_of: parent.map(str::to_string),
        });
        Ok(())
    })
}

async fn resolve_dependencies(
    source: &dyn VersionSource,
    version: &ModVersion,
    owner: &str,
    seen: &mut HashSet<String>,
    plan: &mut Vec<PlannedMod>,
) -> Result<(), ModError> {
    for dep in version.required_dependencies() {
        match (&dep.project_id, &dep.version_id) {
            (Some(project), _) => visit(source, project, Some(owner), seen, plan, false).await?,
            (None, Some(version_id)) => {
                // Dependencia fijada a una version concreta.
                let pinned = source.version_by_id(version_id).await?;
                if !seen.insert(pinned.project_id.clone()) {
                    continue;
                }
                let title = source
                    .project_title(&pinned.project_id)
                    .await
                    .unwrap_or_else(|_| pinned.project_id.clone());
                Box::pin(resolve_dependencies(source, &pinned, &pinned.project_id.clone(), seen, plan)).await?;
                plan.push(PlannedMod {
                    project_id: pinned.project_id.clone(),
                    title,
                    version: pinned,
                    dependency_of: Some(owner.to_string()),
                });
            }
            (None, None) => {}
        }
    }
    Ok(())
}
