use std::path::PathBuf;

use crate::config::default_config_path;

const EXAMPLE_YAML: &str = include_str!("../config.example.yaml");

pub fn run_init_config(force: bool, dest: Option<PathBuf>) -> Result<(), String> {
    let path = dest.unwrap_or_else(default_config_path);
    if path.exists() && !force {
        return Err(format!(
            "existe déjà : {} — utilise --force pour écraser",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, EXAMPLE_YAML).map_err(|e| format!("écriture {}: {e}", path.display()))?;
    println!("Configuration créée : {}", path.display());
    println!("Édite « profiles.default.model » avec le chemin absolu vers ton fichier .gguf puis lance : oaas serve");
    Ok(())
}
