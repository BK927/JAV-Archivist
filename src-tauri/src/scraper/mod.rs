pub mod types;
pub mod http;
pub mod r18dev;
pub mod fc2;
pub mod image;

pub use types::{ScrapedMetadata, ScrapeError, MetadataSource};

use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Mutex;
use rquest_util::Emulation;
use crate::models::ScrapeStatus;

fn merge(base: &mut ScrapedMetadata, incoming: ScrapedMetadata) {
    if base.title.is_none() { base.title = incoming.title; }
    if base.cover_url.is_none() { base.cover_url = incoming.cover_url; }
    if base.series.is_none() { base.series = incoming.series; }
    if base.maker.is_none() { base.maker = incoming.maker; }
    if base.duration.is_none() { base.duration = incoming.duration; }
    if base.released_at.is_none() { base.released_at = incoming.released_at; }
    if base.actors.is_empty() { base.actors = incoming.actors; }
    if base.tags.is_empty() { base.tags = incoming.tags; }
}

fn sources_for(code: &str) -> Vec<MetadataSource> {
    if code.starts_with("FC2-PPV") {
        vec![MetadataSource::Fc2]
    } else {
        vec![MetadataSource::R18Dev]
    }
}

impl MetadataSource {
    pub async fn fetch(
        &self,
        code: &str,
        client: &rquest::Client,
    ) -> Result<ScrapedMetadata, ScrapeError> {
        match self {
            Self::Fc2 => fc2::fetch(code, client).await,
            Self::R18Dev => r18dev::fetch(code, client).await,
        }
    }
}

pub struct ScrapePipeline {
    client: rquest::Client,
    rate_limiter: Mutex<http::RateLimiter>,
    thumbnails_dir: PathBuf,
}

impl ScrapePipeline {
    pub fn new(thumbnails_dir: PathBuf) -> Result<Self, String> {
        let client = rquest::Client::builder()
            .emulation(Emulation::Chrome131)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            rate_limiter: Mutex::new(http::RateLimiter::new(
                Duration::from_secs(3),
                Duration::from_secs(60),
            )),
            thumbnails_dir,
        })
    }

    pub async fn scrape_one(
        &self,
        code: &str,
        video_id: &str,
    ) -> (ScrapedMetadata, Option<PathBuf>, ScrapeStatus) {
        let sources = sources_for(code);
        let mut merged = ScrapedMetadata::default();

        for source in &sources {
            {
                let rl = self.rate_limiter.lock().await;
                rl.wait().await;
            }

            match source.fetch(code, &self.client).await {
                Ok(meta) => {
                    merge(&mut merged, meta);
                    {
                        let mut rl = self.rate_limiter.lock().await;
                        rl.success();
                    }
                    if merged.is_complete(code) {
                        break;
                    }
                }
                Err(ScrapeError::RateLimited) => {
                    let mut rl = self.rate_limiter.lock().await;
                    rl.failure();
                }
                Err(_) => {}
            }
        }

        // Download cover image if available
        let thumbnail_path = if let Some(ref cover_url) = merged.cover_url {
            image::download_cover(&self.client, cover_url, video_id, &self.thumbnails_dir)
                .await
                .ok()
        } else {
            None
        };

        let status = if merged.is_complete(code) {
            ScrapeStatus::Complete
        } else if merged.has_any_field() {
            ScrapeStatus::Partial
        } else {
            ScrapeStatus::NotFound
        };

        (merged, thumbnail_path, status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_fills_empty_fields() {
        let mut base = ScrapedMetadata::default();
        let incoming = ScrapedMetadata {
            title: Some("Title".to_string()),
            cover_url: Some("http://cover.jpg".to_string()),
            actors: vec!["Actor".to_string()],
            tags: vec!["Tag".to_string()],
            series: Some("Series".to_string()),
            maker: Some("Maker".to_string()),
            duration: Some(3600),
            released_at: Some("2024-01-01".to_string()),
        };
        merge(&mut base, incoming);

        assert_eq!(base.title.as_deref(), Some("Title"));
        assert_eq!(base.cover_url.as_deref(), Some("http://cover.jpg"));
        assert_eq!(base.actors, vec!["Actor"]);
        assert_eq!(base.tags, vec!["Tag"]);
        assert_eq!(base.series.as_deref(), Some("Series"));
        assert_eq!(base.maker.as_deref(), Some("Maker"));
        assert_eq!(base.duration, Some(3600));
        assert_eq!(base.released_at.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn test_merge_preserves_existing_fields() {
        let mut base = ScrapedMetadata {
            title: Some("Original".to_string()),
            actors: vec!["Actor A".to_string()],
            ..Default::default()
        };
        let incoming = ScrapedMetadata {
            title: Some("New Title".to_string()),
            actors: vec!["Actor B".to_string()],
            cover_url: Some("http://new.jpg".to_string()),
            ..Default::default()
        };
        merge(&mut base, incoming);

        assert_eq!(base.title.as_deref(), Some("Original")); // NOT overwritten
        assert_eq!(base.actors, vec!["Actor A"]); // NOT overwritten
        assert_eq!(base.cover_url.as_deref(), Some("http://new.jpg")); // filled
    }

    #[test]
    fn test_sources_for_fc2() {
        let sources = sources_for("FC2-PPV-1234567");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name(), "fc2");
    }

    #[test]
    fn test_sources_for_general() {
        let sources = sources_for("ABC-123");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name(), "r18dev");
    }
}
