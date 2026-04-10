use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::Emitter;

use crate::{db, scanner};

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
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| e.to_string())?;

    for folder in folders {
        let path = Path::new(folder);
        if path.exists() {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| format!("watch failed for {}: {}", folder, e))?;
            tracing::info!("watcher: watching {:?}", folder);
        } else {
            tracing::warn!("watcher: folder does not exist, skipping {:?}", folder);
        }
    }

    // 백그라운드 스레드: 이벤트 수신 → 디바운스 → 스캔 → 이벤트 발행
    std::thread::spawn(move || {
        let mut pending = false;

        loop {
            match rx.recv_timeout(Duration::from_secs(DEBOUNCE_SECS)) {
                Ok(Ok(event)) => {
                    if event.paths.iter().any(|p| is_video_file(p)) {
                        tracing::debug!("watcher: video file change detected");
                        pending = true;
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("watcher: error: {}", e);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        pending = false;
                        tracing::info!("watcher: debounce expired, scanning...");
                        trigger_scan(&app, &db_path);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::info!("watcher: channel disconnected, stopping");
                    break;
                }
            }
        }
    });

    Ok(watcher)
}

fn trigger_scan(app: &tauri::AppHandle, db_path: &Path) {
    let conn = match db::open(db_path.to_str().unwrap()) {
        Ok(c) => c,
        Err(e) => { tracing::error!("watcher: db open failed: {}", e); return; }
    };
    let settings = match db::get_settings(&conn) {
        Ok(s) => s,
        Err(e) => { tracing::error!("watcher: get_settings failed: {}", e); return; }
    };
    let scanned = match scanner::scan_folders(&settings.scan_folders) {
        Ok(v) => v,
        Err(e) => { tracing::error!("watcher: scan_folders failed: {}", e); return; }
    };
    if let Err(e) = db::upsert_videos(&conn, &scanned) {
        tracing::error!("watcher: upsert_videos failed: {}", e);
        return;
    }
    match db::get_all_videos(&conn) {
        Ok(videos) => {
            let count = videos.len();
            let _ = app.emit("library-changed", &videos);
            tracing::info!("watcher: emitted library-changed ({} videos)", count);
        }
        Err(e) => tracing::error!("watcher: get_all_videos failed: {}", e),
    }
}
