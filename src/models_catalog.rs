use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::OaasError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelsCatalogRoot {
    pub version: u32,
    #[serde(default)]
    pub defaults: CatalogDefaults,
    pub models: Vec<CatalogModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CatalogDefaults {
    #[serde(default = "default_balanced_usage")]
    pub balanced_usage: String,
}

fn default_balanced_usage() -> String {
    "dev".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogModel {
    pub id: String,
    pub label: String,
    pub family: String,
    pub kind: String,
    pub hf_repo: String,
    pub filename: String,
    #[serde(default)]
    pub approx_size_mb: u32,
    #[serde(default)]
    pub usage: Vec<String>,
    #[serde(default)]
    pub note_fr: String,
}

impl CatalogModel {
    pub fn huggingface_download_url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/main/{}",
            self.hf_repo, self.filename
        )
    }
}

pub fn oaas_data_dir() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".local")
                .join("share")
        })
        .join("oaas")
}

pub fn models_cache_dir() -> PathBuf {
    oaas_data_dir().join("models")
}

pub fn docs_cache_dir() -> PathBuf {
    oaas_data_dir().join("docs")
}

pub fn load_models_catalog() -> Result<ModelsCatalogRoot, OaasError> {
    if let Ok(p) = env::var("OAAS_MODELS_CATALOG") {
        let pb = PathBuf::from(&p);
        let raw = fs::read_to_string(&pb)
            .map_err(|e| OaasError::Config(format!("OAAS_MODELS_CATALOG {}: {e}", pb.display())))?;
        return parse_models_catalog(&raw);
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let installed = bin_dir.join("../share/oaas/data/models_catalog.yaml");
            if installed.is_file() {
                let raw = fs::read_to_string(&installed).map_err(|e| {
                    OaasError::Config(format!("lecture {}: {e}", installed.display()))
                })?;
                return parse_models_catalog(&raw);
            }
        }
    }
    let cwd_path = Path::new("data/models_catalog.yaml");
    if cwd_path.is_file() {
        let raw = fs::read_to_string(cwd_path)
            .map_err(|e| OaasError::Config(format!("lecture {}: {e}", cwd_path.display())))?;
        return parse_models_catalog(&raw);
    }
    parse_models_catalog(include_str!("../data/models_catalog.yaml"))
}

fn parse_models_catalog(raw: &str) -> Result<ModelsCatalogRoot, OaasError> {
    serde_yaml::from_str(raw).map_err(|e| OaasError::Config(format!("catalogue modèles YAML: {e}")))
}

pub fn find_model<'a>(root: &'a ModelsCatalogRoot, id: &str) -> Option<&'a CatalogModel> {
    root.models.iter().find(|m| m.id == id)
}

/// Picks affichés par `oaas models recommend` et par l’UI web `/oaas/`.
#[derive(Debug, Clone, Serialize)]
pub struct RecommendPick {
    pub key: String,
    pub description_fr: String,
    pub model_id: String,
    pub label: String,
}

pub fn recommend_picks(catalog: &ModelsCatalogRoot) -> Vec<RecommendPick> {
    const ROWS: &[(&str, &str, &str)] = &[
        ("polyvalent", "Polyvalent / laptop", "qwen3.5-4b-q4km"),
        ("code", "Code / outillage", "qwen2.5-coder-7b-q4km"),
        ("doc", "Doc / spec", "qwen2.5-7b-instruct-q4km"),
        ("puissant", "Station puissante", "qwen3.5-35b-a3b-q4km"),
    ];
    ROWS
        .iter()
        .filter_map(|(key, desc, id)| {
            catalog.models.iter().find(|m| m.id == *id).map(|m| RecommendPick {
                key: (*key).to_string(),
                description_fr: (*desc).to_string(),
                model_id: m.id.clone(),
                label: m.label.clone(),
            })
        })
        .collect()
}
