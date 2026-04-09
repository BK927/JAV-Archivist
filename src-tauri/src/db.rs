use rusqlite::{params, Connection, Result};
use uuid::Uuid;
use crate::models::{Settings, Video, VideoFile, ScrapeStatus};

pub fn open(path: &str) -> Result<Connection> {
    Connection::open(path)
}

#[cfg(test)]
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

pub fn upsert_videos(conn: &Connection, videos: &[Video]) -> Result<()> {
    conn.execute_batch("BEGIN")?;
    let result = upsert_videos_inner(conn, videos);
    if result.is_ok() {
        conn.execute_batch("COMMIT")?;
    } else {
        let _ = conn.execute_batch("ROLLBACK");
    }
    result
}

fn upsert_videos_inner(conn: &Connection, videos: &[Video]) -> Result<()> {
    for video in videos {
        let existing_id: Option<String> = if video.code != "?" {
            conn.query_row(
                "SELECT id FROM videos WHERE code = ?1",
                [&video.code],
                |row| row.get(0),
            )
            .ok()
        } else {
            None
        };

        let video_id = match existing_id {
            Some(id) => {
                // Existing code: update files only, preserve metadata
                conn.execute("DELETE FROM video_files WHERE video_id = ?1", [&id])?;
                id
            }
            None => {
                // New video: insert record
                conn.execute(
                    "INSERT INTO videos (id, code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        video.id,
                        video.code,
                        video.title,
                        video.thumbnail_path,
                        video.series,
                        video.duration,
                        video.watched as i32,
                        video.favorite as i32,
                        video.added_at,
                        video.released_at,
                    ],
                )?;
                video.id.clone()
            }
        };

        for file in &video.files {
            let file_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR REPLACE INTO video_files (id, video_id, path, size) VALUES (?1, ?2, ?3, ?4)",
                params![file_id, video_id, file.path, file.size as i64],
            )?;
        }
    }
    Ok(())
}

pub fn get_all_videos(conn: &Connection) -> Result<Vec<Video>> {
    let mut stmt = conn.prepare("SELECT id, code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at FROM videos ORDER BY added_at DESC")?;
    let video_rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<u64>>(5)?,
            row.get::<_, i32>(6)?,
            row.get::<_, i32>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;

    let mut videos = Vec::new();
    for row in video_rows {
        let (id, code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at) = row?;
        let files = get_video_files(conn, &id)?;
        let actors = get_video_actors(conn, &id)?;
        let tags = get_video_tags(conn, &id)?;

        videos.push(Video {
            id, code, title, files, thumbnail_path, actors, series, tags, duration,
            watched: watched != 0, favorite: favorite != 0, added_at, released_at,
            scrape_status: ScrapeStatus::NotScraped,
            scraped_at: None,
        });
    }
    Ok(videos)
}

pub fn get_video_by_id(conn: &Connection, id: &str) -> Result<Video> {
    let (code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at) = conn.query_row(
        "SELECT code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at FROM videos WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<u64>>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        },
    )?;

    let files = get_video_files(conn, id)?;
    let actors = get_video_actors(conn, id)?;
    let tags = get_video_tags(conn, id)?;

    Ok(Video {
        id: id.to_string(), code, title, files, thumbnail_path, actors, series, tags, duration,
        watched: watched != 0, favorite: favorite != 0, added_at, released_at,
        scrape_status: ScrapeStatus::NotScraped,
        scraped_at: None,
    })
}

fn get_video_files(conn: &Connection, video_id: &str) -> Result<Vec<VideoFile>> {
    let mut stmt = conn.prepare("SELECT path, size FROM video_files WHERE video_id = ?1")?;
    let files = stmt
        .query_map([video_id], |row| {
            Ok(VideoFile {
                path: row.get(0)?,
                size: row.get::<_, i64>(1)? as u64,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(files)
}

fn get_video_actors(conn: &Connection, video_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT a.name FROM actors a JOIN video_actors va ON a.id = va.actor_id WHERE va.video_id = ?1"
    )?;
    let actors = stmt
        .query_map([video_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(actors)
}

fn get_video_tags(conn: &Connection, video_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t JOIN video_tags vt ON t.id = vt.tag_id WHERE vt.video_id = ?1"
    )?;
    let tags = stmt
        .query_map([video_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tags)
}

pub fn set_watched(conn: &Connection, id: &str, watched: bool) -> Result<()> {
    conn.execute(
        "UPDATE videos SET watched = ?1 WHERE id = ?2",
        params![watched as i32, id],
    )?;
    Ok(())
}

pub fn set_favorite(conn: &Connection, id: &str, favorite: bool) -> Result<()> {
    conn.execute(
        "UPDATE videos SET favorite = ?1 WHERE id = ?2",
        params![favorite as i32, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Settings, VideoFile, ScrapeStatus};

    fn make_test_video(code: &str, title: &str, path: &str) -> Video {
        Video {
            id: uuid::Uuid::new_v4().to_string(),
            code: code.to_string(),
            title: title.to_string(),
            files: vec![VideoFile {
                path: path.to_string(),
                size: 1_000_000,
            }],
            thumbnail_path: None,
            actors: vec![],
            series: None,
            tags: vec![],
            duration: None,
            watched: false,
            favorite: false,
            added_at: "2026-04-09T00:00:00Z".to_string(),
            released_at: None,
            scrape_status: ScrapeStatus::NotScraped,
            scraped_at: None,
        }
    }

    #[test]
    fn test_upsert_and_get_all_videos() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let videos = vec![
            make_test_video("ABC-123", "Test Video 1", "C:/Videos/ABC-123.mp4"),
            make_test_video("DEF-456", "Test Video 2", "C:/Videos/DEF-456.mp4"),
        ];
        upsert_videos(&conn, &videos).unwrap();

        let all = get_all_videos(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].files.len(), 1);
    }

    #[test]
    fn test_upsert_existing_code_updates_files_only() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let v1 = vec![make_test_video("ABC-123", "Original Title", "C:/old.mp4")];
        upsert_videos(&conn, &v1).unwrap();

        let v2 = vec![make_test_video("ABC-123", "New Title", "C:/new.mp4")];
        upsert_videos(&conn, &v2).unwrap();

        let all = get_all_videos(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Original Title"); // title NOT overwritten
        assert_eq!(all[0].files[0].path, "C:/new.mp4"); // file IS updated
    }

    #[test]
    fn test_unknown_code_not_deduped() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let videos = vec![
            make_test_video("?", "Unknown 1", "C:/unknown1.mp4"),
            make_test_video("?", "Unknown 2", "C:/unknown2.mp4"),
        ];
        upsert_videos(&conn, &videos).unwrap();

        let all = get_all_videos(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_video_by_id() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        let id = video.id.clone();
        upsert_videos(&conn, &[video]).unwrap();

        let found = get_video_by_id(&conn, &id).unwrap();
        assert_eq!(found.code, "ABC-123");
    }

    #[test]
    fn test_set_watched() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        let id = video.id.clone();
        upsert_videos(&conn, &[video]).unwrap();

        set_watched(&conn, &id, true).unwrap();
        let v = get_video_by_id(&conn, &id).unwrap();
        assert!(v.watched);

        set_watched(&conn, &id, false).unwrap();
        let v = get_video_by_id(&conn, &id).unwrap();
        assert!(!v.watched);
    }

    #[test]
    fn test_set_favorite() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        let id = video.id.clone();
        upsert_videos(&conn, &[video]).unwrap();

        set_favorite(&conn, &id, true).unwrap();
        let v = get_video_by_id(&conn, &id).unwrap();
        assert!(v.favorite);
    }

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
