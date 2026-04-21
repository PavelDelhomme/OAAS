use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum OaasError {
    #[error("configuration: {0}")]
    Config(String),
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP client: {0}")]
    Http(#[from] reqwest::Error),
    #[error("backend: {0}")]
    Backend(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorInner,
}

#[derive(Serialize)]
struct ErrorInner {
    message: String,
    #[serde(rename = "type")]
    kind: &'static str,
}

impl IntoResponse for OaasError {
    fn into_response(self) -> Response {
        let status = match &self {
            OaasError::Config(_) | OaasError::Backend(_) => StatusCode::BAD_REQUEST,
            OaasError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OaasError::Http(_) => StatusCode::BAD_GATEWAY,
        };
        let body = ErrorBody {
            error: ErrorInner {
                message: self.to_string(),
                kind: "oaas_error",
            },
        };
        (status, Json(body)).into_response()
    }
}
