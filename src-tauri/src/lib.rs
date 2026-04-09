mod db;
mod models;
mod player;
mod scanner;
mod scraper;

use models::{Settings, ScrapeStatus, Video, Actor, Maker, Series as SeriesModel, Tag, SampleImage};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

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
fn get_actors(db: tauri::State<'_, DbPath>) -> Result<Vec<Actor>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_actors(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_series_list(db: tauri::State<'_, DbPath>) -> Result<Vec<SeriesModel>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_series(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tags(db: tauri::State<'_, DbPath>) -> Result<Vec<Tag>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_tags(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_makers(db: tauri::State<'_, DbPath>) -> Result<Vec<Maker>, String> {
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_makers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sample_images(db: tauri::State<'_, DbPath>, video_id: String) -> Result<Vec<SampleImage>, String> {
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
    let pipeline = scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir)?;

    for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
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

        tokio::task::spawn_blocking(move || {
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

            Ok::<(), rusqlite::Error>(())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let _ = app.emit("scrape-progress", ScrapeProgressEvent {
            video_id,
            status: evt_status,
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
        .plugin(tauri_plugin_dialog::init())
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

            let actors_dir = app_data.join("actors");
            std::fs::create_dir_all(&actors_dir)?;
            app.manage(ActorsDir(actors_dir));

            let samples_dir = app_data.join("samples");
            std::fs::create_dir_all(&samples_dir)?;
            app.manage(SamplesDir(samples_dir));

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
            get_actors,
            get_series_list,
            get_tags,
            get_makers,
            get_sample_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
