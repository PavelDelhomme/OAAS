use std::path::Path;

use serde::Serialize;

use crate::config::ConfigFile;
use crate::models_catalog::{recommend_picks, ModelsCatalogRoot, RecommendPick};

#[derive(Debug, Clone, Serialize)]
pub struct OaasProfileRow {
    pub name: String,
    pub model: String,
    pub internal_port: u16,
    pub active: bool,
}

/// Données pour l’UI web et `GET /oaas/status.json` (profils YAML, URL Continue, picks).
#[derive(Debug, Clone, Serialize)]
pub struct OaasStatus {
    pub version: String,
    pub config_path: String,
    pub server_bind: String,
    /// Hôte:port pour les clients (navigateur, Continue) quand `bind` est `0.0.0.0` / `[::]`.
    pub client_host: String,
    pub api_base_url: String,
    pub oaas_ui_url: String,
    pub active_profile: String,
    pub profiles: Vec<OaasProfileRow>,
    pub recommend_picks: Vec<RecommendPick>,
    pub cli_cheatsheet_fr: Vec<String>,
    pub rag_roadmap_fr: String,
}

pub fn client_facing_host(bind: &str) -> String {
    if let Some(port) = bind.strip_prefix("0.0.0.0:") {
        format!("127.0.0.1:{port}")
    } else if let Some(port) = bind.strip_prefix("[::]:") {
        format!("127.0.0.1:{port}")
    } else {
        bind.to_string()
    }
}

pub fn build_oaas_status(
    cfg: &ConfigFile,
    config_path: &Path,
    profile_name: &str,
    catalog: &ModelsCatalogRoot,
) -> OaasStatus {
    let host = client_facing_host(&cfg.server.bind);
    let api_base_url = format!("http://{}/v1", host);
    let oaas_ui_url = format!("http://{}/oaas/", host);
    let profiles: Vec<OaasProfileRow> = cfg
        .profiles
        .iter()
        .map(|(name, p)| OaasProfileRow {
            name: name.clone(),
            model: p.model.display().to_string(),
            internal_port: p.internal_port,
            active: name == profile_name,
        })
        .collect();
    let cli_cheatsheet_fr = vec![
        "oaas status".into(),
        "oaas status --ci".into(),
        "oaas models recommend".into(),
        "oaas models list".into(),
        "oaas models pull <id> --patch-config".into(),
        "oaas doctor".into(),
        "OAAS_PROFILE=autre_profil oaas serve".into(),
        "oaas docs list".into(),
    ];
    OaasStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        config_path: config_path.display().to_string(),
        server_bind: cfg.server.bind.clone(),
        client_host: host,
        api_base_url,
        oaas_ui_url,
        active_profile: profile_name.to_string(),
        profiles,
        recommend_picks: recommend_picks(catalog),
        cli_cheatsheet_fr,
        rag_roadmap_fr: "Feuille de route : index local (embeddings) sur le cache doc + requêtes RAG côté OAAS — pas encore implémenté.".into(),
    }
}
