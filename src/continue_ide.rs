//! Intégration IDE : Continue (`~/.continue/config.yaml`) et ouverture de dossier (Code OSS, Cursor, VSCodium).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use tokio::process::Command;

use crate::error::OaasError;

const OAAS_MODEL_NAME: &str = "OAAS (local)";

/// Dossier global `~/.continue`.
pub fn continue_global_dir() -> Result<PathBuf, OaasError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        OaasError::Config("HOME non défini — impossible de localiser ~/.continue".into())
    })?;
    Ok(PathBuf::from(home).join(".continue"))
}

pub fn continue_global_yaml_path() -> Result<PathBuf, OaasError> {
    Ok(continue_global_dir()?.join("config.yaml"))
}

pub fn continue_global_json_path() -> Result<PathBuf, OaasError> {
    Ok(continue_global_dir()?.join("config.json"))
}

/// `…/projet/.continue` (config Continue **par workspace**).
pub fn workspace_continue_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".continue")
}

/// Règle Continue amont : si `config.yaml` et `config.json` existent, le YAML est prioritaire pour l’écriture cible.
pub fn pick_continue_config_path(continue_dir: &Path) -> PathBuf {
    let yaml = continue_dir.join("config.yaml");
    let json = continue_dir.join("config.json");
    if yaml.is_file() {
        yaml
    } else if json.is_file() {
        json
    } else {
        yaml
    }
}

/// Continue charge `config.yaml` à la place de `config.json` si les deux existent (doc amont).
pub fn continue_resolve_write_path() -> Result<PathBuf, OaasError> {
    Ok(pick_continue_config_path(&continue_global_dir()?))
}

/// Fichier à fusionner : global `~/.continue` ou `<workspace>/.continue` si `workspace_root` est fourni.
pub fn resolve_continue_write_path(workspace_root: Option<&Path>) -> Result<PathBuf, OaasError> {
    match workspace_root {
        Some(w) => Ok(pick_continue_config_path(&workspace_continue_dir(w))),
        None => continue_resolve_write_path(),
    }
}

fn backup_existing_config(path: &Path) -> Result<(), OaasError> {
    if !path.is_file() {
        return Ok(());
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".into());
    let parent = path.parent().unwrap_or(Path::new("."));
    let bak = parent.join(format!("{name}.oaas-backup.{ts}"));
    std::fs::copy(path, &bak).map_err(|e| {
        OaasError::Backend(format!(
            "sauvegarde Continue {} → {} : {e}",
            path.display(),
            bak.display()
        ))
    })?;
    Ok(())
}

/// `api_base` doit être l’URL du **proxy OAAS** (…/v1), pas llama-server seul, pour garder LLMLingua si activé.
pub fn merge_oaas_into_continue_yaml(
    config_path: &Path,
    api_base: &str,
    model_id: &str,
) -> Result<String, OaasError> {
    backup_existing_config(config_path)?;
    let api_base = api_base.trim_end_matches('/').to_string();
    let mut root = if config_path.is_file() {
        let raw = std::fs::read_to_string(config_path)
            .map_err(|e| OaasError::Backend(format!("lecture {}: {e}", config_path.display())))?;
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
        OaasError::Config(
            "Racine de config.yaml Continue doit être un objet (mapping YAML).".into(),
        )
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
        std::fs::create_dir_all(parent)
            .map_err(|e| OaasError::Backend(format!("mkdir {}: {e}", parent.display())))?;
    }
    let out = serde_yaml::to_string(&root)
        .map_err(|e| OaasError::Backend(format!("sérialisation YAML Continue: {e}")))?;
    std::fs::write(config_path, &out)
        .map_err(|e| OaasError::Backend(format!("écriture {}: {e}", config_path.display())))?;

    Ok(format!(
        "Bloc « {} » écrit en tête de « {} » (sauvegarde .oaas-backup.* si fichier existait). Recharge la fenêtre Continue / VS Code.",
        OAAS_MODEL_NAME,
        config_path.display()
    ))
}

/// Ancien format Continue (`config.json`) : fusion sur le tableau `models` (clés `title` ou `name`).
pub fn merge_oaas_into_continue_json(
    config_path: &Path,
    api_base: &str,
    model_id: &str,
) -> Result<String, OaasError> {
    backup_existing_config(config_path)?;
    let api_base = api_base.trim_end_matches('/').to_string();
    let mut root: JsonValue = if config_path.is_file() {
        let raw = std::fs::read_to_string(config_path)
            .map_err(|e| OaasError::Backend(format!("lecture {}: {e}", config_path.display())))?;
        serde_json::from_str(&raw).map_err(|e| {
            OaasError::Config(format!(
                "JSON Continue illisible ({}): {e}",
                config_path.display()
            ))
        })?
    } else {
        JsonValue::Object(serde_json::Map::new())
    };

    let obj = root.as_object_mut().ok_or_else(|| {
        OaasError::Config("Racine de config.json Continue doit être un objet.".into())
    })?;

    let models = obj
        .entry("models".to_string())
        .or_insert_with(|| JsonValue::Array(Vec::new()));
    let arr = models
        .as_array_mut()
        .ok_or_else(|| OaasError::Config("Champ « models » doit être un tableau JSON.".into()))?;

    arr.retain(|item| {
        let Some(o) = item.as_object() else {
            return true;
        };
        let name_hit = o
            .get("name")
            .and_then(|x| x.as_str())
            .map(|s| s == OAAS_MODEL_NAME)
            .unwrap_or(false)
            || o.get("title")
                .and_then(|x| x.as_str())
                .map(|s| s == OAAS_MODEL_NAME)
                .unwrap_or(false);
        let base_hit = o
            .get("apiBase")
            .and_then(|x| x.as_str())
            .map(|s| s.trim_end_matches('/') == api_base)
            .unwrap_or(false);
        !(name_hit || base_hit)
    });

    let entry = serde_json::json!({
        "title": OAAS_MODEL_NAME,
        "name": OAAS_MODEL_NAME,
        "provider": "openai",
        "model": model_id,
        "apiBase": &api_base,
        "apiKey": "local",
    });
    arr.insert(0, entry);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| OaasError::Backend(format!("mkdir {}: {e}", parent.display())))?;
    }
    let out = serde_json::to_string_pretty(&root)
        .map_err(|e| OaasError::Backend(format!("sérialisation JSON Continue: {e}")))?;
    std::fs::write(config_path, out.as_bytes())
        .map_err(|e| OaasError::Backend(format!("écriture {}: {e}", config_path.display())))?;

    Ok(format!(
        "Entrée « {} » fusionnée dans « {} » (backup .oaas-backup.* si besoin).",
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
    let url = llama_upstream
        .join("v1/models")
        .map_err(|e| OaasError::Backend(format!("URL /v1/models upstream: {e}")))?;
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
    let canon = expanded
        .canonicalize()
        .map_err(|e| OaasError::Config(format!("canonicalize « {} » : {e}", expanded.display())))?;
    let home_canon = home
        .canonicalize()
        .map_err(|e| OaasError::Config(format!("canonicalize HOME : {e}")))?;
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
    /// `global` ou `workspace` (écriture / chemins affichés ci‑dessous).
    pub active_scope: String,
    /// Racine du projet si paramètre `workspace` fourni (GET ou POST).
    pub workspace_root: Option<String>,
    /// Toujours les chemins **globaux** `~/.continue/…` (référence).
    pub global_continue_yaml: String,
    pub global_continue_json: String,
    /// Chemins du périmètre actif : globaux **ou** `<workspace>/.continue/…`.
    pub continue_yaml: String,
    pub continue_json: String,
    pub yaml_exists: bool,
    pub json_exists: bool,
    pub write_target: String,
    pub preferred: String,
    pub has_oaas_block: bool,
    pub api_base_target: String,
    pub editors: BTreeMap<String, bool>,
}

pub fn build_continue_ide_status(
    api_base_target: &str,
    workspace_root: Option<&Path>,
) -> Result<ContinueIdeStatus, OaasError> {
    let g_py = continue_global_yaml_path()?;
    let g_pj = continue_global_json_path()?;
    let global_continue_yaml = g_py.display().to_string();
    let global_continue_json = g_pj.display().to_string();

    let (active_scope, py, pj, yaml_exists, json_exists, write_target, preferred, has_oaas_block) =
        if let Some(ws) = workspace_root {
            let d = workspace_continue_dir(ws);
            let wy = d.join("config.yaml");
            let wj = d.join("config.json");
            let yaml_exists = wy.is_file();
            let json_exists = wj.is_file();
            let write_target = pick_continue_config_path(&d);
            let preferred = if yaml_exists {
                "yaml"
            } else if json_exists {
                "json"
            } else {
                "none_yet_defaults_yaml"
            }
            .to_string();
            let has_oaas_block = (yaml_exists
                && continue_yaml_has_oaas_block(&wy, api_base_target))
                || (json_exists && continue_json_has_oaas_block(&wj, api_base_target));
            (
                "workspace",
                wy,
                wj,
                yaml_exists,
                json_exists,
                write_target,
                preferred,
                has_oaas_block,
            )
        } else {
            let yaml_exists = g_py.is_file();
            let json_exists = g_pj.is_file();
            let write_target = continue_resolve_write_path()?;
            let preferred = if yaml_exists {
                "yaml"
            } else if json_exists {
                "json"
            } else {
                "none_yet_defaults_yaml"
            }
            .to_string();
            let has_oaas_block = (yaml_exists
                && continue_yaml_has_oaas_block(&g_py, api_base_target))
                || (json_exists && continue_json_has_oaas_block(&g_pj, api_base_target));
            (
                "global",
                g_py,
                g_pj,
                yaml_exists,
                json_exists,
                write_target,
                preferred,
                has_oaas_block,
            )
        };

    let mut editors = BTreeMap::new();
    for (k, bin) in [("code", "code"), ("cursor", "cursor"), ("codium", "codium")] {
        editors.insert(k.to_string(), which::which(bin).is_ok());
    }
    Ok(ContinueIdeStatus {
        active_scope: active_scope.to_string(),
        workspace_root: workspace_root.map(|p| p.display().to_string()),
        global_continue_yaml,
        global_continue_json,
        continue_yaml: py.display().to_string(),
        continue_json: pj.display().to_string(),
        yaml_exists,
        json_exists,
        write_target: write_target.display().to_string(),
        preferred,
        has_oaas_block,
        api_base_target: api_base_target.to_string(),
        editors,
    })
}

pub fn continue_json_has_oaas_block(path: &Path, api_base: &str) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<JsonValue>(&raw) else {
        return false;
    };
    let Some(arr) = v.get("models").and_then(|m| m.as_array()) else {
        return false;
    };
    let api_base = api_base.trim_end_matches('/');
    arr.iter().any(|item| {
        let Some(o) = item.as_object() else {
            return false;
        };
        let name_hit = o
            .get("name")
            .and_then(|x| x.as_str())
            .map(|s| s == OAAS_MODEL_NAME)
            .unwrap_or(false)
            || o.get("title")
                .and_then(|x| x.as_str())
                .map(|s| s == OAAS_MODEL_NAME)
                .unwrap_or(false);
        let base_hit = o
            .get("apiBase")
            .and_then(|x| x.as_str())
            .map(|s| s.trim_end_matches('/') == api_base)
            .unwrap_or(false);
        name_hit || base_hit
    })
}
