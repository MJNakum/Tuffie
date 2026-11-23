//! Request guards.
//!
//! Local mode has no login. A browser page on another site could still send
//! requests to `127.0.0.1:7777`, or use DNS rebinding to look same-origin.
//! Two checks stop this:
//! 1. The `Host` header must be a loopback name or an allowed host.
//! 2. Requests that change data, and WebSocket upgrades, must have no
//!    `Origin` or an `Origin` that matches the `Host`.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::AppState;

pub async fn guard(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let host = req.headers().get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    if !host_allowed(&host, &state.config.allowed_hosts) {
        return (StatusCode::FORBIDDEN, "host not allowed; add it with --allowed-host").into_response();
    }
    let is_ws = req.headers().get(header::UPGRADE).is_some();
    let unsafe_method = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if is_ws || unsafe_method {
        if let Some(origin) = req.headers().get(header::ORIGIN).and_then(|h| h.to_str().ok()) {
            let origin_host = origin.split("://").nth(1).unwrap_or("").to_ascii_lowercase();
            if origin_host != host {
                return (StatusCode::FORBIDDEN, "cross-origin request blocked").into_response();
            }
        }
    }
    let mut resp = next.run(req).await;
    let h = resp.headers_mut();
    h.insert("x-content-type-options", "nosniff".parse().unwrap());
    h.insert("x-frame-options", "DENY".parse().unwrap());
    h.insert("referrer-policy", "no-referrer".parse().unwrap());
    resp
}

fn host_allowed(host: &str, extra: &[String]) -> bool {
    let name = strip_port(host);
    matches!(name, "localhost" | "127.0.0.1" | "[::1]") || extra.iter().any(|h| h == name || h == host)
}

fn strip_port(host: &str) -> &str {
    if host.starts_with('[') {
        return host.find(']').map(|i| &host[..=i]).unwrap_or(host);
    }
    host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert!(host_allowed("127.0.0.1:7777", &[]));
        assert!(host_allowed("localhost:7777", &[]));
        assert!(host_allowed("[::1]:7777", &[]));
        assert!(!host_allowed("evil.example:7777", &[]));
        assert!(host_allowed("tuffie.internal", &["tuffie.internal".into()]));
    }
}
