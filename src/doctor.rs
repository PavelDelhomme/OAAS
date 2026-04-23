use std::path::PathBuf;

use crate::config::{default_config_path, load_config, patch_prompt_compression_enabled};
use crate::runtime::resolve_llama_binary;

fn llmlingua_import_ok(pc: &crate::config::PromptCompressionConfig) -> bool {
    if pc.command.is_empty() {
        return false;
    }
    std::process::Command::new(&pc.command[0])
        .arg("-c")
        .arg("import llmlingua")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `true` si un argument se terminant par `.py` est absent du disque (chemin relatif ou erroné).
fn llmlingua_script_missing(pc: &crate::config::PromptCompressionConfig) -> bool {
    pc.command
        .iter()
        .filter(|s| s.ends_with(".py"))
        .any(|s| !PathBuf::from(s).exists())
}

fn llmlingua_config_broken(pc: &crate::config::PromptCompressionConfig) -> bool {
    if !pc.enabled {
        return false;
    }
    if pc.command.is_empty() {
        return true;
    }
    if llmlingua_script_missing(pc) {
        return true;
    }
    !llmlingua_import_ok(pc)
}

pub fn run_doctor(config_path: Option<PathBuf>, profile: String, fix: bool) {
    println!("OAAS — diagnostic\n");

    let path = config_path.unwrap_or_else(default_config_path);
    if !path.exists() {
        println!(
            "✗ configuration absente : {}\n  → lance : oaas init-config   ou   make init-config",
            path.display()
        );
        match resolve_llama_binary(&None) {
            Ok(p) => println!("\n(llama-server trouvé dans le PATH : {})", p.display()),
            Err(_) => println!("\n(llama-server absent du PATH — installe llama.cpp.)"),
        }
        return;
    }
    println!("✓ fichier de configuration : {}", path.display());

    let cfg = match load_config(&path) {
        Ok(c) => c,
        Err(e) => {
            println!("✗ lecture / parse YAML : {e}");
            return;
        }
    };

    match resolve_llama_binary(&cfg.runtime.llama_server_binary) {
        Ok(p) => println!("✓ llama-server : {}", p.display()),
        Err(e) => println!("✗ llama-server : {e}"),
    }

    let pc = &cfg.prompt_compression;
    let pc_broken = llmlingua_config_broken(pc);

    if pc.enabled {
        println!("• LLMLingua : activé (compression avant llama-server)");
        if pc.command.is_empty() {
            println!("✗ prompt_compression.command vide");
        } else {
            println!("  commande : {:?}", pc.command);
            if let Some(script) = pc.command.iter().find(|s| s.ends_with(".py")) {
                let p = PathBuf::from(script);
                if p.exists() {
                    println!("✓ script worker : {}", p.display());
                } else {
                    println!(
                        "✗ script worker introuvable : {} — utilise un chemin absolu ou lance depuis la racine du dépôt",
                        p.display()
                    );
                }
            }
            if llmlingua_import_ok(pc) {
                println!("✓ module Python « llmlingua » importable");
            } else {
                println!(
                    "✗ impossible d’importer llmlingua avec {:?} — pip install -r scripts/requirements-llmlingua.txt (voir README)",
                    pc.command[0]
                );
                println!(
                    "  Astuce : make llmlingua-venv dans le dépôt, ou prompt_compression.enabled: false si tu n’utilises pas la compression."
                );
            }
        }
    } else {
        println!("• LLMLingua : désactivé (prompt_compression.enabled: false)");
    }

    let profile_cfg = match cfg.profiles.get(&profile) {
        Some(p) => p,
        None => {
            println!(
                "✗ profil « {profile} » introuvable — disponibles : {:?}",
                cfg.profiles.keys().collect::<Vec<_>>()
            );
            return;
        }
    };

    if profile_cfg.model.exists() {
        println!("✓ modèle GGUF : {}", profile_cfg.model.display());
    } else {
        println!(
            "✗ modèle introuvable : {}\n  Un fichier .gguf contient les poids du réseau (format llama.cpp). Télécharge-en un (ex. Hugging Face) puis mets le chemin absolu dans le YAML.",
            profile_cfg.model.display()
        );
    }

    if fix {
        println!();
        if pc_broken {
            match patch_prompt_compression_enabled(&path, false) {
                Ok(()) => println!(
                    "✓ Correction : prompt_compression.enabled → false dans {}",
                    path.display()
                ),
                Err(e) => println!("✗ impossible d’écrire la correction dans le YAML : {e}"),
            }
            println!("  Relance : make doctor   (le fichier a été réécrit ; les commentaires YAML ne sont pas conservés.)");
        } else {
            println!("• Aucune correction automatique appliquée (LLMLingua déjà OK ou désactivé).");
            println!("  Les autres points (llama-server, chemin .gguf…) ne sont pas modifiés par --fix.");
        }
    } else if pc_broken {
        println!(
            "\n→ Correction automatique :   cargo run -- doctor --fix   ou   make doctor-fix"
        );
    }

    println!(
        "\nRappel GGUF : format de fichier unique pour faire tourner un LLM avec llama.cpp / llama-server (quantifié Q4/Q5/Q8…). Ce n’est pas une « clé API », c’est le modèle sur ton disque."
    );
    println!(
        "\nOAAS remplace Ollama pour ce flux : Continue → URL http://<bind>/v1 (pas de démon Ollama requis)."
    );
    println!(
        "\nContinue : base URL http://<bind>/v1  (ex. http://127.0.0.1:11435/v1 selon server.bind)."
    );
    println!(
        "\nModèles GGUF : oaas models recommend | oaas models list | oaas models pull <id> --patch-config"
    );
    println!(
        "Sans « oaas » dans le PATH (clone du dépôt) : cargo run -- models … ou make models-recommend / make models-pull ID=<id>."
    );
    println!("Documentation : oaas docs list | oaas docs pull <id>   (depuis le dépôt : make docs-list, cargo run -- docs …)");
}
