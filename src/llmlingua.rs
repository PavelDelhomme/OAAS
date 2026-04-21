use std::process::Stdio;
use std::sync::Arc;

use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::config::PromptCompressionConfig;
use crate::error::OaasError;

/// IPC avec le worker Python : trame = u32 big-endian (longueur JSON) + JSON UTF-8.
#[derive(Clone)]
pub struct LlmLinguaClient {
    inner: Arc<Inner>,
}

struct Inner {
    _child: Child,
    io: Mutex<IoPair>,
}

struct IoPair {
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
}

#[derive(Deserialize)]
struct WorkerEnvelope {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    body: Option<serde_json::Value>,
}

impl LlmLinguaClient {
    pub async fn spawn(cfg: &PromptCompressionConfig) -> Result<Self, OaasError> {
        if cfg.command.is_empty() {
            return Err(OaasError::Config(
                "prompt_compression.command ne peut pas être vide lorsque la compression est activée".into(),
            ));
        }

        let mut cmd = Command::new(&cfg.command[0]);
        cmd.args(&cfg.command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| {
            OaasError::Backend(format!(
                "impossible de lancer le worker LLMLingua {:?}: {e}",
                cfg.command
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| OaasError::Backend("worker LLMLingua : stdin indisponible".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| OaasError::Backend("worker LLMLingua : stdout indisponible".into()))?;

        let inner = Arc::new(Inner {
            _child: child,
            io: Mutex::new(IoPair {
                stdin,
                reader: BufReader::new(stdout),
            }),
        });

        let init = serde_json::json!({
            "v": 1,
            "op": "init",
            "model_name": cfg.model_name,
            "use_llmlingua2": cfg.use_llmlingua2,
            "device_map": cfg.device_map,
        });
        let env = Inner::rpc_envelope(&inner.io, &init, cfg.timeout_secs).await?;
        if !env.ok {
            return Err(OaasError::Backend(
                env.error.unwrap_or_else(|| "init LLMLingua refusé".into()),
            ));
        }

        Ok(Self { inner })
    }

    pub async fn compress_chat_json(
        &self,
        cfg: &PromptCompressionConfig,
        chat_body: &serde_json::Value,
    ) -> Result<serde_json::Value, OaasError> {
        let req = serde_json::json!({
            "v": 1,
            "op": "compress",
            "rate": cfg.rate,
            "target_token": cfg.target_token,
            "body": chat_body,
        });
        let env = Inner::rpc_envelope(&self.inner.io, &req, cfg.timeout_secs).await?;
        if !env.ok {
            return Err(OaasError::Backend(
                env.error.unwrap_or_else(|| "compression refusée".into()),
            ));
        }
        env.body
            .ok_or_else(|| OaasError::Backend("réponse compress sans « body »".into()))
    }
}

impl Inner {
    async fn rpc_envelope(
        io: &Mutex<IoPair>,
        payload: &serde_json::Value,
        timeout_secs: u64,
    ) -> Result<WorkerEnvelope, OaasError> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| OaasError::Backend(format!("sérialisation IPC LLMLingua: {e}")))?;

        if bytes.len() > u32::MAX as usize {
            return Err(OaasError::Backend("requête LLMLingua trop grande".into()));
        }

        let len = bytes.len() as u32;
        let deadline = std::time::Duration::from_secs(timeout_secs.max(1));

        let mut guard = io.lock().await;
        let IoPair { stdin, reader } = &mut *guard;

        let out_buf = tokio::time::timeout(deadline, async {
            stdin.write_u32(len).await?;
            stdin.write_all(&bytes).await?;
            stdin.flush().await?;

            let out_len = reader.read_u32().await? as usize;
            if out_len > 64 * 1024 * 1024 {
                return Err(std::io::Error::other("réponse LLMLingua trop grande"));
            }
            let mut buf = vec![0u8; out_len];
            reader.read_exact(&mut buf).await?;
            Ok::<Vec<u8>, std::io::Error>(buf)
        })
        .await
        .map_err(|_| {
            OaasError::Backend(format!(
                "timeout LLMLingua après {timeout_secs}s — augmente prompt_compression.timeout_secs ou vérifie device_map / GPU"
            ))
        })?
        .map_err(|e| OaasError::Backend(format!("IPC LLMLingua: {e}")))?;

        serde_json::from_slice(&out_buf)
            .map_err(|e| OaasError::Backend(format!("réponse LLMLingua JSON invalide: {e}")))
    }
}
