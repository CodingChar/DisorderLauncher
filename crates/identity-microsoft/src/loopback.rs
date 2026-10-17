use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const CALLBACK_PATH: &str = "/callback";

#[derive(Debug, PartialEq, Eq)]
pub enum CallbackError {
    /// El usuario cancelo o Microsoft devolvio un error (`access_denied`, ...).
    Denied(String),
    /// `state` distinto al esperado: se aborta el login.
    StateMismatch,
    /// Sin callback a tiempo.
    TimedOut,
    Io(String),
}

/// Servidor de callback de un solo uso: escucha solo en loopback, en un puerto dinamico, y
/// acepta unicamente el callback de OAuth.
pub struct CallbackServer {
    listener: TcpListener,
    port: u16,
}

impl CallbackServer {
    pub async fn bind() -> std::io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Azure acepta `http://localhost` con puerto libre para clientes publicos de escritorio.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}{CALLBACK_PATH}", self.port)
    }

    /// Espera el callback. Consume el servidor: tras la primera respuesta valida (o invalida)
    /// el puerto se cierra y no se acepta nada mas.
    pub async fn wait_for_code(self, expected_state: &str, timeout: Duration) -> Result<String, CallbackError> {
        match tokio::time::timeout(timeout, self.accept_loop(expected_state)).await {
            Ok(result) => result,
            Err(_) => Err(CallbackError::TimedOut),
        }
    }

    async fn accept_loop(&self, expected_state: &str) -> Result<String, CallbackError> {
        loop {
            let (mut stream, peer) = self.listener.accept().await.map_err(|e| CallbackError::Io(e.to_string()))?;
            // Solo conexiones desde esta misma maquina.
            if !peer.ip().is_loopback() {
                continue;
            }
            let Some(target) = read_request_target(&mut stream).await else {
                respond(&mut stream, "400 Bad Request", "Solicitud no valida.").await;
                continue;
            };
            let Ok(url) = Url::parse(&format!("http://localhost{target}")) else {
                respond(&mut stream, "400 Bad Request", "Solicitud no valida.").await;
                continue;
            };
            if url.path() != CALLBACK_PATH {
                respond(&mut stream, "404 Not Found", "No encontrado.").await;
                continue; // p. ej. /favicon.ico
            }

            let param = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());

            if param("state").as_deref() != Some(expected_state) {
                respond(&mut stream, "400 Bad Request", "No se pudo completar el inicio de sesion. Vuelve a intentarlo desde DisorderLauncher.").await;
                return Err(CallbackError::StateMismatch);
            }
            if let Some(error) = param("error") {
                respond(&mut stream, "200 OK", "El inicio de sesion se cancelo. Ya puedes cerrar esta pestana.").await;
                return Err(CallbackError::Denied(error));
            }
            return match param("code").filter(|c| !c.is_empty()) {
                Some(code) => {
                    respond(&mut stream, "200 OK", "Listo. Ya puedes cerrar esta pestana y volver a DisorderLauncher.").await;
                    Ok(code)
                }
                None => {
                    respond(&mut stream, "400 Bad Request", "Falta el codigo de inicio de sesion.").await;
                    Err(CallbackError::Denied("missing_code".into()))
                }
            };
        }
    }
}

/// Lee la linea de peticion y devuelve el `request-target` de un `GET`.
async fn read_request_target(stream: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = tokio::time::timeout(READ_TIMEOUT, stream.read(&mut chunk)).await.ok()?.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(2).any(|w| w == b"\r\n") || buf.len() >= MAX_REQUEST_BYTES {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let line = text.lines().next()?;
    let mut parts = line.split_whitespace();
    if parts.next()? != "GET" {
        return None;
    }
    let target = parts.next()?;
    target.starts_with('/').then(|| target.to_string())
}

async fn respond(stream: &mut TcpStream, status: &str, message: &str) {
    let body = format!(
        "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><title>DisorderLauncher</title>\
         <body style=\"font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1rem\">\
         <h1>DisorderLauncher</h1><p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get(port: u16, target: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        stream.write_all(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
        let mut out = String::new();
        stream.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn returns_the_code_when_state_matches_and_closes() {
        let server = CallbackServer::bind().await.unwrap();
        let port = server.port();
        assert_eq!(server.redirect_uri(), format!("http://localhost:{port}/callback"));

        let waiter = tokio::spawn(server.wait_for_code("good-state", Duration::from_secs(5)));
        let page = get(port, "/callback?code=abc123&state=good-state").await;
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("Content-Security-Policy"));
        assert_eq!(waiter.await.unwrap().unwrap(), "abc123");

        // Un solo uso: el puerto ya no acepta conexiones.
        assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
    }

    #[tokio::test]
    async fn state_mismatch_aborts_without_returning_the_code() {
        let server = CallbackServer::bind().await.unwrap();
        let port = server.port();
        let waiter = tokio::spawn(server.wait_for_code("expected", Duration::from_secs(5)));
        let page = get(port, "/callback?code=stolen&state=attacker").await;
        assert!(page.starts_with("HTTP/1.1 400"));
        assert_eq!(waiter.await.unwrap(), Err(CallbackError::StateMismatch));
    }

    #[tokio::test]
    async fn missing_state_is_a_mismatch() {
        let server = CallbackServer::bind().await.unwrap();
        let port = server.port();
        let waiter = tokio::spawn(server.wait_for_code("expected", Duration::from_secs(5)));
        get(port, "/callback?code=abc").await;
        assert_eq!(waiter.await.unwrap(), Err(CallbackError::StateMismatch));
    }

    #[tokio::test]
    async fn user_denial_is_reported() {
        let server = CallbackServer::bind().await.unwrap();
        let port = server.port();
        let waiter = tokio::spawn(server.wait_for_code("s", Duration::from_secs(5)));
        get(port, "/callback?error=access_denied&state=s").await;
        assert_eq!(waiter.await.unwrap(), Err(CallbackError::Denied("access_denied".into())));
    }

    #[tokio::test]
    async fn unrelated_requests_do_not_consume_the_session() {
        let server = CallbackServer::bind().await.unwrap();
        let port = server.port();
        let waiter = tokio::spawn(server.wait_for_code("s", Duration::from_secs(5)));
        assert!(get(port, "/favicon.ico").await.starts_with("HTTP/1.1 404"));
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        stream.write_all(b"POST /callback?code=x&state=s HTTP/1.1\r\n\r\n").await.unwrap();
        let mut out = String::new();
        stream.read_to_string(&mut out).await.unwrap();
        assert!(out.starts_with("HTTP/1.1 400"));

        get(port, "/callback?code=ok&state=s").await;
        assert_eq!(waiter.await.unwrap().unwrap(), "ok");
    }

    #[tokio::test]
    async fn times_out() {
        let server = CallbackServer::bind().await.unwrap();
        assert_eq!(server.wait_for_code("s", Duration::from_millis(100)).await, Err(CallbackError::TimedOut));
    }
}
