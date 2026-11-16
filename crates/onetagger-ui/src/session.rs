//! Per-process capability used by the browser, WebView and media endpoints.
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use crate::StartContext;

const COOKIE: &str = "onetagger_session";

#[derive(Clone)]
pub struct SessionSecret(String);

impl Default for SessionSecret {
    fn default() -> Self {
        Self(rand::random::<[u8; 32]>().iter().map(|b| format!("{b:02x}")).collect())
    }
}

impl std::fmt::Debug for SessionSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionSecret([redacted])")
    }
}

impl SessionSecret {
    pub fn launch_url(&self) -> String {
        format!("http://127.0.0.1:{}/#session={}", onetagger_shared::PORT, self.0)
    }

    fn matches(&self, candidate: &str) -> bool {
        candidate.len() == self.0.len() && candidate.bytes().zip(self.0.bytes()).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
    }
}

// Require the origin to match the authority serving this session. Do not trust forwarded headers.
fn trusted_request(headers: &HeaderMap, expose: bool, require_origin: bool) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|h| h.to_str().ok()) else { return false; };
    let Ok(authority) = host.parse::<axum::http::uri::Authority>() else { return false; };
    if !expose && !matches!(authority.host(), "localhost" | "127.0.0.1" | "[::1]") {
        return false;
    }
    match headers.get(header::ORIGIN) {
        Some(origin) => origin.to_str().ok().and_then(|s| s.parse::<Uri>().ok()).is_some_and(|uri| {
            matches!(uri.scheme_str(), Some("http" | "https"))
                && uri.authority().is_some_and(|a| a.as_str() == host)
                && uri.path() == "/" && uri.query().is_none()
        }),
        None => !require_origin,
    }
}

fn has_session(headers: &HeaderMap, context: &StartContext) -> bool {
    headers.get_all(header::COOKIE).iter().filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';')).filter_map(|c| c.trim().split_once('='))
        .any(|(name, value)| name == COOKIE && context.session.matches(value))
}

#[derive(Deserialize)]
pub struct SessionRequest { token: String }

pub async fn establish(State(context): State<StartContext>, headers: HeaderMap, Json(body): Json<SessionRequest>) -> Response {
    if !trusted_request(&headers, context.expose, true) || !context.session.matches(&body.token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let secure = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok()).is_some_and(|s| s.starts_with("https://"));
    let cookie = format!("{COOKIE}={}; HttpOnly; SameSite=Strict; Path=/{}", context.session.0, if secure { "; Secure" } else { "" });
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie), (header::CACHE_CONTROL, "no-store".into())]).into_response()
}

pub async fn require_session(State(context): State<StartContext>, request: Request, next: Next) -> Response {
    let require_origin = request.uri().path() == "/ws";
    if !trusted_request(request.headers(), context.expose, require_origin) || !has_session(request.headers(), &context) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::{get, post}, Router, body::Body, middleware};
    use tower::ServiceExt;

    fn app(context: StartContext) -> Router {
        Router::new().route("/ws", get(|| async { "authorized" }))
            .route("/audio", get(|| async { "audio" }))
            .route("/session", get(|| async { StatusCode::NO_CONTENT }))
            .route_layer(middleware::from_fn_with_state(context.clone(), require_session))
            .route("/session", post(establish)).with_state(context)
    }

    #[tokio::test]
    async fn websocket_requires_both_session_and_same_origin() {
        let context = StartContext::new(false, None, false, false);
        for (origin, cookie, expected) in [
            ("http://127.0.0.1:36913", "", StatusCode::UNAUTHORIZED),
            ("https://untrusted.example", context.session.0.as_str(), StatusCode::UNAUTHORIZED),
            ("null", context.session.0.as_str(), StatusCode::UNAUTHORIZED),
            ("http://127.0.0.1:36913", context.session.0.as_str(), StatusCode::OK),
        ] {
            let request = Request::builder().uri("/ws").header(header::HOST, "127.0.0.1:36913")
                .header(header::ORIGIN, origin).header(header::COOKIE, format!("{COOKIE}={cookie}"))
                .body(Body::empty()).unwrap();
            assert_eq!(app(context.clone()).oneshot(request).await.unwrap().status(), expected);
        }
    }

    #[tokio::test]
    async fn session_exchange_sets_private_cookie_and_rejects_wrong_token() {
        let context = StartContext::new(false, None, false, false);
        for token in ["invalid", context.session.0.as_str()] {
            let request = Request::builder().method("POST").uri("/session")
                .header(header::HOST, "127.0.0.1:36913").header(header::ORIGIN, "http://127.0.0.1:36913")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({"token": token}).to_string())).unwrap();
            let response = app(context.clone()).oneshot(request).await.unwrap();
            if token == "invalid" { assert_eq!(response.status(), StatusCode::FORBIDDEN); }
            else {
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
                let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
                assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
            }
        }
        assert!(!format!("{context:?}").contains(&context.session.0));
        assert!(!serde_json::to_string(&context).unwrap().contains(&context.session.0));
    }

    #[tokio::test]
    async fn media_is_private_and_old_sessions_are_invalid_after_restart() {
        let old = StartContext::new(false, None, false, false);
        let current = StartContext::new(false, None, false, false);
        for token in ["", old.session.0.as_str()] {
            let request = Request::builder().uri("/audio?path=private.mp3").header(header::HOST, "127.0.0.1:36913")
                .header(header::COOKIE, format!("{COOKIE}={token}")).body(Body::empty()).unwrap();
            assert_eq!(app(current.clone()).oneshot(request).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[test]
    fn loopback_rebinding_and_missing_websocket_origin_are_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "attacker.example:36913".parse().unwrap());
        headers.insert(header::ORIGIN, "http://attacker.example:36913".parse().unwrap());
        assert!(!trusted_request(&headers, false, true));
        headers.insert(header::HOST, "127.0.0.1:36913".parse().unwrap());
        headers.remove(header::ORIGIN);
        assert!(!trusted_request(&headers, false, true));
    }
}
