use minecraft_install::{build_asset_artifacts, Artifact, InstallPlan};
use minecraft_meta::AssetIndex;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const HASH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    Missing,
    Corrupted,
}

#[derive(Debug, Clone)]
pub struct FileIssue {
    pub artifact: Artifact,
    pub kind: IssueKind,
}

#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    pub issues: Vec<FileIssue>,
    pub ok: usize,
    /// El indice de assets falta o esta danado: los assets no pudieron revisarse.
    pub assets_unchecked: bool,
}

impl VerifyReport {
    pub fn is_healthy(&self) -> bool {
        self.issues.is_empty() && !self.assets_unchecked
    }

    pub fn missing(&self) -> usize {
        self.count(IssueKind::Missing)
    }

    pub fn corrupted(&self) -> usize {
        self.count(IssueKind::Corrupted)
    }

    fn count(&self, kind: IssueKind) -> usize {
        self.issues.iter().filter(|i| i.kind == kind).count()
    }
}

/// Recalcula la integridad (existencia, tamano y SHA-1) de todo lo que el plan exige.
/// Usa `plan.assets` si ya esta resuelto; si no, lo deriva del indice en disco.
pub async fn verify_installation(plan: &InstallPlan, data_dir: &Path) -> VerifyReport {
    let mut targets: Vec<Artifact> = Vec::new();
    targets.push(plan.client.clone());
    targets.push(plan.asset_index.clone());
    targets.extend(plan.libraries.iter().cloned());
    targets.extend(plan.natives.iter().cloned());

    let mut assets_unchecked = false;
    if plan.assets.is_empty() {
        match load_asset_index(&plan.asset_index.path).await {
            Some(index) => targets.extend(build_asset_artifacts(&index, data_dir)),
            None => assets_unchecked = true,
        }
    } else {
        targets.extend(plan.assets.iter().cloned());
    }

    let mut report = check_all(targets).await;
    report.assets_unchecked = assets_unchecked;
    report
}

async fn load_asset_index(path: &Path) -> Option<AssetIndex> {
    let data = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&data).ok()
}

async fn check_all(targets: Vec<Artifact>) -> VerifyReport {
    let semaphore = Arc::new(Semaphore::new(HASH_CONCURRENCY));
    let mut set = JoinSet::new();

    for artifact in targets {
        let permit = semaphore.clone().acquire_owned().await;
        set.spawn_blocking(move || {
            let _permit = permit;
            let status = check_file(&artifact);
            (artifact, status)
        });
    }

    let mut report = VerifyReport::default();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((_, None)) => report.ok += 1,
            Ok((artifact, Some(kind))) => report.issues.push(FileIssue { artifact, kind }),
            Err(e) => tracing::error!(error = %e, "fallo una tarea de verificacion"),
        }
    }
    report
        .issues
        .sort_by(|a, b| a.artifact.path.cmp(&b.artifact.path));
    report
}

/// `None` si el archivo esta bien.
fn check_file(artifact: &Artifact) -> Option<IssueKind> {
    let Ok(meta) = std::fs::metadata(&artifact.path) else {
        return Some(IssueKind::Missing);
    };
    if !meta.is_file() {
        return Some(IssueKind::Missing);
    }
    if artifact.size.is_some_and(|s| s != meta.len()) {
        return Some(IssueKind::Corrupted);
    }
    let Some(expected) = &artifact.sha1 else {
        return None;
    };
    match hash_file(&artifact.path) {
        Ok(actual) if actual.eq_ignore_ascii_case(expected) => None,
        _ => Some(IssueKind::Corrupted),
    }
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha1_of(data: &[u8]) -> String {
        format!("{:x}", Sha1::digest(data))
    }

    fn artifact(path: std::path::PathBuf, data: &[u8]) -> Artifact {
        Artifact {
            url: "https://example.com/x".into(),
            path,
            size: Some(data.len() as u64),
            sha1: Some(sha1_of(data)),
        }
    }

    #[tokio::test]
    async fn classifies_ok_missing_and_corrupted() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good");
        let bad = dir.path().join("bad");
        std::fs::write(&good, b"hello").unwrap();
        std::fs::write(&bad, b"hellx").unwrap();

        let report = check_all(vec![
            artifact(good, b"hello"),
            artifact(bad, b"hello"),
            artifact(dir.path().join("gone"), b"hello"),
        ])
        .await;

        assert_eq!(report.ok, 1);
        assert_eq!(report.missing(), 1);
        assert_eq!(report.corrupted(), 1);
        assert!(!report.is_healthy());
    }

    #[tokio::test]
    async fn size_mismatch_is_corrupted_without_hashing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"short").unwrap();
        let mut a = artifact(path, b"a much longer content");
        a.sha1 = None;
        let report = check_all(vec![a]).await;
        assert_eq!(report.corrupted(), 1);
    }
}
