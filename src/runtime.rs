use std::path::PathBuf;

use crate::error::OaasError;

pub fn resolve_llama_binary(cfg_path: &Option<String>) -> Result<PathBuf, OaasError> {
    if let Some(p) = cfg_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
        return Err(OaasError::Config(format!(
            "binaire llama-server introuvable (config): {}",
            pb.display()
        )));
    }
    which::which("llama-server").map_err(|_| {
        OaasError::Config(
            "binaire « llama-server » introuvable dans le PATH — installe llama.cpp ou renseigne runtime.llama_server_binary dans le YAML".into(),
        )
    })
}
