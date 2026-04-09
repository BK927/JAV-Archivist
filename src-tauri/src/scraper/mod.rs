pub mod types;
pub mod http;
pub mod r18dev;
pub mod fc2;
pub mod javbus;
pub mod javten;
pub mod image;

use std::collections::HashMap;
pub use types::{ScrapedMetadata, ScrapeError, MetadataSource};

use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Mutex;
use rquest_util::Emulation;
use crate::models::ScrapeStatus;

pub struct ScrapeResult {
    pub metadata: ScrapedMetadata,
    pub cover_path: Option<PathBuf>,
    pub actor_photo_paths: HashMap<String, PathBuf>,
    pub sample_image_paths: Vec<PathBuf>,
    pub status: ScrapeStatus,
}

fn merge(base: &mut ScrapedMetadata, incoming: ScrapedMetadata) {
    if base.title.is_none() { base.title = incoming.title; }
    if base.cover_url.is_none() { base.cover_url = incoming.cover_url; }
    if base.series.is_none() { base.series = incoming.series; }
    if base.maker.is_none() { base.maker = incoming.maker; }
    if base.duration.is_none() { base.duration = incoming.duration; }
    if base.released_at.is_none() { base.released_at = incoming.released_at; }
    if base.actors.is_empty() { base.actors = incoming.actors; }
    if base.actor_details.is_empty() { base.actor_details = incoming.actor_details; }
    if base.tags.is_empty() { base.tags = incoming.tags; }
    if base.sample_image_urls.is_empty() { base.sample_image_urls = incoming.sample_image_urls; }
}

fn sources_for(code: &str) -> Vec<MetadataSource> {
    if code.starts_with("FC2-PPV") {
        vec![MetadataSource::Fc2, MetadataSource::Javten]
    } else {
        vec![MetadataSource::R18Dev, MetadataSource::JavBus]
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
            Self::Javten => javten::fetch(code, client).await,
            Self::R18Dev => r18dev::fetch(code, client).await,
            Self::JavBus => javbus::fetch(code, client).await,
        }
    }
}

pub struct ScrapePipeline {
    client: rquest::Client,
    fc2_limiter: Mutex<http::RateLimiter>,
    javten_limiter: Mutex<http::RateLimiter>,
    r18dev_limiter: Mutex<http::RateLimiter>,
    javbus_limiter: Mutex<http::RateLimiter>,
    thumbnails_dir: PathBuf,
    actors_dir: PathBuf,
    samples_dir: PathBuf,
}

impl ScrapePipeline {
    pub fn new(thumbnails_dir: PathBuf, actors_dir: PathBuf, samples_dir: PathBuf) -> Result<Self, String> {
        let client = rquest::Client::builder()
            .emulation(Emulation::Chrome131)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let make_limiter = || Mutex::new(http::RateLimiter::new(
            Duration::from_secs(3),
            Duration::from_secs(60),
        ));

        Ok(Self {
            client,
            fc2_limiter: make_limiter(),
            javten_limiter: make_limiter(),
            r18dev_limiter: make_limiter(),
            javbus_limiter: make_limiter(),
            thumbnails_dir,
            actors_dir,
            samples_dir,
        })
    }

    fn limiter_for(&self, source: &MetadataSource) -> &Mutex<http::RateLimiter> {
        match source {
            MetadataSource::Fc2 => &self.fc2_limiter,
            MetadataSource::Javten => &self.javten_limiter,
            MetadataSource::R18Dev => &self.r18dev_limiter,
            MetadataSource::JavBus => &self.javbus_limiter,
        }
    }

    async fn fetch_source(
        &self,
        source: &MetadataSource,
        code: &str,
    ) -> Result<ScrapedMetadata, ScrapeError> {
        let limiter = self.limiter_for(source);
        {
            let guard = limiter.lock().await;
            guard.wait().await;
        }
        let result = source.fetch(code, &self.client).await;
        {
            let mut guard = limiter.lock().await;
            match &result {
                Ok(_) => guard.success(),
                Err(ScrapeError::RateLimited) => guard.failure(),
                _ => {}
            }
        }
        result
    }

    pub async fn scrape_one(&self, code: &str, video_id: &str) -> ScrapeResult {
        tracing::info!("scrape_one: code={} video_id={}", code, video_id);
        let sources = sources_for(code);

        // Fetch all sources concurrently
        let fetch_futures = sources.iter().map(|source| self.fetch_source(source, code));
        let results = futures::future::join_all(fetch_futures).await;

        let mut merged = ScrapedMetadata::default();
        for (source, result) in sources.iter().zip(results) {
            match result {
                Ok(meta) => {
                    tracing::info!("scrape_one: source={:?} succeeded for code={}", source, code);
                    merge(&mut merged, meta);
                }
                Err(ScrapeError::RateLimited) => {
                    tracing::warn!("scrape_one: rate limited by source={:?} for code={}", source, code);
                }
                Err(e) => {
                    tracing::error!("scrape_one: source={:?} failed for code={}: {:?}", source, code, e);
                }
            }
        }

        // Cover fallback: use first sample image if no cover
        if merged.cover_url.is_none() && !merged.sample_image_urls.is_empty() {
            merged.cover_url = Some(merged.sample_image_urls[0].clone());
        }

        // Download cover
        let cover_path = if let Some(ref cover_url) = merged.cover_url {
            image::download_cover(&self.client, cover_url, video_id, &self.thumbnails_dir)
                .await
                .ok()
        } else {
            None
        };

        // Download actor photos
        let mut actor_photo_paths = HashMap::new();
        for detail in &merged.actor_details {
            if let Some(ref photo_url) = detail.photo_url {
                if let Ok(path) = image::download_actor_photo(
                    &self.client, photo_url, &self.actors_dir, &detail.name,
                ).await {
                    actor_photo_paths.insert(detail.name.clone(), path);
                }
            }
        }

        // Download sample images
        let sample_image_paths = image::download_sample_images(
            &self.client,
            &merged.sample_image_urls,
            &self.samples_dir,
            code,
        ).await;

        let status = if merged.is_complete(code) {
            ScrapeStatus::Complete
        } else if merged.has_any_field() {
            ScrapeStatus::Partial
        } else {
            ScrapeStatus::NotFound
        };

        tracing::info!("scrape_one: code={} final status={:?}", code, status);

        ScrapeResult {
            metadata: merged,
            cover_path,
            actor_photo_paths,
            sample_image_paths,
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::types::ScrapedActor;

    #[test]
    fn test_merge_fills_empty_fields() {
        let mut base = ScrapedMetadata::default();
        let incoming = ScrapedMetadata {
            title: Some("Title".to_string()),
            cover_url: Some("http://cover.jpg".to_string()),
            actors: vec!["Actor".to_string()],
            actor_details: vec![ScrapedActor {
                name: "Actor".to_string(),
                name_kanji: Some("アクター".to_string()),
                photo_url: Some("http://photo.jpg".to_string()),
            }],
            tags: vec!["Tag".to_string()],
            series: Some("Series".to_string()),
            maker: Some("Maker".to_string()),
            duration: Some(3600),
            released_at: Some("2024-01-01".to_string()),
            sample_image_urls: vec!["http://sample1.jpg".to_string()],
        };
        merge(&mut base, incoming);

        assert_eq!(base.title.as_deref(), Some("Title"));
        assert_eq!(base.cover_url.as_deref(), Some("http://cover.jpg"));
        assert_eq!(base.actors, vec!["Actor"]);
        assert_eq!(base.actor_details.len(), 1);
        assert_eq!(base.actor_details[0].name, "Actor");
        assert_eq!(base.tags, vec!["Tag"]);
        assert_eq!(base.series.as_deref(), Some("Series"));
        assert_eq!(base.maker.as_deref(), Some("Maker"));
        assert_eq!(base.duration, Some(3600));
        assert_eq!(base.released_at.as_deref(), Some("2024-01-01"));
        assert_eq!(base.sample_image_urls.len(), 1);
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
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name(), "fc2");
        assert_eq!(sources[1].name(), "javten");
    }

    #[test]
    fn test_sources_for_general() {
        let sources = sources_for("ABC-123");
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name(), "r18dev");
        assert_eq!(sources[1].name(), "javbus");
    }
}
