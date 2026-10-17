#![allow(dead_code)]

use identity_microsoft::MicrosoftEndpoints;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: String,
    pub authorization: Option<String>,
}

type Handler = Arc<dyn Fn(&Recorded) -> (u16, String) + Send + Sync>;

/// Servidor HTTP minimo que imita Microsoft, Xbox y Minecraft Services en loopback.
pub struct FakeServer {
    pub base: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    routes: Arc<Mutex<HashMap<String, Handler>>>,
}

impl FakeServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let routes: Arc<Mutex<HashMap<String, Handler>>> = Arc::new(Mutex::new(HashMap::new()));

        let (reqs, rts) = (requests.clone(), routes.clone());
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else { break };
                let (reqs, rts) = (reqs.clone(), rts.clone());
                tokio::spawn(async move {
                    let Some(request) = read_request(&mut stream).await else { return };
                    reqs.lock().unwrap().push(request.clone());
                    let handler = rts.lock().unwrap().get(&format!("{} {}", request.method, request.path)).cloned();
                    let (status, body) = match handler {
                        Some(h) => h(&request),
                        None => (404, "{}".to_string()),
                    };
                    let response = format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        Self { base, requests, routes }
    }

    pub fn route(&self, key: &str, handler: impl Fn(&Recorded) -> (u16, String) + Send + Sync + 'static) {
        self.routes.lock().unwrap().insert(key.to_string(), Arc::new(handler));
    }

    pub fn respond(&self, key: &str, status: u16, body: &str) {
        let body = body.to_string();
        self.route(key, move |_| (status, body.clone()));
    }

    pub fn requests_to(&self, key: &str) -> Vec<Recorded> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| format!("{} {}", r.method, r.path) == key)
            .cloned()
            .collect()
    }

    pub fn endpoints(&self) -> MicrosoftEndpoints {
        MicrosoftEndpoints {
            authorize: format!("{}/authorize", self.base),
            token: format!("{}/token", self.base),
            xbox_user: format!("{}/xbox", self.base),
            xsts: format!("{}/xsts", self.base),
            minecraft_login: format!("{}/mc/login", self.base),
            minecraft_entitlements: format!("{}/mc/entitlements", self.base),
            minecraft_profile: format!("{}/mc/profile", self.base),
        }
    }

    /// Respuestas correctas para toda la cadena (el token de refresco se parametriza).
    pub fn happy_chain(&self, refresh_token: &str) {
        self.respond(
            "POST /token",
            200,
            &format!(r#"{{"access_token":"ms-access","refresh_token":"{refresh_token}","expires_in":3600}}"#),
        );
        let xbox = r#"{"Token":"xbl-token","DisplayClaims":{"xui":[{"uhs":"uhs-1"}]}}"#;
        self.respond("POST /xbox", 200, xbox);
        self.respond("POST /xsts", 200, xbox);
        self.respond("POST /mc/login", 200, r#"{"access_token":"mc-token-abcdef","expires_in":86400}"#);
        self.respond("GET /mc/entitlements", 200, r#"{"items":[{"name":"game_minecraft"}]}"#);
        self.respond(
            "GET /mc/profile",
            200,
            r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","skins":[{"id":"skin-1","state":"ACTIVE"}]}"#,
        );
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> Option<Recorded> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let (method, target) = (first.next()?.to_string(), first.next()?.to_string());
    let mut length = 0usize;
    let mut authorization = None;
    for line in lines {
        let lower = line.to_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
        if lower.starts_with("authorization:") {
            authorization = Some(line.splitn(2, ':').nth(1)?.trim().to_string());
        }
    }
    while buf.len() < header_end + length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&buf[header_end..(header_end + length).min(buf.len())]).to_string();
    Some(Recorded { method, path: target.split('?').next()?.to_string(), body, authorization })
}
