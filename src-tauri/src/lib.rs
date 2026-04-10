mod db;
mod logging;
mod models;
mod player;
mod scanner;
mod scraper;
mod watcher;

use models::{Settings, ScrapeStatus, Video, Actor, Maker, Series as SeriesModel, Tag, TagCooccurrence, SampleImage};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

struct DataDir(PathBuf);
struct DbPath(PathBuf);
struct ThumbnailsDir(PathBuf);
struct ActorsDir(PathBuf);
struct SamplesDir(PathBuf);
struct ScrapeCancel(Arc<AtomicBool>);

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
            scope.allow_directory(&path, true).map_err(|e| e.to_string())?;
            tracing::debug!("asset scope allowed directory: {}", path.display());
        } else if path.is_file() {
            scope.allow_file(&path).map_err(|e| e.to_string())?;
            tracing::debug!("asset scope allowed file: {}", path.display());
        } else {
            tracing::warn!("asset scope path does not exist, skipping: {}", path.display());
        }
    }

    Ok(())
}

#[tauri::command]
fn scan_library(db: tauri::State<'_, DbPath>) -> Result<Vec<Video>, String> {
    tracing::info!("cmd: scan_library");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;
    db::get_all_videos(&conn).map_err(|e| e.to_string())
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
    settings: Settings,
) -> Result<(), String> {
    tracing::info!("cmd: save_settings");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::save_settings(&conn, &settings).map_err(|e| e.to_string())?;
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
fn get_tag_cooccurrence(db: tauri::State<'_, DbPath>, tag_id: String) -> Result<Vec<TagCooccurrence>, String> {
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
fn get_sample_images(db: tauri::State<'_, DbPath>, video_id: String) -> Result<Vec<SampleImage>, String> {
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

    if code == "?" {
        tracing::warn!("scrape_video: video_id={} has unknown code, skipping", video_id);
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
    let actor_details: Vec<models::ActorDetail> = result.metadata.actor_details.iter().map(|a| {
        models::ActorDetail {
            name: a.name.clone(),
            name_kanji: a.name_kanji.clone(),
        }
    }).collect();
    let sample_paths: Vec<String> = result.sample_image_paths.iter()
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
async fn scrape_all_new(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    actors_state: tauri::State<'_, ActorsDir>,
    samples_state: tauri::State<'_, SamplesDir>,
    cancel: tauri::State<'_, ScrapeCancel>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tracing::info!("cmd: scrape_all_new");
    let db_path = db.0.clone();
    let thumbnails_dir = thumbnails.0.clone();
    let actors_dir = actors_state.0.clone();
    let samples_dir = samples_state.0.clone();
    let cancel_flag = cancel.0.clone();

    cancel_flag.store(false, Ordering::SeqCst);

    let to_scrape = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        db::get_videos_to_scrape(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let total = to_scrape.len();
    tracing::info!("scrape_all_new: {} videos to scrape", total);
    let pipeline = scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir)?;

    for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            tracing::info!("scrape_all_new: cancelled at {}/{}", i + 1, total);
            break;
        }

        let result = pipeline.scrape_one(&code, &video_id).await;

        let db_path2 = db.0.clone();
        let vid_id = video_id.clone();
        let actor_details: Vec<models::ActorDetail> = result.metadata.actor_details.iter().map(|a| {
            models::ActorDetail {
                name: a.name.clone(),
                name_kanji: a.name_kanji.clone(),
            }
        }).collect();
        let sample_paths: Vec<String> = result.sample_image_paths.iter()
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

            // Update actor photos AFTER upsert so actor rows exist
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

        let _ = app.emit("scrape-progress", ScrapeProgressEvent {
            video_id,
            status: evt_status,
            current: i + 1,
            total,
            video: updated_video,
        });
    }

    tracing::info!("scrape_all_new: complete, processed {}", total);
    let _ = app.emit("scrape-complete", total);
    Ok(())
}

#[tauri::command]
fn cancel_scrape(cancel: tauri::State<'_, ScrapeCancel>) -> Result<(), String> {
    tracing::info!("cmd: cancel_scrape");
    cancel.0.store(true, Ordering::SeqCst);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_app| {
            let exe_path = std::env::current_exe()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let exe_dir = exe_path.parent()
                .ok_or_else(|| "Cannot determine executable directory".to_string())?;
            let data_dir = exe_dir.join("data");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("library.db");
            let conn = db::open(db_path.to_str().unwrap())
                .map_err(|e| e.to_string())?;
            db::init_db(&conn)
                .map_err(|e| e.to_string())?;

            let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
            if settings.log_enabled {
                logging::init_logging(_app.handle().clone(), &data_dir, &settings.log_level);
                logging::cleanup_old_logs(&data_dir);
                tracing::info!("Logging initialized at level: {}", settings.log_level);
            }

            sync_asset_protocol_scope(_app, &settings, &data_dir)
                .map_err(|e| e.to_string())?;

            _app.manage(DataDir(data_dir.clone()));
            _app.manage(DbPath(db_path));

            let thumbnails_dir = data_dir.join("thumbnails");
            std::fs::create_dir_all(&thumbnails_dir)?;
            _app.manage(ThumbnailsDir(thumbnails_dir));

            let actors_dir = data_dir.join("actors");
            std::fs::create_dir_all(&actors_dir)?;
            _app.manage(ActorsDir(actors_dir));

            let samples_dir = data_dir.join("samples");
            std::fs::create_dir_all(&samples_dir)?;
            _app.manage(SamplesDir(samples_dir));

            _app.manage(ScrapeCancel(Arc::new(AtomicBool::new(false))));

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
            reset_data,
            get_actors,
            get_series_list,
            get_tags,
            get_tag_cooccurrence,
            get_makers,
            get_sample_images,
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
