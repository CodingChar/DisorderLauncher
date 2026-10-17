use crate::{verify_installation, VerifyReport};
use minecraft_install::{InstallError, InstallExecutor, InstallPlan, InstallProgress};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    #[error("no se pudo completar la reparacion: {0}")]
    Install(#[from] InstallError),
}

#[derive(Debug)]
pub struct RepairReport {
    /// Problemas encontrados antes de reparar.
    pub found: usize,
    /// Estado final tras reparar.
    pub after: VerifyReport,
}

impl RepairReport {
    pub fn fixed(&self) -> usize {
        self.found.saturating_sub(self.after.issues.len())
    }

    pub fn is_fully_repaired(&self) -> bool {
        self.after.is_healthy()
    }
}

/// Redescarga solo lo que falta o esta danado y reextrae natives.
///
/// El executor ya omite los archivos cuyo hash coincide, por lo que reutilizarlo
/// garantiza el mismo camino (HTTPS, `.part`, verificacion, rename atomico) que la instalacion.
pub async fn repair_installation<F>(
    executor: &InstallExecutor,
    plan: &mut InstallPlan,
    data_dir: &Path,
    progress: F,
) -> Result<RepairReport, RepairError>
where
    F: Fn(InstallProgress) + Send + Sync + 'static,
{
    let before = verify_installation(plan, data_dir).await;
    let found = before.issues.len() + usize::from(before.assets_unchecked);
    if before.is_healthy() {
        return Ok(RepairReport { found: 0, after: before });
    }

    executor.execute(plan, data_dir, progress).await?;

    // `plan.assets` quedo resuelto por el executor; la verificacion final los incluye.
    let after = verify_installation(plan, data_dir).await;
    Ok(RepairReport { found, after })
}
