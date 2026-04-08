use rusqlite::{Connection, Result};
use crate::models::Settings;

pub fn open(path: &str) -> Result<Connection> {
    Connection::open(path)
}

pub fn open_in_memory() -> Result<Connection> {
    Connection::open_in_memory()
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL,
            title TEXT NOT NULL,
            thumbnail_path TEXT,
            series TEXT,
            duration INTEGER,
            watched INTEGER DEFAULT 0,
            favorite INTEGER DEFAULT 0,
            added_at TEXT NOT NULL,
            released_at TEXT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_videos_code ON videos(code) WHERE code != '?';

        CREATE TABLE IF NOT EXISTS video_files (
            id TEXT PRIMARY KEY,
            video_id TEXT NOT NULL REFERENCES videos(id),
            path TEXT NOT NULL UNIQUE,
            size INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS actors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            photo_path TEXT
        );

        CREATE TABLE IF NOT EXISTS video_actors (
            video_id TEXT REFERENCES videos(id),
            actor_id TEXT REFERENCES actors(id),
            PRIMARY KEY (video_id, actor_id)
        );

        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS video_tags (
            video_id TEXT REFERENCES videos(id),
            tag_id TEXT REFERENCES tags(id),
            PRIMARY KEY (video_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    )
}

pub fn get_settings(conn: &Connection) -> Result<Settings> {
    let scan_folders_json: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'scan_folders'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "[]".to_string());

    let player_path: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'player_path'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|v| !v.is_empty());

    Ok(Settings {
        scan_folders: serde_json::from_str(&scan_folders_json).unwrap_or_default(),
        player_path,
    })
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> Result<()> {
    let folders_json = serde_json::to_string(&settings.scan_folders)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('scan_folders', ?1)",
        [&folders_json],
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('player_path', ?1)",
        [settings.player_path.as_deref().unwrap_or("")],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Settings;

    #[test]
    fn test_init_db_creates_tables() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"videos".to_string()));
        assert!(tables.contains(&"video_files".to_string()));
        assert!(tables.contains(&"actors".to_string()));
        assert!(tables.contains(&"video_actors".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"video_tags".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }

    #[test]
    fn test_init_db_is_idempotent() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();
        init_db(&conn).unwrap(); // second call should not error
    }

    #[test]
    fn test_get_settings_defaults() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let settings = get_settings(&conn).unwrap();
        assert!(settings.scan_folders.is_empty());
        assert!(settings.player_path.is_none());
    }

    #[test]
    fn test_save_and_get_settings() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let settings = Settings {
            scan_folders: vec!["C:/Videos".to_string(), "D:/JAV".to_string()],
            player_path: Some("C:/mpv/mpv.exe".to_string()),
        };
        save_settings(&conn, &settings).unwrap();

        let loaded = get_settings(&conn).unwrap();
        assert_eq!(loaded.scan_folders, vec!["C:/Videos", "D:/JAV"]);
        assert_eq!(loaded.player_path, Some("C:/mpv/mpv.exe".to_string()));
    }

    #[test]
    fn test_save_settings_overwrites() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let s1 = Settings {
            scan_folders: vec!["C:/Old".to_string()],
            player_path: Some("old.exe".to_string()),
        };
        save_settings(&conn, &s1).unwrap();

        let s2 = Settings {
            scan_folders: vec!["C:/New".to_string()],
            player_path: None,
        };
        save_settings(&conn, &s2).unwrap();

        let loaded = get_settings(&conn).unwrap();
        assert_eq!(loaded.scan_folders, vec!["C:/New"]);
        assert!(loaded.player_path.is_none());
    }
}
