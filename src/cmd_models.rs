use std::path::PathBuf;

use clap::Subcommand;
use tracing::info;

use crate::config::{default_config_path, patch_profile_model_path};
use crate::download::download_url_to_file;
use crate::error::OaasError;
use crate::models_catalog::{
    find_model, load_models_catalog, models_cache_dir, recommend_picks, CatalogModel,
    ModelsCatalogRoot,
};
use crate::proxy::build_client;

#[derive(Subcommand)]
pub enum ModelsCommand {
    /// Liste les modèles du catalogue (filtre optionnel par usage).
    List {
        #[arg(long, help = "Filtrer : dev, docs, diagnostic, general")]
        usage: Option<String>,
        /// Affiche seulement les recommandations textuelles.
        #[arg(long)]
        recommend: bool,
    },
    /// Détail d’un modèle (URL HF, taille, rôle).
    Info { id: String },
    /// Télécharge le GGUF dans le cache OAAS (XDG) ou --dir.
    Pull {
        id: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Met à jour profiles.<profil>.model (réécrit le YAML, commentaires non garantis).
        #[arg(long)]
        patch_config: bool,
        /// Fichier de configuration à patcher (défaut : $XDG_CONFIG_HOME/oaas/config.yaml).
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(long, default_value = "default")]
        profile: String,
        /// Réécrit le fichier même si le modèle existe déjà.
        #[arg(long)]
        force: bool,
    },
    /// Chemin attendu du GGUF pour un id du catalogue (après pull).
    Path {
        id: String,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Synthèse : Instruct vs Coder et picks selon le profil machine.
    Recommend,
}

pub async fn run_models(cmd: ModelsCommand) -> Result<(), OaasError> {
    let catalog = load_models_catalog()?;
    match cmd {
        ModelsCommand::List { usage, recommend } => {
            if recommend {
                print_recommendations(&catalog);
                return Ok(());
            }
            list_models(&catalog, usage.as_deref());
        }
        ModelsCommand::Info { id } => {
            let m = find_model(&catalog, &id).ok_or_else(|| {
                OaasError::Config(format!("modèle « {id} » inconnu du catalogue"))
            })?;
            print_model_detail(m);
        }
        ModelsCommand::Pull {
            id,
            dir,
            patch_config,
            config,
            profile,
            force,
        } => {
            let m = find_model(&catalog, &id).ok_or_else(|| {
                OaasError::Config(format!("modèle « {id} » inconnu du catalogue"))
            })?;
            let dest = pull_dest(&dir, m);
            if dest.exists() && !force {
                info!(path = %dest.display(), "fichier déjà présent (utilise --force pour retélécharger)");
            } else {
                let url = m.huggingface_download_url();
                info!(%url, dest = %dest.display(), "téléchargement (reprise si fichier .part déjà présent)");
                let client = build_client()?;
                download_url_to_file(&client, &url, &dest).await?;
                info!(path = %dest.display(), "téléchargement terminé");
            }
            println!("{}", dest.display());
            if patch_config {
                let cfg = config.unwrap_or_else(default_config_path);
                patch_profile_model_path(&cfg, &profile, &dest)?;
                println!(
                    "Profil « {profile} » mis à jour dans {} (champ model).",
                    cfg.display()
                );
            }
        }
        ModelsCommand::Path { id, dir } => {
            let m = find_model(&catalog, &id).ok_or_else(|| {
                OaasError::Config(format!("modèle « {id} » inconnu du catalogue"))
            })?;
            let dest = pull_dest(&dir, m);
            println!("{}", dest.display());
        }
        ModelsCommand::Recommend => print_recommendations(&catalog),
    }
    Ok(())
}

fn pull_dest(dir: &Option<PathBuf>, m: &CatalogModel) -> PathBuf {
    let base = dir.clone().unwrap_or_else(models_cache_dir);
    base.join(&m.filename)
}

fn list_models(catalog: &ModelsCatalogRoot, usage: Option<&str>) {
    println!("Catalogue OAAS (v{}) — modèles :\n", catalog.version);
    for m in &catalog.models {
        if let Some(u) = usage {
            let u = u.to_lowercase();
            if !m.usage.iter().any(|x| x.to_lowercase() == u) {
                continue;
            }
        }
        println!(
            "  {:<22} [{}] {} (~{} Mo)",
            m.id, m.kind, m.label, m.approx_size_mb
        );
    }
    println!("\nCommandes : oaas models info <id> | oaas models pull <id> | oaas models path <id>");
}

fn print_model_detail(m: &CatalogModel) {
    println!("{}\n", m.label);
    println!("id          : {}", m.id);
    println!("famille     : {} ({})", m.family, m.kind);
    println!("usage       : {}", m.usage.join(", "));
    println!("taille ~    : {} Mo", m.approx_size_mb);
    println!("URL HF      : {}", m.huggingface_download_url());
    println!("\n{}", m.note_fr);
}

fn print_recommendations(catalog: &ModelsCatalogRoot) {
    println!(
        "Recommandations OAAS (Instruct vs Coder)\n\n\
         • Instruct : meilleur « couteau suisse » pour spec, documentation longue, explications, \
           organisation de projet.\n\
         • Coder : meilleur pour génération / relecture de code, patches, navigation d’arbre source.\n\
         • Pour un poste modeste : commence avec un Qwen3.5 4B Instruct (polyvalent).\n\
         • Pour du dev sérieux + machine correcte : ajoute un Qwen2.5 Coder 7B pour la partie code.\n\
         • Grosse machine : Qwen3.5 35B-A3B (très lourd) pour tout regrouper.\n"
    );
    println!("Picks du catalogue :\n");
    for p in recommend_picks(catalog) {
        println!("  • {} : {} — {}", p.description_fr, p.model_id, p.label);
    }
    println!("\nVoir : oaas models list | oaas models list --usage dev");
}
