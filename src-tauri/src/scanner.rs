use crate::models::{ScrapeStatus, Video, VideoFile};
use chrono::Utc;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;
use walkdir::WalkDir;

static FC2_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)FC2(?:[-_\s]?PPV)?[-_\s]?(\d{5,8})").unwrap());

static GENERAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([A-Z]{2,6})-(\d{3,5})").unwrap());

/// Prefixes that look like JAV codes but are actually website/platform names.
const NON_JAV_PREFIXES: &[&str] = &["FANTIA"];

/// Extract a video code from a text string (filename or folder name).
/// Returns the normalized code or None if no pattern matches.
pub fn extract_code(text: &str) -> Option<String> {
    // FC2 pattern: FC2-PPV-123, FC2PPV 123, FC2PPV123, etc.
    if let Some(caps) = FC2_RE.captures(text) {
        let digits = &caps[1];
        return Some(format!("FC2-PPV-{}", digits));
    }

    // General pattern: ABC-123, ABCD-12345
    if let Some(caps) = GENERAL_RE.captures(text) {
        let prefix = caps[1].to_uppercase();
        if NON_JAV_PREFIXES.contains(&prefix.as_str()) {
            return None;
        }
        let number = &caps[2];
        return Some(format!("{}-{}", prefix, number));
    }

    None
}

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "wmv", "flv", "mov", "ts", "m4v"];

struct ScannedFile {
    path: String,
    size: u64,
    code: String,
    filename: String,
}

pub fn scan_folders(folders: &[String]) -> Result<Vec<Video>, String> {
    let mut scanned: Vec<ScannedFile> = Vec::new();

    for folder in folders {
        tracing::info!("scanner: scanning folder {:?}", folder);
        for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            let filename = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            let code = extract_code(&filename)
                .or_else(|| {
                    path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .and_then(extract_code)
                })
                .unwrap_or_else(|| "?".to_string());

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            let sf = ScannedFile {
                path: path.to_string_lossy().to_string(),
                size,
                code: code.clone(),
                filename,
            };
            if code == "?" {
                tracing::warn!("scanner: no code extracted for {:?}", sf.path);
            }
            scanned.push(sf);
        }
    }

    let videos = group_by_code(scanned);
    tracing::info!("scanner: found {} videos", videos.len());
    Ok(videos)
}

fn group_by_code(files: Vec<ScannedFile>) -> Vec<Video> {
    let mut groups: HashMap<String, Vec<ScannedFile>> = HashMap::new();
    let mut unknown: Vec<Video> = Vec::new();
    let now = Utc::now().to_rfc3339();

    for file in files {
        if file.code == "?" {
            unknown.push(Video {
                id: Uuid::new_v4().to_string(),
                code: "?".to_string(),
                title: file.filename.clone(),
                files: vec![VideoFile {
                    path: file.path,
                    size: file.size,
                }],
                thumbnail_path: None,
                actors: vec![],
                series: None,
                tags: vec![],
                duration: None,
                watched: false,
                favorite: false,
                added_at: now.clone(),
                released_at: None,
                scrape_status: ScrapeStatus::NotScraped,
                scraped_at: None,
                maker_name: None,
            });
        } else {
            groups.entry(file.code.clone()).or_default().push(file);
        }
    }

    let mut videos: Vec<Video> = groups
        .into_iter()
        .map(|(code, files)| {
            let title = files[0].filename.clone();
            Video {
                id: Uuid::new_v4().to_string(),
                code,
                title,
                files: files
                    .into_iter()
                    .map(|f| VideoFile {
                        path: f.path,
                        size: f.size,
                    })
                    .collect(),
                thumbnail_path: None,
                actors: vec![],
                series: None,
                tags: vec![],
                duration: None,
                watched: false,
                favorite: false,
                added_at: now.clone(),
                released_at: None,
                scrape_status: ScrapeStatus::NotScraped,
                scraped_at: None,
                maker_name: None,
            }
        })
        .collect();

    videos.extend(unknown);
    videos
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_general_code() {
        assert_eq!(extract_code("ABC-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("ABCD-12345"), Some("ABCD-12345".to_string()));
        assert_eq!(extract_code("SONE-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_case_insensitive() {
        assert_eq!(extract_code("abc-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("sone-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_in_noisy_filename() {
        assert_eq!(
            extract_code("[1080p] ABC-123 actress_name"),
            Some("ABC-123".to_string())
        );
        assert_eq!(
            extract_code("some_prefix_MIDE-456_suffix"),
            Some("MIDE-456".to_string())
        );
    }

    #[test]
    fn test_fc2_canonical() {
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_no_hyphens() {
        assert_eq!(
            extract_code("FC2PPV1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_with_spaces() {
        assert_eq!(
            extract_code("FC2PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
        assert_eq!(
            extract_code("FC2 PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_case_insensitive() {
        assert_eq!(
            extract_code("fc2-ppv-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_without_ppv_still_extracts_code() {
        assert_eq!(
            extract_code("fc2-521444【個人撮影】肉オナホの使い方"),
            Some("FC2-PPV-521444".to_string())
        );
        assert_eq!(
            extract_code("fc2_1864525"),
            Some("FC2-PPV-1864525".to_string())
        );
    }

    #[test]
    fn test_fc2_with_suffix_noise_still_extracts_code() {
        assert_eq!(
            extract_code("fc2-ppv-1997904-nyap2p.com"),
            Some("FC2-PPV-1997904".to_string())
        );
        assert_eq!(
            extract_code("hhd800.com@FC2-PPV-1802609"),
            Some("FC2-PPV-1802609".to_string())
        );
    }

    #[test]
    fn test_fc2_takes_priority_over_general() {
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_no_match() {
        assert_eq!(extract_code("random_video"), None);
        assert_eq!(extract_code("video_20240301"), None);
        assert_eq!(extract_code(""), None);
    }

    #[test]
    fn test_non_jav_prefixes_ignored() {
        assert_eq!(extract_code("FANTIA-19978"), None);
        assert_eq!(extract_code("FANTIA-27080"), None);
    }

    #[test]
    fn test_scan_empty_folder() {
        let dir = TempDir::new().unwrap();
        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_finds_video_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ABC-123.mp4"), "fake").unwrap();
        fs::write(dir.path().join("DEF-456.mkv"), "fake").unwrap();
        fs::write(dir.path().join("readme.txt"), "not a video").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 2);

        let codes: Vec<&str> = result.iter().map(|v| v.code.as_str()).collect();
        assert!(codes.contains(&"ABC-123"));
        assert!(codes.contains(&"DEF-456"));
    }

    #[test]
    fn test_scan_groups_same_code() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ABC-123.mp4"), "fake").unwrap();
        fs::write(dir.path().join("ABC-123_part2.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
        assert_eq!(result[0].files.len(), 2);
    }

    #[test]
    fn test_scan_extracts_code_from_folder() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("ABC-123");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("video.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
    }

    #[test]
    fn test_scan_unknown_code() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("random_video.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "?");
    }

    #[test]
    fn test_scan_recursive() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("ABC-123.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
    }
}
