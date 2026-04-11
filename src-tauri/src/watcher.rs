use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::time::Duration;
use tauri::Emitter;

use crate::{db, scanner};

const DEBOUNCE_SECS: u64 = 2;

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| scanner::VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 지정된 폴더들을 감시하고, 비디오 파일 변경 시 디바운스 후 스캔하여
/// `library-changed` 이벤트를 발행한다.
/// 반환된 RecommendedWatcher를 drop하면 감시가 중지된다.
pub fn start(
    app: tauri::AppHandle,
    folders: &[String],
    db_path: std::path::PathBuf,
) -> Result<RecommendedWatcher, String> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;

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

    // db_path를 String으로 변환하여 스레드에 전달 (to_str().unwrap() panic 방지)
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| "db_path contains invalid unicode".to_string())?
        .to_string();

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
                        trigger_scan(&app, &db_path_str);
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

fn trigger_scan(app: &tauri::AppHandle, db_path: &str) {
    let conn = match db::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("watcher: db open failed: {}", e);
            return;
        }
    };
    let settings = match db::get_settings(&conn) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("watcher: get_settings failed: {}", e);
            return;
        }
    };
    let scanned = match scanner::scan_folders(&settings.scan_folders) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("watcher: scan_folders failed: {}", e);
            return;
        }
    };
    if let Err(e) = db::upsert_videos(&conn, &scanned) {
        tracing::error!("watcher: upsert_videos failed: {}", e);
        return;
    }
    // Remove orphaned videos (in DB but not on filesystem)
    let scanned_ids: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.id.clone()).collect();
    match db::get_all_video_ids(&conn) {
        Ok(all_db_ids) => {
            let orphan_ids: Vec<String> = all_db_ids
                .into_iter()
                .filter(|id| !scanned_ids.contains(id))
                .collect();
            if !orphan_ids.is_empty() {
                tracing::info!("watcher: removing {} orphaned videos", orphan_ids.len());
                if let Err(e) = db::delete_videos(&conn, &orphan_ids) {
                    tracing::error!("watcher: delete_videos failed: {}", e);
                }
            }
        }
        Err(e) => tracing::error!("watcher: get_all_video_ids failed: {}", e),
    }
    match db::get_all_videos(&conn) {
        Ok(videos) => {
            let count = videos.len();
            let _ = app.emit("library-changed", &videos);
            tracing::info!("watcher: emitted library-changed ({} videos)", count);
        }
        Err(e) => tracing::error!("watcher: get_all_videos failed: {}", e),
    }

    // Trigger auto-scrape for unscraped videos
    match db::get_unscraped_for_auto(&conn) {
        Ok(to_scrape) if !to_scrape.is_empty() => {
            let ids: Vec<String> = to_scrape.into_iter().map(|(id, _)| id).collect();
            tracing::info!("watcher: triggering auto-scrape for {} videos", ids.len());
            let _ = app.emit("auto-scrape-needed", &ids);
        }
        _ => {}
    }
}
