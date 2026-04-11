use std::process::Command;

pub fn open_with_player(file_path: &str, player_path: Option<&str>) -> Result<(), String> {
    match player_path {
        Some(path) => {
            tracing::info!("player: launching {:?} with file {:?}", path, file_path);
            Command::new(path).arg(file_path).spawn().map_err(|e| {
                let msg = format!("Failed to launch player '{}': {}", path, e);
                tracing::error!("player: {}", msg);
                msg
            })?;
        }
        None => {
            tracing::info!("player: opening with system default: {:?}", file_path);
            open::that(file_path).map_err(|e| {
                let msg = format!("Failed to open '{}': {}", file_path, e);
                tracing::error!("player: {}", msg);
                msg
            })?;
        }
    }
    Ok(())
}
