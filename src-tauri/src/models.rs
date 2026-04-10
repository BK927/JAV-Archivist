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

    fn rank(&self) -> u8 {
        match self {
            Self::NotScraped => 0,
            Self::NotFound => 1,
            Self::Partial => 2,
            Self::Complete => 3,
        }
    }

    pub fn merge_with_existing(&self, existing: &Self) -> Self {
        if self.rank() >= existing.rank() {
            self.clone()
        } else {
            existing.clone()
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
    pub maker_name: Option<String>,
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
    pub log_enabled: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub name_kanji: Option<String>,
    pub photo_path: Option<String>,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Maker {
    pub id: String,
    pub name: String,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub name: String,
    pub cover_path: Option<String>,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCooccurrence {
    pub tag_id: String,
    pub tag_name: String,
    pub co_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleImage {
    pub id: String,
    pub video_id: String,
    pub path: String,
    pub sort_order: u32,
}

#[derive(Debug, Clone)]
pub struct ActorDetail {
    pub name: String,
    pub name_kanji: Option<String>,
}
