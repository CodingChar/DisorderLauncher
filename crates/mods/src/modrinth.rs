use crate::ModError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

pub const MODRINTH_API: &str = "https://api.modrinth.com/v2";

/// Modrinth sirve los archivos desde su CDN; cualquier otro host se rechaza.
const TRUSTED_FILE_HOSTS: &[&str] = &["cdn.modrinth.com"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModVersion {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    #[serde(default)]
    pub name: String,
    pub date_published: DateTime<Utc>,
    /// `release`, `beta` o `alpha`.
    pub version_type: String,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub hashes: Hashes,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Hashes {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    /// `required`, `optional`, `incompatible` o `embedded`.
    pub dependency_type: String,
}

impl ModVersion {
    /// Archivo a descargar: el marcado como principal o, si no, el primero.
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files.iter().find(|f| f.primary).or_else(|| self.files.first())
    }

    pub fn required_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies.iter().filter(|d| d.dependency_type == "required")
    }
}

/// Un archivo de Modrinth solo se descarga por HTTPS desde su CDN.
pub fn ensure_trusted_file_url(url: &str) -> Result<(), ModError> {
    let parsed = Url::parse(url).map_err(|_| ModError::UntrustedSource)?;
    let ok = parsed.scheme() == "https" && parsed.host_str().is_some_and(|h| TRUSTED_FILE_HOSTS.contains(&h));
    if ok { Ok(()) } else { Err(ModError::UntrustedSource) }
}

/// Ids y slugs de Modrinth: solo caracteres seguros para ir en una URL.
pub fn validate_project_ref(value: &str) -> Result<(), ModError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok { Ok(()) } else { Err(ModError::NotFound) }
}

/// Facetas de busqueda: solo mods compatibles con la version y el loader de la instancia.
pub fn search_facets(minecraft_version: &str, loader: &str) -> String {
    serde_json::json!([
        ["project_type:mod"],
        [format!("versions:{minecraft_version}")],
        [format!("categories:{loader}")]
    ])
    .to_string()
}

#[derive(Clone)]
pub struct ModrinthClient {
    http: reqwest::Client,
    base_url: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<ProjectHit>,
}

#[derive(Deserialize)]
struct ProjectInfo {
    title: String,
}

impl ModrinthClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http, base_url: MODRINTH_API.to_string() }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub async fn search(
        &self,
        query: &str,
        minecraft_version: &str,
        loader: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ProjectHit>, ModError> {
        let response: SearchResponse = self
            .http
            .get(format!("{}/search", self.base_url))
            .query(&[
                ("query", query.to_string()),
                ("facets", search_facets(minecraft_version, loader)),
                ("limit", limit.min(50).to_string()),
                ("offset", offset.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response.hits)
    }

    pub async fn project_versions(
        &self,
        project: &str,
        minecraft_version: &str,
        loader: &str,
    ) -> Result<Vec<ModVersion>, ModError> {
        validate_project_ref(project)?;
        Ok(self
            .http
            .get(format!("{}/project/{project}/version", self.base_url))
            .query(&[
                ("loaders", serde_json::json!([loader]).to_string()),
                ("game_versions", serde_json::json!([minecraft_version]).to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn version(&self, version_id: &str) -> Result<ModVersion, ModError> {
        validate_project_ref(version_id)?;
        Ok(self
            .http
            .get(format!("{}/version/{version_id}", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn project_title(&self, project: &str) -> Result<String, ModError> {
        validate_project_ref(project)?;
        let info: ProjectInfo = self
            .http
            .get(format!("{}/project/{project}", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(info.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facets_filter_by_version_and_loader() {
        let facets: serde_json::Value = serde_json::from_str(&search_facets("1.20.1", "fabric")).unwrap();
        assert_eq!(facets[0][0], "project_type:mod");
        assert_eq!(facets[1][0], "versions:1.20.1");
        assert_eq!(facets[2][0], "categories:fabric");
    }

    #[test]
    fn trusted_file_hosts() {
        assert!(ensure_trusted_file_url("https://cdn.modrinth.com/data/x/versions/y/a.jar").is_ok());
        assert!(ensure_trusted_file_url("http://cdn.modrinth.com/a.jar").is_err());
        assert!(ensure_trusted_file_url("https://cdn.modrinth.com.evil.io/a.jar").is_err());
        assert!(ensure_trusted_file_url("https://example.com/a.jar").is_err());
    }

    #[test]
    fn project_refs() {
        assert!(validate_project_ref("AANobbMI").is_ok());
        assert!(validate_project_ref("fabric-api").is_ok());
        for bad in ["", "a/b", "../x", "a b", "a?x=1", &"x".repeat(65)] {
            assert!(validate_project_ref(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parses_a_version_payload() {
        let json = r#"{
          "id": "ver1", "project_id": "proj1", "version_number": "1.2.3", "name": "Release",
          "date_published": "2024-05-01T10:00:00.000000Z", "version_type": "release",
          "files": [
            {"url": "https://cdn.modrinth.com/a-sources.jar", "filename": "a-sources.jar", "primary": false, "size": 1, "hashes": {"sha1": "aa", "sha512": "bb"}},
            {"url": "https://cdn.modrinth.com/a.jar", "filename": "a.jar", "primary": true, "size": 2, "hashes": {"sha1": "cc", "sha512": "dd"}}
          ],
          "dependencies": [
            {"version_id": null, "project_id": "fapi", "dependency_type": "required"},
            {"version_id": null, "project_id": "opt", "dependency_type": "optional"}
          ]
        }"#;
        let version: ModVersion = serde_json::from_str(json).unwrap();
        assert_eq!(version.primary_file().unwrap().filename, "a.jar");
        let required: Vec<_> = version.required_dependencies().collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].project_id.as_deref(), Some("fapi"));
    }
}
