mod db;
mod ffmpeg;
mod logging;
mod media;
mod models;
mod player;
mod scanner;
mod scraper;
mod watcher;

use models::{
    Actor, Maker, SampleImage, ScanResult, ScrapeStatus, Series as SeriesModel, Settings, Tag,
    TagCooccurrence, Video,
};
use notify::RecommendedWatcher;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;

struct DataDir(PathBuf);
struct DbPath(PathBuf);
struct ThumbnailsDir(PathBuf);
struct ActorsDir(PathBuf);
struct SamplesDir(PathBuf);
struct ScrapeCancel(Arc<AtomicBool>);
struct ScrapeRunning(Arc<AtomicBool>);
struct SampleExtractionRunning(Arc<AtomicBool>);
struct WatcherHandle(Mutex<Option<RecommendedWatcher>>);
struct SpritesDir(PathBuf);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeProgressEvent {
    video_id: String,
    status: ScrapeStatus,
    current: usize,
    total: usize,
    video: Option<Video>,
}

fn asset_scope_paths(settings: &Settings, data_dir: &Path) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();

    let mut push_unique = |path: PathBuf| {
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    };

    push_unique(data_dir.to_path_buf());

    for folder in &settings.scan_folders {
        let trimmed = folder.trim();
        if trimmed.is_empty() {
            continue;
        }
        push_unique(PathBuf::from(trimmed));
    }

    paths
}

fn sync_asset_protocol_scope<R: tauri::Runtime, M: tauri::Manager<R>>(
    manager: &M,
    settings: &Settings,
    data_dir: &Path,
) -> Result<(), String> {
    let scope = manager.asset_protocol_scope();

    for path in asset_scope_paths(settings, data_dir) {
        if path.is_dir() {
            scope
                .allow_directory(&path, true)
                .map_err(|e| e.to_string())?;
            tracing::debug!("asset scope allowed directory: {}", path.display());
        } else if path.is_file() {
            scope.allow_file(&path).map_err(|e| e.to_string())?;
            tracing::debug!("asset scope allowed file: {}", path.display());
        } else {
            tracing::warn!(
                "asset scope path does not exist, skipping: {}",
                path.display()
            );
        }
    }

    Ok(())
}

#[tauri::command]
fn scan_library(
    app: tauri::AppHandle,
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
) -> Result<ScanResult, String> {
    tracing::info!("cmd: scan_library");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    let added = db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;

    // Remove orphaned videos (in DB but not on filesystem)
    // Compare by CODE (stable) — scanner generates new UUIDs each run,
    // so comparing by ID would incorrectly flag all videos as orphans.
    let scanned_codes: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.code.clone()).collect();
    let all_db = db::get_all_video_id_codes(&conn).map_err(|e| e.to_string())?;
    let orphan_ids: Vec<String> = all_db
        .into_iter()
        .filter(|(_, code)| !scanned_codes.contains(code))
        .map(|(id, _)| id)
        .collect();
    let removed = orphan_ids.len() as u32;
    if !orphan_ids.is_empty() {
        tracing::info!("scan_library: removing {} orphaned videos", orphan_ids.len());
        db::delete_videos(&conn, &orphan_ids).map_err(|e| e.to_string())?;
    }

    // Generate thumbnails for videos without one
    let need_thumbs = db::get_videos_without_thumbnail(&conn).map_err(|e| e.to_string())?;
    if !need_thumbs.is_empty() {
        tracing::info!("scan_library: generating thumbnails for {} videos", need_thumbs.len());
    }
    for (video_id, file_path) in &need_thumbs {
        if let Some(thumb_path) = ffmpeg::extract_thumbnail(file_path, video_id, &thumbnails.0) {
            let _ = db::set_thumbnail_path(&conn, video_id, &thumb_path);
            tracing::info!("scan_library: thumbnail generated for {}", video_id);
        }
    }

    let videos = db::get_all_videos(&conn).map_err(|e| e.to_string())?;

    if !added.is_empty() {
        let _ = app.emit("auto-scrape-needed", ());
    }

    let _ = app.emit("local-samples-needed", ());

    Ok(ScanResult { videos, added, removed })
}

#[tauri::command]
fn get_videos(db: tauri::State<'_, DbPath>) -> Result<Vec<Video>, String> {
    tracing::info!("cmd: get_videos");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_all_videos(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_video(db: tauri::State<'_, DbPath>, id: String) -> Result<Video, String> {
    tracing::info!("cmd: get_video id={}", id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_video_by_id(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_with_player(db: tauri::State<'_, DbPath>, file_path: String) -> Result<(), String> {
    tracing::info!("cmd: open_with_player path={}", file_path);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    player::open_with_player(&file_path, settings.player_path.as_deref())
}

#[tauri::command]
fn open_folder(file_path: String) -> Result<(), String> {
    tracing::info!("cmd: open_folder path={}", file_path);
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&file_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn mark_watched(db: tauri::State<'_, DbPath>, id: String, watched: bool) -> Result<(), String> {
    tracing::info!("cmd: mark_watched id={} watched={}", id, watched);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::set_watched(&conn, &id, watched).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_favorite(db: tauri::State<'_, DbPath>, id: String) -> Result<(), String> {
    tracing::info!("cmd: toggle_favorite id={}", id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let video = db::get_video_by_id(&conn, &id).map_err(|e| e.to_string())?;
    db::set_favorite(&conn, &id, !video.favorite).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(db: tauri::State<'_, DbPath>) -> Result<Settings, String> {
    tracing::info!("cmd: get_settings");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    db: tauri::State<'_, DbPath>,
    data_dir: tauri::State<'_, DataDir>,
    watcher_handle: tauri::State<'_, WatcherHandle>,
    settings: Settings,
) -> Result<(), String> {
    tracing::info!("cmd: save_settings");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::save_settings(&conn, &settings).map_err(|e| e.to_string())?;

    // 워처 재시작 (폴더 목록이 변경되었을 수 있음)
    let new_watcher = watcher::start(app.clone(), &settings.scan_folders, db.0.clone())
        .map_err(|e| {
            tracing::warn!("watcher restart failed: {}", e);
            e
        })
        .ok();
    *watcher_handle.0.lock().unwrap() = new_watcher;

    sync_asset_protocol_scope(&app, &settings, &data_dir.0)
}

#[tauri::command]
fn get_actors(db: tauri::State<'_, DbPath>) -> Result<Vec<Actor>, String> {
    tracing::info!("cmd: get_actors");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_actors(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_series_list(db: tauri::State<'_, DbPath>) -> Result<Vec<SeriesModel>, String> {
    tracing::info!("cmd: get_series_list");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_series(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tags(db: tauri::State<'_, DbPath>) -> Result<Vec<Tag>, String> {
    tracing::info!("cmd: get_tags");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_tags(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tag_cooccurrence(
    db: tauri::State<'_, DbPath>,
    tag_id: String,
) -> Result<Vec<TagCooccurrence>, String> {
    tracing::info!("cmd: get_tag_cooccurrence tag_id={}", tag_id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_tag_cooccurrence(&conn, &tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_makers(db: tauri::State<'_, DbPath>) -> Result<Vec<Maker>, String> {
    tracing::info!("cmd: get_makers");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_makers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sample_images(
    db: tauri::State<'_, DbPath>,
    video_id: String,
) -> Result<Vec<SampleImage>, String> {
    tracing::info!("cmd: get_sample_images video_id={}", video_id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_sample_images(&conn, &video_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scrape_video(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    actors_state: tauri::State<'_, ActorsDir>,
    samples_state: tauri::State<'_, SamplesDir>,
    video_id: String,
) -> Result<Video, String> {
    tracing::info!("cmd: scrape_video video_id={}", video_id);
    let db_path = db.0.clone();
    let vid_id = video_id.clone();
    let code = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        let video = db::get_video_by_id(&conn, &vid_id).map_err(|e| e.to_string())?;
        Ok::<String, String>(video.code)
    })
    .await
    .map_err(|e| e.to_string())??;

    if code == "?" || code.starts_with("?:") {
        tracing::warn!(
            "scrape_video: video_id={} has unknown code, skipping",
            video_id
        );
        return Err("Cannot scrape video with unknown code".to_string());
    }

    let pipeline = scraper::ScrapePipeline::new(
        thumbnails.0.clone(),
        actors_state.0.clone(),
        samples_state.0.clone(),
    )?;
    let result = pipeline.scrape_one(&code, &video_id).await;

    let db_path = db.0.clone();
    let vid_id = video_id.clone();
    let actor_details: Vec<models::ActorDetail> = result
        .metadata
        .actor_details
        .iter()
        .map(|a| models::ActorDetail {
            name: a.name.clone(),
            name_kanji: a.name_kanji.clone(),
        })
        .collect();
    let sample_paths: Vec<String> = result
        .sample_image_paths
        .iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();
    let actor_photo_map = result.actor_photo_paths.clone();
    let meta = result.metadata;
    let cover_path = result.cover_path;
    let status = result.status;

    tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;

        db::update_video_metadata(
            &conn,
            &vid_id,
            meta.title.as_deref(),
            cover_path.as_ref().and_then(|p| p.to_str()),
            meta.series.as_deref(),
            meta.duration,
            meta.released_at.as_deref(),
            &actor_details,
            &meta.tags,
            meta.maker.as_deref(),
            &sample_paths,
            status,
        )
        .map_err(|e| e.to_string())?;

        // Update actor photos AFTER upsert so actor rows exist
        for (actor_name, photo_path) in &actor_photo_map {
            let _ = conn.execute(
                "UPDATE actors SET photo_path = ?1 WHERE name = ?2 AND photo_path IS NULL",
                rusqlite::params![photo_path.to_str(), actor_name],
            );
        }

        db::get_video_by_id(&conn, &vid_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scrape_videos(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    actors_state: tauri::State<'_, ActorsDir>,
    samples_state: tauri::State<'_, SamplesDir>,
    cancel: tauri::State<'_, ScrapeCancel>,
    scrape_running: tauri::State<'_, ScrapeRunning>,
    app: tauri::AppHandle,
    video_ids: Vec<String>,
) -> Result<(), String> {
    tracing::info!("cmd: scrape_videos count={}", video_ids.len());
    // Mark scrape as running; prevents concurrent auto-scrape
    scrape_running.0.store(true, Ordering::SeqCst);
    let running_flag = scrape_running.0.clone();
    let db_path = db.0.clone();
    let thumbnails_dir = thumbnails.0.clone();
    let actors_dir = actors_state.0.clone();
    let samples_dir = samples_state.0.clone();
    let cancel_flag = cancel.0.clone();

    cancel_flag.store(false, Ordering::SeqCst);

    // Fetch codes for the requested video IDs
    let ids = video_ids.clone();
    let to_scrape = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for id in &ids {
            if let Ok(video) = db::get_video_by_id(&conn, id) {
                if video.code != "?" {
                    result.push((video.id, video.code));
                }
            }
        }
        Ok::<Vec<(String, String)>, String>(result)
    })
    .await
    .map_err(|e| e.to_string())??;

    let total = to_scrape.len();
    tracing::info!("scrape_videos: {} videos to scrape", total);
    let pipeline = scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir)?;

    for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            tracing::info!("scrape_videos: cancelled at {}/{}", i + 1, total);
            break;
        }

        let result = pipeline.scrape_one(&code, &video_id).await;

        let db_path2 = db.0.clone();
        let vid_id = video_id.clone();
        let actor_details: Vec<models::ActorDetail> = result
            .metadata
            .actor_details
            .iter()
            .map(|a| models::ActorDetail {
                name: a.name.clone(),
                name_kanji: a.name_kanji.clone(),
            })
            .collect();
        let sample_paths: Vec<String> = result
            .sample_image_paths
            .iter()
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();
        let actor_photo_map = result.actor_photo_paths.clone();
        let meta = result.metadata;
        let cover_path = result.cover_path;
        let evt_status = result.status.clone();
        let status = result.status;

        let updated_video = tokio::task::spawn_blocking(move || {
            let conn = db::open(db_path2.to_str().unwrap())?;

            db::update_video_metadata(
                &conn,
                &vid_id,
                meta.title.as_deref(),
                cover_path.as_ref().and_then(|p| p.to_str()),
                meta.series.as_deref(),
                meta.duration,
                meta.released_at.as_deref(),
                &actor_details,
                &meta.tags,
                meta.maker.as_deref(),
                &sample_paths,
                status,
            )?;

            for (actor_name, photo_path) in &actor_photo_map {
                let _ = conn.execute(
                    "UPDATE actors SET photo_path = ?1 WHERE name = ?2 AND photo_path IS NULL",
                    rusqlite::params![photo_path.to_str(), actor_name],
                );
            }

            let video = db::get_video_by_id(&conn, &vid_id).ok();
            Ok::<Option<Video>, rusqlite::Error>(video)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        // Increment retry_count for transient failures (status stays NotScraped)
        if matches!(evt_status, ScrapeStatus::NotScraped) {
            let db_path3 = db.0.clone();
            let vid_id2 = video_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = db::open(db_path3.to_str().unwrap()) {
                    let _ = db::increment_retry_count(&conn, &vid_id2);
                }
            })
            .await;
        }

        let _ = app.emit(
            "scrape-progress",
            ScrapeProgressEvent {
                video_id,
                status: evt_status,
                current: i + 1,
                total,
                video: updated_video,
            },
        );
    }

    tracing::info!("scrape_videos: complete, processed {}", total);
    let _ = app.emit("scrape-complete", total);
    running_flag.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn cancel_scrape(cancel: tauri::State<'_, ScrapeCancel>) -> Result<(), String> {
    tracing::info!("cmd: cancel_scrape");
    cancel.0.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn reset_scrape_status(db: tauri::State<'_, DbPath>, video_ids: Vec<String>) -> Result<(), String> {
    tracing::info!("cmd: reset_scrape_status count={}", video_ids.len());
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::reset_scrape_status(&conn, &video_ids).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reset_data(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    actors_state: tauri::State<'_, ActorsDir>,
    samples_state: tauri::State<'_, SamplesDir>,
) -> Result<(), String> {
    tracing::info!("cmd: reset_data");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::reset_data(&conn).map_err(|e| e.to_string())?;

    // Delete downloaded files
    for dir in [&thumbnails.0, &actors_state.0, &samples_state.0] {
        if dir.exists() {
            let _ = std::fs::remove_dir_all(dir);
            let _ = std::fs::create_dir_all(dir);
        }
    }

    tracing::info!("reset_data: complete");
    Ok(())
}

#[tauri::command]
fn get_or_generate_sprite(
    sprites: tauri::State<'_, SpritesDir>,
    video_id: String,
    file_path: String,
    part_index: u32,
) -> Option<models::SpriteInfo> {
    tracing::info!("cmd: get_or_generate_sprite video_id={} part={}", video_id, part_index);
    ffmpeg::generate_sprite_sheet(&file_path, &video_id, part_index, &sprites.0)
}

#[tauri::command]
fn assign_code(
    db: tauri::State<'_, DbPath>,
    video_id: String,
    new_code: String,
) -> Result<Video, String> {
    tracing::info!("cmd: assign_code video_id={} new_code={}", video_id, new_code);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let final_id = db::assign_code(&conn, &video_id, &new_code).map_err(|e| e.to_string())?;
    db::get_video_by_id(&conn, &final_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn generate_local_samples(
    db: tauri::State<'_, DbPath>,
    samples: tauri::State<'_, SamplesDir>,
    video_id: String,
) -> Result<Vec<SampleImage>, String> {
    tracing::info!("cmd: generate_local_samples video_id={}", video_id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;

    let file_path: String = conn
        .query_row(
            "SELECT path FROM video_files WHERE video_id = ?1 ORDER BY rowid LIMIT 1",
            [&video_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let paths = ffmpeg::extract_sample_images(&file_path, &video_id, &samples.0, 8);
    if !paths.is_empty() {
        db::save_local_sample_images(&conn, &video_id, &paths).map_err(|e| e.to_string())?;
    }

    db::get_sample_images(&conn, &video_id).map_err(|e| e.to_string())
}

fn start_auto_scrape(app: &tauri::AppHandle, db_path: &std::path::Path, thumbnails_dir: &std::path::Path, actors_dir: &std::path::Path, samples_dir: &std::path::Path, cancel_flag: Arc<AtomicBool>, scrape_running: Arc<AtomicBool>) {
    // Skip if a scrape (manual or auto) is already running
    if scrape_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        tracing::info!("auto_scrape: skipped, scrape already in progress");
        return;
    }

    let db_path = db_path.to_path_buf();
    let thumbnails_dir = thumbnails_dir.to_path_buf();
    let actors_dir = actors_dir.to_path_buf();
    let samples_dir = samples_dir.to_path_buf();
    let app = app.clone();
    let running = scrape_running.clone();

    tauri::async_runtime::spawn(async move {
        let to_scrape = {
            let db_str = db_path.to_str().unwrap();
            let conn = match db::open(db_str) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("auto_scrape: db open failed: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            match db::get_unscraped_for_auto(&conn) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("auto_scrape: get_unscraped_for_auto failed: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            }
        };

        if to_scrape.is_empty() {
            running.store(false, Ordering::SeqCst);
            return;
        }

        tracing::info!("auto_scrape: starting for {} videos", to_scrape.len());
        let total = to_scrape.len();
        let pipeline = match scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("auto_scrape: pipeline init failed: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        cancel_flag.store(false, Ordering::SeqCst);

        for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
            if cancel_flag.load(Ordering::SeqCst) {
                tracing::info!("auto_scrape: cancelled at {}/{}", i + 1, total);
                break;
            }

            let result = pipeline.scrape_one(&code, &video_id).await;
            let db_str = db_path.to_str().unwrap().to_string();
            let vid_id = video_id.clone();
            let actor_details: Vec<models::ActorDetail> = result
                .metadata
                .actor_details
                .iter()
                .map(|a| models::ActorDetail {
                    name: a.name.clone(),
                    name_kanji: a.name_kanji.clone(),
                })
                .collect();
            let sample_paths: Vec<String> = result
                .sample_image_paths
                .iter()
                .filter_map(|p| p.to_str().map(|s| s.to_string()))
                .collect();
            let actor_photo_map = result.actor_photo_paths.clone();
            let meta = result.metadata;
            let cover_path = result.cover_path;
            let evt_status = result.status.clone();
            let status = result.status;

            let vid_id_log = video_id.clone();
            let updated_video = tokio::task::spawn_blocking(move || {
                let conn = db::open(&db_str)?;
                db::update_video_metadata(
                    &conn,
                    &vid_id,
                    meta.title.as_deref(),
                    cover_path.as_ref().and_then(|p| p.to_str()),
                    meta.series.as_deref(),
                    meta.duration,
                    meta.released_at.as_deref(),
                    &actor_details,
                    &meta.tags,
                    meta.maker.as_deref(),
                    &sample_paths,
                    status,
                )?;
                for (actor_name, photo_path) in &actor_photo_map {
                    let _ = conn.execute(
                        "UPDATE actors SET photo_path = ?1 WHERE name = ?2 AND photo_path IS NULL",
                        rusqlite::params![photo_path.to_str(), actor_name],
                    );
                }
                let video = db::get_video_by_id(&conn, &vid_id).ok();
                Ok::<Option<Video>, rusqlite::Error>(video)
            })
            .await;
            let updated_video = match &updated_video {
                Ok(Ok(v)) => v.clone(),
                Ok(Err(e)) => {
                    tracing::error!("auto_scrape: [{}] DB error: {}", vid_id_log, e);
                    None
                }
                Err(e) => {
                    tracing::error!("auto_scrape: [{}] spawn_blocking panic: {}", vid_id_log, e);
                    None
                }
            };

            // Increment retry_count for transient failures
            if matches!(evt_status, ScrapeStatus::NotScraped) {
                let db_str2 = db_path.to_str().unwrap().to_string();
                let vid_id2 = video_id.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = db::open(&db_str2) {
                        let _ = db::increment_retry_count(&conn, &vid_id2);
                    }
                })
                .await;
            }

            let _ = app.emit(
                "scrape-progress",
                ScrapeProgressEvent {
                    video_id,
                    status: evt_status,
                    current: i + 1,
                    total,
                    video: updated_video,
                },
            );
        }

        tracing::info!("auto_scrape: complete");
        let _ = app.emit("scrape-complete", total);
        running.store(false, Ordering::SeqCst);
    });
}

fn start_local_sample_extraction(
    db_path: &std::path::Path,
    samples_dir: &std::path::Path,
    running: Arc<AtomicBool>,
) {
    if running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        tracing::info!("local_samples: skipped, extraction already in progress");
        return;
    }

    let db_path = db_path.to_path_buf();
    let samples_dir = samples_dir.to_path_buf();

    tauri::async_runtime::spawn(async move {
        let sem = Arc::new(tokio::sync::Semaphore::new(3));

        // Phase 1: videos with no samples at all
        let no_samples = {
            let db_str = db_path.to_str().unwrap();
            let conn = match db::open(db_str) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("local_samples: db open failed: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            match db::get_videos_needing_samples(&conn) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("local_samples: get_videos_needing_samples failed: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            }
        };

        // Phase 2: videos with low-quality samples
        let low_quality = {
            let db_str = db_path.to_str().unwrap();
            let conn = match db::open(db_str) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("local_samples: db open failed (phase 2): {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            let all_with_samples: Vec<(String, String)> = conn
                .prepare(
                    "SELECT v.id, vf.path FROM videos v
                     JOIN video_files vf ON vf.video_id = v.id
                     JOIN sample_images si ON si.video_id = v.id
                     GROUP BY v.id"
                )
                .and_then(|mut stmt| {
                    stmt.query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?.collect()
                })
                .unwrap_or_else(|e| {
                    tracing::error!("local_samples: query all_with_samples failed: {}", e);
                    Vec::new()
                });

            let mut result = Vec::new();
            for (vid, file_path) in all_with_samples {
                let sample_paths = match db::get_sample_image_paths(&conn, &vid) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("local_samples: get_sample_image_paths failed for {}: {}", vid, e);
                        Vec::new()
                    }
                };
                let any_low = sample_paths.iter().any(|p| ffmpeg::is_low_quality_image(Path::new(p)));
                if any_low {
                    result.push((vid, file_path));
                }
            }
            result
        };

        let mut all_targets: Vec<(String, String)> = no_samples;
        all_targets.extend(low_quality);

        if all_targets.is_empty() {
            running.store(false, Ordering::SeqCst);
            return;
        }

        tracing::info!("local_samples: processing {} videos", all_targets.len());

        let mut handles = Vec::new();
        for (video_id, file_path) in all_targets {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let samples = samples_dir.clone();
            let db = db_path.clone();

            let handle = tokio::task::spawn_blocking(move || {
                let paths = ffmpeg::extract_sample_images(
                    &file_path, &video_id, &samples, 8,
                );
                if !paths.is_empty() {
                    if let Ok(conn) = db::open(db.to_str().unwrap()) {
                        let _ = db::save_local_sample_images(&conn, &video_id, &paths);
                        tracing::info!("local_samples: generated {} samples for {}", paths.len(), video_id);
                    }
                }
                drop(permit);
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.await;
        }

        tracing::info!("local_samples: complete");
        running.store(false, Ordering::SeqCst);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            let exe_path =
                std::env::current_exe().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let exe_dir = exe_path
                .parent()
                .ok_or_else(|| "Cannot determine executable directory".to_string())?;
            let data_dir = exe_dir.join("data");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("library.db");
            let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
            db::init_db(&conn).map_err(|e| e.to_string())?;

            let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
            if settings.log_enabled {
                logging::init_logging(_app.handle().clone(), &data_dir, &settings.log_level);
                logging::cleanup_old_logs(&data_dir);
                tracing::info!("Logging initialized at level: {}", settings.log_level);
            }

            sync_asset_protocol_scope(_app, &settings, &data_dir).map_err(|e| e.to_string())?;

            _app.manage(DataDir(data_dir.clone()));
            _app.manage(DbPath(db_path.clone()));

            let thumbnails_dir = data_dir.join("thumbnails");
            std::fs::create_dir_all(&thumbnails_dir)?;
            _app.manage(ThumbnailsDir(thumbnails_dir));

            let actors_dir = data_dir.join("actors");
            std::fs::create_dir_all(&actors_dir)?;
            _app.manage(ActorsDir(actors_dir));

            let samples_dir = data_dir.join("samples");
            std::fs::create_dir_all(&samples_dir)?;
            _app.manage(SamplesDir(samples_dir));

            let sprites_dir = data_dir.join("sprites");
            std::fs::create_dir_all(&sprites_dir)?;
            _app.manage(SpritesDir(sprites_dir));

            _app.manage(ScrapeCancel(Arc::new(AtomicBool::new(false))));
            _app.manage(ScrapeRunning(Arc::new(AtomicBool::new(false))));
            _app.manage(SampleExtractionRunning(Arc::new(AtomicBool::new(false))));

            // 파일 시스템 워처 시작
            let watcher = watcher::start(
                _app.handle().clone(),
                &settings.scan_folders,
                db_path.clone(),
            )
            .map_err(|e| {
                tracing::warn!("watcher failed to start: {}", e);
                e
            })
            .ok();
            _app.manage(WatcherHandle(Mutex::new(watcher)));

            // Auto-scrape unscraped videos on startup
            start_auto_scrape(
                _app.handle(),
                &db_path,
                &_app.state::<ThumbnailsDir>().0,
                &_app.state::<ActorsDir>().0,
                &_app.state::<SamplesDir>().0,
                _app.state::<ScrapeCancel>().0.clone(),
                _app.state::<ScrapeRunning>().0.clone(),
            );

            // Listen for watcher auto-scrape requests
            let app_handle = _app.handle().clone();
            let db_path2 = db_path.clone();
            _app.listen("auto-scrape-needed", move |_event| {
                let thumbnails = app_handle.state::<ThumbnailsDir>().0.clone();
                let actors = app_handle.state::<ActorsDir>().0.clone();
                let samples = app_handle.state::<SamplesDir>().0.clone();
                let cancel = app_handle.state::<ScrapeCancel>().0.clone();
                let running = app_handle.state::<ScrapeRunning>().0.clone();
                start_auto_scrape(
                    &app_handle,
                    &db_path2,
                    &thumbnails,
                    &actors,
                    &samples,
                    cancel,
                    running,
                );
            });

            // Local sample extraction on startup
            start_local_sample_extraction(
                &db_path,
                &_app.state::<SamplesDir>().0,
                _app.state::<SampleExtractionRunning>().0.clone(),
            );

            // Listen for local sample extraction requests (after scan)
            let app_handle2 = _app.handle().clone();
            let db_path3 = db_path.clone();
            _app.listen("local-samples-needed", move |_event| {
                start_local_sample_extraction(
                    &db_path3,
                    &app_handle2.state::<SamplesDir>().0,
                    app_handle2.state::<SampleExtractionRunning>().0.clone(),
                );
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_library,
            get_videos,
            get_video,
            open_with_player,
            open_folder,
            mark_watched,
            toggle_favorite,
            get_settings,
            save_settings,
            scrape_video,
            scrape_videos,
            cancel_scrape,
            reset_scrape_status,
            reset_data,
            get_actors,
            get_series_list,
            get_tags,
            get_tag_cooccurrence,
            get_makers,
            get_sample_images,
            assign_code,
            get_or_generate_sprite,
            generate_local_samples,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::asset_scope_paths;
    use crate::models::Settings;
    use std::path::Path;

    #[test]
    fn asset_scope_paths_includes_data_dir_and_unique_scan_folders() {
        let settings = Settings {
            scan_folders: vec![
                "C:/Videos".to_string(),
                "  C:/Videos  ".to_string(),
                "".to_string(),
                "D:/Archive".to_string(),
            ],
            player_path: None,
            log_enabled: true,
            log_level: "info".to_string(),
        };

        let paths = asset_scope_paths(&settings, Path::new("C:/App/data"));
        let rendered: Vec<String> = paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();

        assert_eq!(
            rendered,
            vec![
                "C:/App/data".to_string(),
                "C:/Videos".to_string(),
                "D:/Archive".to_string(),
            ]
        );
    }
}
