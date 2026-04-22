use axum::extract::State;
use axum::response::Html;
use axum::Json;
use serde::Serialize;

use crate::app_state::AppState;
use crate::models_catalog::ModelsCatalogRoot;
use crate::serve_dashboard::OaasStatus;

#[derive(Serialize)]
pub struct RootInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub endpoints: RootEndpoints,
    pub gguf: &'static str,
    pub ollama: &'static str,
    pub llmlingua: &'static str,
    pub catalog_entries: usize,
    pub models_ui: &'static str,
    pub models_catalog_json: &'static str,
    pub oaas_status_json: &'static str,
    pub oaas_ide_continue_status: &'static str,
    pub oaas_ide_apply_continue: &'static str,
    pub oaas_ide_open_folder: &'static str,
}

#[derive(Serialize)]
pub struct RootEndpoints {
    pub openai_compatible: &'static str,
    pub models: &'static str,
    pub chat: &'static str,
}

pub async fn root(State(state): State<AppState>) -> Json<RootInfo> {
    Json(RootInfo {
        service: "OAAS",
        version: env!("CARGO_PKG_VERSION"),
        endpoints: RootEndpoints {
            openai_compatible: "Toutes les routes /v1/* sont proxifiées vers llama-server.",
            models: "GET /v1/models",
            chat: "POST /v1/chat/completions (streaming supporté)",
        },
        gguf: "Les modèles locaux sont des fichiers .gguf (poids quantifiés pour llama.cpp). Place le chemin dans profiles.<nom>.model du fichier de configuration.",
        ollama: "OAAS ne parle pas à Ollama : tout passe par llama-server + ce proxy. Tu peux arrêter Ollama si Continue pointe ici.",
        llmlingua: "Si prompt_compression.enabled est true dans le YAML, les POST /v1/chat/completions sont compressés via le worker Python LLMLingua avant llama-server.",
        catalog_entries: state.catalog.models.len(),
        models_ui: "/oaas/",
        models_catalog_json: "/oaas/catalog.json",
        oaas_status_json: "/oaas/status.json",
        oaas_ide_continue_status: "/oaas/ide/continue-status",
        oaas_ide_apply_continue: "/oaas/ide/apply-continue",
        oaas_ide_open_folder: "/oaas/ide/open-folder",
    })
}

pub async fn oaas_catalog(State(state): State<AppState>) -> Json<ModelsCatalogRoot> {
    Json((*state.catalog).clone())
}

pub async fn oaas_status(State(state): State<AppState>) -> Json<OaasStatus> {
    Json((*state.status).clone())
}

pub async fn oaas_ui() -> Html<&'static str> {
    Html(include_str!("../static/oaas_ui.html"))
}
