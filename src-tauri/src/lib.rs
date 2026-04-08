mod db;
mod models;
mod player;
mod scanner;

use models::{Settings, Video};
use std::path::PathBuf;
use tauri::Manager;

struct DbPath(PathBuf);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
