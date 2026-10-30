use crate::{
    Catalog, ContentHit, ContentKind, ContentProvider, ModError, ProviderInfo, SearchPage, SearchQuery, Side, SideSupport, SortKey, CATEGORIES,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use url::Url;

pub const MODRINTH_API: &str = "https://api.modrinth.com/v2";

/// Id con el que Modrinth se identifica en `ContentHit::provider`, en el registro de mods y en el IPC.
pub const MODRINTH: &str = "modrinth";

/// Modrinth sirve los archivos desde su CDN; cualquier otro host se rechaza.
const TRUSTED_FILE_HOSTS: &[&str] = &["cdn.modrinth.com"];

/// Resultado tal como lo devuelve `/search` de Modrinth.
#[derive(Debug, Deserialize)]
struct RawHit {
    project_id: String,
    slug: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    follows: u64,
    #[serde(default)]
    icon_url: Option<String>,
    /// Mezcla categorias y loaders.
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    client_side: Option<String>,
    #[serde(default)]
    server_side: Option<String>,
    #[serde(default)]
    latest_version: Option<String>,
    #[serde(default)]
    date_modified: Option<DateTime<Utc>>,
    #[serde(default)]
    project_type: Option<String>,
}

/// Loaders que Modrinth mezcla con las categorias en sus resultados.
const LOADER_IDS: &[&str] = &["fabric", "forge", "neoforge", "quilt", "liteloader", "rift", "modloader"];

fn side_support(raw: Option<&str>) -> Option<SideSupport> {
    match raw? {
        "required" => Some(SideSupport::Required),
        "optional" => Some(SideSupport::Optional),
        "unsupported" => Some(SideSupport::Unsupported),
        _ => None,
    }
}

impl RawHit {
    fn into_hit(self) -> ContentHit {
        let kind = self.project_type.as_deref().unwrap_or("mod");
        ContentHit {
            provider: MODRINTH.to_string(),
            page_url: Some(format!("https://modrinth.com/{kind}/{}", self.slug)),
            categories: self.categories.iter().filter(|c| CATEGORIES.contains(&c.as_str())).cloned().collect(),
            loaders: self.categories.iter().filter(|c| LOADER_IDS.contains(&c.as_str())).cloned().collect(),
            client_side: side_support(self.client_side.as_deref()),
            server_side: side_support(self.server_side.as_deref()),
            project_id: self.project_id,
            slug: self.slug,
            title: self.title,
            summary: self.description,
            author: self.author,
            downloads: self.downloads,
            follows: self.follows,
            icon_url: self.icon_url,
            latest_version: self.latest_version,
            updated_at: self.date_modified,
        }
    }
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

    /// Proyectos que el autor declara incompatibles con esta version.
    pub fn incompatible_projects(&self) -> Vec<String> {
        self.dependencies
            .iter()
            .filter(|d| d.dependency_type == "incompatible")
            .filter_map(|d| d.project_id.clone())
            .collect()
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

/// Valores que pueden ir tal cual dentro de una faceta (versiones, loaders, categorias).
fn facet_safe(value: &str) -> bool {
    !value.is_empty() && value.len() <= 48 && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

fn project_type(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mod",
        ContentKind::Modpack => "modpack",
        ContentKind::ResourcePack => "resourcepack",
        ContentKind::Shader => "shader",
    }
}

/// Facetas de Modrinth: cada grupo se cumple (AND) y dentro de un grupo vale cualquiera (OR).
pub fn search_facets(query: &SearchQuery) -> String {
    let mut groups: Vec<Vec<String>> = vec![vec![format!("project_type:{}", project_type(query.kind))]];
    if let Some(version) = query.minecraft_version.as_deref().filter(|v| facet_safe(v)) {
        groups.push(vec![format!("versions:{version}")]);
    }
    let loaders: Vec<String> = query.loaders.iter().filter(|l| facet_safe(l)).map(|l| format!("categories:{l}")).collect();
    if !loaders.is_empty() {
        groups.push(loaders);
    }
    for category in query.categories.iter().filter(|c| facet_safe(c)) {
        groups.push(vec![format!("categories:{category}")]);
    }
    match query.side {
        Some(Side::Client) => groups.push(vec!["client_side:required".into(), "client_side:optional".into()]),
        Some(Side::Server) => groups.push(vec!["server_side:required".into(), "server_side:optional".into()]),
        None => {}
    }
    serde_json::json!(groups).to_string()
}

fn sort_index(sort: SortKey) -> &'static str {
    match sort {
        SortKey::Relevance => "relevance",
        SortKey::Downloads => "downloads",
        SortKey::Follows => "follows",
        SortKey::Newest => "newest",
        SortKey::Updated => "updated",
    }
}

#[derive(Clone)]
pub struct ModrinthClient {
    http: reqwest::Client,
    base_url: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<RawHit>,
    #[serde(default)]
    total_hits: u64,
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

    pub async fn search(&self, query: &SearchQuery) -> Result<SearchPage, ModError> {
        let response: SearchResponse = self
            .http
            .get(format!("{}/search", self.base_url))
            .query(&[
                ("query", query.trimmed_text().to_string()),
                ("facets", search_facets(query)),
                ("index", sort_index(query.sort).to_string()),
                ("limit", query.page_size().to_string()),
                ("offset", query.offset.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(SearchPage { hits: response.hits.into_iter().map(RawHit::into_hit).collect(), total: response.total_hits, offset: query.offset })
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

    /// Versiones de Modrinth que corresponden a archivos ya descargados, por su SHA-1.
    /// Los archivos que Modrinth no conoce simplemente no aparecen en el resultado.
    pub async fn versions_by_sha1(&self, hashes: &[String]) -> Result<std::collections::HashMap<String, ModVersion>, ModError> {
        if hashes.is_empty() {
            return Ok(Default::default());
        }
        Ok(self
            .http
            .post(format!("{}/version_files", self.base_url))
            .json(&serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Titulos de varios proyectos de una vez.
    pub async fn project_titles(&self, ids: &[String]) -> Result<std::collections::HashMap<String, String>, ModError> {
        #[derive(Deserialize)]
        struct Project {
            id: String,
            title: String,
        }
        if ids.is_empty() || ids.iter().any(|id| validate_project_ref(id).is_err()) {
            return Ok(Default::default());
        }
        let projects: Vec<Project> = self
            .http
            .get(format!("{}/projects", self.base_url))
            .query(&[("ids", serde_json::json!(ids).to_string())])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(projects.into_iter().map(|p| (p.id, p.title)).collect())
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

/// Modrinth como proveedor del catalogo.
#[derive(Clone)]
pub struct ModrinthProvider {
    client: ModrinthClient,
}

impl ModrinthProvider {
    pub fn new(client: ModrinthClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ContentProvider for ModrinthProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: MODRINTH.to_string(),
            name: "Modrinth".to_string(),
            kinds: vec![ContentKind::Mod, ContentKind::Modpack, ContentKind::ResourcePack, ContentKind::Shader],
            sorts: vec![SortKey::Relevance, SortKey::Downloads, SortKey::Follows, SortKey::Newest, SortKey::Updated],
            side_filter: true,
            reports_total: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchPage, ModError> {
        self.client.search(query).await
    }
}

impl Catalog {
    /// Catalogo con solo Modrinth.
    pub fn modrinth(client: ModrinthClient) -> Self {
        Catalog::new().with_provider(Arc::new(ModrinthProvider::new(client)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facets(query: &SearchQuery) -> serde_json::Value {
        serde_json::from_str(&search_facets(query)).unwrap()
    }

    #[test]
    fn facets_filter_by_version_and_loader() {
        let query = SearchQuery { minecraft_version: Some("1.20.1".into()), loaders: vec!["fabric".into()], ..SearchQuery::default() };
        let facets = facets(&query);
        assert_eq!(facets[0][0], "project_type:mod");
        assert_eq!(facets[1][0], "versions:1.20.1");
        assert_eq!(facets[2][0], "categories:fabric");
    }

    #[test]
    fn no_filters_means_only_the_project_type() {
        assert_eq!(facets(&SearchQuery::default()), serde_json::json!([["project_type:mod"]]));
    }

    #[test]
    fn several_loaders_are_alternatives_and_each_category_is_required() {
        let query = SearchQuery {
            loaders: vec!["quilt".into(), "fabric".into()],
            categories: vec!["optimization".into(), "library".into()],
            side: Some(Side::Client),
            ..SearchQuery::default()
        };
        let facets = facets(&query);
        assert_eq!(facets[1], serde_json::json!(["categories:quilt", "categories:fabric"]));
        assert_eq!(facets[2], serde_json::json!(["categories:optimization"]));
        assert_eq!(facets[3], serde_json::json!(["categories:library"]));
        assert_eq!(facets[4], serde_json::json!(["client_side:required", "client_side:optional"]));
    }

    #[test]
    fn unsafe_facet_values_are_dropped() {
        let query = SearchQuery { minecraft_version: Some("1.20\"],[\"x".into()), categories: vec!["a b".into()], ..SearchQuery::default() };
        assert_eq!(facets(&query), serde_json::json!([["project_type:mod"]]));
    }

    #[test]
    fn hits_are_translated_to_the_neutral_shape() {
        let raw: RawHit = serde_json::from_str(
            r#"{"project_id":"AANobbMI","slug":"sodium","title":"Sodium","description":"Motor","author":"jelly","downloads":9,"follows":3,
                "categories":["fabric","optimization","quilt","raro"],"client_side":"required","server_side":"unsupported",
                "latest_version":"1.20.1","date_modified":"2024-05-01T10:00:00Z","project_type":"mod"}"#,
        )
        .unwrap();
        let hit = raw.into_hit();
        assert_eq!(hit.provider, "modrinth");
        assert_eq!(hit.categories, ["optimization"]);
        assert_eq!(hit.loaders, ["fabric", "quilt"]);
        assert_eq!((hit.client_side, hit.server_side), (Some(SideSupport::Required), Some(SideSupport::Unsupported)));
        assert_eq!(hit.page_url.as_deref(), Some("https://modrinth.com/mod/sodium"));
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
