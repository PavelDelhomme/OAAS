mod app_state;
mod backend;
mod cmd_docs;
mod cmd_models;
mod cmd_status;
mod config;
mod continue_ide;
mod docs_catalog;
mod doctor;
mod download;
mod error;
mod http_ide;
mod http_root;
mod init_config;
mod llmlingua;
mod models_catalog;
mod project_status;
mod proxy;
mod runtime;
mod serve_dashboard;
mod system_snapshot;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::response::Response;
use axum::routing::{any, get, post};
use axum::Router;
use clap::{Parser, Subcommand};
use tower_http::compression::CompressionLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

use crate::app_state::AppState;
use crate::backend::{wait_upstream_ready, LlamaBackend};
use crate::cmd_docs::DocsCommand;
use crate::cmd_models::ModelsCommand;
use crate::cmd_status::{run_status, StatusCli};
use crate::config::{default_config_path, load_config};
use crate::error::OaasError;
use crate::http_ide::{get_continue_status, post_apply_continue, post_open_folder};
use crate::http_root::{oaas_catalog, oaas_status, oaas_system, oaas_ui, oaas_workstation, root};
use crate::models_catalog::load_models_catalog;
use crate::proxy::{build_proxy_client, forward_request, ProxyState};
use crate::runtime::resolve_llama_binary;
use crate::serve_dashboard::build_oaas_status;

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
    Doctor {
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(long, default_value = "default", env = "OAAS_PROFILE")]
        profile: String,
        /// Corrige dans le YAML ce qui est sûr sans réseau (ex. désactive LLMLingua si import / script / commande cassés).
        #[arg(long, visible_alias = "fix-issues")]
        fix: bool,
    },
    InitConfig {
        #[arg(long)]
        force: bool,
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Catalogue et téléchargement de modèles GGUF (Hugging Face).
    Models {
        #[command(subcommand)]
        command: ModelsCommand,
    },
    /// Mise en cache de paquets de documentation (git / fetch).
    Docs {
        #[command(subcommand)]
        command: DocsCommand,
    },
    Serve {
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        #[arg(long, default_value = "default", env = "OAAS_PROFILE")]
        profile: String,
    },
    /// Synthèse : config, GGUF, port OAAS, UI, Continue ; `--ci` lance fmt/clippy/build.
    Status(StatusCli),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    if let Err(e) = run_cli(cli).await {
        tracing::error!(error = %e, "arrêt sur erreur");
        std::process::exit(1);
    }
}

async fn run_cli(cli: Cli) -> Result<(), OaasError> {
    match cli.command {
        Commands::Doctor {
            config,
            profile,
            fix,
        } => {
            crate::doctor::run_doctor(config, profile, fix);
            Ok(())
        }
        Commands::InitConfig { force, output } => {
            crate::init_config::run_init_config(force, output).map_err(OaasError::Config)?;
            Ok(())
        }
        Commands::Models { command } => crate::cmd_models::run_models(command).await,
        Commands::Docs { command } => crate::cmd_docs::run_docs(command).await,
        Commands::Serve { config, profile } => run_serve(config, profile).await,
        Commands::Status(args) => run_status(args).await,
    }
}

async fn run_serve(config_path: Option<PathBuf>, profile_name: String) -> Result<(), OaasError> {
    let path = config_path.unwrap_or_else(default_config_path);
    if !path.exists() {
        return Err(OaasError::Config(format!(
            "fichier de configuration absent: {} — copie config.example.yaml ou oaas init-config.",
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
    let backend = LlamaBackend::spawn(&llama_bin, &profile).await?;
    let llama_pid = backend.llama_pid();

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

    let client = build_proxy_client()?;
    let proxy = ProxyState {
        client,
        upstream: upstream_url,
        llmlingua,
        prompt_compression,
    };

    let catalog_root = load_models_catalog()?;
    let status = Arc::new(build_oaas_status(&cfg, &path, &profile_name, &catalog_root));
    let catalog = Arc::new(catalog_root);
    let app_state = AppState {
        proxy,
        catalog: catalog.clone(),
        status,
        llama_pid,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/oaas/", get(oaas_ui))
        .route("/oaas/catalog.json", get(oaas_catalog))
        .route("/oaas/status.json", get(oaas_status))
        .route("/oaas/ide/continue-status", get(get_continue_status))
        .route("/oaas/ide/apply-continue", post(post_apply_continue))
        .route("/oaas/ide/open-folder", post(post_open_folder))
        .route("/oaas/system.json", get(oaas_system))
        .route("/oaas/workstation.json", get(oaas_workstation))
        .fallback(any(proxy_handler))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(CompressionLayer::new())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&cfg.server.bind)
        .await
        .map_err(|e| OaasError::Config(format!("impossible de binder {}: {e}", cfg.server.bind)))?;

    info!(
        listen = %cfg.server.bind,
        upstream = %upstream_base,
        profile = %profile_name,
        models_ui = %format!("http://{}/oaas/", cfg.server.bind),
        status_json = %format!("http://{}/oaas/status.json", cfg.server.bind),
        "OAAS prêt — /v1 ; /oaas/ ; status.json ; system.json ; workstation.json ; /oaas/ide/*"
    );

    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    server
        .await
        .map_err(|e| OaasError::Backend(e.to_string()))?;

    Ok(())
}

async fn proxy_handler(State(state): State<AppState>, req: Request<Body>) -> Response {
    forward_request(&state.proxy, req).await
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
