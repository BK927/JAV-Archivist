use super::types::ScrapeError;
use std::path::{Path, PathBuf};

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn download_cover(
    client: &rquest::Client,
    url: &str,
    video_id: &str,
    thumbnails_dir: &Path,
) -> Result<PathBuf, ScrapeError> {
    tracing::info!("image: downloading cover for video_id={}", video_id);
    tracing::debug!("image: cover url={}", url);
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        tracing::error!(
            "image: cover download failed HTTP {} for video_id={}",
            resp.status().as_u16(),
            video_id
        );
        return Err(ScrapeError::NetworkError(format!(
            "HTTP {}",
            resp.status().as_u16()
        )));
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

    std::fs::write(&file_path, &bytes).map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    tracing::info!("image: cover saved to {:?}", file_path);
    Ok(file_path)
}

pub async fn download_actor_photo(
    client: &rquest::Client,
    url: &str,
    actors_dir: &Path,
    actor_name: &str,
) -> Result<PathBuf, ScrapeError> {
    tracing::debug!(
        "image: downloading actor photo for {:?} url={}",
        actor_name,
        url
    );
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        tracing::warn!(
            "image: actor photo download failed HTTP {} for {:?}",
            resp.status().as_u16(),
            actor_name
        );
        return Err(ScrapeError::NetworkError(format!(
            "HTTP {}",
            resp.status().as_u16()
        )));
    }

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
        .unwrap_or("jpg");

    let sanitized = sanitize_filename(actor_name);
    let file_path = actors_dir.join(format!("{}.{}", sanitized, ext));

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;
    std::fs::write(&file_path, &bytes).map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    tracing::debug!("image: actor photo saved to {:?}", file_path);
    Ok(file_path)
}

pub async fn download_sample_images(
    client: &rquest::Client,
    urls: &[String],
    samples_dir: &Path,
    video_code: &str,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let sanitized_code = video_code.replace('-', "_").to_lowercase();

    tracing::info!(
        "image: downloading {} sample images for code={}",
        urls.len(),
        video_code
    );
    for (i, url) in urls.iter().enumerate() {
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::warn!(
                    "image: sample #{} HTTP {} for code={}",
                    i + 1,
                    r.status().as_u16(),
                    video_code
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    "image: sample #{} fetch error for code={}: {}",
                    i + 1,
                    video_code,
                    e
                );
                continue;
            }
        };

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
            .unwrap_or("jpg");

        let file_path = samples_dir.join(format!("{}_{:02}.{}", sanitized_code, i + 1, ext));

        if let Ok(bytes) = resp.bytes().await {
            if std::fs::write(&file_path, &bytes).is_ok() {
                paths.push(file_path);
            }
        }
    }

    tracing::info!(
        "image: saved {}/{} sample images for code={}",
        paths.len(),
        urls.len(),
        video_code
    );
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Aoi Rena"), "Aoi Rena");
        assert_eq!(sanitize_filename("Test/Actor:Name"), "Test_Actor_Name");
    }
}
