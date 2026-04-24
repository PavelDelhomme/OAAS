use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::OaasError;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub server: ServerSection,
    pub runtime: RuntimeSection,
    /// Défaut : désactivé (`PromptCompressionConfig::default`) si la clé est absente du YAML.
    #[serde(default)]
    pub prompt_compression: PromptCompressionConfig,
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSection {
    pub bind: String,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeSection {
    pub llama_server_binary: Option<String>,
}

/// Compression automatique des prompts (Microsoft LLMLingua) avant envoi à llama-server.
#[derive(Debug, Deserialize, Clone)]
pub struct PromptCompressionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default = "default_pc_timeout")]
    pub timeout_secs: u64,
    /// Si vrai, une erreur LLMLingua fait échouer la requête ; si faux, on envoie le prompt non compressé.
    #[serde(default)]
    pub strict: bool,
    #[serde(default = "default_pc_rate")]
    pub rate: f64,
    /// Si > 0, passé à compress_prompt en plus de `rate`.
    #[serde(default)]
    pub target_token: u32,
    #[serde(default = "default_pc_model")]
    pub model_name: String,
    #[serde(default = "default_pc_use2")]
    pub use_llmlingua2: bool,
    #[serde(default = "default_pc_device")]
    pub device_map: String,
}

fn default_pc_timeout() -> u64 {
    180
}

fn default_pc_rate() -> f64 {
    0.5
}

fn default_pc_model() -> String {
    "microsoft/llmlingua-2-bert-base-multilingual-cased-meetingbank".to_string()
}

fn default_pc_use2() -> bool {
    true
}

fn default_pc_device() -> String {
    "cpu".to_string()
}

impl Default for PromptCompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: Vec::new(),
            timeout_secs: default_pc_timeout(),
            strict: true,
            rate: default_pc_rate(),
            target_token: 0,
            model_name: default_pc_model(),
            use_llmlingua2: default_pc_use2(),
            device_map: default_pc_device(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Profile {
    pub model: PathBuf,
    pub internal_port: u16,
    pub ctx_size: u32,
    pub n_gpu_layers: i32,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

pub fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".config")
        });
    base.join("oaas").join("config.yaml")
}

pub fn load_config(path: &Path) -> Result<ConfigFile, OaasError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| OaasError::Config(format!("impossible de lire {}: {e}", path.display())))?;
    serde_yaml::from_str(&raw)
        .map_err(|e| OaasError::Config(format!("YAML invalide dans {}: {e}", path.display())))
}

/// Met à jour `profiles.<profil>.model` dans un YAML (réécriture complète du fichier).
pub fn patch_profile_model_path(
    config_path: &Path,
    profile: &str,
    model_path: &Path,
) -> Result<(), OaasError> {
    let raw = std::fs::read_to_string(config_path).map_err(|e| {
        OaasError::Config(format!("impossible de lire {}: {e}", config_path.display()))
    })?;
    let mut v: serde_yaml::Value = serde_yaml::from_str(&raw).map_err(|e| {
        OaasError::Config(format!("YAML invalide dans {}: {e}", config_path.display()))
    })?;
    let profiles = v
        .get_mut("profiles")
        .and_then(|x| x.as_mapping_mut())
        .ok_or_else(|| {
            OaasError::Config(format!(
                "clef « profiles » manquante ou invalide dans {}",
                config_path.display()
            ))
        })?;
    let key = serde_yaml::Value::String(profile.to_string());
    if !profiles.contains_key(&key) {
        return Err(OaasError::Config(format!(
            "profil « {profile} » absent de {}",
            config_path.display()
        )));
    }
    let prof = profiles.get_mut(&key).ok_or_else(|| {
        OaasError::Config(format!("profil « {profile} » introuvable après contrôle"))
    })?;
    let pmap = prof.as_mapping_mut().ok_or_else(|| {
        OaasError::Config(format!("profil « {profile} » : attendu un mapping YAML"))
    })?;
    pmap.insert(
        serde_yaml::Value::String("model".to_string()),
        serde_yaml::Value::String(model_path.to_string_lossy().into_owned()),
    );
    let out = serde_yaml::to_string(&v)
        .map_err(|e| OaasError::Config(format!("sérialisation YAML: {e}")))?;
    std::fs::write(config_path, out)
        .map_err(|e| OaasError::Config(format!("écriture {}: {e}", config_path.display())))?;
    Ok(())
}

/// Met à jour `prompt_compression.enabled` dans le YAML (réécriture complète du fichier).
pub fn patch_prompt_compression_enabled(
    config_path: &Path,
    enabled: bool,
) -> Result<(), OaasError> {
    let raw = std::fs::read_to_string(config_path).map_err(|e| {
        OaasError::Config(format!("impossible de lire {}: {e}", config_path.display()))
    })?;
    let mut v: serde_yaml::Value = serde_yaml::from_str(&raw).map_err(|e| {
        OaasError::Config(format!("YAML invalide dans {}: {e}", config_path.display()))
    })?;
    let root = v
        .as_mapping_mut()
        .ok_or_else(|| OaasError::Config("racine YAML : attendu un mapping".into()))?;
    let pc_key = serde_yaml::Value::String("prompt_compression".into());
    if !root.contains_key(&pc_key) {
        root.insert(
            pc_key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
    let pc_node = root
        .get_mut(&pc_key)
        .ok_or_else(|| OaasError::Config("prompt_compression : entrée YAML introuvable".into()))?;
    let m = pc_node
        .as_mapping_mut()
        .ok_or_else(|| OaasError::Config("prompt_compression : attendu un mapping YAML".into()))?;
    m.insert(
        serde_yaml::Value::String("enabled".into()),
        serde_yaml::Value::Bool(enabled),
    );
    let out = serde_yaml::to_string(&v)
        .map_err(|e| OaasError::Config(format!("sérialisation YAML: {e}")))?;
    std::fs::write(config_path, out)
        .map_err(|e| OaasError::Config(format!("écriture {}: {e}", config_path.display())))?;
    Ok(())
}
