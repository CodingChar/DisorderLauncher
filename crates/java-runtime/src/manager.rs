use crate::adoptium::{download_and_extract_runtime, find_java_home_in_extracted};
use crate::detect::{detect_java_at, detect_system_java};
use crate::{JavaError, JavaInstallation, JavaRequirement};
use std::path::PathBuf;

/// Version de Java por debajo de la cual solo se acepta una coincidencia exacta.
const LEGACY_JAVA_MAJOR: u32 = 8;

pub struct JavaManager {
    http: reqwest::Client,
    managed_runtimes_dir: PathBuf,
}

impl JavaManager {
    pub fn new(http: reqwest::Client, managed_runtimes_dir: impl Into<PathBuf>) -> Self {
        Self {
            http,
            managed_runtimes_dir: managed_runtimes_dir.into(),
        }
    }

    pub async fn detect_installed(&self) -> Vec<JavaInstallation> {
        let mut all = detect_system_java().await;

        if let Ok(mut entries) = tokio::fs::read_dir(&self.managed_runtimes_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    // El archivo de Adoptium se extrae en una subcarpeta (jdk-.../).
                    let home = find_java_home_in_extracted(&path).await.unwrap_or(path);
                    if let Ok(mut inst) = detect_java_at(&home).await {
                        inst.managed = true;
                        if !all.iter().any(|a| a.path == inst.path) {
                            all.push(inst);
                        }
                    }
                }
            }
        }

        all
    }

    pub async fn select_for_version(
        &self,
        requirement: &JavaRequirement,
    ) -> Result<JavaInstallation, JavaError> {
        let installed = self.detect_installed().await;

        select_best_java(&installed, requirement.major_version)
            .cloned()
            .ok_or(JavaError::NoCompatibleVersion(requirement.major_version))
    }

    pub async fn ensure_java(
        &self,
        requirement: &JavaRequirement,
    ) -> Result<JavaInstallation, JavaError> {
        match self.select_for_version(requirement).await {
            Ok(inst) => Ok(inst),
            Err(JavaError::NoCompatibleVersion(_)) => {
                self.download_runtime(requirement).await
            }
            Err(e) => Err(e),
        }
    }

    pub async fn download_runtime(
        &self,
        requirement: &JavaRequirement,
    ) -> Result<JavaInstallation, JavaError> {
        download_and_extract_runtime(
            &self.http,
            requirement.major_version,
            &self.managed_runtimes_dir,
        )
        .await
    }

    pub fn managed_runtime_path(&self, major: u32) -> PathBuf {
        self.managed_runtimes_dir.join(format!("jre-{}", major))
    }
}

/// Elige el Java a usar para `required_major`.
///
/// Prefiere la version exacta. Una version superior solo se acepta cuando el juego
/// exige Java 9 o mas: los clientes que piden Java 8 (launchwrapper, <= 1.16) fallan
/// con ClassCastException en cualquier Java posterior.
pub fn select_best_java(
    installations: &[JavaInstallation],
    required_major: u32,
) -> Option<&JavaInstallation> {
    if let Some(exact) = installations.iter().find(|j| j.major == required_major) {
        return Some(exact);
    }
    if required_major <= LEGACY_JAVA_MAJOR {
        return None;
    }
    installations
        .iter()
        .filter(|j| j.major >= required_major)
        .min_by_key(|j| j.major)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_best_exact_match() {
        let installations = vec![
            JavaInstallation { path: "a".into(), version: "17.0.9".into(), major: 17, managed: false },
            JavaInstallation { path: "b".into(), version: "21.0.2".into(), major: 21, managed: true },
        ];
        let best = select_best_java(&installations, 21).unwrap();
        assert_eq!(best.major, 21);
    }

    #[test]
    fn select_best_higher_compatible() {
        let installations = vec![
            JavaInstallation { path: "a".into(), version: "21.0.2".into(), major: 21, managed: false },
        ];
        let best = select_best_java(&installations, 17).unwrap();
        assert_eq!(best.major, 21);
    }

    #[test]
    fn java8_requirement_never_accepts_newer_java() {
        let installations = vec![
            JavaInstallation { path: "a".into(), version: "17.0.9".into(), major: 17, managed: false },
            JavaInstallation { path: "b".into(), version: "21.0.2".into(), major: 21, managed: true },
        ];
        assert!(select_best_java(&installations, 8).is_none());
    }

    #[test]
    fn java8_requirement_accepts_exact_java8() {
        let installations = vec![
            JavaInstallation { path: "a".into(), version: "21.0.2".into(), major: 21, managed: false },
            JavaInstallation { path: "b".into(), version: "1.8.0_392".into(), major: 8, managed: false },
        ];
        assert_eq!(select_best_java(&installations, 8).unwrap().major, 8);
    }

    #[test]
    fn select_best_none_compatible() {
        let installations = vec![
            JavaInstallation { path: "a".into(), version: "8.0.392".into(), major: 8, managed: false },
        ];
        assert!(select_best_java(&installations, 21).is_none());
    }
}
