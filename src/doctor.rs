use std::path::PathBuf;

use crate::config::{default_config_path, load_config};
use crate::runtime::resolve_llama_binary;

pub fn run_doctor(config_path: Option<PathBuf>, profile: String) {
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
            if std::process::Command::new(&pc.command[0])
                .arg("-c")
                .arg("import llmlingua")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                println!("✓ module Python « llmlingua » importable");
            } else {
                println!(
                    "✗ impossible d’importer llmlingua avec {:?} — pip install -r scripts/requirements-llmlingua.txt (voir README)",
                    pc.command[0]
                );
            }
        }
    } else {
        println!("• LLMLingua : désactivé (prompt_compression.enabled: false)");
    }

    let profile = match cfg.profiles.get(&profile) {
        Some(p) => p,
        None => {
            println!(
                "✗ profil « {profile} » introuvable — disponibles : {:?}",
                cfg.profiles.keys().collect::<Vec<_>>()
            );
            return;
        }
    };

    if profile.model.exists() {
        println!("✓ modèle GGUF : {}", profile.model.display());
    } else {
        println!(
            "✗ modèle introuvable : {}\n  Un fichier .gguf contient les poids du réseau (format llama.cpp). Télécharge-en un (ex. Hugging Face) puis mets le chemin absolu dans le YAML.",
            profile.model.display()
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
    println!("Documentation : oaas docs list | oaas docs pull <id>");
}
