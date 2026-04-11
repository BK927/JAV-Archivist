# Auto-Scrape + Orphan Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically scrape metadata for new videos on detection and app startup, with retry budgeting and orphan file cleanup.

**Architecture:** Add `retry_count` column to DB. Classify scrape errors as retryable (default) vs permanent (NotFound only). Watcher emits new event after scan; lib.rs listens and triggers auto-scrape. App startup also auto-scrapes. Orphan videos (files deleted) are removed from DB during scan.

**Tech Stack:** Rust (Tauri v2), SQLite, notify crate

---

### Task 1: Add retry_count column + DB helpers

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Add retry_count migration**

In `src-tauri/src/db.rs`, after the existing migrations (line ~106, after `series_id` migration), add:

```rust
let _ = conn.execute("ALTER TABLE videos ADD COLUMN retry_count INTEGER DEFAULT 0", []);
```

- [ ] **Step 2: Add `get_unscraped_for_auto` function**

After the existing `get_videos_to_scrape` function (line ~743), add:

```rust
pub fn get_unscraped_for_auto(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, code FROM videos WHERE code != '?' AND scrape_status = 'not_scraped' AND retry_count < 3",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}
```

- [ ] **Step 3: Add `increment_retry_count` function**

```rust
pub fn increment_retry_count(conn: &Connection, video_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE videos SET retry_count = retry_count + 1 WHERE id = ?1",
        [video_id],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Update `reset_scrape_status` to also reset retry_count**

Change the existing `reset_scrape_status` SQL from:

```rust
"UPDATE videos SET scrape_status = 'not_scraped' WHERE id IN ({})",
```

to:

```rust
"UPDATE videos SET scrape_status = 'not_scraped', retry_count = 0 WHERE id IN ({})",
```

- [ ] **Step 5: Add `delete_videos` function**

```rust
pub fn delete_videos(conn: &Connection, video_ids: &[String]) -> Result<()> {
    if video_ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<&str> = video_ids.iter().map(|_| "?").collect();
    let in_clause = placeholders.join(",");
    let params: Vec<&dyn rusqlite::types::ToSql> = video_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();

    conn.execute_batch("BEGIN")?;
    let result = (|| -> Result<()> {
        conn.execute(&format!("DELETE FROM sample_images WHERE video_id IN ({})", in_clause), params.as_slice())?;
        conn.execute(&format!("DELETE FROM video_tags WHERE video_id IN ({})", in_clause), params.as_slice())?;
        conn.execute(&format!("DELETE FROM video_actors WHERE video_id IN ({})", in_clause), params.as_slice())?;
        conn.execute(&format!("DELETE FROM video_files WHERE video_id IN ({})", in_clause), params.as_slice())?;
        conn.execute(&format!("DELETE FROM videos WHERE id IN ({})", in_clause), params.as_slice())?;
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

- [ ] **Step 6: Add `get_all_video_ids` helper**

```rust
pub fn get_all_video_ids(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM videos")?;
    let rows = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}
```

- [ ] **Step 7: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add retry_count column, delete_videos, get_unscraped_for_auto"
```

---

### Task 2: Classify scrape errors as retryable vs permanent

**Files:**
- Modify: `src-tauri/src/scraper/mod.rs:213-284`

- [ ] **Step 1: Track error types instead of formatted strings**

In `scrape_one`, change `failed_sources` to track whether the error was `NotFound`:

Replace this block (lines 213-236):

```rust
        let mut merged = ScrapedMetadata::default();
        let mut failed_sources = Vec::new();
        for (source, result) in sources.iter().zip(results) {
            match result {
                Ok(meta) => {
                    tracing::info!(
                        "scrape_one: source={:?} succeeded for code={}",
                        source,
                        code
                    );
                    merge(&mut merged, meta);
                }
                Err(ScrapeError::RateLimited) => {
                    tracing::warn!(
                        "scrape_one: rate limited by source={:?} for code={}",
                        source,
                        code
                    );
                }
                Err(e) => {
                    failed_sources.push((format!("{:?}", source), format!("{:?}", e)));
                }
            }
        }
```

With:

```rust
        let mut merged = ScrapedMetadata::default();
        let mut failed_sources: Vec<(String, String)> = Vec::new();
        let mut has_transient_error = false;
        for (source, result) in sources.iter().zip(results) {
            match result {
                Ok(meta) => {
                    tracing::info!(
                        "scrape_one: source={:?} succeeded for code={}",
                        source,
                        code
                    );
                    merge(&mut merged, meta);
                }
                Err(ScrapeError::RateLimited) => {
                    tracing::warn!(
                        "scrape_one: rate limited by source={:?} for code={}",
                        source,
                        code
                    );
                    has_transient_error = true;
                }
                Err(ScrapeError::NotFound) => {
                    failed_sources.push((format!("{:?}", source), "NotFound".to_string()));
                }
                Err(e) => {
                    failed_sources.push((format!("{:?}", source), format!("{:?}", e)));
                    has_transient_error = true;
                }
            }
        }
```

- [ ] **Step 2: Update status decision to use retryable classification**

Replace the status decision block (lines 278-284):

```rust
        let status = if merged.is_complete(code) {
            ScrapeStatus::Complete
        } else if merged.has_any_field() {
            ScrapeStatus::Partial
        } else {
            ScrapeStatus::NotFound
        };
```

With:

```rust
        let status = if merged.is_complete(code) {
            ScrapeStatus::Complete
        } else if merged.has_any_field() {
            ScrapeStatus::Partial
        } else if has_transient_error {
            // Transient errors (network, parse, rate limit) → keep as NotScraped for auto-retry
            ScrapeStatus::NotScraped
        } else {
            // All sources returned NotFound → permanent failure
            ScrapeStatus::NotFound
        };
```

- [ ] **Step 3: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/scraper/mod.rs
git commit -m "feat: classify scrape errors as retryable vs permanent"
```

---

### Task 3: Increment retry_count on failed scrape

**Files:**
- Modify: `src-tauri/src/lib.rs:343-414` (scrape_videos loop)

- [ ] **Step 1: Add retry_count increment after scrape result**

In the `scrape_videos` function, after the `updated_video` spawn_blocking block (around line 403), and before the `app.emit("scrape-progress", ...)` call, add retry_count increment when status is `NotScraped` (transient failure):

After this line:
```rust
        .map_err(|e| e.to_string())?;
```

And before this line:
```rust
        let _ = app.emit(
```

Insert:

```rust
        // Increment retry_count for transient failures (status stays NotScraped)
        if matches!(evt_status, ScrapeStatus::NotScraped) {
            let db_path3 = db.0.clone();
            let vid_id2 = video_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = db::open(db_path3.to_str().unwrap()) {
                    let _ = db::increment_retry_count(&conn, &vid_id2);
                }
            })
            .await;
        }
```

Add the `ScrapeStatus` import at the top of lib.rs if not already present. Check existing imports:

```rust
use crate::models::ScrapeStatus;
```

- [ ] **Step 2: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: increment retry_count on transient scrape failure"
```

---

### Task 4: Orphan cleanup in scan

**Files:**
- Modify: `src-tauri/src/lib.rs:89-97` (scan_library command)
- Modify: `src-tauri/src/watcher.rs:80-114` (trigger_scan)

- [ ] **Step 1: Add orphan cleanup to `scan_library` command**

Replace the `scan_library` function:

```rust
#[tauri::command]
fn scan_library(db: tauri::State<'_, DbPath>) -> Result<Vec<Video>, String> {
    tracing::info!("cmd: scan_library");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;

    // Remove orphaned videos (in DB but not on filesystem)
    let scanned_ids: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.id.clone()).collect();
    let all_db_ids = db::get_all_video_ids(&conn).map_err(|e| e.to_string())?;
    let orphan_ids: Vec<String> = all_db_ids
        .into_iter()
        .filter(|id| !scanned_ids.contains(id))
        .collect();
    if !orphan_ids.is_empty() {
        tracing::info!("scan_library: removing {} orphaned videos", orphan_ids.len());
        db::delete_videos(&conn, &orphan_ids).map_err(|e| e.to_string())?;
    }

    db::get_all_videos(&conn).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Add orphan cleanup to `trigger_scan` in watcher**

In `src-tauri/src/watcher.rs`, in the `trigger_scan` function, after the `upsert_videos` call and before `get_all_videos`, add:

```rust
    // Remove orphaned videos (in DB but not on filesystem)
    let scanned_ids: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.id.clone()).collect();
    match db::get_all_video_ids(&conn) {
        Ok(all_db_ids) => {
            let orphan_ids: Vec<String> = all_db_ids
                .into_iter()
                .filter(|id| !scanned_ids.contains(id))
                .collect();
            if !orphan_ids.is_empty() {
                tracing::info!("watcher: removing {} orphaned videos", orphan_ids.len());
                if let Err(e) = db::delete_videos(&conn, &orphan_ids) {
                    tracing::error!("watcher: delete_videos failed: {}", e);
                }
            }
        }
        Err(e) => tracing::error!("watcher: get_all_video_ids failed: {}", e),
    }
```

Add `use std::collections::HashSet;` at the top of `watcher.rs` if not present, or use inline `std::collections::HashSet` as shown.

- [ ] **Step 3: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/watcher.rs
git commit -m "feat: remove orphaned videos during scan"
```

---

### Task 5: Auto-scrape on watcher detection

**Files:**
- Modify: `src-tauri/src/watcher.rs`

- [ ] **Step 1: Emit auto-scrape event after scan**

The watcher runs in a background thread without access to Tauri managed state (no `State<DbPath>` etc.), so it cannot call `scrape_videos` directly. Instead, emit a new event that the frontend or lib.rs setup listener can handle.

In `trigger_scan`, after the `library-changed` emit, add auto-scrape trigger:

```rust
    match db::get_all_videos(&conn) {
        Ok(videos) => {
            let count = videos.len();
            let _ = app.emit("library-changed", &videos);
            tracing::info!("watcher: emitted library-changed ({} videos)", count);
        }
        Err(e) => tracing::error!("watcher: get_all_videos failed: {}", e),
    }

    // Trigger auto-scrape for unscraped videos
    match db::get_unscraped_for_auto(&conn) {
        Ok(to_scrape) if !to_scrape.is_empty() => {
            let ids: Vec<String> = to_scrape.into_iter().map(|(id, _)| id).collect();
            tracing::info!("watcher: triggering auto-scrape for {} videos", ids.len());
            let _ = app.emit("auto-scrape-needed", &ids);
        }
        _ => {}
    }
```

- [ ] **Step 2: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/watcher.rs
git commit -m "feat: watcher emits auto-scrape-needed event after scan"
```

---

### Task 6: Auto-scrape on app startup + event listener

**Files:**
- Modify: `src-tauri/src/lib.rs` (setup closure + new auto-scrape function)

- [ ] **Step 1: Add auto-scrape helper function**

Add a new function in `lib.rs` (before the `run()` function):

```rust
fn start_auto_scrape(app: &tauri::AppHandle, db_path: &std::path::Path, thumbnails_dir: &std::path::Path, actors_dir: &std::path::Path, samples_dir: &std::path::Path, cancel_flag: Arc<AtomicBool>) {
    let db_path = db_path.to_path_buf();
    let thumbnails_dir = thumbnails_dir.to_path_buf();
    let actors_dir = actors_dir.to_path_buf();
    let samples_dir = samples_dir.to_path_buf();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        let to_scrape = {
            let db_str = db_path.to_str().unwrap();
            let conn = match db::open(db_str) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("auto_scrape: db open failed: {}", e);
                    return;
                }
            };
            match db::get_unscraped_for_auto(&conn) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("auto_scrape: get_unscraped_for_auto failed: {}", e);
                    return;
                }
            }
        };

        if to_scrape.is_empty() {
            return;
        }

        tracing::info!("auto_scrape: starting for {} videos", to_scrape.len());
        let total = to_scrape.len();
        let pipeline = match scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("auto_scrape: pipeline init failed: {}", e);
                return;
            }
        };

        cancel_flag.store(false, Ordering::SeqCst);

        for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
            if cancel_flag.load(Ordering::SeqCst) {
                tracing::info!("auto_scrape: cancelled at {}/{}", i + 1, total);
                break;
            }

            let result = pipeline.scrape_one(&code, &video_id).await;
            let db_str = db_path.to_str().unwrap().to_string();
            let vid_id = video_id.clone();
            let actor_details: Vec<models::ActorDetail> = result
                .metadata
                .actor_details
                .iter()
                .map(|a| models::ActorDetail {
                    name: a.name.clone(),
                    name_kanji: a.name_kanji.clone(),
                })
                .collect();
            let sample_paths: Vec<String> = result
                .sample_image_paths
                .iter()
                .filter_map(|p| p.to_str().map(|s| s.to_string()))
                .collect();
            let actor_photo_map = result.actor_photo_paths.clone();
            let meta = result.metadata;
            let cover_path = result.cover_path;
            let evt_status = result.status.clone();
            let status = result.status;

            let updated_video = tokio::task::spawn_blocking(move || {
                let conn = db::open(&db_str)?;
                db::update_video_metadata(
                    &conn,
                    &vid_id,
                    meta.title.as_deref(),
                    cover_path.as_ref().and_then(|p| p.to_str()),
                    meta.series.as_deref(),
                    meta.duration,
                    meta.released_at.as_deref(),
                    &actor_details,
                    &meta.tags,
                    meta.maker.as_deref(),
                    &sample_paths,
                    status,
                )?;
                for (actor_name, photo_path) in &actor_photo_map {
                    let _ = conn.execute(
                        "UPDATE actors SET photo_path = ?1 WHERE name = ?2 AND photo_path IS NULL",
                        rusqlite::params![photo_path.to_str(), actor_name],
                    );
                }
                let video = db::get_video_by_id(&conn, &vid_id).ok();
                Ok::<Option<Video>, rusqlite::Error>(video)
            })
            .await
            .unwrap_or(Ok(None))
            .unwrap_or(None);

            // Increment retry_count for transient failures
            if matches!(evt_status, ScrapeStatus::NotScraped) {
                let db_str2 = db_path.to_str().unwrap().to_string();
                let vid_id2 = video_id.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(conn) = db::open(&db_str2) {
                        let _ = db::increment_retry_count(&conn, &vid_id2);
                    }
                })
                .await;
            }

            let _ = app.emit(
                "scrape-progress",
                ScrapeProgressEvent {
                    video_id,
                    status: evt_status,
                    current: i + 1,
                    total,
                    video: updated_video,
                },
            );
        }

        tracing::info!("auto_scrape: complete");
        let _ = app.emit("scrape-complete", total);
    });
}
```

- [ ] **Step 2: Call auto-scrape on app startup**

In the `setup` closure, after the watcher is started and before `Ok(())`, add:

```rust
            // Auto-scrape unscraped videos on startup
            start_auto_scrape(
                _app.handle(),
                &db_path,
                &_app.state::<ThumbnailsDir>().0,
                &_app.state::<ActorsDir>().0,
                &_app.state::<SamplesDir>().0,
                _app.state::<ScrapeCancel>().0.clone(),
            );
```

- [ ] **Step 3: Listen for auto-scrape-needed event from watcher**

Also in the `setup` closure, after the auto-scrape call above, add an event listener:

```rust
            // Listen for watcher auto-scrape requests
            let app_handle = _app.handle().clone();
            let db_path2 = db_path.clone();
            _app.listen("auto-scrape-needed", move |_event| {
                let thumbnails = app_handle.state::<ThumbnailsDir>().0.clone();
                let actors = app_handle.state::<ActorsDir>().0.clone();
                let samples = app_handle.state::<SamplesDir>().0.clone();
                let cancel = app_handle.state::<ScrapeCancel>().0.clone();
                start_auto_scrape(
                    &app_handle,
                    &db_path2,
                    &thumbnails,
                    &actors,
                    &samples,
                    cancel,
                );
            });
```

- [ ] **Step 4: Compile check**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: auto-scrape on app startup + watcher event"
```

---

### Task 7: Set scrape mode from backend for auto-scrape progress

**Files:**
- Modify: `src/components/layout/AppShell.tsx`

- [ ] **Step 1: Set scrapeMode to progress when auto-scrape starts**

The existing `scrape-progress` listener in AppShell only updates progress when `scrapeMode === 'progress'`. For auto-scrape, the mode starts as `idle` because the user didn't click anything. We need to auto-enter progress mode when the first `scrape-progress` event arrives while idle.

In `src/components/layout/AppShell.tsx`, update the `scrape-progress` listener:

Replace:

```typescript
        const u3 = await listen<{ current: number; total: number; status: string; video?: Video }>(
          'scrape-progress',
          (e) => {
            const store = useLibraryStore.getState()
            if (store.scrapeMode !== 'progress') return
```

With:

```typescript
        const u3 = await listen<{ current: number; total: number; status: string; video?: Video }>(
          'scrape-progress',
          (e) => {
            const store = useLibraryStore.getState()
            // Auto-enter progress mode on first event (for auto-scrape)
            if (store.scrapeMode === 'idle') {
              store.setScrapeMode('progress')
              store.setScrapeProgress({ current: 0, total: e.payload.total, success: 0, fail: 0 })
            }
            if (store.scrapeMode !== 'progress') return
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 3: Commit**

```bash
git add src/components/layout/AppShell.tsx
git commit -m "feat: auto-enter progress mode for auto-scrape events"
```

---

### Task 8: Final build verification

- [ ] **Step 1: Full Rust compile**

Run: `cd src-tauri && cargo check`
Expected: compiles successfully

- [ ] **Step 2: Full TypeScript check**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 3: Run frontend tests**

Run: `npx vitest run`
Expected: all tests pass
