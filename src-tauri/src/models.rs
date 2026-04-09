use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScrapeStatus {
    NotScraped,
    Partial,
    Complete,
    NotFound,
}

impl ScrapeStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NotScraped => "not_scraped",
            Self::Partial => "partial",
            Self::Complete => "complete",
            Self::NotFound => "not_found",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "partial" => Self::Partial,
            "complete" => Self::Complete,
            "not_found" => Self::NotFound,
            _ => Self::NotScraped,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub code: String,
    pub title: String,
    pub files: Vec<VideoFile>,
    pub thumbnail_path: Option<String>,
    pub actors: Vec<String>,
    pub series: Option<String>,
    pub tags: Vec<String>,
    pub duration: Option<u64>,
    pub watched: bool,
    pub favorite: bool,
    pub added_at: String,
    pub released_at: Option<String>,
    pub scrape_status: ScrapeStatus,
    pub scraped_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub scan_folders: Vec<String>,
    pub player_path: Option<String>,
}
