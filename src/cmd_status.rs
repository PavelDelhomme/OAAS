use std::path::PathBuf;

use clap::Args;

use crate::doctor::LLMLINGUA_REMEDIATION_FR;
use crate::error::OaasError;
use crate::project_status::gather_workstation_status;

#[derive(Args)]
pub struct StatusCli {
    /// Sortie JSON (pour scripts / monitoring).
    #[arg(long)]
    pub json: bool,
    /// Lance aussi `cargo fmt --check`, `clippy`, `build` dans le dépôt (plus lent).
    #[arg(long)]
    pub ci: bool,
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[arg(long, default_value = "default", env = "OAAS_PROFILE")]
    pub profile: String,
}

pub async fn run_status(args: StatusCli) -> Result<(), OaasError> {
    let r = gather_workstation_status(args.config, &args.profile, args.ci).await;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&r)
                .map_err(|e| OaasError::Backend(format!("json: {e}")))?
        );
    } else {
        print_human(&r);
    }
    Ok(())
}

fn print_human(r: &crate::project_status::WorkstationStatus) {
    println!("OAAS — statut poste de travail\n");

    if let (Some(ui), Some(api)) = (r.oaas_ui_url.as_deref(), r.api_base_url.as_deref()) {
        println!("  ▶ Où est l’interface ?");
        println!("     Tableau de bord (UI web) : {ui}");
        println!("     API OpenAI-like          : {api}   (Continue, clients /v1)");
        if let Some(ref bind) = r.oaas_bind {
            println!(
                "     Écoute (server.bind)     : {bind}   → lancer : make serve  ou  oaas serve"
            );
        }
        println!(
            "     Rappel : pas d’éditeur intégré ici — l’UI configure Continue et le proxy ; tu codes dans Cursor/VS Code + extension Continue."
        );
        println!();
    } else if !r.config_present {
        println!(
            "  ▶ Pas d’URL tant que la config n’existe pas →  make init-config  puis  make serve\n"
        );
    } else if !r.config_parse_ok {
        println!(
            "  ▶ Corrige le YAML {} pour afficher les URL (server.bind, etc.).\n",
            r.config_path
        );
    }

    println!(
        "  Config YAML     : {} {}",
        r.config_path,
        flag(r.config_present)
    );
    println!(
        "  Parse config    : {}",
        if r.config_parse_ok { "✓" } else { "✗" }
    );
    println!(
        "  Profil « {} »  : {}",
        r.profile,
        if r.profile_found { "✓" } else { "✗" }
    );
    if let Some(ref m) = r.model_path {
        println!("  Fichier .gguf   : {} {}", m, flag(r.model_file_ok));
    } else {
        println!("  Fichier .gguf   : — (profil introuvable)");
    }
    println!(
        "  llama-server    : {}",
        if r.llama_binary_ok {
            format!("✓ {}", r.llama_binary_note.as_deref().unwrap_or(""))
        } else {
            format!("✗ {}", r.llama_binary_note.as_deref().unwrap_or("?"))
        }
    );
    if r.llmlingua_enabled {
        println!(
            "  LLMLingua       : {} {}",
            if r.llmlingua_ok { "✓" } else { "✗" },
            r.llmlingua_note.as_deref().unwrap_or("")
        );
        if !r.llmlingua_ok {
            println!("    → {}", LLMLINGUA_REMEDIATION_FR);
        }
    } else {
        println!("  LLMLingua       : — (désactivé)");
    }
    if let Some(ref b) = r.oaas_bind {
        println!("  server.bind     : {b}");
        println!(
            "  Port OAAS (TCP) : {}",
            if r.tcp_listen_open {
                "✓ (processus qui écoute — souvent OAAS après make serve)"
            } else {
                "✗ (rien n’écoute — lance « oaas serve » ou « make serve »)"
            }
        );
        println!(
            "  GET /oaas/      : {}",
            if r.http_oaas_ui_ok {
                "✓ (UI répond)"
            } else {
                "✗"
            }
        );
        if let Some(ref n) = r.http_note {
            println!("    → {n}");
        }
    } else {
        println!("  Serveur OAAS    : — (config absente ou invalide)");
    }
    println!(
        "  Continue YAML   : {} {}",
        r.continue_yaml,
        flag(r.continue_yaml_exists)
    );
    println!(
        "  Continue JSON   : {} {}",
        r.continue_json,
        flag(r.continue_json_exists)
    );
    println!(
        "  Config Continue : préférence « {} »",
        r.continue_preferred
    );
    println!(
        "  Bloc OAAS       : {}",
        if r.continue_has_oaas {
            "✓ (détecté)"
        } else {
            "✗ (absent — UI /oaas/ → Continue)"
        }
    );
    println!("  Binaire courant : [{}] {}", r.build_kind, r.current_exe);
    println!("  Dépôt (CI)      : {}", r.cargo_manifest_dir);
    if let (Some(fmt), Some(cl), Some(bd)) =
        (&r.cargo_fmt_ok, &r.cargo_clippy_ok, &r.cargo_build_ok)
    {
        println!("  cargo fmt       : {}", flag(*fmt));
        println!("  cargo clippy    : {}", flag(*cl));
        println!("  cargo build     : {}", flag(*bd));
    }

    if !r.config_present {
        println!(
            "\n  ▶ Prochaine étape : crée la config OAAS avec   make init-config   ou   oaas init-config"
        );
        println!(
            "    Ensuite : chemin .gguf dans le YAML, ou make models-recommend puis make models-pull ID=<id> (ou oaas models … si installé)."
        );
    } else if !r.config_parse_ok {
        println!(
            "\n  ▶ Le fichier {} existe mais le YAML est invalide — corrige-le (indentation, clés).",
            r.config_path
        );
    } else if !r.model_file_ok {
        println!(
            "\n  ▶ Fichier .gguf manquant ou chemin incorrect dans le YAML — télécharge un modèle ou mets à jour profiles.<profil>.model."
        );
        println!(
            "    Depuis le dépôt : make models-recommend puis make models-pull ID=<id-du-catalogue>."
        );
    }

    println!("\nAstuce : make dev  (logs)  |  make doctor  (diagnostic détaillé)");
}

fn flag(ok: bool) -> &'static str {
    if ok {
        "✓"
    } else {
        "✗"
    }
}
