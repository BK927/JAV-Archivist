use rusqlite::{params, Connection, Result};
use uuid::Uuid;
use crate::models::{Settings, Video, VideoFile, ScrapeStatus, Actor, Maker, Series as SeriesModel, Tag, SampleImage};

pub fn open(path: &str) -> Result<Connection> {
    Connection::open(path)
}

#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    Connection::open_in_memory()
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL,
            title TEXT NOT NULL,
            thumbnail_path TEXT,
            series TEXT,
            series_id TEXT,
            maker_id TEXT,
            duration INTEGER,
            watched INTEGER DEFAULT 0,
            favorite INTEGER DEFAULT 0,
            added_at TEXT NOT NULL,
            released_at TEXT,
            scrape_status TEXT DEFAULT 'not_scraped',
            scraped_at TEXT
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
            name_kanji TEXT,
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
        );

        CREATE TABLE IF NOT EXISTS makers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS series (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            cover_path TEXT
        );

        CREATE TABLE IF NOT EXISTS sample_images (
            id TEXT PRIMARY KEY,
            video_id TEXT NOT NULL REFERENCES videos(id),
            path TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0
        );"
    )?;

    // Migration for existing databases: add scrape columns
    let _ = conn.execute_batch(
        "ALTER TABLE videos ADD COLUMN scrape_status TEXT DEFAULT 'not_scraped';
         ALTER TABLE videos ADD COLUMN scraped_at TEXT;"
    );

    // Migration: add new columns
    let _ = conn.execute("ALTER TABLE actors ADD COLUMN name_kanji TEXT", []);
    let _ = conn.execute("ALTER TABLE videos ADD COLUMN maker_id TEXT", []);
    let _ = conn.execute("ALTER TABLE videos ADD COLUMN series_id TEXT", []);

    migrate_series_to_table(conn)?;

    Ok(())
}

pub fn migrate_series_to_table(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT series FROM videos WHERE series IS NOT NULL AND series != '' AND series_id IS NULL"
    )?;
    let series_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for name in &series_names {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO series (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
    }

    conn.execute_batch(
        "UPDATE videos SET series_id = (SELECT id FROM series WHERE name = videos.series)
         WHERE series IS NOT NULL AND series != '' AND series_id IS NULL"
    )?;

    Ok(())
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
    let mut stmt = conn.prepare(
        "SELECT v.id, v.code, v.title, v.thumbnail_path, v.series, v.duration, v.watched, v.favorite, v.added_at, v.released_at, v.scrape_status, v.scraped_at, m.name as maker_name
         FROM videos v
         LEFT JOIN makers m ON v.maker_id = m.id
         ORDER BY v.added_at DESC"
    )?;
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
            row.get::<_, String>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
        ))
    })?;

    let mut videos = Vec::new();
    for row in video_rows {
        let (id, code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at, scrape_status_str, scraped_at, maker_name) = row?;
        let files = get_video_files(conn, &id)?;
        let actors = get_video_actors(conn, &id)?;
        let tags = get_video_tags(conn, &id)?;

        videos.push(Video {
            id, code, title, files, thumbnail_path, actors, series, tags, duration,
            watched: watched != 0, favorite: favorite != 0, added_at, released_at,
            scrape_status: ScrapeStatus::from_str(&scrape_status_str),
            scraped_at,
            maker_name,
        });
    }
    Ok(videos)
}

pub fn get_video_by_id(conn: &Connection, id: &str) -> Result<Video> {
    let (code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at, scrape_status_str, scraped_at, maker_name) = conn.query_row(
        "SELECT v.code, v.title, v.thumbnail_path, v.series, v.duration, v.watched, v.favorite, v.added_at, v.released_at, v.scrape_status, v.scraped_at, m.name as maker_name
         FROM videos v
         LEFT JOIN makers m ON v.maker_id = m.id
         WHERE v.id = ?1",
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
                row.get::<_, String>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        },
    )?;

    let files = get_video_files(conn, id)?;
    let actors = get_video_actors(conn, id)?;
    let tags = get_video_tags(conn, id)?;

    Ok(Video {
        id: id.to_string(), code, title, files, thumbnail_path, actors, series, tags, duration,
        watched: watched != 0, favorite: favorite != 0, added_at, released_at,
        scrape_status: ScrapeStatus::from_str(&scrape_status_str),
        scraped_at,
        maker_name,
    })
}

pub fn get_actors(conn: &Connection) -> Result<Vec<Actor>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.name, a.name_kanji, a.photo_path, COUNT(va.video_id) as video_count
         FROM actors a
         LEFT JOIN video_actors va ON a.id = va.actor_id
         GROUP BY a.id
         ORDER BY video_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Actor {
            id: row.get(0)?,
            name: row.get(1)?,
            name_kanji: row.get(2)?,
            photo_path: row.get(3)?,
            video_count: row.get::<_, u32>(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_makers(conn: &Connection) -> Result<Vec<Maker>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.name, COUNT(v.id) as video_count
         FROM makers m
         LEFT JOIN videos v ON v.maker_id = m.id
         GROUP BY m.id
         ORDER BY video_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Maker {
            id: row.get(0)?,
            name: row.get(1)?,
            video_count: row.get::<_, u32>(2)?,
        })
    })?;
    rows.collect()
}

pub fn get_series(conn: &Connection) -> Result<Vec<SeriesModel>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, s.cover_path, COUNT(v.id) as video_count
         FROM series s
         LEFT JOIN videos v ON v.series_id = s.id
         GROUP BY s.id
         ORDER BY video_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SeriesModel {
            id: row.get(0)?,
            name: row.get(1)?,
            cover_path: row.get(2)?,
            video_count: row.get::<_, u32>(3)?,
        })
    })?;
    rows.collect()
}

pub fn get_tags(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, COUNT(vt.video_id) as video_count
         FROM tags t
         LEFT JOIN video_tags vt ON t.id = vt.tag_id
         GROUP BY t.id
         ORDER BY video_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            video_count: row.get::<_, u32>(2)?,
        })
    })?;
    rows.collect()
}

pub fn get_sample_images(conn: &Connection, video_id: &str) -> Result<Vec<SampleImage>> {
    let mut stmt = conn.prepare(
        "SELECT id, video_id, path, sort_order FROM sample_images WHERE video_id = ?1 ORDER BY sort_order ASC"
    )?;
    let rows = stmt.query_map([video_id], |row| {
        Ok(SampleImage {
            id: row.get(0)?,
            video_id: row.get(1)?,
            path: row.get(2)?,
            sort_order: row.get::<_, u32>(3)?,
        })
    })?;
    rows.collect()
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

pub fn update_video_metadata(
    conn: &Connection,
    video_id: &str,
    title: Option<&str>,
    thumbnail_path: Option<&str>,
    series: Option<&str>,
    duration: Option<u64>,
    released_at: Option<&str>,
    actors: &[String],
    tags: &[String],
    status: ScrapeStatus,
) -> Result<()> {
    conn.execute_batch("BEGIN")?;

    let result = (|| -> Result<()> {
        conn.execute(
            "UPDATE videos SET
                title = COALESCE(?1, title),
                thumbnail_path = COALESCE(?2, thumbnail_path),
                series = COALESCE(?3, series),
                duration = COALESCE(?4, duration),
                released_at = COALESCE(?5, released_at),
                scrape_status = ?6,
                scraped_at = ?7
             WHERE id = ?8",
            params![
                title,
                thumbnail_path,
                series,
                duration.map(|d| d as i64),
                released_at,
                status.as_str(),
                chrono::Utc::now().to_rfc3339(),
                video_id,
            ],
        )?;

        for actor_name in actors {
            let actor_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO actors (id, name) VALUES (?1, ?2)",
                params![actor_id, actor_name],
            )?;
            let actual_id: String = conn.query_row(
                "SELECT id FROM actors WHERE name = ?1",
                [actor_name],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO video_actors (video_id, actor_id) VALUES (?1, ?2)",
                params![video_id, actual_id],
            )?;
        }

        for tag_name in tags {
            let tag_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO tags (id, name) VALUES (?1, ?2)",
                params![tag_id, tag_name],
            )?;
            let actual_id: String = conn.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                [tag_name],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO video_tags (video_id, tag_id) VALUES (?1, ?2)",
                params![video_id, actual_id],
            )?;
        }

        Ok(())
    })();

    if result.is_ok() {
        conn.execute_batch("COMMIT")?;
    } else {
        let _ = conn.execute_batch("ROLLBACK");
    }
    result
}

pub fn get_videos_to_scrape(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, code FROM videos WHERE code != '?' AND scrape_status = 'not_scraped'"
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
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
            maker_name: None,
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

    #[test]
    fn test_init_db_adds_scrape_columns() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("TEST-001", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();

        let row: (String, Option<String>) = conn.query_row(
            "SELECT scrape_status, scraped_at FROM videos WHERE id = ?1",
            [&video.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();

        assert_eq!(row.0, "not_scraped");
        assert!(row.1.is_none());
    }

    #[test]
    fn test_update_video_metadata() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Original Title", "C:/test.mp4");
        let id = video.id.clone();
        upsert_videos(&conn, &[video]).unwrap();

        update_video_metadata(
            &conn,
            &id,
            Some("Scraped Title"),
            None,
            Some("Test Series"),
            Some(7200),
            Some("2023-12-12"),
            &["Actor One".to_string(), "Actor Two".to_string()],
            &["Tag A".to_string(), "Tag B".to_string()],
            ScrapeStatus::Complete,
        ).unwrap();

        let v = get_video_by_id(&conn, &id).unwrap();
        assert_eq!(v.title, "Scraped Title");
        assert_eq!(v.series, Some("Test Series".to_string()));
        assert_eq!(v.duration, Some(7200));
        assert_eq!(v.released_at, Some("2023-12-12".to_string()));
        assert_eq!(v.scrape_status, ScrapeStatus::Complete);
        assert!(v.scraped_at.is_some());
        let mut actors = v.actors.clone();
        actors.sort();
        assert_eq!(actors, vec!["Actor One", "Actor Two"]);
        let mut tags = v.tags.clone();
        tags.sort();
        assert_eq!(tags, vec!["Tag A", "Tag B"]);
    }

    #[test]
    fn test_update_video_metadata_preserves_existing() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Original Title", "C:/test.mp4");
        let id = video.id.clone();
        upsert_videos(&conn, &[video]).unwrap();

        update_video_metadata(
            &conn,
            &id,
            None,
            None,
            None,
            None,
            None,
            &[],
            &["Tag X".to_string()],
            ScrapeStatus::Partial,
        ).unwrap();

        let v = get_video_by_id(&conn, &id).unwrap();
        assert_eq!(v.title, "Original Title");
        assert_eq!(v.scrape_status, ScrapeStatus::Partial);
        assert_eq!(v.tags, vec!["Tag X"]);
    }

    #[test]
    fn test_get_videos_to_scrape() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let v1 = make_test_video("ABC-123", "Video 1", "C:/v1.mp4");
        let v2 = make_test_video("?", "Unknown", "C:/unknown.mp4");
        upsert_videos(&conn, &[v1, v2]).unwrap();

        let to_scrape = get_videos_to_scrape(&conn).unwrap();
        assert_eq!(to_scrape.len(), 1);
        assert_eq!(to_scrape[0].1, "ABC-123");
    }

    #[test]
    fn test_init_db_creates_new_tables() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"makers".to_string()));
        assert!(tables.contains(&"series".to_string()));
        assert!(tables.contains(&"sample_images".to_string()));
    }

    #[test]
    fn test_actors_table_has_name_kanji() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        conn.execute(
            "INSERT INTO actors (id, name, name_kanji, photo_path) VALUES ('a1', 'Test', '테스트', NULL)",
            [],
        ).unwrap();

        let kanji: Option<String> = conn
            .query_row("SELECT name_kanji FROM actors WHERE id = 'a1'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kanji, Some("테스트".to_string()));
    }

    #[test]
    fn test_series_migration_from_string() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        // Insert a video with series as string (old format)
        conn.execute(
            "INSERT INTO videos (id, code, title, series, added_at) VALUES ('v1', 'ABC-123', 'Test', 'SONE', '2026-01-01')",
            [],
        ).unwrap();

        // Run migration
        migrate_series_to_table(&conn).unwrap();

        // series table should have an entry
        let series_name: String = conn
            .query_row("SELECT name FROM series LIMIT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(series_name, "SONE");

        // video should have series_id set
        let series_id: Option<String> = conn
            .query_row("SELECT series_id FROM videos WHERE id = 'v1'", [], |row| row.get(0))
            .unwrap();
        assert!(series_id.is_some());
    }

    #[test]
    fn test_get_actors() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();

        let actor_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO actors (id, name, name_kanji) VALUES (?1, 'Aoi Rena', '葵レナ')",
            params![actor_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO video_actors (video_id, actor_id) VALUES (?1, ?2)",
            params![video.id, actor_id],
        ).unwrap();

        let actors = get_actors(&conn).unwrap();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].name, "Aoi Rena");
        assert_eq!(actors[0].name_kanji.as_deref(), Some("葵レナ"));
        assert_eq!(actors[0].video_count, 1);
    }

    #[test]
    fn test_get_makers() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let maker_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO makers (id, name) VALUES (?1, 'S1 STYLE')",
            params![maker_id],
        ).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();
        conn.execute(
            "UPDATE videos SET maker_id = ?1 WHERE id = ?2",
            params![maker_id, video.id],
        ).unwrap();

        let makers = get_makers(&conn).unwrap();
        assert_eq!(makers.len(), 1);
        assert_eq!(makers[0].name, "S1 STYLE");
        assert_eq!(makers[0].video_count, 1);
    }

    #[test]
    fn test_get_series_from_table() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let series_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO series (id, name) VALUES (?1, 'SONE')",
            params![series_id],
        ).unwrap();

        let video = make_test_video("SONE-001", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();
        conn.execute(
            "UPDATE videos SET series_id = ?1 WHERE id = ?2",
            params![series_id, video.id],
        ).unwrap();

        let series_list = get_series(&conn).unwrap();
        assert_eq!(series_list.len(), 1);
        assert_eq!(series_list[0].name, "SONE");
        assert_eq!(series_list[0].video_count, 1);
    }

    #[test]
    fn test_get_tags_with_count() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();

        let tag_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tags (id, name) VALUES (?1, '巨乳')",
            params![tag_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO video_tags (video_id, tag_id) VALUES (?1, ?2)",
            params![video.id, tag_id],
        ).unwrap();

        let tags = get_tags(&conn).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "巨乳");
        assert_eq!(tags[0].video_count, 1);
    }

    #[test]
    fn test_get_sample_images() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let video = make_test_video("FC2-PPV-123", "FC2 Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();

        conn.execute(
            "INSERT INTO sample_images (id, video_id, path, sort_order) VALUES ('s1', ?1, '/samples/123_01.jpg', 0)",
            params![video.id],
        ).unwrap();
        conn.execute(
            "INSERT INTO sample_images (id, video_id, path, sort_order) VALUES ('s2', ?1, '/samples/123_02.jpg', 1)",
            params![video.id],
        ).unwrap();

        let images = get_sample_images(&conn, &video.id).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].sort_order, 0);
        assert_eq!(images[1].sort_order, 1);
    }

    #[test]
    fn test_get_video_includes_maker_name() {
        let conn = open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let maker_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO makers (id, name) VALUES (?1, 'S1 STYLE')",
            params![maker_id],
        ).unwrap();

        let video = make_test_video("ABC-123", "Test", "C:/test.mp4");
        upsert_videos(&conn, &[video.clone()]).unwrap();
        conn.execute(
            "UPDATE videos SET maker_id = ?1 WHERE id = ?2",
            params![maker_id, video.id],
        ).unwrap();

        let v = get_video_by_id(&conn, &video.id).unwrap();
        assert_eq!(v.maker_name.as_deref(), Some("S1 STYLE"));
    }
}
