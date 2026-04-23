//! Résumé machine / dépôt pour `oaas status` et `make status`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{default_config_path, load_config};
use crate::continue_ide::{
    continue_global_json_path, continue_global_yaml_path, continue_json_has_oaas_block,
    continue_yaml_has_oaas_block,
};
use crate::runtime::resolve_llama_binary;
use crate::serve_dashboard::client_facing_host;
use serde::Serialize;

/// Exposé aussi en `GET /oaas/workstation.json` quand le serveur tourne.
#[derive(Debug, Serialize, Clone)]
pub struct WorkstationStatus {
    pub config_path: String,
    pub config_present: bool,
    pub config_parse_ok: bool,
    pub profile: String,
    pub profile_found: bool,
    pub model_path: Option<String>,
    pub model_file_ok: bool,
    pub llama_binary_ok: bool,
    pub llama_binary_note: Option<String>,
    pub llmlingua_enabled: bool,
    pub llmlingua_ok: bool,
    pub llmlingua_note: Option<String>,
    pub oaas_bind: Option<String>,
    pub tcp_listen_open: bool,
    pub http_oaas_ui_ok: bool,
    pub http_note: Option<String>,
    pub continue_yaml: String,
    pub continue_json: String,
    pub continue_yaml_exists: bool,
    pub continue_json_exists: bool,
    pub continue_preferred: String,
    pub continue_has_oaas: bool,
    pub current_exe: String,
    pub build_kind: String,
    pub cargo_manifest_dir: String,
    pub cargo_fmt_ok: Option<bool>,
    pub cargo_clippy_ok: Option<bool>,
    pub cargo_build_ok: Option<bool>,
}

pub fn manifest_dir() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn classify_exe(exe: &Path) -> String {
    let s = exe.to_string_lossy();
    if s.contains("target/debug") {
        "debug".into()
    } else if s.contains("target/release") {
        "release".into()
    } else {
        "installed_or_other".into()
    }
}

pub async fn gather_workstation_status(
    config_path: Option<PathBuf>,
    profile: &str,
    run_ci: bool,
) -> WorkstationStatus {
    let path = config_path.unwrap_or_else(default_config_path);
    let config_path_str = path.display().to_string();
    let config_present = path.is_file();
    let mut config_parse_ok = false;
    let mut profile_found = false;
    let mut model_path = None::<String>;
    let mut model_file_ok = false;
    let mut llama_binary_ok = false;
    let mut llama_binary_note = None::<String>;
    let mut llmlingua_enabled = false;
    let mut llmlingua_ok = false;
    let mut llmlingua_note = None::<String>;
    let mut oaas_bind = None::<String>;
    let mut tcp_listen_open = false;
    let mut http_oaas_ui_ok = false;
    let mut http_note = None::<String>;

    let (yaml_path, json_path) = match (continue_global_yaml_path(), continue_global_json_path()) {
        (Ok(y), Ok(j)) => (y, j),
        _ => (PathBuf::new(), PathBuf::new()),
    };
    let continue_yaml = yaml_path.display().to_string();
    let continue_json = json_path.display().to_string();
    let continue_yaml_exists = yaml_path.is_file();
    let continue_json_exists = json_path.is_file();
    let continue_preferred = if continue_yaml_exists {
        "yaml"
    } else if continue_json_exists {
        "json"
    } else {
        "none_yet"
    }
    .to_string();

    let exe = std::env::current_exe().unwrap_or_default();
    let current_exe = exe.display().to_string();
    let build_kind = classify_exe(&exe);

    let mut cargo_fmt_ok = None;
    let mut cargo_clippy_ok = None;
    let mut cargo_build_ok = None;

    if config_present {
        if let Ok(cfg) = load_config(&path) {
            config_parse_ok = true;
            llmlingua_enabled = cfg.prompt_compression.enabled;
            if llmlingua_enabled {
                if cfg.prompt_compression.command.is_empty() {
                    llmlingua_note = Some("command vide".into());
                } else {
                    let c0 = cfg.prompt_compression.command[0].clone();
                    let ok = std::process::Command::new(&c0)
                        .arg("-c")
                        .arg("import llmlingua")
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);
                    llmlingua_ok = ok;
                    if !ok {
                        llmlingua_note = Some(format!("import llmlingua via {c0:?}"));
                    }
                }
            } else {
                llmlingua_ok = true;
            }

            match resolve_llama_binary(&cfg.runtime.llama_server_binary) {
                Ok(p) => {
                    llama_binary_ok = true;
                    llama_binary_note = Some(p.display().to_string());
                }
                Err(e) => {
                    llama_binary_note = Some(e.to_string());
                }
            }

            if let Some(prof) = cfg.profiles.get(profile) {
                profile_found = true;
                let mp = prof.model.display().to_string();
                model_file_ok = prof.model.is_file();
                model_path = Some(mp);
            }

            oaas_bind = Some(cfg.server.bind.clone());
            let host = client_facing_host(&cfg.server.bind);
            tcp_listen_open = host
                .parse::<std::net::SocketAddr>()
                .ok()
                .map(|addr| {
                    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(400)).is_ok()
                })
                .unwrap_or(false);

            if tcp_listen_open {
                let base = format!("http://{host}");
                match reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()
                {
                    Ok(client) => match client.get(format!("{base}/oaas/")).send().await {
                        Ok(r) if r.status().is_success() => {
                            http_oaas_ui_ok = true;
                        }
                        Ok(r) => {
                            http_note = Some(format!("GET /oaas/ → {}", r.status()));
                        }
                        Err(e) => {
                            http_note = Some(e.to_string());
                        }
                    },
                    Err(e) => http_note = Some(e.to_string()),
                }
            }
        }
    }

    if llama_binary_note.is_none() {
        match resolve_llama_binary(&None) {
            Ok(p) => {
                llama_binary_ok = true;
                llama_binary_note = Some(p.display().to_string());
            }
            Err(e) => {
                llama_binary_note = Some(e.to_string());
            }
        }
    }

    let api_hint = oaas_bind
        .as_ref()
        .map(|b| format!("http://{}/v1", client_facing_host(b)));
    let api_for_continue = api_hint
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:11435/v1".into());
    let continue_has_oaas = (continue_yaml_exists
        && continue_yaml_has_oaas_block(&yaml_path, &api_for_continue))
        || (continue_json_exists && continue_json_has_oaas_block(&json_path, &api_for_continue));

    if run_ci {
        let dir = manifest_dir();
        cargo_fmt_ok = Some(
            std::process::Command::new("cargo")
                .args(["fmt", "--", "--check"])
                .current_dir(dir)
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        );
        cargo_clippy_ok = Some(
            std::process::Command::new("cargo")
                .args(["clippy", "--all-targets", "--", "-D", "warnings"])
                .current_dir(dir)
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        );
        cargo_build_ok = Some(
            std::process::Command::new("cargo")
                .args(["build"])
                .current_dir(dir)
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        );
    }

    WorkstationStatus {
        config_path: config_path_str,
        config_present,
        config_parse_ok,
        profile: profile.to_string(),
        profile_found,
        model_path,
        model_file_ok,
        llama_binary_ok,
        llama_binary_note,
        llmlingua_enabled,
        llmlingua_ok,
        llmlingua_note,
        oaas_bind,
        tcp_listen_open,
        http_oaas_ui_ok,
        http_note,
        continue_yaml,
        continue_json,
        continue_yaml_exists,
        continue_json_exists,
        continue_preferred,
        continue_has_oaas,
        current_exe,
        build_kind,
        cargo_manifest_dir: manifest_dir().to_string(),
        cargo_fmt_ok,
        cargo_clippy_ok,
        cargo_build_ok,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{classify_exe, manifest_dir};

    #[test]
    fn classify_exe_kinds() {
        assert_eq!(
            classify_exe(Path::new("/home/u/proj/target/debug/oaas")),
            "debug"
        );
        assert_eq!(
            classify_exe(Path::new("/home/u/proj/target/release/oaas")),
            "release"
        );
        assert_eq!(classify_exe(Path::new("/usr/local/bin/oaas")), "installed_or_other");
    }

    #[test]
    fn manifest_dir_is_crate_root() {
        let d = manifest_dir();
        assert!(std::path::Path::new(d).join("Cargo.toml").is_file());
    }
}
