use crate::{JavaError, JavaInstallation};
use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const ADOPTIUM_API: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    binary: AdoptiumBinary,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    size: u64,
    #[allow(dead_code)]
    checksum: Option<String>,
}

fn adoptium_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

fn adoptium_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

fn adoptium_archive_ext() -> &'static str {
    if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

pub async fn fetch_adoptium_release_url(
    http: &reqwest::Client,
    major: u32,
) -> Result<(String, u64), JavaError> {
    let url = format!(
        "{}/assets/latest/{}/hotspot?os={}&architecture={}&image_type=jre",
        ADOPTIUM_API,
        major,
        adoptium_os(),
        adoptium_arch()
    );

    let releases: Vec<AdoptiumRelease> = http.get(&url).send().await?.json().await?;
    let release = releases
        .first()
        .ok_or(JavaError::NoAdoptiumRelease(major))?;

    Ok((
        release.binary.package.link.clone(),
        release.binary.package.size,
    ))
}

pub async fn download_and_extract_runtime(
    http: &reqwest::Client,
    major: u32,
    runtimes_dir: &Path,
) -> Result<JavaInstallation, JavaError> {
    let (url, _size) = fetch_adoptium_release_url(http, major).await?;

    fs::create_dir_all(runtimes_dir).await?;

    let archive_name = format!("jre-{}.{}", major, adoptium_archive_ext());
    let archive_path = runtimes_dir.join(&archive_name);

    let response = http.get(&url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;

    let part_path = archive_path.with_extension(format!("{}.part", adoptium_archive_ext()));
    let mut file = fs::File::create(&part_path).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;
    drop(file);
    fs::rename(&part_path, &archive_path).await?;

    let extract_dir = runtimes_dir.join(format!("jre-{}", major));
    extract_archive(&archive_path, &extract_dir).await?;

    let _ = fs::remove_file(&archive_path).await;

    let java_home = find_java_home_in_extracted(&extract_dir).await?;

    Ok(JavaInstallation {
        path: java_home,
        version: format!("{}.0.0", major),
        major,
        managed: true,
    })
}

async fn extract_archive(archive: &Path, dest: &Path) -> Result<(), JavaError> {
    fs::create_dir_all(dest).await?;

    #[cfg(target_os = "windows")]
    {
        let status = tokio::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive.display(),
                    dest.display()
                ),
            ])
            .status()
            .await
            .map_err(|e| JavaError::ExtractionFailed(e.to_string()))?;

        if !status.success() {
            return Err(JavaError::ExtractionFailed("Expand-Archive failed".into()));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let status = tokio::process::Command::new("tar")
            .args(["xzf", &archive.display().to_string(), "-C", &dest.display().to_string()])
            .status()
            .await
            .map_err(|e| JavaError::ExtractionFailed(e.to_string()))?;

        if !status.success() {
            return Err(JavaError::ExtractionFailed("tar extraction failed".into()));
        }
    }

    Ok(())
}

async fn find_java_home_in_extracted(dir: &Path) -> Result<std::path::PathBuf, JavaError> {
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let java_exe = if cfg!(target_os = "windows") {
                path.join("bin").join("java.exe")
            } else {
                path.join("bin").join("java")
            };
            if java_exe.exists() {
                return Ok(path);
            }
            let contents_home = path.join("Contents").join("Home");
            let java_exe_mac = contents_home.join("bin").join("java");
            if java_exe_mac.exists() {
                return Ok(contents_home);
            }
        }
    }
    Err(JavaError::ExtractionFailed(
        "could not find java home in extracted archive".into(),
    ))
}
