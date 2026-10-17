use crate::{JavaError, JavaInstallation};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn detect_java_at(path: &Path) -> Result<JavaInstallation, JavaError> {
    let java_exe = java_executable(path);
    if !java_exe.exists() {
        return Err(JavaError::DetectionFailed(format!(
            "java executable not found at {}",
            java_exe.display()
        )));
    }

    let output = Command::new(&java_exe)
        .arg("-version")
        .output()
        .await
        .map_err(|e| JavaError::DetectionFailed(e.to_string()))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = parse_java_version(&stderr)
        .ok_or_else(|| JavaError::DetectionFailed("could not parse version".into()))?;
    let major = parse_major_version(&version);

    Ok(JavaInstallation {
        path: path.to_path_buf(),
        version,
        major,
        managed: false,
    })
}

pub async fn detect_system_java() -> Vec<JavaInstallation> {
    let mut found = Vec::new();

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        if let Ok(inst) = detect_java_at(Path::new(&java_home)).await {
            found.push(inst);
        }
    }

    let candidate_dirs = system_java_candidates();
    for dir in candidate_dirs {
        if let Ok(inst) = detect_java_at(&dir).await {
            if !found.iter().any(|f| f.path == inst.path) {
                found.push(inst);
            }
        }
    }

    found
}

pub fn java_executable(java_home: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        java_home.join("bin").join("java.exe")
    } else {
        java_home.join("bin").join("java")
    }
}

fn system_java_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let base = PathBuf::from(&program_files);
            for dir_name in &["Java", "Eclipse Adoptium", "AdoptOpenJDK", "Zulu"] {
                let dir = base.join(dir_name);
                if dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            candidates.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let lib_jvms = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if lib_jvms.exists() {
            if let Ok(entries) = std::fs::read_dir(&lib_jvms) {
                for entry in entries.flatten() {
                    candidates.push(entry.path().join("Contents/Home"));
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let usr_lib = PathBuf::from("/usr/lib/jvm");
        if usr_lib.exists() {
            if let Ok(entries) = std::fs::read_dir(&usr_lib) {
                for entry in entries.flatten() {
                    candidates.push(entry.path());
                }
            }
        }
    }

    candidates
}

pub fn parse_java_version(output: &str) -> Option<String> {
    for line in output.lines() {
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

pub fn parse_major_version(version: &str) -> u32 {
    let cleaned = version.trim_start_matches("1.");
    cleaned
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modern_version() {
        assert_eq!(parse_java_version(r#"openjdk version "21.0.2" 2024-01-16"#), Some("21.0.2".into()));
    }

    #[test]
    fn parse_legacy_version() {
        assert_eq!(parse_java_version(r#"java version "1.8.0_392""#), Some("1.8.0_392".into()));
    }

    #[test]
    fn major_from_modern() {
        assert_eq!(parse_major_version("21.0.2"), 21);
        assert_eq!(parse_major_version("17.0.9"), 17);
    }

    #[test]
    fn major_from_legacy() {
        assert_eq!(parse_major_version("1.8.0_392"), 8);
    }
}
