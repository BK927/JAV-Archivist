use std::path::{Path, PathBuf};
use super::types::ScrapeError;

pub async fn download_cover(
    client: &rquest::Client,
    url: &str,
    video_id: &str,
    thumbnails_dir: &Path,
) -> Result<PathBuf, ScrapeError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(ScrapeError::NetworkError(
            format!("HTTP {}", resp.status().as_u16()),
        ));
    }

    // Determine extension from Content-Type, URL, or default to jpg
    let ext = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| match ct {
            "image/jpeg" | "image/jpg" => Some("jpg"),
            "image/png" => Some("png"),
            "image/webp" => Some("webp"),
            _ => None,
        })
        .or_else(|| {
            url.rsplit('/')
                .next()
                .and_then(|filename| filename.rsplit('.').next())
                .filter(|ext| matches!(*ext, "jpg" | "jpeg" | "png" | "webp"))
        })
        .unwrap_or("jpg");

    let file_path = thumbnails_dir.join(format!("{}.{}", video_id, ext));

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    std::fs::write(&file_path, &bytes)
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    Ok(file_path)
}
