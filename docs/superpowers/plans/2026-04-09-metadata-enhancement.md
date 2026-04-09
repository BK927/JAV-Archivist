# Metadata Enhancement & Frontend Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store all high-value metadata (actor photos/kanji, maker, FC2 sample images), add backend query APIs, and connect all frontend pages to real data.

**Architecture:** Fully normalized DB schema (actors, makers, series, sample_images tables with FK relationships). Scraper extended to download actor photos and sample images. Five new Tauri query commands. Frontend pages replace mock data with Tauri calls. New MakersPage added as 5th nav tab.

**Tech Stack:** Rust (rusqlite, rquest, scraper crate), Tauri 2, React 19, TypeScript, Zustand, Shadcn/UI, Tailwind CSS

**Spec:** `docs/superpowers/specs/2026-04-09-metadata-enhancement-design.md`

---

### Task 1: DB Schema Migration

Add new tables (makers, series, sample_images), new columns (actors.name_kanji, videos.maker_id, videos.series_id), and migrate existing series string data.

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write test for new schema**

Add to the `#[cfg(test)] mod tests` block in `db.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib db::tests::test_init_db_creates_new_tables db::tests::test_actors_table_has_name_kanji db::tests::test_series_migration_from_string 2>&1`
Expected: compilation errors (tables/columns/function don't exist yet)

- [ ] **Step 3: Update init_db schema**

In `db.rs`, update `init_db` function. Replace the existing `CREATE TABLE IF NOT EXISTS actors` block and add new tables after it:

```rust
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
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    )?;

    // Migrations for existing databases
    let _ = conn.execute_batch(
        "ALTER TABLE videos ADD COLUMN scrape_status TEXT DEFAULT 'not_scraped';
         ALTER TABLE videos ADD COLUMN scraped_at TEXT;"
    );
    let _ = conn.execute_batch("ALTER TABLE actors ADD COLUMN name_kanji TEXT;");
    let _ = conn.execute_batch("ALTER TABLE videos ADD COLUMN maker_id TEXT;");
    let _ = conn.execute_batch("ALTER TABLE videos ADD COLUMN series_id TEXT;");

    // Migrate series strings to series table
    migrate_series_to_table(conn)?;

    Ok(())
}

pub fn migrate_series_to_table(conn: &Connection) -> Result<()> {
    // Find videos with series string but no series_id
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

    // Link videos to series_id
    conn.execute_batch(
        "UPDATE videos SET series_id = (SELECT s.id FROM series s WHERE s.name = videos.series)
         WHERE series IS NOT NULL AND series != '' AND series_id IS NULL"
    )?;

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib db::tests 2>&1`
Expected: all tests pass (including existing ones)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add makers, series, sample_images tables and schema migration"
```

---

### Task 2: New Rust Models

Add Actor, Maker, Series, Tag, SampleImage, ActorDetail structs to models.rs. Add maker_name to Video.

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add new structs and update Video**

Append to `models.rs` after the existing `Settings` struct, and add `maker_name` to `Video`:

```rust
// In the existing Video struct, add after scraped_at:
    pub maker_name: Option<String>,

// New structs at bottom of file:

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub name_kanji: Option<String>,
    pub photo_path: Option<String>,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Maker {
    pub id: String,
    pub name: String,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub name: String,
    pub cover_path: Option<String>,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub video_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleImage {
    pub id: String,
    pub video_id: String,
    pub path: String,
    pub sort_order: u32,
}

/// Used in update_video_metadata to pass actor name + optional kanji
#[derive(Debug, Clone)]
pub struct ActorDetail {
    pub name: String,
    pub name_kanji: Option<String>,
}
```

- [ ] **Step 2: Fix all compilation errors from Video.maker_name addition**

Every place that constructs a `Video` needs `maker_name: None` added. Update these locations:

In `db.rs` — `get_all_videos` Video construction (around line 209):
```rust
maker_name: None, // will be filled by JOIN in a later task
```

In `db.rs` — `get_video_by_id` Video construction (around line 244):
```rust
maker_name: None,
```

In `scanner.rs` — both Video constructions in `group_by_code` (unknown block ~line 104 and grouped block ~line 133):
```rust
maker_name: None,
```

- [ ] **Step 3: Verify compilation and tests pass**

Run: `cd src-tauri && cargo test --lib 2>&1`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/scanner.rs
git commit -m "feat: add Actor, Maker, Series, Tag, SampleImage models"
```

---

### Task 3: DB Query Functions

Add get_actors, get_series, get_tags, get_makers, get_sample_images. Update get_all_videos and get_video_by_id to JOIN makers for maker_name.

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write tests for query functions**

Add to `db.rs` tests:

```rust
#[test]
fn test_get_actors() {
    let conn = open_in_memory().unwrap();
    init_db(&conn).unwrap();

    // Insert actor and link to video
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib db::tests::test_get_actors db::tests::test_get_makers db::tests::test_get_series_from_table db::tests::test_get_tags_with_count db::tests::test_get_sample_images db::tests::test_get_video_includes_maker_name 2>&1`
Expected: compilation errors (functions don't exist)

- [ ] **Step 3: Implement query functions**

Add to `db.rs`:

```rust
use crate::models::{Settings, Video, VideoFile, ScrapeStatus, Actor, Maker, Series as SeriesModel, Tag, SampleImage};

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
```

Note: The `Series` model name conflicts with Rust's `Series` if any. Use `SeriesModel` as the import alias: `use crate::models::{Series as SeriesModel, ...}`.

- [ ] **Step 4: Update get_all_videos and get_video_by_id to JOIN makers**

In `get_all_videos`, change the SQL query to:
```sql
SELECT v.id, v.code, v.title, v.thumbnail_path, v.series, v.duration, v.watched, v.favorite, v.added_at, v.released_at, v.scrape_status, v.scraped_at, m.name as maker_name
FROM videos v
LEFT JOIN makers m ON v.maker_id = m.id
ORDER BY v.added_at DESC
```

Add column index 12 to extract `maker_name: Option<String>` and pass it to `Video { ..., maker_name }`.

In `get_video_by_id`, change the SQL similarly:
```sql
SELECT v.code, v.title, v.thumbnail_path, v.series, v.duration, v.watched, v.favorite, v.added_at, v.released_at, v.scrape_status, v.scraped_at, m.name as maker_name
FROM videos v
LEFT JOIN makers m ON v.maker_id = m.id
WHERE v.id = ?1
```

Add column index 11 for `maker_name` in the tuple extraction.

- [ ] **Step 5: Run all tests**

Run: `cd src-tauri && cargo test --lib db::tests 2>&1`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add query functions for actors, makers, series, tags, sample_images"
```

---

### Task 4: Update update_video_metadata

Change the function signature to accept maker, ActorDetail (with kanji), and sample_image_paths. Handle maker upsert, series normalization, actor kanji updates, and sample image insertion.

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Write test for new signature**

```rust
#[test]
fn test_update_video_metadata_with_maker_and_details() {
    let conn = open_in_memory().unwrap();
    init_db(&conn).unwrap();

    let video = make_test_video("ABC-123", "Original", "C:/test.mp4");
    let id = video.id.clone();
    upsert_videos(&conn, &[video]).unwrap();

    update_video_metadata(
        &conn,
        &id,
        Some("Scraped Title"),
        None,
        Some("SONE"),
        Some(7200),
        Some("2023-12-12"),
        &[
            ActorDetail { name: "Aoi Rena".to_string(), name_kanji: Some("葵レナ".to_string()) },
            ActorDetail { name: "Mita Marin".to_string(), name_kanji: None },
        ],
        &["巨乳".to_string()],
        Some("S1 STYLE"),
        &["/samples/abc123_01.jpg".to_string(), "/samples/abc123_02.jpg".to_string()],
        ScrapeStatus::Complete,
    ).unwrap();

    let v = get_video_by_id(&conn, &id).unwrap();
    assert_eq!(v.title, "Scraped Title");
    assert_eq!(v.maker_name.as_deref(), Some("S1 STYLE"));

    // Verify series was created in series table
    let series_list = get_series(&conn).unwrap();
    assert_eq!(series_list.len(), 1);
    assert_eq!(series_list[0].name, "SONE");

    // Verify actor kanji was stored
    let actors = get_actors(&conn).unwrap();
    let aoi = actors.iter().find(|a| a.name == "Aoi Rena").unwrap();
    assert_eq!(aoi.name_kanji.as_deref(), Some("葵レナ"));

    // Verify sample images
    let images = get_sample_images(&conn, &id).unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].path, "/samples/abc123_01.jpg");
    assert_eq!(images[1].sort_order, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib db::tests::test_update_video_metadata_with_maker_and_details 2>&1`
Expected: compile error (signature mismatch)

- [ ] **Step 3: Rewrite update_video_metadata**

Replace the existing `update_video_metadata` function:

```rust
use crate::models::ActorDetail;

pub fn update_video_metadata(
    conn: &Connection,
    video_id: &str,
    title: Option<&str>,
    thumbnail_path: Option<&str>,
    series: Option<&str>,
    duration: Option<u64>,
    released_at: Option<&str>,
    actor_details: &[ActorDetail],
    tags: &[String],
    maker: Option<&str>,
    sample_image_paths: &[String],
    status: ScrapeStatus,
) -> Result<()> {
    conn.execute_batch("BEGIN")?;

    let result = (|| -> Result<()> {
        // 1. Upsert maker → get maker_id
        let maker_id: Option<String> = if let Some(maker_name) = maker {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO makers (id, name) VALUES (?1, ?2)",
                params![id, maker_name],
            )?;
            Some(conn.query_row(
                "SELECT id FROM makers WHERE name = ?1",
                [maker_name],
                |row| row.get(0),
            )?)
        } else {
            None
        };

        // 2. Upsert series → get series_id
        let series_id: Option<String> = if let Some(series_name) = series {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO series (id, name) VALUES (?1, ?2)",
                params![id, series_name],
            )?;
            Some(conn.query_row(
                "SELECT id FROM series WHERE name = ?1",
                [series_name],
                |row| row.get(0),
            )?)
        } else {
            None
        };

        // 3. Update videos table
        conn.execute(
            "UPDATE videos SET
                title = COALESCE(?1, title),
                thumbnail_path = COALESCE(?2, thumbnail_path),
                series = COALESCE(?3, series),
                series_id = COALESCE(?4, series_id),
                maker_id = COALESCE(?5, maker_id),
                duration = COALESCE(?6, duration),
                released_at = COALESCE(?7, released_at),
                scrape_status = ?8,
                scraped_at = ?9
             WHERE id = ?10",
            params![
                title,
                thumbnail_path,
                series,
                series_id,
                maker_id,
                duration.map(|d| d as i64),
                released_at,
                status.as_str(),
                chrono::Utc::now().to_rfc3339(),
                video_id,
            ],
        )?;

        // 4. Upsert actors with name_kanji
        for detail in actor_details {
            let actor_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO actors (id, name, name_kanji) VALUES (?1, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET name_kanji = COALESCE(excluded.name_kanji, actors.name_kanji)",
                params![actor_id, detail.name, detail.name_kanji],
            )?;
            let actual_id: String = conn.query_row(
                "SELECT id FROM actors WHERE name = ?1",
                [&detail.name],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO video_actors (video_id, actor_id) VALUES (?1, ?2)",
                params![video_id, actual_id],
            )?;
        }

        // 5. Upsert tags (unchanged logic)
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

        // 6. Replace sample images
        if !sample_image_paths.is_empty() {
            conn.execute("DELETE FROM sample_images WHERE video_id = ?1", [video_id])?;
            for (i, path) in sample_image_paths.iter().enumerate() {
                let img_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO sample_images (id, video_id, path, sort_order) VALUES (?1, ?2, ?3, ?4)",
                    params![img_id, video_id, path, i as u32],
                )?;
            }
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
```

- [ ] **Step 4: Fix existing callers of update_video_metadata**

The old signature had `actors: &[String]` — now it's `actor_details: &[ActorDetail]` plus `maker` and `sample_image_paths`. Update callers in `lib.rs`:

In `scrape_video` (around line 109):
```rust
db::update_video_metadata(
    &conn,
    &vid_id,
    meta.title.as_deref(),
    thumb_path.as_ref().and_then(|p| p.to_str()),
    meta.series.as_deref(),
    meta.duration,
    meta.released_at.as_deref(),
    &meta.actors.iter().map(|name| crate::models::ActorDetail {
        name: name.clone(),
        name_kanji: None,
    }).collect::<Vec<_>>(),
    &meta.tags,
    meta.maker.as_deref(),
    &[],
    status,
)
```

In `scrape_all_new` (around line 165), same transformation.

- [ ] **Step 5: Fix existing tests that call update_video_metadata**

Update `test_update_video_metadata` and `test_update_video_metadata_preserves_existing` to use the new signature. The actor parameter changes from `&["Actor One".to_string()]` to `&[ActorDetail { name: "Actor One".to_string(), name_kanji: None }]`. Add `None` for maker and `&[]` for sample_image_paths.

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && cargo test --lib 2>&1`
Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: update_video_metadata with maker, actor details, sample images"
```

---

### Task 5: Scraper Types — ScrapedActor & sample_image_urls

Extend ScrapedMetadata with actor_details (ScrapedActor) and sample_image_urls.

**Files:**
- Modify: `src-tauri/src/scraper/types.rs`

- [ ] **Step 1: Add ScrapedActor and new fields**

```rust
#[derive(Debug, Clone, Default)]
pub struct ScrapedActor {
    pub name: String,
    pub name_kanji: Option<String>,
    pub photo_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ScrapedMetadata {
    pub title: Option<String>,
    pub cover_url: Option<String>,
    pub actors: Vec<String>,
    pub actor_details: Vec<ScrapedActor>,
    pub tags: Vec<String>,
    pub series: Option<String>,
    pub maker: Option<String>,
    pub duration: Option<u64>,
    pub released_at: Option<String>,
    pub sample_image_urls: Vec<String>,
}
```

Update `has_any_field` to include `|| !self.actor_details.is_empty() || !self.sample_image_urls.is_empty()`.

Update `is_complete` — no change needed (actor_details supplements actors, doesn't replace the completeness check).

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo test --lib 2>&1`
Expected: all pass (new fields have Default)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/scraper/types.rs
git commit -m "feat: add ScrapedActor and sample_image_urls to ScrapedMetadata"
```

---

### Task 6: r18dev Parser — Extract Actor Details

Update the r18dev parser to populate actor_details with name_kanji and image_url.

**Files:**
- Modify: `src-tauri/src/scraper/r18dev.rs`
- Modify: `src-tauri/tests/fixtures/r18_sample.json` (already has the data)

- [ ] **Step 1: Update test to verify actor details**

Add to existing `test_parse_r18_json` test:

```rust
assert_eq!(meta.actor_details.len(), 2);
assert_eq!(meta.actor_details[0].name, "Marin Mita");
assert_eq!(meta.actor_details[0].name_kanji.as_deref(), Some("三田真鈴"));
assert_eq!(
    meta.actor_details[0].photo_url.as_deref(),
    Some("https://pics.dmm.co.jp/mono/actjpgs/mita_marin.jpg")
);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib scraper::r18dev::tests::test_parse_r18_json 2>&1`
Expected: FAIL (actor_details is empty)

- [ ] **Step 3: Update R18Actress struct and parse_r18_json**

```rust
use super::types::{ScrapedMetadata, ScrapedActor, ScrapeError};

#[derive(Deserialize)]
struct R18Actress {
    name_romaji: Option<String>,
    name_kanji: Option<String>,
    image_url: Option<String>,
}
```

In `parse_r18_json`, after extracting `actors`, add:

```rust
let actor_details = resp
    .actresses
    .unwrap_or_default()
    .into_iter()
    .filter_map(|a| {
        a.name_romaji.map(|name| ScrapedActor {
            name,
            name_kanji: a.name_kanji,
            photo_url: a.image_url,
        })
    })
    .collect();
```

And add `actor_details` to the returned `ScrapedMetadata`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib scraper::r18dev::tests 2>&1`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scraper/r18dev.rs
git commit -m "feat: extract actor kanji names and photo URLs from r18.dev"
```

---

### Task 7: FC2 Parser — Extract Sample Images

Update the FC2 parser to extract sample image URLs from the HTML.

**Files:**
- Modify: `src-tauri/src/scraper/fc2.rs`
- Verify: `src-tauri/tests/fixtures/fc2_sample.html` (already has sample images area)

- [ ] **Step 1: Update test to verify sample images**

Add to existing `test_parse_fc2_html`:

```rust
assert_eq!(meta.sample_image_urls.len(), 2);
assert_eq!(meta.sample_image_urls[0], "https://storage200000.contents.fc2.com/file/123/sample1.jpg");
assert_eq!(meta.sample_image_urls[1], "https://storage200000.contents.fc2.com/file/123/sample2.jpg");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib scraper::fc2::tests::test_parse_fc2_html 2>&1`
Expected: FAIL (sample_image_urls is empty)

- [ ] **Step 3: Add sample image extraction to parse_fc2_html**

Add after the release_date parsing block (before the `has_any_field` check):

```rust
// 4. Parse sample images
let sample_sel = Selector::parse(".items_article_SampleImagesArea img[src]").unwrap();
for el in document.select(&sample_sel) {
    if let Some(src) = el.value().attr("src") {
        meta.sample_image_urls.push(src.to_string());
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib scraper::fc2::tests 2>&1`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scraper/fc2.rs
git commit -m "feat: extract FC2 sample image URLs"
```

---

### Task 8: Image Downloads — Actor Photos & Sample Images

Add `download_actor_photo` and `download_sample_images` functions.

**Files:**
- Modify: `src-tauri/src/scraper/image.rs`

- [ ] **Step 1: Add download_actor_photo function**

```rust
/// Sanitize actor name for use as filename (replace problematic chars)
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn download_actor_photo(
    client: &rquest::Client,
    url: &str,
    actors_dir: &Path,
    actor_name: &str,
) -> Result<PathBuf, ScrapeError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(ScrapeError::NetworkError(format!("HTTP {}", resp.status().as_u16())));
    }

    let ext = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| match ct {
            "image/jpeg" | "image/jpg" => Some("jpg"),
            "image/png" => Some("png"),
            "image/webp" => Some("webp"),
            _ => None,
        })
        .unwrap_or("jpg");

    let sanitized = sanitize_filename(actor_name);
    let file_path = actors_dir.join(format!("{}.{}", sanitized, ext));

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    std::fs::write(&file_path, &bytes)
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    Ok(file_path)
}
```

- [ ] **Step 2: Add download_sample_images function**

```rust
pub async fn download_sample_images(
    client: &rquest::Client,
    urls: &[String],
    samples_dir: &Path,
    video_code: &str,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let sanitized_code = video_code.replace('-', "_").to_lowercase();

    for (i, url) in urls.iter().enumerate() {
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let ext = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .and_then(|ct| match ct {
                "image/jpeg" | "image/jpg" => Some("jpg"),
                "image/png" => Some("png"),
                "image/webp" => Some("webp"),
                _ => None,
            })
            .unwrap_or("jpg");

        let file_path = samples_dir.join(format!("{}_{:02}.{}", sanitized_code, i + 1, ext));

        if let Ok(bytes) = resp.bytes().await {
            if std::fs::write(&file_path, &bytes).is_ok() {
                paths.push(file_path);
            }
        }
    }

    paths
}
```

- [ ] **Step 3: Add unit test for sanitize_filename**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Aoi Rena"), "Aoi Rena");
        assert_eq!(sanitize_filename("葵レナ"), "___");
        assert_eq!(sanitize_filename("Test/Actor:Name"), "Test_Actor_Name");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib scraper::image::tests 2>&1`
Expected: pass

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scraper/image.rs
git commit -m "feat: add actor photo and sample image download functions"
```

---

### Task 9: ScrapePipeline — ScrapeResult & Updated scrape_one

Extend the pipeline to download actor photos and sample images, return a ScrapeResult, and update merge for new fields.

**Files:**
- Modify: `src-tauri/src/scraper/mod.rs`

- [ ] **Step 1: Add ScrapeResult and update merge**

```rust
use std::collections::HashMap;
pub use types::{ScrapedMetadata, ScrapedActor, ScrapeError, MetadataSource};

pub struct ScrapeResult {
    pub metadata: ScrapedMetadata,
    pub cover_path: Option<PathBuf>,
    pub actor_photo_paths: HashMap<String, PathBuf>,
    pub sample_image_paths: Vec<PathBuf>,
    pub status: ScrapeStatus,
}
```

Update `merge` to handle new fields:

```rust
fn merge(base: &mut ScrapedMetadata, incoming: ScrapedMetadata) {
    if base.title.is_none() { base.title = incoming.title; }
    if base.cover_url.is_none() { base.cover_url = incoming.cover_url; }
    if base.series.is_none() { base.series = incoming.series; }
    if base.maker.is_none() { base.maker = incoming.maker; }
    if base.duration.is_none() { base.duration = incoming.duration; }
    if base.released_at.is_none() { base.released_at = incoming.released_at; }
    if base.actors.is_empty() { base.actors = incoming.actors; }
    if base.actor_details.is_empty() { base.actor_details = incoming.actor_details; }
    if base.tags.is_empty() { base.tags = incoming.tags; }
    if base.sample_image_urls.is_empty() { base.sample_image_urls = incoming.sample_image_urls; }
}
```

- [ ] **Step 2: Add actors_dir and samples_dir to ScrapePipeline**

```rust
pub struct ScrapePipeline {
    client: rquest::Client,
    rate_limiter: Mutex<http::RateLimiter>,
    thumbnails_dir: PathBuf,
    actors_dir: PathBuf,
    samples_dir: PathBuf,
}

impl ScrapePipeline {
    pub fn new(thumbnails_dir: PathBuf, actors_dir: PathBuf, samples_dir: PathBuf) -> Result<Self, String> {
        let client = rquest::Client::builder()
            .emulation(Emulation::Chrome131)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            rate_limiter: Mutex::new(http::RateLimiter::new(
                Duration::from_secs(3),
                Duration::from_secs(60),
            )),
            thumbnails_dir,
            actors_dir,
            samples_dir,
        })
    }
```

- [ ] **Step 3: Rewrite scrape_one to return ScrapeResult**

```rust
    pub async fn scrape_one(&self, code: &str, video_id: &str) -> ScrapeResult {
        let sources = sources_for(code);
        let mut merged = ScrapedMetadata::default();

        for source in &sources {
            {
                let rl = self.rate_limiter.lock().await;
                rl.wait().await;
            }

            match source.fetch(code, &self.client).await {
                Ok(meta) => {
                    merge(&mut merged, meta);
                    {
                        let mut rl = self.rate_limiter.lock().await;
                        rl.success();
                    }
                    if merged.is_complete(code) {
                        break;
                    }
                }
                Err(ScrapeError::RateLimited) => {
                    let mut rl = self.rate_limiter.lock().await;
                    rl.failure();
                }
                Err(_) => {}
            }
        }

        // Cover fallback: use first sample image if no cover
        if merged.cover_url.is_none() && !merged.sample_image_urls.is_empty() {
            merged.cover_url = Some(merged.sample_image_urls[0].clone());
        }

        // Download cover
        let cover_path = if let Some(ref cover_url) = merged.cover_url {
            image::download_cover(&self.client, cover_url, video_id, &self.thumbnails_dir)
                .await
                .ok()
        } else {
            None
        };

        // Download actor photos
        let mut actor_photo_paths = HashMap::new();
        for detail in &merged.actor_details {
            if let Some(ref photo_url) = detail.photo_url {
                if let Ok(path) = image::download_actor_photo(
                    &self.client, photo_url, &self.actors_dir, &detail.name,
                ).await {
                    actor_photo_paths.insert(detail.name.clone(), path);
                }
            }
        }

        // Download sample images
        let sample_image_paths = image::download_sample_images(
            &self.client,
            &merged.sample_image_urls,
            &self.samples_dir,
            code,
        ).await;

        let status = if merged.is_complete(code) {
            ScrapeStatus::Complete
        } else if merged.has_any_field() {
            ScrapeStatus::Partial
        } else {
            ScrapeStatus::NotFound
        };

        ScrapeResult {
            metadata: merged,
            cover_path,
            actor_photo_paths,
            sample_image_paths,
            status,
        }
    }
```

- [ ] **Step 4: Update merge test for new fields**

Add to `test_merge_fills_empty_fields`:

```rust
let incoming = ScrapedMetadata {
    // ...existing fields...
    actor_details: vec![ScrapedActor {
        name: "Actor".to_string(),
        name_kanji: Some("アクター".to_string()),
        photo_url: Some("http://photo.jpg".to_string()),
    }],
    sample_image_urls: vec!["http://sample1.jpg".to_string()],
};
// ...
assert_eq!(base.actor_details.len(), 1);
assert_eq!(base.sample_image_urls.len(), 1);
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test --lib scraper::tests 2>&1`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/scraper/mod.rs
git commit -m "feat: ScrapePipeline downloads actor photos and sample images"
```

---

### Task 10: Tauri Commands — Query APIs & Updated Scraping

Add 5 new query commands. Update scrape_video and scrape_all_new to use ScrapeResult and pass full data to update_video_metadata. Add ActorsDir and SamplesDir state.

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add new state types and directory setup**

```rust
struct ActorsDir(PathBuf);
struct SamplesDir(PathBuf);
```

In `run()` setup, after `thumbnails_dir`:

```rust
let actors_dir = app_data.join("actors");
std::fs::create_dir_all(&actors_dir)?;
app.manage(ActorsDir(actors_dir));

let samples_dir = app_data.join("samples");
std::fs::create_dir_all(&samples_dir)?;
app.manage(SamplesDir(samples_dir));
```

- [ ] **Step 2: Add 5 query commands**

```rust
use models::{Settings, ScrapeStatus, Video, Actor, Maker, Series as SeriesModel, Tag, SampleImage};

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
```

- [ ] **Step 3: Update scrape_video to use ScrapeResult**

```rust
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

    tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;

        // Update actor photo paths in actors table
        for (actor_name, photo_path) in &actor_photo_map {
            let _ = conn.execute(
                "UPDATE actors SET photo_path = ?1 WHERE name = ?2 AND photo_path IS NULL",
                params![photo_path.to_str(), actor_name],
            );
        }

        db::update_video_metadata(
            &conn,
            &vid_id,
            result.metadata.title.as_deref(),
            result.cover_path.as_ref().and_then(|p| p.to_str()),
            result.metadata.series.as_deref(),
            result.metadata.duration,
            result.metadata.released_at.as_deref(),
            &actor_details,
            &result.metadata.tags,
            result.metadata.maker.as_deref(),
            &sample_paths,
            result.status,
        )
        .map_err(|e| e.to_string())?;
        db::get_video_by_id(&conn, &vid_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 4: Update scrape_all_new similarly**

Same pattern as scrape_video but in the loop. Add `actors_state` and `samples_state` parameters. Create pipeline with all 3 dirs. Use `result.metadata`, `result.cover_path`, etc. Update actor photos in actors table.

- [ ] **Step 5: Register new commands in invoke_handler**

```rust
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
```

- [ ] **Step 6: Verify compilation**

Run: `cd src-tauri && cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add query commands and update scrape commands for full metadata"
```

---

### Task 11: Frontend Types, Routing & TopNav

Update TypeScript types, add MakersPage route, and add 제작사 tab to TopNav.

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/App.tsx`
- Modify: `src/components/layout/TopNav.tsx`

- [ ] **Step 1: Update TypeScript types**

In `src/types/index.ts`, update `Actor` and `Video`, add `Maker`, `Tag`, `SampleImage`:

```typescript
export interface Actor {
  id: string
  name: string
  nameKanji: string | null
  photoPath: string | null
  videoCount: number
}

export interface Maker {
  id: string
  name: string
  videoCount: number
}

export interface Tag {
  id: string
  name: string
  videoCount: number
}

export interface SampleImage {
  id: string
  videoId: string
  path: string
  sortOrder: number
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
  scrapeStatus: ScrapeStatus
  scrapedAt: string | null
  makerName: string | null
}
```

Keep existing `Series`, `FilterState`, `AppSettings` unchanged.

- [ ] **Step 2: Add 제작사 tab to TopNav**

In `src/components/layout/TopNav.tsx`, add to TABS array:

```typescript
const TABS = [
  { path: '/library', label: '라이브러리' },
  { path: '/actors', label: '배우' },
  { path: '/series', label: '시리즈' },
  { path: '/tags', label: '태그' },
  { path: '/makers', label: '제작사' },
]
```

- [ ] **Step 3: Create MakersPage and add route**

Create `src/pages/MakersPage.tsx`:

```typescript
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import MakerGrid from '@/components/makers/MakerGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Maker } from '@/types'

export default function MakersPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [makers, setMakers] = useState<Maker[]>([])

  useEffect(() => {
    run<Maker[]>('get_makers', {}, []).then(setMakers)
  }, [run])

  const handleSelect = (maker: Maker) => {
    navigate(`/library?maker=${encodeURIComponent(maker.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <MakerGrid makers={makers} onSelect={handleSelect} />
    </div>
  )
}
```

Create `src/components/makers/MakerGrid.tsx`:

```typescript
import { Factory } from 'lucide-react'
import type { Maker } from '@/types'

interface MakerGridProps {
  makers: Maker[]
  onSelect: (maker: Maker) => void
}

export default function MakerGrid({ makers, onSelect }: MakerGridProps) {
  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))' }}
    >
      {makers.map((maker) => (
        <button
          key={maker.id}
          onClick={() => onSelect(maker)}
          className="flex flex-col rounded-md bg-card border border-border hover:border-primary/50 transition-colors overflow-hidden text-left"
        >
          <div className="aspect-video bg-secondary flex items-center justify-center">
            <Factory className="w-8 h-8 text-muted-foreground/30" />
          </div>
          <div className="p-2">
            <p className="text-xs font-medium text-foreground truncate">{maker.name}</p>
            <p className="text-[11px] text-muted-foreground">{maker.videoCount}편</p>
          </div>
        </button>
      ))}
    </div>
  )
}
```

In `src/App.tsx`, add the route:

```typescript
import MakersPage from '@/pages/MakersPage'

// In Routes:
<Route path="makers" element={<MakersPage />} />
```

- [ ] **Step 4: Verify frontend compiles**

Run: `cd C:/Users/dead4/repo/JAV-Archivist && pnpm tsc --noEmit 2>&1`
Expected: no errors (or only pre-existing ones)

- [ ] **Step 5: Commit**

```bash
git add src/types/index.ts src/components/layout/TopNav.tsx src/App.tsx src/pages/MakersPage.tsx src/components/makers/MakerGrid.tsx
git commit -m "feat: add Maker/Tag/SampleImage types, MakersPage, and 제작사 tab"
```

---

### Task 12: Frontend Pages — Backend Integration

Replace mock data in ActorsPage, SeriesPage, TagsPage with Tauri command calls. Update ActorGrid to show kanji names.

**Files:**
- Modify: `src/pages/ActorsPage.tsx`
- Modify: `src/pages/SeriesPage.tsx`
- Modify: `src/pages/TagsPage.tsx`
- Modify: `src/components/actors/ActorGrid.tsx`
- Modify: `src/components/tags/TagGrid.tsx`

- [ ] **Step 1: Update ActorsPage to call backend**

```typescript
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import ActorGrid from '@/components/actors/ActorGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Actor } from '@/types'

export default function ActorsPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [actors, setActors] = useState<Actor[]>([])

  useEffect(() => {
    run<Actor[]>('get_actors', {}, []).then(setActors)
  }, [run])

  const handleSelect = (actor: Actor) => {
    navigate(`/library?actor=${encodeURIComponent(actor.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <ActorGrid actors={actors} onSelect={handleSelect} />
    </div>
  )
}
```

- [ ] **Step 2: Update ActorGrid to show kanji name**

Add kanji subtitle under the actor name:

```typescript
<span className="text-xs text-center text-foreground leading-snug line-clamp-2">
  {actor.name}
</span>
{actor.nameKanji && (
  <span className="text-[10px] text-center text-muted-foreground leading-snug line-clamp-1">
    {actor.nameKanji}
  </span>
)}
<span className="text-[11px] text-muted-foreground">{actor.videoCount}편</span>
```

- [ ] **Step 3: Update SeriesPage to call backend**

```typescript
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import SeriesGrid from '@/components/series/SeriesGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Series } from '@/types'

export default function SeriesPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [series, setSeries] = useState<Series[]>([])

  useEffect(() => {
    run<Series[]>('get_series_list', {}, []).then(setSeries)
  }, [run])

  const handleSelect = (s: Series) => {
    navigate(`/library?series=${encodeURIComponent(s.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <SeriesGrid series={series} onSelect={handleSelect} />
    </div>
  )
}
```

- [ ] **Step 4: Update TagsPage to call backend**

```typescript
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import TagGrid from '@/components/tags/TagGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Tag } from '@/types'

export default function TagsPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [tags, setTags] = useState<Tag[]>([])

  useEffect(() => {
    run<Tag[]>('get_tags', {}, []).then(setTags)
  }, [run])

  const handleSelect = (tag: Tag) => {
    navigate(`/library?tag=${encodeURIComponent(tag.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <TagGrid tags={tags} onSelect={handleSelect} />
    </div>
  )
}
```

- [ ] **Step 5: Update TagGrid to accept Tag objects**

Change TagGrid to receive `Tag[]` instead of `string[]`:

```typescript
import { Badge } from '@/components/ui/badge'
import type { Tag } from '@/types'

interface TagGridProps {
  tags: Tag[]
  onSelect: (tag: Tag) => void
}

export default function TagGrid({ tags, onSelect }: TagGridProps) {
  return (
    <div className="p-6 flex flex-wrap gap-3">
      {tags.map((tag) => (
        <button
          key={tag.id}
          onClick={() => onSelect(tag)}
          className="flex items-center gap-2 px-4 py-2 rounded-full bg-card border border-border hover:border-primary/50 transition-colors"
        >
          <span className="text-sm text-foreground">{tag.name}</span>
          <Badge variant="secondary" className="text-xs h-5">
            {tag.videoCount}
          </Badge>
        </button>
      ))}
    </div>
  )
}
```

- [ ] **Step 6: Update FilterBar tag list to use backend data**

In `src/components/library/FilterBar.tsx`, replace `MOCK_TAGS` import with a `tags` prop or fetch from backend. Simplest approach — pass tags from LibraryPage as a prop:

Change FilterBar to accept `tags: string[]` prop:

```typescript
interface FilterBarProps {
  totalCount: number
  tags: string[]
}

export default function FilterBar({ totalCount, tags }: FilterBarProps) {
```

Replace `MOCK_TAGS` usage with the `tags` prop. Remove the `import { MOCK_TAGS } from '@/lib/mockData'` line.

In `LibraryPage.tsx`, derive tags from videos:

```typescript
const allTags = [...new Set(videos.flatMap((v) => v.tags))]
// Pass to FilterBar:
<FilterBar totalCount={filtered.length} tags={allTags} />
```

Remove `import { MOCK_VIDEOS } from '@/lib/mockData'` from LibraryPage if scan_library returns real data (keep fallback to `[]`).

- [ ] **Step 7: Verify compilation**

Run: `pnpm tsc --noEmit 2>&1`
Expected: no errors

- [ ] **Step 8: Commit**

```bash
git add src/pages/ActorsPage.tsx src/pages/SeriesPage.tsx src/pages/TagsPage.tsx src/components/actors/ActorGrid.tsx src/components/tags/TagGrid.tsx src/components/library/FilterBar.tsx src/pages/LibraryPage.tsx
git commit -m "feat: connect ActorsPage, SeriesPage, TagsPage to backend"
```

---

### Task 13: VideoDetail Enhancements

Add actor photos + kanji names, maker display, sample images gallery, and individual scrape button to VideoDetail.

**Files:**
- Modify: `src/components/detail/VideoDetail.tsx`

- [ ] **Step 1: Add actor data fetching and enhanced display with photos**

VideoDetail needs to fetch actor details (photos, kanji) since `video.actors` is just `string[]`. Add state and fetch:

```typescript
import type { Video, SampleImage, Actor } from '@/types'

// Inside component:
const [actorDetails, setActorDetails] = useState<Actor[]>([])

useEffect(() => {
  run<Actor[]>('get_actors', {}, []).then((all) => {
    setActorDetails(all.filter((a) => video.actors.includes(a.name)))
  })
}, [video.actors, run])
```

Replace the existing actors `<p>` line with an enhanced section:

```typescript
{/* 배우 — with photos and kanji */}
{video.actors.length > 0 && (
  <div>
    <span className="text-foreground text-sm">배우</span>
    <div className="flex flex-wrap gap-3 mt-2">
      {video.actors.map((name) => {
        const detail = actorDetails.find((a) => a.name === name)
        return (
          <button
            key={name}
            onClick={() => navigate(`/library?actor=${encodeURIComponent(name)}`)}
            className="flex items-center gap-2 hover:bg-secondary/50 rounded px-2 py-1 transition-colors"
          >
            <div className="w-8 h-8 rounded-full bg-secondary flex items-center justify-center overflow-hidden shrink-0">
              {detail?.photoPath ? (
                <img src={detail.photoPath} alt={name} className="w-full h-full object-cover" />
              ) : (
                <User className="w-4 h-4 text-muted-foreground/40" />
              )}
            </div>
            <div className="text-left">
              <p className="text-sm text-foreground leading-tight">{name}</p>
              {detail?.nameKanji && (
                <p className="text-[10px] text-muted-foreground leading-tight">{detail.nameKanji}</p>
              )}
            </div>
          </button>
        )
      })}
    </div>
  </div>
)}

{/* 제작사 */}
{video.makerName && (
  <p className="text-sm text-muted-foreground">
    <span className="text-foreground">제작사</span>:{' '}
    <button
      onClick={() => navigate(`/library?maker=${encodeURIComponent(video.makerName!)}`)}
      className="hover:text-foreground transition-colors underline"
    >
      {video.makerName}
    </button>
  </p>
)}
```

Add `User` to the lucide-react imports: `import { ArrowLeft, Play, Star, Monitor, Download, User } from 'lucide-react'`.

- [ ] **Step 2: Add sample images gallery**

Add imports and state:

```typescript
import { useEffect, useState } from 'react'
import type { Video, SampleImage } from '@/types'
```

Inside the component, after existing state:

```typescript
const [sampleImages, setSampleImages] = useState<SampleImage[]>([])
const [lightboxIdx, setLightboxIdx] = useState<number | null>(null)

useEffect(() => {
  run<SampleImage[]>('get_sample_images', { videoId: video.id }, []).then(setSampleImages)
}, [video.id, run])
```

Add gallery section after the action buttons:

```typescript
{/* 샘플 이미지 갤러리 */}
{sampleImages.length > 0 && (
  <div className="space-y-2">
    <span className="text-sm text-foreground">샘플 이미지</span>
    <div className="flex gap-2 overflow-x-auto pb-2">
      {sampleImages.map((img, idx) => (
        <button
          key={img.id}
          onClick={() => setLightboxIdx(idx)}
          className="shrink-0 w-24 h-16 rounded overflow-hidden border border-border hover:border-primary/50 transition-colors"
        >
          <img
            src={img.path}
            alt={`Sample ${idx + 1}`}
            className="w-full h-full object-cover"
          />
        </button>
      ))}
    </div>
  </div>
)}

{/* 라이트박스 */}
{lightboxIdx !== null && (
  <div
    className="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
    onClick={() => setLightboxIdx(null)}
  >
    <img
      src={sampleImages[lightboxIdx].path}
      alt="Sample"
      className="max-w-[90vw] max-h-[90vh] object-contain"
      onClick={(e) => e.stopPropagation()}
    />
  </div>
)}
```

- [ ] **Step 3: Add individual scrape button**

Add after the favorite button in the action buttons section:

```typescript
import { Download } from 'lucide-react'

// State for scraping
const [isScraping, setIsScraping] = useState(false)

const handleScrape = async () => {
  setIsScraping(true)
  try {
    const updated = await run<Video>('scrape_video', { videoId: video.id }, undefined)
    if (updated) {
      const newVideos = videos.map((v) => v.id === updated.id ? updated : v)
      setVideos(newVideos)
    }
  } finally {
    setIsScraping(false)
  }
}

// In the button row:
{(video.scrapeStatus === 'not_scraped' || video.scrapeStatus === 'not_found') && (
  <Button
    variant="outline"
    size="sm"
    onClick={handleScrape}
    disabled={isScraping}
  >
    <Download className={`w-4 h-4 mr-1 ${isScraping ? 'animate-spin' : ''}`} />
    {isScraping ? '수집 중...' : '메타데이터 수집'}
  </Button>
)}
```

- [ ] **Step 4: Verify compilation**

Run: `pnpm tsc --noEmit 2>&1`

- [ ] **Step 5: Commit**

```bash
git add src/components/detail/VideoDetail.tsx
git commit -m "feat: VideoDetail with maker, sample gallery, and scrape button"
```

---

### Task 14: FilterBar Scraping UI & Library Filter Extensions

Add scraping progress UI to FilterBar. Add URL query param filters for actor/series/maker navigation from other pages.

**Files:**
- Modify: `src/components/library/FilterBar.tsx`
- Modify: `src/pages/LibraryPage.tsx`
- Modify: `src/hooks/useFilteredVideos.ts`

- [ ] **Step 1: Add scraping UI to FilterBar**

Extend FilterBar props and add scraping controls:

```typescript
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useLibraryStore } from '@/stores/libraryStore'
import { Download, X } from 'lucide-react'

interface FilterBarProps {
  totalCount: number
  tags: string[]
  unscrapedCount: number
  isScraping: boolean
  scrapeProgress: { current: number; total: number } | null
  onScrapeAll: () => void
  onCancelScrape: () => void
  activeFilter: { type: string; value: string } | null
  onClearFilter: () => void
}
```

Add scraping button after the tag badges, before the count:

```typescript
{/* 스크래핑 버튼 */}
{!isScraping && unscrapedCount > 0 && (
  <Button variant="outline" size="sm" className="h-7 text-xs" onClick={onScrapeAll}>
    <Download className="w-3 h-3 mr-1" />
    메타데이터 수집 ({unscrapedCount})
  </Button>
)}

{isScraping && scrapeProgress && (
  <div className="flex items-center gap-2">
    <div className="w-24 h-1.5 bg-secondary rounded-full overflow-hidden">
      <div
        className="h-full bg-primary transition-all"
        style={{ width: `${(scrapeProgress.current / scrapeProgress.total) * 100}%` }}
      />
    </div>
    <span className="text-xs text-muted-foreground">
      {scrapeProgress.current}/{scrapeProgress.total}
    </span>
    <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onCancelScrape}>
      <X className="w-3 h-3" />
    </Button>
  </div>
)}

{/* 활성 필터 뱃지 */}
{activeFilter && (
  <Badge variant="default" className="h-7 px-2 text-xs gap-1">
    {activeFilter.type}: {activeFilter.value}
    <button onClick={onClearFilter} className="ml-1">
      <X className="w-3 h-3" />
    </button>
  </Badge>
)}
```

- [ ] **Step 2: Add scraping state and event listener to LibraryPage**

```typescript
import { useEffect, useState, useCallback } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'

export default function LibraryPage() {
  const { id } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery, setVideos } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const { run } = useTauriCommand()

  // Scraping state
  const [isScraping, setIsScraping] = useState(false)
  const [scrapeProgress, setScrapeProgress] = useState<{ current: number; total: number } | null>(null)

  // URL query param filter
  const activeFilter = searchParams.get('actor')
    ? { type: '배우', value: searchParams.get('actor')! }
    : searchParams.get('series')
    ? { type: '시리즈', value: searchParams.get('series')! }
    : searchParams.get('maker')
    ? { type: '제작사', value: searchParams.get('maker')! }
    : searchParams.get('tag')
    ? { type: '태그', value: searchParams.get('tag')! }
    : null

  const clearFilter = () => setSearchParams({})

  const filtered = useFilteredVideos(videos, filters, searchQuery, activeFilter)

  const unscrapedCount = videos.filter((v) => v.scrapeStatus === 'not_scraped' && v.code !== '?').length
  const allTags = [...new Set(videos.flatMap((v) => v.tags))]

  useEffect(() => {
    run<Video[]>('scan_library', {}, []).then(setVideos)
  }, [run, setVideos])

  // Listen for scrape events
  useEffect(() => {
    let unlisten: (() => void) | undefined

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        const u1 = await listen<{ current: number; total: number }>('scrape-progress', (e) => {
          setScrapeProgress(e.payload)
        })
        const u2 = await listen('scrape-complete', () => {
          setIsScraping(false)
          setScrapeProgress(null)
          run<Video[]>('get_videos', {}, []).then(setVideos)
        })
        unlisten = () => { u1(); u2() }
      } catch {
        // Not in Tauri env
      }
    }
    setup()
    return () => unlisten?.()
  }, [run, setVideos])

  const handleScrapeAll = async () => {
    setIsScraping(true)
    setScrapeProgress({ current: 0, total: unscrapedCount })
    await run('scrape_all_new', {}, undefined)
  }

  const handleCancelScrape = async () => {
    await run('cancel_scrape', {}, undefined)
    setIsScraping(false)
    setScrapeProgress(null)
  }

  // ... rest of component, pass new props to FilterBar:
  return (
    <div className="flex flex-col h-full">
      <FilterBar
        totalCount={filtered.length}
        tags={allTags}
        unscrapedCount={unscrapedCount}
        isScraping={isScraping}
        scrapeProgress={scrapeProgress}
        onScrapeAll={handleScrapeAll}
        onCancelScrape={handleCancelScrape}
        activeFilter={activeFilter}
        onClearFilter={clearFilter}
      />
      <div className="flex-1 overflow-auto">
        <VideoGrid videos={filtered} onSelect={handleSelect} />
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Extend useFilteredVideos for URL param filters**

```typescript
export function useFilteredVideos(
  videos: Video[],
  filters: FilterState,
  searchQuery: string,
  activeFilter: { type: string; value: string } | null
): Video[] {
  return useMemo(() => {
    let result = [...videos]

    // URL param filter
    if (activeFilter) {
      switch (activeFilter.type) {
        case '배우':
          result = result.filter((v) => v.actors.includes(activeFilter.value))
          break
        case '시리즈':
          result = result.filter((v) => v.series === activeFilter.value)
          break
        case '제작사':
          result = result.filter((v) => v.makerName === activeFilter.value)
          break
        case '태그':
          result = result.filter((v) => v.tags.includes(activeFilter.value))
          break
      }
    }

    // ... rest of existing filtering logic unchanged ...
  }, [videos, filters, searchQuery, activeFilter])
}
```

- [ ] **Step 4: Clean up mock data imports**

Remove `MOCK_VIDEOS` import from `LibraryPage.tsx` — use `[]` as fallback directly:

```typescript
run<Video[]>('scan_library', {}, []).then(setVideos)
```

Check if `src/lib/mockData.ts` still has any consumers. If MOCK_SETTINGS is the only remaining export used, keep it. If nothing references MOCK_ACTORS, MOCK_SERIES, MOCK_TAGS, MOCK_VIDEOS, remove those exports.

- [ ] **Step 5: Verify compilation**

Run: `pnpm tsc --noEmit 2>&1`

- [ ] **Step 6: Commit**

```bash
git add src/components/library/FilterBar.tsx src/pages/LibraryPage.tsx src/hooks/useFilteredVideos.ts src/lib/mockData.ts
git commit -m "feat: scraping UI in FilterBar and URL param filters for library"
```

---

### Summary

| Task | Description | Difficulty |
|------|-------------|-----------|
| 1 | DB Schema Migration | Medium |
| 2 | Rust Models | Easy |
| 3 | DB Query Functions | Medium |
| 4 | update_video_metadata rewrite | Medium-Hard |
| 5 | Scraper types extension | Easy |
| 6 | r18dev actor details | Easy |
| 7 | FC2 sample images | Easy |
| 8 | Image download functions | Easy |
| 9 | ScrapePipeline update | Medium |
| 10 | Tauri commands | Medium |
| 11 | Frontend types + routing | Easy |
| 12 | Frontend pages integration | Easy |
| 13 | VideoDetail enhancements | Medium |
| 14 | FilterBar scraping UI + filters | Medium |
