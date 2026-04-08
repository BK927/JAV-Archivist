use std::process::Command;

pub fn open_with_player(file_path: &str, player_path: Option<&str>) -> Result<(), String> {
    match player_path {
        Some(path) => {
            Command::new(path)
                .arg(file_path)
                .spawn()
                .map_err(|e| format!("Failed to launch player '{}': {}", path, e))?;
        }
        None => {
            open::that(file_path)
                .map_err(|e| format!("Failed to open '{}': {}", file_path, e))?;
        }
    }
    Ok(())
}
