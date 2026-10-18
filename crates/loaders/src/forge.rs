//! Forge y NeoForge: se instalan ejecutando el instalador oficial (`--installClient`) con el Java
//! gestionado y se consume el perfil que genera, igual que el de Fabric (ver ADR-002).

use crate::fabric::{load_profile_from_disk, overlay_from_profile};
use crate::{ensure_trusted, validate_component, InstalledLoader, LoaderError, LoaderInstallContext, LoaderProvider, LoaderVersion};
use instance_manager::LoaderKind;
use minecraft_meta::LoaderProfile;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub const NEOFORGE_VERSIONS_URL: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
pub const FORGE_METADATA_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";
pub const FORGE_PROMOTIONS_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

const INSTALLER_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const MAX_INSTALLER_BYTES: usize = 300 * 1024 * 1024;
const OUTPUT_TAIL_LINES: usize = 30;

/// Variables que el instalador necesita; el resto del entorno del launcher no se hereda.
const INHERITED_ENV: &[&str] = &["PATH", "SystemRoot", "SYSTEMDRIVE", "windir", "TEMP", "TMP", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "HOME", "LANG"];

pub struct ForgeProvider;
pub struct NeoForgeProvider;

impl ForgeProvider {
    pub fn create(http: reqwest::Client) -> ForgeFamilyProvider {
        ForgeFamilyProvider { kind: LoaderKind::Forge, http }
    }

    pub fn profile_id(mc: &str, version: &str) -> String {
        format!("{mc}-forge-{version}")
    }
}

impl NeoForgeProvider {
    pub fn create(http: reqwest::Client) -> ForgeFamilyProvider {
        ForgeFamilyProvider { kind: LoaderKind::NeoForge, http }
    }

    pub fn profile_id(version: &str) -> String {
        format!("neoforge-{version}")
    }
}

pub struct ForgeFamilyProvider {
    kind: LoaderKind,
    http: reqwest::Client,
}

// ---- Funciones puras (probadas sin red) ---------------------------------------------------------

/// NeoForge numera `<minor>.<parche>.<build>` segun Minecraft `1.<minor>.<parche>`:
/// `1.21.1` -> prefijo `21.1.`, `1.21` -> `21.0.`. Solo existe desde 1.20.2.
pub fn neoforge_prefix(mc: &str) -> Option<String> {
    let rest = mc.strip_prefix("1.")?;
    let mut parts = rest.split('.');
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch: u32 = parts.next().map(str::parse).transpose().ok()?.unwrap_or(0);
    if parts.next().is_some() || minor < 20 || (minor == 20 && patch < 2) {
        return None;
    }
    Some(format!("{minor}.{patch}."))
}

#[derive(Deserialize)]
struct NeoForgeVersions {
    versions: Vec<String>,
}

/// Versiones de NeoForge para `mc`, la mas reciente primero.
pub fn parse_neoforge_versions(json: &[u8], mc: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
    let prefix = neoforge_prefix(mc).ok_or_else(|| LoaderError::NoVersionFor(mc.to_string()))?;
    let all: NeoForgeVersions = serde_json::from_slice(json)?;
    let mut versions: Vec<LoaderVersion> = all
        .versions
        .into_iter()
        .filter(|v| v.starts_with(&prefix))
        .map(|v| LoaderVersion { stable: !["beta", "alpha", "rc"].iter().any(|t| v.contains(t)), version: v })
        .collect();
    versions.sort_by_key(|v| std::cmp::Reverse(numeric_key(&v.version)));
    Ok(versions)
}

/// Versiones de Minecraft con NeoForge: `21.1.172` -> `1.21.1`, `21.0.5` -> `1.21`.
pub fn neoforge_game_versions(json: &[u8]) -> Result<Vec<String>, LoaderError> {
    let all: NeoForgeVersions = serde_json::from_slice(json)?;
    let mut out: Vec<String> = Vec::new();
    for v in all.versions {
        let mut parts = v.split(|c: char| !c.is_ascii_digit());
        let (Some(Ok(minor)), Some(Ok(patch))) = (parts.next().map(str::parse::<u32>), parts.next().map(str::parse::<u32>)) else {
            continue;
        };
        if !(20..26).contains(&minor) {
            continue;
        }
        let mc = if patch == 0 { format!("1.{minor}") } else { format!("1.{minor}.{patch}") };
        if !out.contains(&mc) {
            out.push(mc);
        }
    }
    Ok(out)
}

/// Versiones de Minecraft con Forge: las claves de `maven-metadata.json`.
pub fn forge_game_versions(metadata: &[u8]) -> Result<Vec<String>, LoaderError> {
    let all: HashMap<String, Vec<String>> = serde_json::from_slice(metadata)?;
    Ok(all.into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| k).collect())
}

/// Clave de orden numerica (`21.1.172` > `21.1.99`); lo que no es numero cuenta como 0.
fn numeric_key(version: &str) -> Vec<u64> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or(0))
        .collect()
}

/// Versiones de Forge para `mc`: `maven-metadata.json` mapea `mc -> ["mc-forge", ...]` y las
/// promociones marcan la recomendada (o, si no hay, la ultima) como estable.
pub fn parse_forge_versions(metadata: &[u8], promotions: &[u8], mc: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
    let all: HashMap<String, Vec<String>> = serde_json::from_slice(metadata)?;
    let entries = all.get(mc).ok_or_else(|| LoaderError::NoVersionFor(mc.to_string()))?;
    let mut versions: Vec<String> = entries
        .iter()
        .filter_map(|e| e.strip_prefix(&format!("{mc}-")).map(str::to_string))
        .collect();
    if versions.is_empty() {
        return Err(LoaderError::NoVersionFor(mc.to_string()));
    }
    versions.sort_by_key(|v| std::cmp::Reverse(numeric_key(v)));

    #[derive(Deserialize)]
    struct Promos {
        promos: HashMap<String, String>,
    }
    let promos = serde_json::from_slice::<Promos>(promotions).map(|p| p.promos).unwrap_or_default();
    let stable = promos
        .get(&format!("{mc}-recommended"))
        .or_else(|| promos.get(&format!("{mc}-latest")))
        .cloned()
        .unwrap_or_else(|| versions[0].clone());

    Ok(versions.into_iter().map(|v| LoaderVersion { stable: v == stable, version: v }).collect())
}

pub fn installer_url(kind: LoaderKind, mc: &str, version: &str) -> Option<String> {
    match kind {
        LoaderKind::NeoForge => Some(format!("{NEOFORGE_MAVEN}/{version}/neoforge-{version}-installer.jar")),
        LoaderKind::Forge => Some(format!("{FORGE_MAVEN}/{mc}-{version}/forge-{mc}-{version}-installer.jar")),
        _ => None,
    }
}

/// Primer token de un `.sha1` publicado en Maven (a veces "hash  nombre").
pub fn parse_sha1_sidecar(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?.to_lowercase();
    (token.len() == 40 && token.chars().all(|c| c.is_ascii_hexdigit())).then_some(token)
}

fn tail(lines: &[String]) -> String {
    lines.iter().rev().take(OUTPUT_TAIL_LINES).rev().cloned().collect::<Vec<_>>().join("\n")
}

fn sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data))
}

// ---- Proveedor ---------------------------------------------------------------------------------

impl ForgeFamilyProvider {
    fn profile_id(&self, mc: &str, version: &str) -> String {
        match self.kind {
            LoaderKind::NeoForge => NeoForgeProvider::profile_id(version),
            _ => ForgeProvider::profile_id(mc, version),
        }
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, LoaderError> {
        Ok(self.http.get(url).send().await?.error_for_status()?.bytes().await?.to_vec())
    }

    /// Descarga el instalador y comprueba su SHA-1 publicado junto al artefacto.
    async fn download_installer(&self, url: &str, dest: &Path) -> Result<(), LoaderError> {
        ensure_trusted(url)?;
        let bytes = self.get_bytes(url).await?;
        if bytes.len() > MAX_INSTALLER_BYTES {
            return Err(LoaderError::ChecksumMismatch);
        }
        let checksum_url = format!("{url}.sha1");
        ensure_trusted(&checksum_url)?;
        let published = String::from_utf8_lossy(&self.get_bytes(&checksum_url).await?).to_string();
        let expected = parse_sha1_sidecar(&published).ok_or(LoaderError::ChecksumMismatch)?;
        if sha1_hex(&bytes) != expected {
            return Err(LoaderError::ChecksumMismatch);
        }
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(dest, &bytes).await?;
        Ok(())
    }
}

/// Ejecuta `java -jar installer --installClient <dir>`: argv directo, sin shell, entorno reducido,
/// tiempo maximo y salida acotada (solo las ultimas lineas se conservan para el error).
async fn run_installer(java: &Path, installer: &Path, target: &Path) -> Result<(), LoaderError> {
    let mut command = Command::new(java);
    command
        .arg("-jar")
        .arg(installer)
        .arg("--installClient")
        .arg(target)
        .current_dir(target)
        .env_clear()
        .envs(INHERITED_ENV.iter().filter_map(|k| std::env::var_os(k).map(|v| (OsString::from(k), v))))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);

    let mut child = command.spawn()?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    for stream in [child.stdout.take().map(Pipe::Out), child.stderr.take().map(Pipe::Err)].into_iter().flatten() {
        let tx = tx.clone();
        tokio::spawn(async move {
            match stream {
                Pipe::Out(s) => pump(BufReader::new(s), tx).await,
                Pipe::Err(s) => pump(BufReader::new(s), tx).await,
            }
        });
    }
    drop(tx);

    let collector = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Some(line) = rx.recv().await {
            lines.push(line);
            if lines.len() > 500 {
                lines.drain(..250);
            }
        }
        lines
    });

    let status = match tokio::time::timeout(INSTALLER_TIMEOUT, child.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            let _ = child.kill().await;
            return Err(LoaderError::InstallerTimeout);
        }
    };
    let lines = collector.await.unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        Err(LoaderError::InstallerFailed(tail(&lines)))
    }
}

enum Pipe {
    Out(tokio::process::ChildStdout),
    Err(tokio::process::ChildStderr),
}

async fn pump<R: tokio::io::AsyncRead + Unpin>(mut reader: BufReader<R>, tx: tokio::sync::mpsc::UnboundedSender<String>) {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let _ = tx.send(String::from_utf8_lossy(&buf).trim_end().to_string());
            }
        }
    }
}

#[async_trait::async_trait]
impl LoaderProvider for ForgeFamilyProvider {
    fn kind(&self) -> LoaderKind {
        self.kind
    }

    async fn game_versions(&self) -> Result<Vec<String>, LoaderError> {
        match self.kind {
            LoaderKind::NeoForge => neoforge_game_versions(&self.get_bytes(NEOFORGE_VERSIONS_URL).await?),
            _ => forge_game_versions(&self.get_bytes(FORGE_METADATA_URL).await?),
        }
    }

    async fn versions(&self, minecraft_version: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        validate_component(minecraft_version)?;
        let versions = match self.kind {
            LoaderKind::NeoForge => parse_neoforge_versions(&self.get_bytes(NEOFORGE_VERSIONS_URL).await?, minecraft_version)?,
            _ => {
                let metadata = self.get_bytes(FORGE_METADATA_URL).await?;
                let promotions = self.get_bytes(FORGE_PROMOTIONS_URL).await.unwrap_or_default();
                parse_forge_versions(&metadata, &promotions, minecraft_version)?
            }
        };
        if versions.is_empty() {
            return Err(LoaderError::NoVersionFor(minecraft_version.to_string()));
        }
        Ok(versions)
    }

    async fn install(&self, ctx: &LoaderInstallContext<'_>) -> Result<InstalledLoader, LoaderError> {
        validate_component(ctx.minecraft_version)?;
        validate_component(ctx.loader_version)?;
        let java = ctx.java_executable.ok_or(LoaderError::JavaRequired)?;
        let url = installer_url(self.kind, ctx.minecraft_version, ctx.loader_version)
            .ok_or_else(|| LoaderError::NoVersionFor(ctx.minecraft_version.to_string()))?;

        let installer: PathBuf = ctx
            .data_dir
            .join("installers")
            .join(format!("{}-{}-{}.jar", self.profile_id(ctx.minecraft_version, ctx.loader_version), "installer", uuid_suffix()));
        (ctx.progress)(0, 2);
        self.download_installer(&url, &installer).await?;

        // El instalador exige un launcher_profiles.json en el destino; vacio y sin datos del usuario.
        tokio::fs::create_dir_all(ctx.data_dir).await?;
        let profiles = ctx.data_dir.join("launcher_profiles.json");
        if !profiles.exists() {
            tokio::fs::write(&profiles, br#"{"profiles":{}}"#).await?;
        }

        (ctx.progress)(1, 2);
        let result = run_installer(java, &installer, ctx.data_dir).await;
        let _ = tokio::fs::remove_file(&installer).await;
        result?;
        (ctx.progress)(2, 2);

        let profile_id = self.profile_id(ctx.minecraft_version, ctx.loader_version);
        let installed = load_profile_from_disk(ctx.data_dir, &profile_id)
            .await?
            .ok_or_else(|| LoaderError::InstallerFailed("el instalador no genero el perfil esperado".into()))?;
        check_inherits(&installed.profile, ctx.minecraft_version)?;
        Ok(InstalledLoader { overlay: overlay_from_profile(&installed.profile), profile_id })
    }

    async fn load_installed(
        &self,
        data_dir: &Path,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<Option<InstalledLoader>, LoaderError> {
        validate_component(minecraft_version)?;
        validate_component(loader_version)?;
        let profile_id = self.profile_id(minecraft_version, loader_version);
        Ok(load_profile_from_disk(data_dir, &profile_id)
            .await?
            .map(|p| InstalledLoader { overlay: overlay_from_profile(&p.profile), profile_id }))
    }
}

fn check_inherits(profile: &LoaderProfile, mc: &str) -> Result<(), LoaderError> {
    match profile.inherits_from.as_deref() {
        Some(parent) if parent != mc => Err(LoaderError::ProfileMismatch),
        _ => Ok(()),
    }
}

fn uuid_suffix() -> String {
    // Nombre unico sin dependencia extra: el pid y el reloj bastan para un archivo temporal.
    format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_prefixes_follow_minecraft_versions() {
        assert_eq!(neoforge_prefix("1.21.1").as_deref(), Some("21.1."));
        assert_eq!(neoforge_prefix("1.21").as_deref(), Some("21.0."));
        assert_eq!(neoforge_prefix("1.20.4").as_deref(), Some("20.4."));
        assert_eq!(neoforge_prefix("1.20.2").as_deref(), Some("20.2."));
        for none in ["1.20.1", "1.19.4", "1.12.2", "24w14a", "2.0", ""] {
            assert_eq!(neoforge_prefix(none), None, "{none}");
        }
    }

    #[test]
    fn neoforge_versions_are_filtered_sorted_and_flagged() {
        let json = br#"{"isSnapshot":false,"versions":["20.4.100","21.1.9","21.1.172","21.1.100-beta","21.0.5","21.1.20"]}"#;
        let v = parse_neoforge_versions(json, "1.21.1").unwrap();
        let names: Vec<_> = v.iter().map(|x| x.version.as_str()).collect();
        assert_eq!(names, ["21.1.172", "21.1.100-beta", "21.1.20", "21.1.9"]);
        assert!(v[0].stable && !v[1].stable);
        assert!(matches!(parse_neoforge_versions(json, "1.19.4"), Err(LoaderError::NoVersionFor(_))));
    }

    #[test]
    fn forge_versions_use_the_recommended_promotion() {
        let metadata = br#"{"1.20.1":["1.20.1-47.1.0","1.20.1-47.3.0","1.20.1-47.2.20"],"1.14.4":["1.14.4-28.2.26"]}"#;
        let promos = br#"{"promos":{"1.20.1-latest":"47.3.0","1.20.1-recommended":"47.2.20"}}"#;
        let v = parse_forge_versions(metadata, promos, "1.20.1").unwrap();
        assert_eq!(v.iter().map(|x| x.version.as_str()).collect::<Vec<_>>(), ["47.3.0", "47.2.20", "47.1.0"]);
        assert_eq!(v.iter().filter(|x| x.stable).map(|x| x.version.as_str()).collect::<Vec<_>>(), ["47.2.20"]);

        // Sin promociones, la mas reciente cuenta como estable.
        let v = parse_forge_versions(metadata, b"{}", "1.20.1").unwrap();
        assert!(v[0].stable && !v[1].stable);
        assert!(matches!(parse_forge_versions(metadata, b"{}", "1.7.10"), Err(LoaderError::NoVersionFor(_))));
    }

    #[test]
    fn installer_urls_are_trusted_and_well_formed() {
        let neo = installer_url(LoaderKind::NeoForge, "1.21.1", "21.1.172").unwrap();
        assert_eq!(neo, "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.172/neoforge-21.1.172-installer.jar");
        let forge = installer_url(LoaderKind::Forge, "1.20.1", "47.3.0").unwrap();
        assert_eq!(forge, "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-installer.jar");
        assert!(ensure_trusted(&neo).is_ok() && ensure_trusted(&forge).is_ok());
        assert_eq!(installer_url(LoaderKind::Fabric, "1.20.1", "x"), None);
    }

    #[test]
    fn sha1_sidecars_are_parsed_strictly() {
        let h = "a".repeat(40);
        assert_eq!(parse_sha1_sidecar(&format!("{h}\n")).as_deref(), Some(h.as_str()));
        assert_eq!(parse_sha1_sidecar(&format!("{}  installer.jar", h.to_uppercase())).as_deref(), Some(h.as_str()));
        for bad in ["", "zzzz", &"a".repeat(39), "<html>404</html>"] {
            assert_eq!(parse_sha1_sidecar(bad), None, "{bad}");
        }
    }

    #[test]
    fn profile_ids_follow_each_installer_convention() {
        assert_eq!(NeoForgeProvider::profile_id("21.1.172"), "neoforge-21.1.172");
        assert_eq!(ForgeProvider::profile_id("1.20.1", "47.3.0"), "1.20.1-forge-47.3.0");
    }

    #[test]
    fn numeric_ordering_is_not_lexicographic() {
        assert!(numeric_key("21.1.172") > numeric_key("21.1.99"));
        assert!(numeric_key("47.10.0") > numeric_key("47.9.5"));
    }

    #[tokio::test]
    async fn installing_without_java_or_with_bad_versions_fails_before_touching_anything() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ForgeProvider::create(reqwest::Client::new());
        let ctx = |mc: &'static str, v: &'static str, java: Option<&'static Path>, dir: &'static Path| LoaderInstallContext {
            data_dir: dir,
            minecraft_version: mc,
            loader_version: v,
            concurrency: 1,
            java_executable: java,
            progress: std::sync::Arc::new(|_, _| {}),
        };
        let path: &'static Path = Box::leak(dir.path().to_path_buf().into_boxed_path());
        assert!(matches!(provider.install(&ctx("1.20.1", "47.3.0", None, path)).await, Err(LoaderError::JavaRequired)));
        assert!(matches!(provider.install(&ctx("../x", "1", Some(Path::new("java")), path)).await, Err(LoaderError::InvalidComponent(_))));
        assert!(!dir.path().join("installers").exists());
        assert!(!dir.path().join("launcher_profiles.json").exists());
    }

    #[test]
    fn game_versions_are_derived_from_each_catalog() {
        let neo = br#"{"versions":["20.2.3-beta","20.4.1","21.0.5","21.1.172","21.1.173","19.0.1","bad"]}"#;
        assert_eq!(neoforge_game_versions(neo).unwrap(), ["1.20.2", "1.20.4", "1.21", "1.21.1"]);
        let forge = br#"{"1.12.2":["1.12.2-14.23.5.2860"],"1.20.1":["1.20.1-47.3.0"],"1.0":[]}"#;
        let mut v = forge_game_versions(forge).unwrap();
        v.sort();
        assert_eq!(v, ["1.12.2", "1.20.1"]);
        assert_eq!(crate::parse_game_versions(br#"[{"version":"1.21","stable":true},{"version":"24w14a","stable":false}]"#).unwrap(), ["1.21", "24w14a"]);
    }
}
