use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::OaasError;
use crate::models_catalog::docs_cache_dir;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DocsCatalogRoot {
    pub version: u32,
    pub packs: Vec<DocPack>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DocPack {
    pub id: String,
    pub title: String,
    pub method: String,
    /// Vide pour `method: devdocs` (non utilisé).
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub note_fr: String,
    /// Image Docker DevDocs officielle (https://github.com/freeCodeCamp/devdocs).
    #[serde(default)]
    pub docker_image: String, // vide = image par défaut côté OAAS
    /// Uniquement pour `method: devdocs`.
    #[serde(default)]
    pub docker_port: Option<u16>,
}

pub fn load_docs_catalog() -> Result<DocsCatalogRoot, OaasError> {
    if let Ok(p) = env::var("OAAS_DOCS_CATALOG") {
        let pb = PathBuf::from(&p);
        let raw = fs::read_to_string(&pb)
            .map_err(|e| OaasError::Config(format!("OAAS_DOCS_CATALOG {}: {e}", pb.display())))?;
        return serde_yaml::from_str(&raw)
            .map_err(|e| OaasError::Config(format!("catalogue docs YAML: {e}")));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            let installed = bin_dir.join("../share/oaas/data/docs_catalog.yaml");
            if installed.is_file() {
                let raw = fs::read_to_string(&installed).map_err(|e| {
                    OaasError::Config(format!("lecture {}: {e}", installed.display()))
                })?;
                return serde_yaml::from_str(&raw)
                    .map_err(|e| OaasError::Config(format!("catalogue docs YAML: {e}")));
            }
        }
    }
    let cwd = Path::new("data/docs_catalog.yaml");
    if cwd.is_file() {
        let raw = fs::read_to_string(cwd)
            .map_err(|e| OaasError::Config(format!("lecture {}: {e}", cwd.display())))?;
        return serde_yaml::from_str(&raw)
            .map_err(|e| OaasError::Config(format!("catalogue docs YAML: {e}")));
    }
    serde_yaml::from_str(include_str!("../data/docs_catalog.yaml"))
        .map_err(|e| OaasError::Config(format!("catalogue docs YAML: {e}")))
}

pub fn find_doc_pack<'a>(root: &'a DocsCatalogRoot, id: &str) -> Option<&'a DocPack> {
    root.packs.iter().find(|p| p.id == id)
}

pub fn doc_pack_dest(id: &str) -> PathBuf {
    docs_cache_dir().join(id)
}
