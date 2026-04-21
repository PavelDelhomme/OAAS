mod backend;
mod config;
mod doctor;
mod error;
mod http_root;
mod init_config;
mod llmlingua;
mod proxy;
mod runtime;

use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::Request;
use axum::response::Response;
use axum::routing::{any, get};
use axum::Router;
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

use crate::backend::{wait_upstream_ready, LlamaBackend};
use crate::config::{default_config_path, load_config};
use crate::error::OaasError;
use crate::http_root::root as root_json;
use crate::proxy::{build_client, forward_request, ProxyState};
use crate::runtime::resolve_llama_binary;

#[derive(Parser)]
#[command(
    name = "oaas",
    about = "Orchestrateur local de LLM (API compatible OpenAI)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Vérifie llama-server, le YAML et la présence du fichier .gguf.
    Doctor {
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(long, default_value = "default", env = "OAAS_PROFILE")]
        profile: String,
    },
    /// Écrit la configuration d’exemple (intégrée) vers ~/.config/oaas/config.yaml.
    InitConfig {
        #[arg(long, help = "Écrase le fichier s’il existe déjà")]
        force: bool,
        #[arg(
            long,
            value_name = "FILE",
            help = "Chemin du fichier à créer (défaut: XDG)"
        )]
        output: Option<PathBuf>,
    },
    /// Lance le démon HTTP et un processus llama-server pour le profil choisi.
    Serve {
        /// Fichier YAML (défaut: $XDG_CONFIG_HOME/oaas/config.yaml)
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Nom du profil dans le fichier de configuration.
        #[arg(long, default_value = "default", env = "OAAS_PROFILE")]
        profile: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor { config, profile } => {
            crate::doctor::run_doctor(config, profile);
        }
        Commands::InitConfig { force, output } => {
            if let Err(e) = crate::init_config::run_init_config(force, output) {
                eprintln!("init-config: {e}");
                std::process::exit(1);
            }
        }
        Commands::Serve { config, profile } => {
            if let Err(e) = run_serve(config, profile).await {
                tracing::error!(error = %e, "arrêt sur erreur");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

async fn run_serve(config_path: Option<PathBuf>, profile_name: String) -> Result<(), OaasError> {
    let path = config_path.unwrap_or_else(default_config_path);
    if !path.exists() {
        return Err(OaasError::Config(format!(
            "fichier de configuration absent: {} — copie config.example.yaml vers ce chemin.",
            path.display()
        )));
    }

    let cfg = load_config(&path)?;
    let profile = cfg
        .profiles
        .get(&profile_name)
        .ok_or_else(|| {
            OaasError::Config(format!(
                "profil « {profile_name} » introuvable — profils disponibles: {:?}",
                cfg.profiles.keys().collect::<Vec<_>>()
            ))
        })?
        .clone();

    let llama_bin = resolve_llama_binary(&cfg.runtime.llama_server_binary)?;
    let _backend = LlamaBackend::spawn(&llama_bin, &profile).await?;

    let upstream_base = format!("http://127.0.0.1:{}", profile.internal_port);
    wait_upstream_ready(&upstream_base, Duration::from_secs(120)).await?;

    let upstream_url = Url::parse(&upstream_base)
        .map_err(|e| OaasError::Config(format!("URL interne invalide: {e}")))?;

    let pc = cfg.prompt_compression.clone();
    let (llmlingua, prompt_compression) = if pc.enabled {
        if pc.command.is_empty() {
            return Err(OaasError::Config(
                "prompt_compression.enabled est true mais command est vide — remplis la liste (python + worker)".into(),
            ));
        }
        let client_ling = crate::llmlingua::LlmLinguaClient::spawn(&pc).await?;
        info!(
            model = %pc.model_name,
            llmlingua2 = pc.use_llmlingua2,
            "worker LLMLingua prêt (compression des prompts activée)"
        );
        (Some(client_ling), Some(pc))
    } else {
        (None, None)
    };

    let client = build_client()?;
    let state = ProxyState {
        client,
        upstream: upstream_url,
        llmlingua,
        prompt_compression,
    };

    let app = Router::new()
        .route("/", get(root_json))
        .fallback(any(proxy_handler))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.server.bind)
        .await
        .map_err(|e| OaasError::Config(format!("impossible de binder {}: {e}", cfg.server.bind)))?;

    info!(
        listen = %cfg.server.bind,
        upstream = %upstream_base,
        profile = %profile_name,
        "OAAS prêt — configure Continue sur cette URL (OpenAI-compatible)"
    );

    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    server
        .await
        .map_err(|e| OaasError::Backend(e.to_string()))?;

    Ok(())
}

async fn proxy_handler(State(state): State<ProxyState>, req: Request<Body>) -> Response {
    forward_request(&state, req).await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_ok() {
            warn!("signal Ctrl+C reçu, arrêt…");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
            warn!("signal SIGTERM reçu, arrêt…");
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
