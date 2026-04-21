use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use reqwest::Url;
use tracing::warn;

use crate::config::PromptCompressionConfig;
use crate::error::OaasError;
use crate::llmlingua::LlmLinguaClient;

#[derive(Clone)]
pub struct ProxyState {
    pub client: reqwest::Client,
    pub upstream: Url,
    pub llmlingua: Option<LlmLinguaClient>,
    pub prompt_compression: Option<PromptCompressionConfig>,
}

static HOP_HEADERS: &[HeaderName] = &[
    header::CONNECTION,
    header::TRANSFER_ENCODING,
    header::TE,
    header::TRAILER,
    header::UPGRADE,
    header::HOST,
];

pub async fn forward_request(state: &ProxyState, req: Request<Body>) -> Response {
    match forward_inner(state, req).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

async fn forward_inner(state: &ProxyState, req: Request<Body>) -> Result<Response, OaasError> {
    let (parts, body) = req.into_parts();
    let collected = body
        .collect()
        .await
        .map_err(|e| OaasError::Backend(format!("corps de requête invalide: {e}")))?;
    let mut body_bytes = collected.to_bytes().to_vec();

    let path = parts.uri.path();
    let query = parts
        .uri
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    if parts.method == Method::POST && path == "/v1/chat/completions" {
        if let (Some(ref ling), Some(ref pcfg)) = (&state.llmlingua, &state.prompt_compression) {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                match ling.compress_chat_json(pcfg, &val).await {
                    Ok(out) => {
                        body_bytes = serde_json::to_vec(&out).map_err(|e| {
                            OaasError::Backend(format!(
                                "sérialisation du corps après LLMLingua: {e}"
                            ))
                        })?;
                    }
                    Err(e) => {
                        if pcfg.strict {
                            return Err(e);
                        }
                        warn!(error = %e, "LLMLingua ignoré pour cette requête (strict=false)");
                    }
                }
            }
        }
    }

    let joined = format!("{}{}", path.trim_start_matches('/'), query);
    let url = state
        .upstream
        .join(&joined)
        .map_err(|e| OaasError::Backend(format!("URL upstream invalide: {e}")))?;

    let mut rb = state
        .client
        .request(parts.method, url.as_str())
        .body(body_bytes);

    for (k, v) in parts.headers.iter() {
        if HOP_HEADERS.contains(k) {
            continue;
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_bytes(v.as_bytes()) {
                rb = rb.header(name, val);
            }
        }
    }

    let upstream = rb.send().await?;

    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut res = Response::builder().status(status);

    for (k, v) in upstream.headers().iter() {
        if k == header::TRANSFER_ENCODING {
            continue;
        }
        if let Ok(name) = HeaderName::from_bytes(k.as_str().as_bytes()) {
            if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
                res = res.header(name, val);
            }
        }
    }

    let stream = upstream
        .bytes_stream()
        .map(|chunk| chunk.map_err(|e| std::io::Error::other(e.to_string())));

    let body = Body::from_stream(stream);
    res.body(body)
        .map_err(|e| OaasError::Backend(format!("réponse proxy invalide: {e}")))
}

pub fn build_client() -> Result<reqwest::Client, OaasError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| OaasError::Backend(e.to_string()))
}
