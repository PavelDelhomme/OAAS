use std::env;
use std::time::Duration;

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{header, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::{Buf, Bytes};
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

/// En-têtes réponse upstream qu’on ne recopie pas : `tower-http` compression / chunked réécriront ce qu’il faut.
static SKIP_UPSTREAM_RESP_HEADERS: &[HeaderName] = &[
    header::TRANSFER_ENCODING,
    header::CONTENT_LENGTH,
    header::CONTENT_ENCODING,
];

pub async fn forward_request(state: &ProxyState, req: Request<Body>) -> Response {
    match forward_inner(state, req).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

fn needs_buffered_body(parts: &Parts, state: &ProxyState) -> bool {
    parts.method == Method::POST
        && parts.uri.path() == "/v1/chat/completions"
        && state.llmlingua.is_some()
}

async fn forward_inner(state: &ProxyState, req: Request<Body>) -> Result<Response, OaasError> {
    let (parts, body) = req.into_parts();
    if needs_buffered_body(&parts, state) {
        forward_buffered(state, parts, body).await
    } else {
        forward_streaming(state, parts, body).await
    }
}

/// Chemin LLMLingua : corps JSON entier en mémoire (obligation métier).
async fn forward_buffered(
    state: &ProxyState,
    parts: Parts,
    body: Body,
) -> Result<Response, OaasError> {
    let collected = body
        .collect()
        .await
        .map_err(|e| OaasError::Backend(format!("corps de requête invalide: {e}")))?;
    let mut body_data: Bytes = collected.to_bytes();

    let path = parts.uri.path();
    let query = parts
        .uri
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    if let (Some(ref ling), Some(ref pcfg)) = (&state.llmlingua, &state.prompt_compression) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body_data.as_ref()) {
            match ling.compress_chat_json(pcfg, &val).await {
                Ok(out) => {
                    body_data = Bytes::from(serde_json::to_vec(&out).map_err(|e| {
                        OaasError::Backend(format!("sérialisation du corps après LLMLingua: {e}"))
                    })?);
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

    let joined = format!("{}{}", path.trim_start_matches('/'), query);
    let url = state
        .upstream
        .join(&joined)
        .map_err(|e| OaasError::Backend(format!("URL upstream invalide: {e}")))?;

    let rb = state.client.request(parts.method.clone(), url.as_str());
    let rb = copy_request_headers(&parts, rb);
    let rb = rb.body(body_data);

    build_upstream_response(rb.send().await?).await
}

/// Proxy « zéro copie » côté requête : flux Axum → flux reqwest sans `collect()` préalable.
async fn forward_streaming(
    state: &ProxyState,
    parts: Parts,
    body: Body,
) -> Result<Response, OaasError> {
    let path = parts.uri.path();
    let query = parts
        .uri
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();
    let joined = format!("{}{}", path.trim_start_matches('/'), query);
    let url = state
        .upstream
        .join(&joined)
        .map_err(|e| OaasError::Backend(format!("URL upstream invalide: {e}")))?;

    let byte_stream = body.into_data_stream().map(|frame| {
        frame
            .map_err(|e| std::io::Error::other(format!("corps requête: {e}")))
            .map(|mut data| data.copy_to_bytes(data.remaining()))
    });

    let rb = state.client.request(parts.method.clone(), url.as_str());
    let rb = copy_request_headers(&parts, rb);
    let rb = rb.body(reqwest::Body::wrap_stream(byte_stream));

    build_upstream_response(rb.send().await?).await
}

fn copy_request_headers(parts: &Parts, mut rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
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
    rb
}

async fn build_upstream_response(upstream: reqwest::Response) -> Result<Response, OaasError> {
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut res = Response::builder().status(status);

    for (k, v) in upstream.headers().iter() {
        if SKIP_UPSTREAM_RESP_HEADERS.contains(k) {
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

fn env_secs(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn base_http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
}

/// Client pour **téléchargements** (`models pull`, `docs pull`) : pas de limite totale, mais coupure si le réseau ne fournit plus d’octets.
pub fn build_client() -> Result<reqwest::Client, OaasError> {
    let read_s = env_secs("OAAS_HTTP_READ_TIMEOUT_SECS", 180);
    base_http_client_builder()
        .timeout(Duration::ZERO)
        .read_timeout(Duration::from_secs(read_s))
        .build()
        .map_err(|e| OaasError::Backend(e.to_string()))
}

/// Client pour le **proxy** vers llama-server : complétions longues, tokens espacés ; pas de timeout global.
pub fn build_proxy_client() -> Result<reqwest::Client, OaasError> {
    let read_s = env_secs("OAAS_PROXY_READ_TIMEOUT_SECS", 900);
    base_http_client_builder()
        .timeout(Duration::ZERO)
        .read_timeout(Duration::from_secs(read_s))
        .build()
        .map_err(|e| OaasError::Backend(e.to_string()))
}
