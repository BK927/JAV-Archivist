use rusqlite::{Connection, Result};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
