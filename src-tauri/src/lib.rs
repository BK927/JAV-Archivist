mod db;
mod models;
mod player;
mod scanner;
mod scraper;

use models::{Settings, ScrapeStatus, Video};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

struct DbPath(PathBuf);
struct ThumbnailsDir(PathBuf);
struct ScrapeCancel(Arc<AtomicBool>);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeProgressEvent {
    video_id: String,
    status: ScrapeStatus,
    current: usize,
    total: usize,
}

#[tauri::command]
fn scan_library(db: tauri::State<'_, DbPath>) -> Result<Vec<Video>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;
    db::get_all_videos(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_videos(db: tauri::State<'_, DbPath>) -> Result<Vec<Video>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_all_videos(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_video(db: tauri::State<'_, DbPath>, id: String) -> Result<Video, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_video_by_id(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_with_player(db: tauri::State<'_, DbPath>, file_path: String) -> Result<(), String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    player::open_with_player(&file_path, settings.player_path.as_deref())
}

#[tauri::command]
fn mark_watched(db: tauri::State<'_, DbPath>, id: String, watched: bool) -> Result<(), String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::set_watched(&conn, &id, watched).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_favorite(db: tauri::State<'_, DbPath>, id: String) -> Result<(), String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let video = db::get_video_by_id(&conn, &id).map_err(|e| e.to_string())?;
    db::set_favorite(&conn, &id, !video.favorite).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(db: tauri::State<'_, DbPath>) -> Result<Settings, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(db: tauri::State<'_, DbPath>, settings: Settings) -> Result<(), String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::save_settings(&conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scrape_video(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    video_id: String,
) -> Result<Video, String> {
    let db_path = db.0.clone();

    // Get video code from DB
    let vid_id = video_id.clone();
    let code = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        let video = db::get_video_by_id(&conn, &vid_id).map_err(|e| e.to_string())?;
        Ok::<String, String>(video.code)
    })
    .await
    .map_err(|e| e.to_string())??;

    if code == "?" {
        return Err("Cannot scrape video with unknown code".to_string());
    }

    let pipeline = scraper::ScrapePipeline::new(thumbnails.0.clone());
    let (meta, thumb_path, status) = pipeline.scrape_one(&code, &video_id).await;

    let db_path = db.0.clone();
    let vid_id = video_id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        db::update_video_metadata(
            &conn,
            &vid_id,
            meta.title.as_deref(),
            thumb_path.as_ref().and_then(|p| p.to_str()),
            meta.series.as_deref(),
            meta.duration,
            meta.released_at.as_deref(),
            &meta.actors,
            &meta.tags,
            status,
        )
        .map_err(|e| e.to_string())?;
        db::get_video_by_id(&conn, &vid_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scrape_all_new(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    cancel: tauri::State<'_, ScrapeCancel>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let db_path = db.0.clone();
    let thumbnails_dir = thumbnails.0.clone();
    let cancel_flag = cancel.0.clone();

    // Reset cancel flag
    cancel_flag.store(false, Ordering::SeqCst);

    // Get videos to scrape
    let to_scrape = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        db::get_videos_to_scrape(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let total = to_scrape.len();
    let pipeline = scraper::ScrapePipeline::new(thumbnails_dir);

    for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let (meta, thumb_path, status) = pipeline.scrape_one(&code, &video_id).await;

        let db_path2 = db.0.clone();
        let vid_id = video_id.clone();
        let evt_status = status.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db::open(db_path2.to_str().unwrap())?;
            db::update_video_metadata(
                &conn,
                &vid_id,
                meta.title.as_deref(),
                thumb_path.as_ref().and_then(|p| p.to_str()),
                meta.series.as_deref(),
                meta.duration,
                meta.released_at.as_deref(),
                &meta.actors,
                &meta.tags,
                evt_status,
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let _ = app.emit("scrape-progress", ScrapeProgressEvent {
            video_id,
            status,
            current: i + 1,
            total,
        });
    }

    let _ = app.emit("scrape-complete", total);
    Ok(())
}

#[tauri::command]
fn cancel_scrape(cancel: tauri::State<'_, ScrapeCancel>) -> Result<(), String> {
    cancel.0.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("library.db");
            let conn = db::open(db_path.to_str().unwrap())
                .map_err(|e| e.to_string())?;
            db::init_db(&conn)
                .map_err(|e| e.to_string())?;
            app.manage(DbPath(db_path));

            let thumbnails_dir = app_data.join("thumbnails");
            std::fs::create_dir_all(&thumbnails_dir)?;
            app.manage(ThumbnailsDir(thumbnails_dir));

            app.manage(ScrapeCancel(Arc::new(AtomicBool::new(false))));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_library,
            get_videos,
            get_video,
            open_with_player,
            mark_watched,
            toggle_favorite,
            get_settings,
            save_settings,
            scrape_video,
            scrape_all_new,
            cancel_scrape,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
