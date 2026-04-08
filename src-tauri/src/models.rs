use serde::{Deserialize, Serialize};

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
