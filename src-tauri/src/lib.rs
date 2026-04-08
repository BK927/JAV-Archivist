#[tauri::command]
fn scan_library() -> Vec<serde_json::Value> {
    // TODO: 실제 파일 스캔 구현 (별도 설계)
    vec![]
}

#[tauri::command]
fn open_with_player(file_path: String) -> Result<(), String> {
    // TODO: 외부 플레이어 실행 구현
    println!("open_with_player: {}", file_path);
    Ok(())
}

#[tauri::command]
fn mark_watched(id: String) -> Result<(), String> {
    println!("mark_watched: {}", id);
    Ok(())
}

#[tauri::command]
fn toggle_favorite(id: String) -> Result<(), String> {
    println!("toggle_favorite: {}", id);
    Ok(())
}

#[tauri::command]
fn get_settings() -> serde_json::Value {
    serde_json::json!({
        "scanFolders": [],
        "playerPath": ""
    })
}

#[tauri::command]
fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    println!("save_settings: {:?}", settings);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_library,
            open_with_player,
            mark_watched,
            toggle_favorite,
            get_settings,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
