//! Integridad de instancias: lo que pasa cuando algo se borra o se corta desde fuera del launcher.
//!
//! La lista de instancias sale siempre del disco (`instances/<id>/instance.json`), asi que una instancia
//! borrada a mano deja de aparecer en cuanto se vuelve a leer. Aqui se cubre el resto:
//! - carpetas internas (`game/`, `logs/`) borradas: se recrean solas, antes de lanzar y en la revision;
//! - restos de un borrado incompleto (carpeta sin `instance.json`): se limpian;
//! - importaciones de modpack interrumpidas (el launcher se cerro a medias): se deshacen;
//! - perfil o Java elegido que ya no existen: se informan para que la persona elija otro (nunca se cambian solos).

use crate::{Launcher, LauncherError};
use account_storage::AccountRepository;
use instance_manager::{FolderProblemKind, Instance, InstanceRepository, JavaSelection};
use serde::Serialize;
use std::path::Path;

/// Marca dentro de la instancia mientras se importa un modpack. Si sigue ahi al arrancar, la importacion se corto.
const IMPORT_MARKER: &str = ".disorder/import.pending";

pub(crate) fn import_marker(game_dir: &Path) -> std::path::PathBuf {
    game_dir.join(IMPORT_MARKER)
}

pub(crate) async fn mark_import_pending(game_dir: &Path) -> std::io::Result<()> {
    let marker = import_marker(game_dir);
    if let Some(parent) = marker.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(marker, b"importando").await
}

pub(crate) async fn clear_import_marker(game_dir: &Path) {
    let _ = tokio::fs::remove_file(import_marker(game_dir)).await;
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IssueKind {
    /// El perfil de la instancia ya no existe: hay que elegir otro.
    ProfileMissing,
    /// El Java elegido a mano ya no esta en esa carpeta.
    JavaMissing,
    /// Faltaban carpetas internas y se recrearon.
    LayoutRepaired,
    /// Resto de un borrado incompleto: se elimino.
    LeftoverRemoved,
    /// Importacion de modpack cortada a medias: se deshizo.
    ImportRolledBack,
    /// La ficha de la instancia no se puede leer.
    Unreadable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityIssue {
    pub instance_id: String,
    /// Nombre para mostrar (vacio si la ficha no se pudo leer).
    pub name: String,
    pub kind: IssueKind,
    /// `true` si el launcher ya lo resolvio; `false` si hace falta una decision de la persona.
    pub fixed: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub issues: Vec<IntegrityIssue>,
}

impl IntegrityReport {
    /// Problemas que esperan una decision de la persona.
    pub fn needs_attention(&self) -> impl Iterator<Item = &IntegrityIssue> {
        self.issues.iter().filter(|i| !i.fixed)
    }
}

impl Launcher {
    /// Revisa todas las instancias y repara lo que es seguro reparar. Se omiten las que estan en uso.
    pub async fn instances_integrity(&self) -> Result<IntegrityReport, LauncherError> {
        let mut report = IntegrityReport::default();

        for problem in self.instances.scan().await? {
            if self.is_busy(&problem.id) {
                continue;
            }
            match problem.kind {
                FolderProblemKind::Orphan => {
                    if self.instances.remove_orphan(&problem.id).await.unwrap_or(false) {
                        report.issues.push(issue(&problem.id, "", IssueKind::LeftoverRemoved, true));
                    }
                }
                FolderProblemKind::Unreadable(detail) => {
                    tracing::warn!(instance = %problem.id, error = %detail, "ficha de instancia ilegible");
                    report.issues.push(issue(&problem.id, "", IssueKind::Unreadable, false));
                }
            }
        }

        for instance in self.instances.list().await? {
            if self.is_busy(&instance.id) {
                continue;
            }
            self.check_instance(&instance, &mut report).await;
        }
        Ok(report)
    }

    async fn check_instance(&self, instance: &Instance, report: &mut IntegrityReport) {
        let (id, name) = (instance.id.as_str(), instance.name.as_str());

        // Una importacion cortada deja una instancia a medio llenar: se deshace entera.
        if import_marker(&instance.game_dir).is_file() {
            if self.instances.delete(id).await.is_ok() {
                report.issues.push(issue(id, name, IssueKind::ImportRolledBack, true));
            }
            return;
        }

        if let Ok(true) = self.instances.ensure_layout(id).await {
            report.issues.push(issue(id, name, IssueKind::LayoutRepaired, true));
        }

        if !matches!(self.accounts.get(&instance.profile_id).await, Ok(Some(_))) {
            report.issues.push(issue(id, name, IssueKind::ProfileMissing, false));
        }
        if let JavaSelection::Path { path } = &instance.java {
            if !java_runtime::java_executable(path).is_file() {
                report.issues.push(issue(id, name, IssueKind::JavaMissing, false));
            }
        }
    }

    fn is_busy(&self, id: &str) -> bool {
        self.running_guard().contains_key(id)
    }

    /// Antes de lanzar: recrea carpetas que faltan y rechaza instancias cuya importacion no termino.
    pub(crate) async fn prepare_instance_dirs(&self, instance: &Instance) -> Result<(), LauncherError> {
        if import_marker(&instance.game_dir).is_file() {
            return Err(LauncherError::InvalidInput(
                "Esta instancia no termino de importarse. Eliminala e importa el modpack otra vez.".into(),
            ));
        }
        if self.instances.ensure_layout(&instance.id).await? {
            tracing::warn!(instance = %instance.id, "se recrearon carpetas de la instancia que faltaban");
        }
        Ok(())
    }
}

fn issue(id: &str, name: &str, kind: IssueKind, fixed: bool) -> IntegrityIssue {
    IntegrityIssue { instance_id: id.to_string(), name: name.to_string(), kind, fixed }
}
