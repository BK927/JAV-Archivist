#[derive(Debug, Clone, Default)]
pub struct ScrapedMetadata {
    pub title: Option<String>,
    pub cover_url: Option<String>,
    pub actors: Vec<String>,
    pub tags: Vec<String>,
    pub series: Option<String>,
    pub maker: Option<String>,
    pub duration: Option<u64>,
    pub released_at: Option<String>,
}

impl ScrapedMetadata {
    pub fn has_any_field(&self) -> bool {
        self.title.is_some()
            || self.cover_url.is_some()
            || !self.actors.is_empty()
            || !self.tags.is_empty()
            || self.series.is_some()
            || self.duration.is_some()
            || self.released_at.is_some()
    }

    /// Complete if title + cover present, and actors present (or FC2 code which has no actors)
    pub fn is_complete(&self, code: &str) -> bool {
        self.title.is_some()
            && self.cover_url.is_some()
            && (!self.actors.is_empty() || code.starts_with("FC2-PPV"))
    }
}

#[derive(Debug)]
pub enum ScrapeError {
    NotFound,
    NetworkError(String),
    ParseError(String),
    RateLimited,
}

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::NetworkError(e) => write!(f, "network error: {}", e),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::RateLimited => write!(f, "rate limited"),
        }
    }
}

pub enum MetadataSource {
    Fc2,
    R18Dev,
}

impl MetadataSource {
    pub fn name(&self) -> &str {
        match self {
            Self::Fc2 => "fc2",
            Self::R18Dev => "r18dev",
        }
    }
}
