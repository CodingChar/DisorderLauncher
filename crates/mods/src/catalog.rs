//! Busqueda de contenido independiente del proveedor.
//!
//! La interfaz y el resto del launcher hablan con `Catalog`, nunca con Modrinth directamente. Cada
//! proveedor (hoy Modrinth; mañana CurseForge, Hangar...) implementa `ContentProvider` y traduce
//! la consulta neutra (`SearchQuery`) a su API y sus resultados a `ContentHit`. Para anadir uno:
//!
//! 1. implementar `ContentProvider` (ver `ModrinthProvider`), mapeando `CATEGORIES` y los
//!    `SortKey` a lo suyo y declarando en `ProviderInfo` lo que NO soporta;
//! 2. registrarlo con `Catalog::with_provider`.
//!
//! Nada mas cambia: los filtros que un proveedor no declara se ocultan o se ignoran, y cada
//! resultado lleva el `provider` del que viene para poder instalarlo despues.

use crate::ModError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximo de resultados por pagina que se acepta de la interfaz.
pub const MAX_PAGE_SIZE: u32 = 50;

/// Tipo de contenido. Hoy la interfaz solo ofrece mods; los demas existen para no rehacer el modelo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    #[default]
    Mod,
    Modpack,
    ResourcePack,
    Shader,
}

/// Orden de los resultados. Cada proveedor declara cuales entiende.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

/// Donde tiene que funcionar el mod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Client,
    Server,
}

/// Si un mod sirve en un lado del juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SideSupport {
    Required,
    Optional,
    Unsupported,
}

/// Categorias neutras (las comunes a casi todos los catalogos). Cada proveedor las traduce a las suyas.
pub const CATEGORIES: &[&str] = &[
    "adventure",
    "decoration",
    "economy",
    "equipment",
    "food",
    "game-mechanics",
    "library",
    "magic",
    "management",
    "minigame",
    "mobs",
    "optimization",
    "social",
    "storage",
    "technology",
    "transportation",
    "utility",
    "worldgen",
];

/// Consulta neutra. `None` / vacio significa «sin filtro».
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    pub text: String,
    pub kind: ContentKind,
    pub minecraft_version: Option<String>,
    /// Cualquiera de estos loaders sirve (p. ej. Quilt acepta `quilt` y `fabric`).
    pub loaders: Vec<String>,
    /// Ids de `CATEGORIES`; el resultado debe tenerlas todas.
    pub categories: Vec<String>,
    pub side: Option<Side>,
    pub sort: SortKey,
    pub offset: u32,
    pub limit: u32,
}

impl SearchQuery {
    /// Limite efectivo: entre 1 y `MAX_PAGE_SIZE`, 20 si no se indico.
    pub fn page_size(&self) -> u32 {
        if self.limit == 0 { 20 } else { self.limit.min(MAX_PAGE_SIZE) }
    }

    /// Texto sin espacios sobrantes.
    pub fn trimmed_text(&self) -> &str {
        self.text.trim()
    }
}

/// Un resultado, con los mismos campos sea cual sea el proveedor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentHit {
    /// Proveedor del que viene (`ProviderInfo::id`); junto con `project_id` identifica el proyecto.
    pub provider: String,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub author: String,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: Option<String>,
    /// Solo ids de `CATEGORIES`.
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    pub latest_version: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub client_side: Option<SideSupport>,
    pub server_side: Option<SideSupport>,
    /// Pagina publica del proyecto, si el proveedor la tiene.
    pub page_url: Option<String>,
}

/// Una pagina de resultados.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub hits: Vec<ContentHit>,
    /// Total de coincidencias (todas las paginas).
    pub total: u64,
    pub offset: u32,
}

impl SearchPage {
    pub fn has_more(&self) -> bool {
        u64::from(self.offset) + (self.hits.len() as u64) < self.total
    }
}

/// Lo que un proveedor sabe hacer: la interfaz oculta lo que no ofrece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kinds: Vec<ContentKind>,
    pub sorts: Vec<SortKey>,
    pub side_filter: bool,
    /// Si el proveedor cuenta el total de coincidencias.
    pub reports_total: bool,
}

#[async_trait]
pub trait ContentProvider: Send + Sync {
    fn info(&self) -> ProviderInfo;
    async fn search(&self, query: &SearchQuery) -> Result<SearchPage, ModError>;
}

/// Proveedores disponibles. El primero es el predeterminado.
#[derive(Clone, Default)]
pub struct Catalog {
    providers: Vec<Arc<dyn ContentProvider>>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Anade (o reemplaza, si ya hay uno con el mismo id) un proveedor.
    pub fn with_provider(mut self, provider: Arc<dyn ContentProvider>) -> Self {
        let id = provider.info().id;
        match self.providers.iter().position(|p| p.info().id == id) {
            Some(i) => self.providers[i] = provider,
            None => self.providers.push(provider),
        }
        self
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.providers.iter().map(|p| p.info()).collect()
    }

    pub fn provider(&self, id: Option<&str>) -> Result<&Arc<dyn ContentProvider>, ModError> {
        match id {
            Some(id) => self.providers.iter().find(|p| p.info().id == id).ok_or_else(|| ModError::ProviderUnavailable(id.to_string())),
            None => self.providers.first().ok_or_else(|| ModError::ProviderUnavailable("ninguno".into())),
        }
    }

    /// Busca en un proveedor (el predeterminado si no se indica). Descarta de antemano lo que el
    /// proveedor no admite, para que un filtro no soportado no vacie los resultados.
    pub async fn search(&self, provider: Option<&str>, query: &SearchQuery) -> Result<SearchPage, ModError> {
        let provider = self.provider(provider)?;
        let info = provider.info();
        let mut query = query.clone();
        query.limit = query.page_size();
        if !info.sorts.contains(&query.sort) {
            query.sort = SortKey::Relevance;
        }
        if !info.side_filter {
            query.side = None;
        }
        if !info.kinds.contains(&query.kind) {
            return Ok(SearchPage { offset: query.offset, ..SearchPage::default() });
        }
        query.categories.retain(|c| CATEGORIES.contains(&c.as_str()));
        provider.search(&query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Proveedor de mentira: anota lo que recibe y devuelve una pagina fija.
    struct Fake {
        id: &'static str,
        sorts: Vec<SortKey>,
        side_filter: bool,
        seen: Mutex<Vec<SearchQuery>>,
    }

    impl Fake {
        fn new(id: &'static str, sorts: Vec<SortKey>, side_filter: bool) -> Arc<Self> {
            Arc::new(Self { id, sorts, side_filter, seen: Mutex::new(Vec::new()) })
        }
    }

    #[async_trait]
    impl ContentProvider for Fake {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: self.id.into(),
                name: self.id.into(),
                kinds: vec![ContentKind::Mod],
                sorts: self.sorts.clone(),
                side_filter: self.side_filter,
                reports_total: true,
            }
        }

        async fn search(&self, query: &SearchQuery) -> Result<SearchPage, ModError> {
            self.seen.lock().unwrap().push(query.clone());
            Ok(SearchPage { hits: Vec::new(), total: 0, offset: query.offset })
        }
    }

    #[tokio::test]
    async fn routes_by_provider_and_defaults_to_the_first() {
        let a = Fake::new("a", vec![SortKey::Relevance], false);
        let b = Fake::new("b", vec![SortKey::Relevance], false);
        let catalog = Catalog::new().with_provider(a.clone()).with_provider(b.clone());
        catalog.search(None, &SearchQuery::default()).await.unwrap();
        catalog.search(Some("b"), &SearchQuery::default()).await.unwrap();
        assert_eq!((a.seen.lock().unwrap().len(), b.seen.lock().unwrap().len()), (1, 1));
        assert!(matches!(catalog.search(Some("zzz"), &SearchQuery::default()).await, Err(ModError::ProviderUnavailable(_))));
        assert_eq!(catalog.providers().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[tokio::test]
    async fn what_a_provider_does_not_support_is_dropped_before_it_is_asked() {
        let limited = Fake::new("limited", vec![SortKey::Relevance, SortKey::Downloads], false);
        let catalog = Catalog::new().with_provider(limited.clone());
        let query = SearchQuery {
            sort: SortKey::Follows,
            side: Some(Side::Server),
            categories: vec!["optimization".into(), "no-existe".into()],
            limit: 500,
            ..SearchQuery::default()
        };
        catalog.search(None, &query).await.unwrap();
        let seen = limited.seen.lock().unwrap();
        assert_eq!(seen[0].sort, SortKey::Relevance, "orden no soportado -> relevancia");
        assert_eq!(seen[0].side, None);
        assert_eq!(seen[0].categories, ["optimization"]);
        assert_eq!(seen[0].limit, MAX_PAGE_SIZE);
    }

    #[tokio::test]
    async fn an_unsupported_kind_returns_an_empty_page_instead_of_failing() {
        let catalog = Catalog::new().with_provider(Fake::new("a", vec![SortKey::Relevance], false));
        let page = catalog.search(None, &SearchQuery { kind: ContentKind::Shader, ..SearchQuery::default() }).await.unwrap();
        assert!(page.hits.is_empty() && !page.has_more());
    }

    #[test]
    fn has_more_counts_what_was_already_shown() {
        let hit = || ContentHit {
            provider: "a".into(),
            project_id: "p".into(),
            slug: "s".into(),
            title: "t".into(),
            summary: String::new(),
            author: String::new(),
            downloads: 0,
            follows: 0,
            icon_url: None,
            categories: vec![],
            loaders: vec![],
            latest_version: None,
            updated_at: None,
            client_side: None,
            server_side: None,
            page_url: None,
        };
        let page = SearchPage { hits: vec![hit(), hit()], total: 5, offset: 2 };
        assert!(page.has_more());
        assert!(!SearchPage { hits: vec![hit()], total: 3, offset: 2 }.has_more());
    }
}
