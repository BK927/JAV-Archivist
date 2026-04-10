use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::Emitter;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "wmv", "flv", "mov", "ts", "m4v"];
const DEBOUNCE_SECS: u64 = 2;

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 지정된 폴더들을 감시하고, 비디오 파일 변경 시 디바운스 후 스캔하여
/// `library-changed` 이벤트를 발행한다.
/// 반환된 RecommendedWatcher를 drop하면 감시가 중지된다.
pub fn start(
    app: tauri::AppHandle,
    folders: &[String],
    db_path: PathBuf,
) -> Result<RecommendedWatcher, String> {
    todo!("Task 2에서 구현")
}
