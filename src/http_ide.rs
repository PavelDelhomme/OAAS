use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::continue_ide::{
    build_continue_ide_status, continue_resolve_write_path, fetch_first_model_id,
    merge_oaas_into_continue_json, merge_oaas_into_continue_yaml, resolve_dir_under_home,
    spawn_editor_open_folder, ContinueIdeStatus,
};
use crate::error::OaasError;

#[derive(Deserialize)]
pub struct ApplyContinueBody {
    /// Si absent ou vide, interroge llama-server (upstream OAAS) via `GET /v1/models`.
    pub model: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenFolderBody {
    pub path: String,
    /// `code` | `cursor` | `codium`
    pub editor: String,
}

pub async fn get_continue_status(
    State(state): State<AppState>,
) -> Result<Json<ContinueIdeStatus>, OaasError> {
    let s = build_continue_ide_status(&state.status.api_base_url)?;
    Ok(Json(s))
}

pub async fn post_apply_continue(
    State(state): State<AppState>,
    Json(body): Json<ApplyContinueBody>,
) -> Result<Json<serde_json::Value>, OaasError> {
    let api = state.status.api_base_url.clone();
    let model_id = if let Some(m) = body.model.filter(|s| !s.is_empty()) {
        m
    } else {
        fetch_first_model_id(&state.proxy.client, &state.proxy.upstream).await?
    };
    let path = continue_resolve_write_path()?;
    let msg = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => merge_oaas_into_continue_json(&path, &api, &model_id)?,
        _ => merge_oaas_into_continue_yaml(&path, &api, &model_id)?,
    };
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": msg,
        "model": model_id,
        "continue_config": path.display().to_string(),
    })))
}

pub async fn post_open_folder(
    Json(body): Json<OpenFolderBody>,
) -> Result<Json<serde_json::Value>, OaasError> {
    let folder = resolve_dir_under_home(&body.path)?;
    let launched = spawn_editor_open_folder(&body.editor, &folder).await?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": launched,
        "folder": folder.display().to_string(),
    })))
}
