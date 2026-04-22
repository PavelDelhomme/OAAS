//! Intégration IDE : Continue (`~/.continue/config.yaml`) et ouverture de dossier (Code OSS, Cursor, VSCodium).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use tokio::process::Command;

use crate::error::OaasError;

const OAAS_MODEL_NAME: &str = "OAAS (local)";

pub fn continue_global_yaml_path() -> Result<PathBuf, OaasError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        OaasError::Config("HOME non défini — impossible de localiser ~/.continue/config.yaml".into())
    })?;
    Ok(PathBuf::from(home).join(".continue").join("config.yaml"))
}

/// `api_base` doit être l’URL du **proxy OAAS** (…/v1), pas llama-server seul, pour garder LLMLingua si activé.
pub fn merge_oaas_into_continue_yaml(
    config_path: &Path,
    api_base: &str,
    model_id: &str,
) -> Result<String, OaasError> {
    let api_base = api_base.trim_end_matches('/').to_string();
    let mut root = if config_path.is_file() {
        let raw = std::fs::read_to_string(config_path).map_err(|e| {
            OaasError::Backend(format!("lecture {}: {e}", config_path.display()))
        })?;
        serde_yaml::from_str::<YamlValue>(&raw).map_err(|e| {
            OaasError::Config(format!(
                "YAML Continue illisible ({}). Corrige le fichier ou renomme-le avant réessai : {e}",
                config_path.display()
            ))
        })?
    } else {
        YamlValue::Mapping(Mapping::new())
    };

    let map = root.as_mapping_mut().ok_or_else(|| {
        OaasError::Config("Racine de config.yaml Continue doit être un objet (mapping YAML).".into())
    })?;

    let name_k = YamlValue::String("name".into());
    if !map.contains_key(&name_k) {
        map.insert(name_k, YamlValue::String("OAAS".into()));
    }
    let ver_k = YamlValue::String("version".into());
    if !map.contains_key(&ver_k) {
        map.insert(ver_k, YamlValue::String("0.0.1".into()));
    }
    let schema_k = YamlValue::String("schema".into());
    if !map.contains_key(&schema_k) {
        map.insert(schema_k, YamlValue::String("v1".into()));
    }

    let models_k = YamlValue::String("models".into());
    if !map.contains_key(&models_k) {
        map.insert(models_k.clone(), YamlValue::Sequence(Vec::new()));
    }
    let models_val = map.get_mut(&models_k).expect("models key inserted");
    let seq = models_val.as_sequence_mut().ok_or_else(|| {
        OaasError::Config("Champ « models » de Continue doit être une liste.".into())
    })?;

    seq.retain(|item| {
        let Some(m) = item.as_mapping() else {
            return true;
        };
        if m.get(YamlValue::String("name".into()))
            == Some(&YamlValue::String(OAAS_MODEL_NAME.into()))
        {
            return false;
        }
        if let Some(YamlValue::String(ab)) = m.get(YamlValue::String("apiBase".into())) {
            if ab.trim_end_matches('/') == api_base {
                return false;
            }
        }
        true
    });

    let entry = oaas_yaml_model(&api_base, model_id);
    seq.insert(0, entry);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            OaasError::Backend(format!("mkdir {}: {e}", parent.display()))
        })?;
    }
    let out = serde_yaml::to_string(&root)
        .map_err(|e| OaasError::Backend(format!("sérialisation YAML Continue: {e}")))?;
    std::fs::write(config_path, &out).map_err(|e| {
        OaasError::Backend(format!("écriture {}: {e}", config_path.display()))
    })?;

    Ok(format!(
        "Bloc « {} » écrit en tête de « {} » (recharge la fenêtre Continue / VS Code si besoin).",
        OAAS_MODEL_NAME,
        config_path.display()
    ))
}

fn oaas_yaml_model(api_base: &str, model_id: &str) -> YamlValue {
    let mut m = Mapping::new();
    m.insert(
        YamlValue::String("name".into()),
        YamlValue::String(OAAS_MODEL_NAME.into()),
    );
    m.insert(
        YamlValue::String("provider".into()),
        YamlValue::String("openai".into()),
    );
    m.insert(
        YamlValue::String("model".into()),
        YamlValue::String(model_id.to_string()),
    );
    m.insert(
        YamlValue::String("apiBase".into()),
        YamlValue::String(api_base.to_string()),
    );
    m.insert(
        YamlValue::String("apiKey".into()),
        YamlValue::String("local".into()),
    );
    m.insert(
        YamlValue::String("roles".into()),
        YamlValue::Sequence(vec![YamlValue::String("chat".into())]),
    );
    YamlValue::Mapping(m)
}

pub async fn fetch_first_model_id(
    client: &reqwest::Client,
    llama_upstream: &reqwest::Url,
) -> Result<String, OaasError> {
    let url = llama_upstream.join("v1/models").map_err(|e| {
        OaasError::Backend(format!("URL /v1/models upstream: {e}"))
    })?;
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(OaasError::Backend(format!(
            "GET /v1/models upstream → {}",
            resp.status()
        )));
    }
    let v: JsonValue = resp.json().await?;
    let id = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m.get("id"))
        .and_then(|x| x.as_str())
        .map(str::to_string);
    id.ok_or_else(|| {
        OaasError::Backend(
            "Réponse /v1/models sans id exploitable — indique le modèle manuellement depuis l’UI."
                .into(),
        )
    })
}

pub fn continue_yaml_has_oaas_block(config_path: &Path, api_base: &str) -> bool {
    let Ok(raw) = std::fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(YamlValue::Mapping(map)) = serde_yaml::from_str::<YamlValue>(&raw) else {
        return false;
    };
    let Some(YamlValue::Sequence(seq)) = map.get(YamlValue::String("models".into())) else {
        return false;
    };
    let api_base = api_base.trim_end_matches('/');
    seq.iter().any(|item| {
        let Some(m) = item.as_mapping() else {
            return false;
        };
        m.get(YamlValue::String("name".into())) == Some(&YamlValue::String(OAAS_MODEL_NAME.into()))
            || m.get(YamlValue::String("apiBase".into()))
                .and_then(|v| v.as_str())
                .map(|s| s.trim_end_matches('/') == api_base)
                .unwrap_or(false)
    })
}

pub fn resolve_dir_under_home(path_str: &str) -> Result<PathBuf, OaasError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        OaasError::Config("HOME non défini — ouverture de dossier refusée.".into())
    })?;
    let home = PathBuf::from(&home);
    let expanded = if let Some(rest) = path_str.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path_str)
    };
    let meta = std::fs::metadata(&expanded).map_err(|e| {
        OaasError::Config(format!("chemin projet « {} » : {e}", expanded.display()))
    })?;
    if !meta.is_dir() {
        return Err(OaasError::Config(format!(
            "« {} » n’est pas un répertoire.",
            expanded.display()
        )));
    }
    let canon = expanded.canonicalize().map_err(|e| {
        OaasError::Config(format!("canonicalize « {} » : {e}", expanded.display()))
    })?;
    let home_canon = home.canonicalize().map_err(|e| {
        OaasError::Config(format!("canonicalize HOME : {e}"))
    })?;
    if !canon.starts_with(&home_canon) {
        return Err(OaasError::Config(
            "Pour des raisons de sécurité, seuls les dossiers sous $HOME sont acceptés.".into(),
        ));
    }
    Ok(canon)
}

pub async fn spawn_editor_open_folder(editor: &str, folder: &Path) -> Result<String, OaasError> {
    let bin = match editor {
        "code" => "code",
        "cursor" => "cursor",
        "codium" => "codium",
        other => {
            return Err(OaasError::Config(format!(
                "éditeur « {other} » inconnu — utilise code, cursor ou codium"
            )));
        }
    };
    if which::which(bin).is_err() {
        return Err(OaasError::Backend(format!(
            "exécutable « {bin} » introuvable dans le PATH — installe VS Code / Cursor / VSCodium ou ajoute la commande shell."
        )));
    }
    let st = Command::new(bin)
        .arg(folder.as_os_str())
        .status()
        .await
        .map_err(|e| OaasError::Backend(format!("lancement {bin}: {e}")))?;
    if !st.success() {
        return Err(OaasError::Backend(format!(
            "{bin} a retourné un code d’erreur — vérifie la sortie du terminal."
        )));
    }
    Ok(format!("{} « {} »", bin, folder.display()))
}

/// Réponse JSON pour `GET /oaas/ide/continue-status`.
#[derive(serde::Serialize)]
pub struct ContinueIdeStatus {
    pub continue_yaml: String,
    pub yaml_exists: bool,
    pub has_oaas_block: bool,
    pub api_base_target: String,
    pub editors: BTreeMap<String, bool>,
}

pub fn build_continue_ide_status(api_base_target: &str) -> Result<ContinueIdeStatus, OaasError> {
    let p = continue_global_yaml_path()?;
    let yaml_exists = p.is_file();
    let has_oaas_block = yaml_exists && continue_yaml_has_oaas_block(&p, api_base_target);
    let mut editors = BTreeMap::new();
    for (k, bin) in [("code", "code"), ("cursor", "cursor"), ("codium", "codium")] {
        editors.insert(k.to_string(), which::which(bin).is_ok());
    }
    Ok(ContinueIdeStatus {
        continue_yaml: p.display().to_string(),
        yaml_exists,
        has_oaas_block,
        api_base_target: api_base_target.to_string(),
        editors,
    })
}
