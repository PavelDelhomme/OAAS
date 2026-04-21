use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct RootInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub endpoints: RootEndpoints,
    pub gguf: &'static str,
    pub ollama: &'static str,
    pub llmlingua: &'static str,
}

#[derive(Serialize)]
pub struct RootEndpoints {
    pub openai_compatible: &'static str,
    pub models: &'static str,
    pub chat: &'static str,
}

pub async fn root() -> Json<RootInfo> {
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
    })
}
