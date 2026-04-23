use std::path::Path;
use std::time::Duration;

use tokio::process::{Child, Command};
use tracing::info;

use crate::config::Profile;
use crate::error::OaasError;

pub struct LlamaBackend {
    /// Garde le processus vivant ; `kill_on_drop` l’arrête quand le démon OAAS se termine.
    #[allow(dead_code)]
    child: Child,
}

impl LlamaBackend {
    pub async fn spawn(llama_server_binary: &Path, profile: &Profile) -> Result<Self, OaasError> {
        if !profile.model.exists() {
            return Err(OaasError::Config(format!(
                "fichier modèle introuvable: {}",
                profile.model.display()
            )));
        }

        let mut cmd = Command::new(llama_server_binary);
        cmd.kill_on_drop(true)
            .arg("-m")
            .arg(&profile.model)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(profile.internal_port.to_string())
            .arg("-c")
            .arg(profile.ctx_size.to_string())
            .arg("-ngl")
            .arg(profile.n_gpu_layers.to_string());

        for a in &profile.extra_args {
            cmd.arg(a);
        }

        info!(
            binary = %llama_server_binary.display(),
            model = %profile.model.display(),
            port = profile.internal_port,
            "démarrage de llama-server"
        );

        let child = cmd.spawn().map_err(|e| {
            OaasError::Backend(format!(
                "échec du lancement de {}: {e}",
                llama_server_binary.display()
            ))
        })?;

        if let Some(pid) = child.id() {
            info!(pid, "processus llama-server");
        }

        Ok(Self { child })
    }

    /// PID du processus `llama-server` (Unix), pour métriques `/oaas/system.json`.
    #[cfg(unix)]
    pub fn llama_pid(&self) -> Option<u32> {
        self.child.id()
    }

    #[cfg(not(unix))]
    pub fn llama_pid(&self) -> Option<u32> {
        None
    }
}

/// Attend que l’API llama-server réponde (GET /v1/models ou /health selon version).
pub async fn wait_upstream_ready(base: &str, timeout: Duration) -> Result<(), OaasError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| OaasError::Backend(e.to_string()))?;

    let models_url = format!("{}/v1/models", base.trim_end_matches('/'));
    let health_url = format!("{}/health", base.trim_end_matches('/'));

    let started = tokio::time::Instant::now();
    loop {
        if started.elapsed() > timeout {
            return Err(OaasError::Backend(format!(
                "timeout après {:?} — vérifie le binaire llama-server et le modèle",
                timeout
            )));
        }

        if let Ok(resp) = client.get(&models_url).send().await {
            if resp.status().is_success() {
                info!("llama-server prêt ({models_url})");
                return Ok(());
            }
        }

        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                info!("llama-server prêt ({health_url})");
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
