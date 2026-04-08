# Core Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Rust backend for JAV Archivist — file scanning, SQLite storage, external player, and settings management.

**Architecture:** Modular Rust backend with `models.rs` (types), `db.rs` (SQLite CRUD), `scanner.rs` (file scan + code extraction), `player.rs` (external player). `lib.rs` is a thin Tauri command layer that delegates to modules. DB path managed via `Tauri::State`.

**Tech Stack:** Rust, Tauri 2, rusqlite (bundled), regex, walkdir, open, uuid, chrono

**Spec:** `docs/superpowers/specs/2026-04-09-core-backend-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| Create: `src-tauri/src/models.rs` | Shared types: Video, VideoFile, Settings |
| Create: `src-tauri/src/db.rs` | SQLite init, schema, all CRUD operations |
| Create: `src-tauri/src/scanner.rs` | File scanning, code extraction, grouping |
| Create: `src-tauri/src/player.rs` | External player launch |
| Modify: `src-tauri/src/lib.rs` | Replace stubs with real Tauri commands |
| Modify: `src-tauri/Cargo.toml` | Add dependencies |
| Modify: `src/types/index.ts` | Update Video, AppSettings types |
| Modify: `src/lib/mockData.ts` | Update mock data to match new types |
| Modify: `src/components/detail/VideoDetail.tsx` | Use `files[0]` instead of `filePath` |
| Modify: `src/components/detail/InAppPlayer.tsx` | Use `files[0]` instead of `filePath` |
| Modify: `src/pages/SettingsPage.tsx` | Handle nullable `playerPath` |

---

### Task 1: Dependencies & Data Models

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs` (add module declarations only)

- [ ] **Step 1: Add Cargo dependencies**

Edit `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
rusqlite = { version = "0.34", features = ["bundled"] }
regex = "1"
walkdir = "2"
open = "5"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

Add under a new `[dev-dependencies]` section:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create `models.rs`**

Create `src-tauri/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub code: String,
    pub title: String,
    pub files: Vec<VideoFile>,
    pub thumbnail_path: Option<String>,
    pub actors: Vec<String>,
    pub series: Option<String>,
    pub tags: Vec<String>,
    pub duration: Option<u64>,
    pub watched: bool,
    pub favorite: bool,
    pub added_at: String,
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub scan_folders: Vec<String>,
    pub player_path: Option<String>,
}
```

Note: `#[serde(rename_all = "camelCase")]` ensures Rust's `snake_case` fields serialize to `camelCase` for the TypeScript frontend (e.g., `thumbnail_path` → `thumbnailPath`).

- [ ] **Step 3: Add module declarations to `lib.rs`**

Add these lines at the top of `src-tauri/src/lib.rs` (before the existing `#[tauri::command]` functions):

```rust
mod db;
mod models;
mod player;
mod scanner;
```

- [ ] **Step 4: Create empty module files**

Create these placeholder files so the project compiles:

`src-tauri/src/db.rs`:
```rust
// DB module — implemented in Task 2
```

`src-tauri/src/scanner.rs`:
```rust
// Scanner module — implemented in Task 5
```

`src-tauri/src/player.rs`:
```rust
// Player module — implemented in Task 7
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles with warnings about unused modules (that's fine).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/scanner.rs src-tauri/src/player.rs src-tauri/src/lib.rs
git commit -m "feat: add cargo deps and data models for core backend"
```

---

### Task 2: DB Schema & Initialization

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write tests for DB initialization**

Replace the contents of `src-tauri/src/db.rs` with:

```rust
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

        // Verify all tables exist by querying sqlite_master
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
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test db::tests`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add DB schema initialization with tests"
```

---

### Task 3: Settings CRUD

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write tests for settings**

Add these tests to the `mod tests` block at the bottom of `src-tauri/src/db.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test db::tests`
Expected: FAIL — `get_settings` and `save_settings` don't exist yet.

- [ ] **Step 3: Implement settings CRUD**

Add these imports and functions to `src-tauri/src/db.rs` (after `init_db`, before `#[cfg(test)]`):

```rust
use crate::models::Settings;

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
```

Also add at the top of the file, after the existing `use` statement:

```rust
use crate::models::{Settings, Video, VideoFile};
```

(Replace the `use crate::models::Settings;` line added above — include all types we'll need.)

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test db::tests`
Expected: 5 tests pass (2 from Task 2 + 3 new).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add settings CRUD with tests"
```

---

### Task 4: Video CRUD

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write tests for video operations**

Add these tests to the `mod tests` block in `src-tauri/src/db.rs`:

```rust
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

        // First insert
        let v1 = vec![make_test_video("ABC-123", "Original Title", "C:/old.mp4")];
        upsert_videos(&conn, &v1).unwrap();

        // Upsert with same code but different file
        let v2 = vec![make_test_video("ABC-123", "New Title", "C:/new.mp4")];
        upsert_videos(&conn, &v2).unwrap();

        let all = get_all_videos(&conn).unwrap();
        assert_eq!(all.len(), 1); // still one video
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
        assert_eq!(all.len(), 2); // both kept, not deduped
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test db::tests`
Expected: FAIL — `upsert_videos`, `get_all_videos`, `get_video_by_id`, `set_watched`, `set_favorite` don't exist yet.

- [ ] **Step 3: Implement video CRUD functions**

Add these functions to `src-tauri/src/db.rs` (after `save_settings`, before `#[cfg(test)]`):

```rust
use rusqlite::params;
use uuid::Uuid;

pub fn upsert_videos(conn: &Connection, videos: &[Video]) -> Result<()> {
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
            row.get::<_, String>(0)?,  // id
            row.get::<_, String>(1)?,  // code
            row.get::<_, String>(2)?,  // title
            row.get::<_, Option<String>>(3)?, // thumbnail_path
            row.get::<_, Option<String>>(4)?, // series
            row.get::<_, Option<u64>>(5)?,    // duration
            row.get::<_, i32>(6)?,     // watched
            row.get::<_, i32>(7)?,     // favorite
            row.get::<_, String>(8)?,  // added_at
            row.get::<_, Option<String>>(9)?, // released_at
        ))
    })?;

    let mut videos = Vec::new();
    for row in video_rows {
        let (id, code, title, thumbnail_path, series, duration, watched, favorite, added_at, released_at) = row?;

        let files = get_video_files(conn, &id)?;
        let actors = get_video_actors(conn, &id)?;
        let tags = get_video_tags(conn, &id)?;

        videos.push(Video {
            id,
            code,
            title,
            files,
            thumbnail_path,
            actors,
            series,
            tags,
            duration,
            watched: watched != 0,
            favorite: favorite != 0,
            added_at,
            released_at,
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
        id: id.to_string(),
        code,
        title,
        files,
        thumbnail_path,
        actors,
        series,
        tags,
        duration,
        watched: watched != 0,
        favorite: favorite != 0,
        added_at,
        released_at,
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
```

Note: Move the `use` statements to the top of the file. The final import block should be:

```rust
use rusqlite::{params, Connection, Result};
use uuid::Uuid;
use crate::models::{Settings, Video, VideoFile};
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test db::tests`
Expected: 11 tests pass (2 init + 3 settings + 6 video).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add video CRUD operations with tests"
```

---

### Task 5: Code Extraction

**Files:**
- Modify: `src-tauri/src/scanner.rs`

- [ ] **Step 1: Write tests for code extraction**

Replace the contents of `src-tauri/src/scanner.rs` with:

```rust
use regex::Regex;

/// Extract a video code from a text string (filename or folder name).
/// Returns the normalized code or None if no pattern matches.
pub fn extract_code(text: &str) -> Option<String> {
    // FC2 pattern: FC2-PPV-123, FC2PPV 123, FC2PPV123, etc.
    let fc2_re = Regex::new(r"(?i)FC2[-\s]?PPV[-\s]?(\d+)").unwrap();
    if let Some(caps) = fc2_re.captures(text) {
        let digits = &caps[1];
        return Some(format!("FC2-PPV-{}", digits));
    }

    // General pattern: ABC-123, ABCD-12345
    let general_re = Regex::new(r"(?i)([A-Z]{2,6})-(\d{3,5})").unwrap();
    if let Some(caps) = general_re.captures(text) {
        let prefix = caps[1].to_uppercase();
        let number = &caps[2];
        return Some(format!("{}-{}", prefix, number));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_code() {
        assert_eq!(extract_code("ABC-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("ABCD-12345"), Some("ABCD-12345".to_string()));
        assert_eq!(extract_code("SONE-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_case_insensitive() {
        assert_eq!(extract_code("abc-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("sone-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_in_noisy_filename() {
        assert_eq!(
            extract_code("[1080p] ABC-123 actress_name"),
            Some("ABC-123".to_string())
        );
        assert_eq!(
            extract_code("some_prefix_MIDE-456_suffix"),
            Some("MIDE-456".to_string())
        );
    }

    #[test]
    fn test_fc2_canonical() {
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_no_hyphens() {
        assert_eq!(
            extract_code("FC2PPV1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_with_spaces() {
        assert_eq!(
            extract_code("FC2PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
        assert_eq!(
            extract_code("FC2 PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_case_insensitive() {
        assert_eq!(
            extract_code("fc2-ppv-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_takes_priority_over_general() {
        // "FC2" matches both FC2 pattern and general pattern.
        // FC2 pattern should be checked first.
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_no_match() {
        assert_eq!(extract_code("random_video"), None);
        assert_eq!(extract_code("video_20240301"), None);
        assert_eq!(extract_code(""), None);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test scanner::tests`
Expected: 9 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "feat: add video code extraction with regex patterns"
```

---

### Task 6: File Scanning & Grouping

**Files:**
- Modify: `src-tauri/src/scanner.rs`

- [ ] **Step 1: Write tests for scanning**

Add these imports and tests to `src-tauri/src/scanner.rs`.

Add imports at the top of the file:

```rust
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;
use uuid::Uuid;
use chrono::Utc;
use crate::models::{Video, VideoFile};
```

Add these tests to the `mod tests` block:

```rust
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_scan_empty_folder() {
        let dir = TempDir::new().unwrap();
        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_finds_video_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ABC-123.mp4"), "fake").unwrap();
        fs::write(dir.path().join("DEF-456.mkv"), "fake").unwrap();
        fs::write(dir.path().join("readme.txt"), "not a video").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 2);

        let codes: Vec<&str> = result.iter().map(|v| v.code.as_str()).collect();
        assert!(codes.contains(&"ABC-123"));
        assert!(codes.contains(&"DEF-456"));
    }

    #[test]
    fn test_scan_groups_same_code() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("ABC-123.mp4"), "fake").unwrap();
        fs::write(dir.path().join("ABC-123_part2.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
        assert_eq!(result[0].files.len(), 2);
    }

    #[test]
    fn test_scan_extracts_code_from_folder() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("ABC-123");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("video.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
    }

    #[test]
    fn test_scan_unknown_code() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("random_video.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "?");
    }

    #[test]
    fn test_scan_recursive() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("ABC-123.mp4"), "fake").unwrap();

        let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "ABC-123");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test scanner::tests`
Expected: FAIL — `scan_folders` doesn't exist yet.

- [ ] **Step 3: Implement scan_folders and group_by_code**

Add these constants and functions to `src-tauri/src/scanner.rs` (after `extract_code`, before `#[cfg(test)]`):

```rust
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "wmv", "flv", "mov", "ts", "m4v"];

struct ScannedFile {
    path: String,
    size: u64,
    code: String,
    filename: String,
}

pub fn scan_folders(folders: &[String]) -> Result<Vec<Video>, String> {
    let mut scanned: Vec<ScannedFile> = Vec::new();

    for folder in folders {
        for entry in WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            let filename = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            let code = extract_code(&filename)
                .or_else(|| {
                    path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .and_then(extract_code)
                })
                .unwrap_or_else(|| "?".to_string());

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            scanned.push(ScannedFile {
                path: path.to_string_lossy().to_string(),
                size,
                code,
                filename,
            });
        }
    }

    Ok(group_by_code(scanned))
}

fn group_by_code(files: Vec<ScannedFile>) -> Vec<Video> {
    let mut groups: HashMap<String, Vec<ScannedFile>> = HashMap::new();
    let mut unknown: Vec<Video> = Vec::new();
    let now = Utc::now().to_rfc3339();

    for file in files {
        if file.code == "?" {
            unknown.push(Video {
                id: Uuid::new_v4().to_string(),
                code: "?".to_string(),
                title: file.filename.clone(),
                files: vec![VideoFile {
                    path: file.path,
                    size: file.size,
                }],
                thumbnail_path: None,
                actors: vec![],
                series: None,
                tags: vec![],
                duration: None,
                watched: false,
                favorite: false,
                added_at: now.clone(),
                released_at: None,
            });
        } else {
            groups.entry(file.code.clone()).or_default().push(file);
        }
    }

    let mut videos: Vec<Video> = groups
        .into_iter()
        .map(|(code, files)| {
            let title = files[0].filename.clone();
            Video {
                id: Uuid::new_v4().to_string(),
                code,
                title,
                files: files
                    .into_iter()
                    .map(|f| VideoFile {
                        path: f.path,
                        size: f.size,
                    })
                    .collect(),
                thumbnail_path: None,
                actors: vec![],
                series: None,
                tags: vec![],
                duration: None,
                watched: false,
                favorite: false,
                added_at: now.clone(),
                released_at: None,
            }
        })
        .collect();

    videos.extend(unknown);
    videos
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test scanner::tests`
Expected: 15 tests pass (9 extract_code + 6 scan).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "feat: add file scanning with code grouping"
```

---

### Task 7: External Player

**Files:**
- Modify: `src-tauri/src/player.rs`

- [ ] **Step 1: Implement player module**

Replace the contents of `src-tauri/src/player.rs` with:

```rust
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
```

No unit tests for this module — it launches external processes which can't be meaningfully tested without side effects. The function is 10 lines of straightforward delegation.

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/player.rs
git commit -m "feat: add external player launcher"
```

---

### Task 8: Tauri Integration

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace lib.rs with real Tauri commands**

Replace the entire contents of `src-tauri/src/lib.rs` with:

```rust
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
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            db::init_db(&conn)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
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
```

Note: The `setup` closure uses `anyhow::anyhow!` for error conversion. Check if `anyhow` is needed — Tauri's setup expects `Box<dyn Error>`. If `anyhow` isn't available, use `.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)` instead, or add `anyhow = "1"` to Cargo.toml.

Alternative without `anyhow` — use this for the setup closure:

```rust
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
```

Use whichever compiles. The Tauri `setup` closure returns `Result<(), Box<dyn Error>>`, so `.map_err(|e| e.to_string())?` should work since `String` implements `Error` via `Into<Box<dyn Error>>`.

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: Compiles successfully. If `anyhow` is needed, add it to `Cargo.toml` dependencies and retry.

- [ ] **Step 3: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass (db + scanner tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire up Tauri commands to backend modules"
```

---

### Task 9: Frontend Type Updates

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/lib/mockData.ts`
- Modify: `src/components/detail/VideoDetail.tsx`
- Modify: `src/components/detail/InAppPlayer.tsx`
- Modify: `src/pages/SettingsPage.tsx`

- [ ] **Step 1: Update types**

In `src/types/index.ts`, make these changes:

Replace the `Video` interface:

```typescript
export interface VideoFile {
  path: string
  size: number
}

export interface Video {
  id: string
  code: string
  title: string
  files: VideoFile[]
  thumbnailPath: string | null
  actors: string[]
  series: string | null
  tags: string[]
  duration: number | null
  watched: boolean
  favorite: boolean
  addedAt: string
  releasedAt: string | null
}
```

Replace the `AppSettings` interface:

```typescript
export interface AppSettings {
  scanFolders: string[]
  playerPath: string | null
}
```

- [ ] **Step 2: Update mock data**

In `src/lib/mockData.ts`, replace every `filePath: '...'` with a `files` array. For each mock video, change:

```typescript
filePath: 'C:/Videos/SONE-001.mp4',
```

to:

```typescript
files: [{ path: 'C:/Videos/SONE-001.mp4', size: 1_000_000 }],
```

Apply this to all 8 mock videos (ids 1–8). The paths stay the same, just wrapped in the `files` array format.

Also update `MOCK_SETTINGS`:

```typescript
export const MOCK_SETTINGS: AppSettings = {
  scanFolders: ['C:/Videos'],
  playerPath: 'C:/Program Files/mpv/mpv.exe',
}
```

No change needed here — `'C:/Program Files/mpv/mpv.exe'` is already a valid `string | null`.

- [ ] **Step 3: Update InAppPlayer.tsx**

In `src/components/detail/InAppPlayer.tsx`, change the props interface from:

```typescript
interface InAppPlayerProps {
  filePath: string
```

to:

```typescript
interface InAppPlayerProps {
  filePath: string | undefined
```

And add a null check in the `src` computation. Where it builds the `asset://` URL from `filePath`, add a fallback:

```typescript
const src = filePath
  ? (window as any).__TAURI_INTERNALS__
    ? `asset://localhost/${filePath.replace(/\\/g, '/')}`
    : ''
  : ''
```

- [ ] **Step 4: Update VideoDetail.tsx**

In `src/components/detail/VideoDetail.tsx`:

1. Where it calls `open_with_player`, change `video.filePath` to `video.files[0]?.path`:

```typescript
await run('open_with_player', { filePath: video.files[0]?.path ?? '' }, undefined)
```

2. Where it passes `filePath` to `InAppPlayer`, change:

```typescript
<InAppPlayer filePath={video.files[0]?.path} onClose={() => setShowPlayer(false)} />
```

3. Where `formatDuration` is called with `video.duration`, add a null check:

```typescript
{video.duration != null ? formatDuration(video.duration) : '-'}
```

- [ ] **Step 5: Update SettingsPage.tsx**

In `src/pages/SettingsPage.tsx`, where the `playerPath` input is rendered:

Change `value={settings.playerPath}` to `value={settings.playerPath ?? ''}`.

Change the `onChange` handler to store empty string as null:

```typescript
onChange={(e) => save({ ...settings, playerPath: e.target.value || null })}
```

- [ ] **Step 6: Run frontend tests**

Run: `pnpm test:run`
Expected: All 12 existing tests pass. The test files don't reference `filePath` directly, so they should still pass.

- [ ] **Step 7: Run TypeScript build**

Run: `pnpm build`
Expected: No TypeScript errors. If there are errors, fix any remaining references to `filePath` or `duration` type mismatches.

- [ ] **Step 8: Commit**

```bash
git add src/types/index.ts src/lib/mockData.ts src/components/detail/VideoDetail.tsx src/components/detail/InAppPlayer.tsx src/pages/SettingsPage.tsx
git commit -m "feat: update frontend types for backend integration"
```
