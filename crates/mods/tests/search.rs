use mods::{ModrinthClient, ModsService, SearchQuery, Side, SortKey};
use test_support::FakeServer;

const RESPONSE: &str = r#"{
  "hits": [
    {"project_id":"AANobbMI","slug":"sodium","title":"Sodium","description":"Motor de dibujo","author":"jellysquid3","downloads":50000000,"follows":9000,
     "categories":["fabric","optimization"],"client_side":"required","server_side":"unsupported","latest_version":"1.20.1",
     "date_modified":"2024-05-01T10:00:00Z","project_type":"mod"}
  ],
  "offset": 20, "limit": 20, "total_hits": 41
}"#;

fn service(server: &FakeServer) -> ModsService {
    let client = ModrinthClient::new(reqwest::Client::new()).with_base_url(server.base.clone());
    ModsService::new(reqwest::Client::new()).with_client(client)
}

fn decoded(query: &str) -> String {
    url::form_urlencoded::parse(query.as_bytes()).map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
}

#[tokio::test]
async fn the_query_reaches_modrinth_as_facets_sort_and_paging() {
    let server = FakeServer::start().await;
    server.respond("GET /search", 200, RESPONSE);
    let query = SearchQuery {
        text: "  sodium ".into(),
        minecraft_version: Some("1.20.1".into()),
        loaders: vec!["fabric".into()],
        categories: vec!["optimization".into()],
        side: Some(Side::Client),
        sort: SortKey::Downloads,
        offset: 20,
        limit: 20,
        ..SearchQuery::default()
    };
    let page = service(&server).search(None, &query).await.unwrap();

    let sent = decoded(&server.requests_to("GET /search")[0].query);
    assert!(sent.contains("query=sodium&"), "{sent}");
    assert!(sent.contains("index=downloads"), "{sent}");
    assert!(sent.contains("offset=20") && sent.contains("limit=20"), "{sent}");
    for facet in ["project_type:mod", "versions:1.20.1", "categories:fabric", "categories:optimization", "client_side:required"] {
        assert!(sent.contains(facet), "{facet} en {sent}");
    }

    assert_eq!((page.total, page.offset, page.hits.len()), (41, 20, 1));
    assert!(page.has_more(), "20 ya vistos + 1 de esta pagina son 21 de 41");
}

#[tokio::test]
async fn a_second_page_keeps_asking_while_there_is_more() {
    let server = FakeServer::start().await;
    server.respond("GET /search", 200, RESPONSE);
    let page = service(&server).search(Some("modrinth"), &SearchQuery { offset: 0, ..SearchQuery::default() }).await.unwrap();
    assert!(page.has_more());
    assert_eq!(page.hits[0].provider, "modrinth");
    assert_eq!(page.hits[0].title, "Sodium");
}

#[tokio::test]
async fn an_unknown_provider_is_a_clear_error() {
    let server = FakeServer::start().await;
    let error = service(&server).search(Some("curseforge"), &SearchQuery::default()).await.unwrap_err();
    assert!(error.to_string().contains("curseforge"), "{error}");
}
