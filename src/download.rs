use std::path::Path;

use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::error::OaasError;

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

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| OaasError::Backend(format!("GET {url}: {e}")))?;

    if !res.status().is_success() {
        return Err(OaasError::Backend(format!(
            "GET {url} → HTTP {}",
            res.status()
        )));
    }

    let mut file = File::create(dest)
        .await
        .map_err(|e| OaasError::Backend(format!("création {}: {e}", dest.display())))?;

    let mut stream = res.bytes_stream();
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
