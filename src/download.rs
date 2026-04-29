use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::header::RANGE;
use reqwest::StatusCode;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::error::OaasError;

/// Fichier temporaire `…/nom.ext.part` pour téléchargement avec reprise (`Range`).
fn partial_download_path(dest: &Path) -> PathBuf {
    let mut name = dest
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_else(|| "download".into());
    name.push(".part");
    dest.parent().unwrap_or(Path::new(".")).join(name)
}

async fn copy_stream_to_file<S>(mut stream: S, file: &mut File) -> Result<(), OaasError>
where
    S: futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    let mut n: u64 = 0;
    let mut last_log: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let b = chunk.map_err(|e| OaasError::Backend(format!("flux téléchargement: {e}")))?;
        file.write_all(&b)
            .await
            .map_err(|e| OaasError::Backend(format!("écriture disque: {e}")))?;
        n += b.len() as u64;
        if n.saturating_sub(last_log) >= 50 * 1024 * 1024 {
            last_log = n;
            tracing::info!(mo = n / (1024 * 1024), "téléchargement en cours…");
        }
    }
    file.flush()
        .await
        .map_err(|e| OaasError::Backend(format!("flush: {e}")))?;
    Ok(())
}

/// Télécharge `url` vers `dest` avec fichier intermédiaire `dest.part`.
/// Si `dest.part` existe déjà, envoie `Range: bytes=<taille>-` pour **reprendre** (HTTP 206).
/// Hugging Face / la plupart des CDN supportent les requêtes par plages.
pub async fn download_url_to_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), OaasError> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| OaasError::Backend(format!("mkdir {}: {e}", parent.display())))?;
    }

    let part = partial_download_path(dest);

    for attempt in 0u32..8 {
        let offset: u64 = match tokio::fs::metadata(&part).await {
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        if attempt > 0 && offset == 0 {
            tracing::debug!(attempt, "nouvelle tentative téléchargement depuis le début");
        } else if offset > 0 {
            tracing::info!(
                path = %part.display(),
                bytes = offset,
                "reprise depuis .part (en-tête Range)"
            );
        }

        let mut req = client.get(url);
        if offset > 0 {
            req = req.header(RANGE, format!("bytes={offset}-"));
        }

        let res = req
            .send()
            .await
            .map_err(|e| OaasError::Backend(format!("GET {url}: {e}")))?;

        let status = res.status();

        if status == StatusCode::RANGE_NOT_SATISFIABLE {
            tokio::fs::remove_file(&part).await.ok();
            tracing::warn!(
                "HTTP 416 : le .part dépasse la taille distante — suppression, nouvelle tentative"
            );
            continue;
        }

        if !status.is_success() {
            return Err(OaasError::Backend(format!("GET {url} → HTTP {}", status)));
        }

        let use_append = offset > 0 && status == StatusCode::PARTIAL_CONTENT;

        if offset > 0 && !use_append {
            tracing::info!(
                "réponse HTTP {} sans 206 : réécriture complète du .part",
                status.as_u16()
            );
            tokio::fs::remove_file(&part).await.ok();
        }

        let mut file = if use_append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part)
                .await
                .map_err(|e| {
                    OaasError::Backend(format!("ouverture append {}: {e}", part.display()))
                })?
        } else {
            File::create(&part)
                .await
                .map_err(|e| OaasError::Backend(format!("création {}: {e}", part.display())))?
        };

        let stream = res.bytes_stream();
        copy_stream_to_file(stream, &mut file).await?;

        if dest.exists() {
            tokio::fs::remove_file(dest).await.map_err(|e| {
                OaasError::Backend(format!(
                    "suppression ancienne cible {}: {e}",
                    dest.display()
                ))
            })?;
        }
        tokio::fs::rename(&part, dest).await.map_err(|e| {
            OaasError::Backend(format!(
                "renommage {} → {}: {e} (le .part est conservé pour reprise manuelle)",
                part.display(),
                dest.display()
            ))
        })?;

        return Ok(());
    }

    Err(OaasError::Backend(
        "téléchargement : abandon après plusieurs tentatives (voir messages 416 / .part)".into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn partial_download_path_suffix() {
        let dest = Path::new("/home/u/.local/share/oaas/models/foo.gguf");
        let p = super::partial_download_path(dest);
        assert!(p.to_string_lossy().ends_with("foo.gguf.part"));
    }
}
