# Local Thumbnail Generation, Folder Grouping & Seek Bar Preview — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add FFmpeg-based local thumbnail generation, folder-based grouping for unidentified videos, manual code assignment, and seek bar sprite sheet preview.

**Architecture:** FFmpeg is called as an external process via `std::process::Command` — no library linking, no Tauri shell plugin. Binary path is resolved at startup (next to exe for production, system PATH for development). `tauri.conf.json` `externalBin` handles production bundling. Scanner groups code="?" files by parent folder. Sprite sheets are lazily generated on Cinema Mode entry.

**Tech Stack:** Rust (std::process::Command for FFmpeg), Tauri v2 (externalBin sidecar), React 19, TypeScript, Zustand

**Spec:** `docs/superpowers/specs/2026-04-12-local-thumbnails-folder-grouping.md`

---

## File Structure

### Backend (Rust) — Create
| File | Responsibility |
|------|---------------|
| `src-tauri/src/ffmpeg.rs` | FFmpeg binary resolution, thumbnail extraction, sprite sheet generation |

### Backend (Rust) — Modify
| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | New commands (`check_ffmpeg`, `assign_code`, `get_or_generate_sprite`), `SpritesDir` state, `FfmpegPath` state, thumbnail generation in `scan_library` |
| `src-tauri/src/scanner.rs` | Folder-based grouping for code="?" files in `group_by_code` |
| `src-tauri/src/models.rs` | `SpriteInfo` struct |
| `src-tauri/src/db.rs` | `set_thumbnail_path`, `assign_code`, `get_videos_without_thumbnail` functions |
| `src-tauri/tauri.conf.json` | `bundle.externalBin` for FFmpeg/FFprobe |

### Frontend (TypeScript/React) — Modify
| File | Changes |
|------|---------|
| `src/types/index.ts` | `SpriteInfo` interface, `isUnidentified` helper, `FilterState.unidentifiedOnly` |
| `src/components/library/VideoCard.tsx` | "미식별" badge for unidentified videos |
| `src/components/detail/VideoMetadata.tsx` | "미식별" badge + manual code input |
| `src/components/library/FilterBar.tsx` | "미식별" filter toggle |
| `src/hooks/useFilteredVideos.ts` | Unidentified filter logic |
| `src/components/detail/CinemaPlayer.tsx` | Fetch sprite info on Cinema Mode entry |
| `src/components/detail/PlayerControls.tsx` | Sprite thumbnail in seek bar tooltip |
| `src/pages/SettingsPage.tsx` | FFmpeg + LGPL license notice |

---

## Task 1: FFmpeg Module + Sidecar Configuration

**Files:**
- Create: `src-tauri/src/ffmpeg.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

### Step-by-step

- [ ] **Step 1: Add `externalBin` to tauri.conf.json**

In `src-tauri/tauri.conf.json`, add `externalBin` inside the existing `bundle` object:

```json
"bundle": {
    "active": true,
    "targets": "all",
    "externalBin": ["binaries/ffmpeg", "binaries/ffprobe"],
    "icon": [...]
}
```

> **Dev note:** For development, FFmpeg does NOT need to be present — the code gracefully returns `None`/`false` when the binary isn't found. For production builds, place LGPL-only static FFmpeg/FFprobe binaries at `src-tauri/binaries/ffmpeg-{target_triple}{.exe}` and `src-tauri/binaries/ffprobe-{target_triple}{.exe}`. Get your target triple with `rustc --print host-tuple`.

- [ ] **Step 2: Add `SpriteInfo` to models.rs**

At the end of `src-tauri/src/models.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteInfo {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub interval: u32,
    pub total_frames: u32,
}
```

- [ ] **Step 3: Create `src-tauri/src/ffmpeg.rs`**

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::SpriteInfo;

/// Resolve the FFmpeg or FFprobe binary path.
/// Checks next to the executable first (production), then system PATH (development).
pub fn resolve_binary(name: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    // 1. Next to the executable (production bundle)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join(&exe_name);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // 2. System PATH (development)
    let check = Command::new(name)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if check.map(|s| s.success()).unwrap_or(false) {
        return Some(PathBuf::from(name));
    }

    None
}

/// Check if FFmpeg is available.
pub fn check(ffmpeg_path: &Option<PathBuf>) -> bool {
    ffmpeg_path.is_some()
}

/// Get video duration in seconds using ffprobe.
fn get_duration(ffprobe_path: &Path, file_path: &str) -> Option<f64> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            file_path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<f64>().ok()
}

/// Extract a single frame as JPEG at the given timestamp.
fn extract_frame(ffmpeg_path: &Path, file_path: &str, timestamp: f64, output_path: &Path) -> bool {
    let ts = format!("{:.2}", timestamp);
    let status = Command::new(ffmpeg_path)
        .args([
            "-y",
            "-ss", &ts,
            "-i", file_path,
            "-frames:v", "1",
            "-q:v", "3",
        ])
        .arg(output_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    status.map(|s| s.success()).unwrap_or(false)
}

/// Check if a JPEG file is likely a black frame (< 3KB).
fn is_black_frame(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() < 3_000)
        .unwrap_or(true)
}

/// Extract a thumbnail for a video. Tries 10%, 25%, 50% of duration.
/// Returns the path to the generated thumbnail, or None on failure.
pub fn extract_thumbnail(
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
    file_path: &str,
    video_id: &str,
    thumbnails_dir: &Path,
) -> Option<String> {
    let duration = get_duration(ffprobe_path, file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let output_path = thumbnails_dir.join(format!("{video_id}_local.jpg"));
    let percentages = [0.10, 0.25, 0.50];

    for pct in percentages {
        let timestamp = duration * pct;
        if extract_frame(ffmpeg_path, file_path, timestamp, &output_path) && !is_black_frame(&output_path) {
            return Some(output_path.to_string_lossy().to_string());
        }
    }

    // All attempts were black frames — use the last one anyway
    if output_path.exists() {
        Some(output_path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Generate a sprite sheet for seek bar preview.
/// Returns SpriteInfo or None on failure.
pub fn generate_sprite_sheet(
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
    file_path: &str,
    video_id: &str,
    part_index: u32,
    sprites_dir: &Path,
) -> Option<SpriteInfo> {
    let duration = get_duration(ffprobe_path, file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let interval = (duration / 100.0).ceil().max(10.0) as u32;
    let total_frames = (duration / interval as f64).ceil() as u32;
    let columns: u32 = 10;
    let rows = (total_frames as f64 / columns as f64).ceil() as u32;

    let sprite_path = sprites_dir.join(format!("{video_id}_part{part_index}.jpg"));
    let meta_path = sprites_dir.join(format!("{video_id}_part{part_index}.json"));

    // Check cache
    if sprite_path.exists() && meta_path.exists() {
        let json = std::fs::read_to_string(&meta_path).ok()?;
        return serde_json::from_str::<SpriteInfo>(&json).ok();
    }

    let vf = format!("fps=1/{interval},scale=160:-1,tile={columns}x{rows}");
    let status = Command::new(ffmpeg_path)
        .args([
            "-y",
            "-i", file_path,
            "-vf", &vf,
            "-q:v", "5",
        ])
        .arg(&sprite_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if !status.map(|s| s.success()).unwrap_or(false) || !sprite_path.exists() {
        return None;
    }

    // Read actual frame dimensions from the sprite image
    // Each frame is 160px wide; height is auto-scaled by FFmpeg.
    // We can calculate from the sprite image dimensions.
    let (sprite_w, sprite_h) = image_dimensions(&sprite_path)?;
    let frame_w = sprite_w / columns;
    let frame_h = sprite_h / rows;

    let info = SpriteInfo {
        url: sprite_path.to_string_lossy().to_string(),
        width: frame_w,
        height: frame_h,
        columns,
        rows,
        interval,
        total_frames,
    };

    // Cache metadata
    if let Ok(json) = serde_json::to_string(&info) {
        let _ = std::fs::write(&meta_path, json);
    }

    Some(info)
}

/// Get image dimensions (width, height) by reading JPEG header.
/// Minimal approach: use the imagesize crate or parse JPEG SOF marker.
fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    // Read file and find JPEG SOF0 (0xFFC0) or SOF2 (0xFFC2) marker
    let data = std::fs::read(path).ok()?;
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            let marker = data[i + 1];
            if marker == 0xC0 || marker == 0xC2 {
                // SOF marker: skip marker (2) + length (2) + precision (1) = 5
                if i + 8 < data.len() {
                    let height = ((data[i + 5] as u32) << 8) | (data[i + 6] as u32);
                    let width = ((data[i + 7] as u32) << 8) | (data[i + 8] as u32);
                    return Some((width, height));
                }
            }
            if marker != 0x00 && marker != 0xFF {
                if i + 3 < data.len() {
                    let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                    i += 2 + len;
                } else {
                    break;
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    None
}
```

- [ ] **Step 4: Wire `ffmpeg` module + `check_ffmpeg` command in lib.rs**

In `src-tauri/src/lib.rs`:

1. Add module declaration at top:
```rust
mod ffmpeg;
```

2. Add managed state structs (after existing structs):
```rust
struct FfmpegPath(Option<PathBuf>);
struct FfprobePath(Option<PathBuf>);
struct SpritesDir(PathBuf);
```

3. Add the command:
```rust
#[tauri::command]
fn check_ffmpeg(ffmpeg_path: tauri::State<'_, FfmpegPath>) -> bool {
    ffmpeg::check(&ffmpeg_path.0)
}
```

4. In the `setup` closure, after `samples_dir` setup, add:
```rust
let sprites_dir = data_dir.join("sprites");
std::fs::create_dir_all(&sprites_dir)?;
_app.manage(SpritesDir(sprites_dir));

let ffmpeg_path = ffmpeg::resolve_binary("ffmpeg");
let ffprobe_path = ffmpeg::resolve_binary("ffprobe");
if ffmpeg_path.is_some() {
    tracing::info!("FFmpeg found: {:?}", ffmpeg_path.as_ref().unwrap());
} else {
    tracing::warn!("FFmpeg not found — thumbnail/sprite generation will be disabled");
}
_app.manage(FfmpegPath(ffmpeg_path));
_app.manage(FfprobePath(ffprobe_path));
```

5. Add `check_ffmpeg` to the `invoke_handler` macro.

- [ ] **Step 5: Verify**

```bash
cd src-tauri && cargo check
```
Expected: no errors. Warnings about unused functions in ffmpeg.rs are OK at this stage.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ffmpeg.rs src-tauri/src/models.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "feat: add FFmpeg module with sidecar config and check command"
```

---

## Task 2: Scanner Folder-Based Grouping

**Files:**
- Modify: `src-tauri/src/scanner.rs`

### Context

Currently, `group_by_code` creates individual `Video` records for each code="?" file. After this change, code="?" files in the same subfolder are grouped into one `Video` with `code="?:{folder_name}"`. Files directly in a scan root folder stay individual (code="?").

### Step-by-step

- [ ] **Step 1: Add `parent_dir` field to `ScannedFile`**

In `scanner.rs`, update the struct:

```rust
struct ScannedFile {
    path: String,
    size: u64,
    code: String,
    filename: String,
    parent_dir: String,
}
```

- [ ] **Step 2: Populate `parent_dir` in `scan_folders`**

In the scanning loop, after `let size = ...`:

```rust
let parent_dir = path
    .parent()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_default();

let sf = ScannedFile {
    path: path.to_string_lossy().to_string(),
    size,
    code: code.clone(),
    filename,
    parent_dir,
};
```

- [ ] **Step 3: Pass scan roots to `group_by_code`**

Update `scan_folders` to pass the folder list:

```rust
let videos = group_by_code(scanned, folders);
```

Update `group_by_code` signature:

```rust
fn group_by_code(files: Vec<ScannedFile>, scan_roots: &[String]) -> Vec<Video> {
```

- [ ] **Step 4: Implement folder grouping in `group_by_code`**

Replace the entire `group_by_code` function body:

```rust
fn group_by_code(files: Vec<ScannedFile>, scan_roots: &[String]) -> Vec<Video> {
    use std::collections::HashSet;
    use std::path::Path;

    let mut code_groups: HashMap<String, Vec<ScannedFile>> = HashMap::new();
    let mut unknown_individual: Vec<Video> = Vec::new();
    let mut unknown_folder_groups: HashMap<String, Vec<ScannedFile>> = HashMap::new();
    let now = Utc::now().to_rfc3339();

    // Normalize scan roots for comparison
    let root_set: HashSet<String> = scan_roots
        .iter()
        .map(|s| {
            let p = Path::new(s.trim());
            p.to_string_lossy().to_string()
        })
        .collect();

    for file in files {
        if file.code == "?" {
            // Check if file is directly in a scan root folder
            let is_in_root = root_set.contains(&file.parent_dir);

            if is_in_root {
                // Root folder files stay individual
                unknown_individual.push(Video {
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
                    scrape_status: ScrapeStatus::NotScraped,
                    scraped_at: None,
                    maker_name: None,
                });
            } else {
                // Subfolder files → group by parent_dir
                unknown_folder_groups
                    .entry(file.parent_dir.clone())
                    .or_default()
                    .push(file);
            }
        } else {
            code_groups.entry(file.code.clone()).or_default().push(file);
        }
    }

    // Build videos from code groups (unchanged logic)
    let mut videos: Vec<Video> = code_groups
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
                scrape_status: ScrapeStatus::NotScraped,
                scraped_at: None,
                maker_name: None,
            }
        })
        .collect();

    // Build videos from folder-grouped unknown files
    for (parent_dir, files) in unknown_folder_groups {
        let folder_name = Path::new(&parent_dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        videos.push(Video {
            id: Uuid::new_v4().to_string(),
            code: format!("?:{folder_name}"),
            title: folder_name,
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
            scrape_status: ScrapeStatus::NotScraped,
            scraped_at: None,
            maker_name: None,
        });
    }

    videos.extend(unknown_individual);
    videos
}
```

- [ ] **Step 5: Add tests for folder grouping**

Add to the existing `#[cfg(test)] mod tests` in `scanner.rs`:

```rust
#[test]
fn test_scan_groups_unknown_by_folder() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("My_Folder");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("part1.mp4"), "fake").unwrap();
    fs::write(sub.join("part2.mp4"), "fake").unwrap();

    let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();

    let grouped: Vec<&Video> = result.iter().filter(|v| v.code == "?:My_Folder").collect();
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].title, "My_Folder");
    assert_eq!(grouped[0].files.len(), 2);
}

#[test]
fn test_scan_root_unknown_stays_individual() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("random1.mp4"), "fake").unwrap();
    fs::write(dir.path().join("random2.mp4"), "fake").unwrap();

    let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();

    let unknowns: Vec<&Video> = result.iter().filter(|v| v.code == "?").collect();
    assert_eq!(unknowns.len(), 2);
}

#[test]
fn test_scan_mixed_folder() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("Mixed");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("part1.mp4"), "fake").unwrap();      // code="?"
    fs::write(sub.join("ABC-123.mp4"), "fake").unwrap();     // code="ABC-123"

    let result = scan_folders(&[dir.path().to_string_lossy().to_string()]).unwrap();

    assert!(result.iter().any(|v| v.code == "ABC-123"));
    assert!(result.iter().any(|v| v.code == "?:Mixed" && v.files.len() == 1));
}
```

- [ ] **Step 6: Verify**

```bash
cd src-tauri && cargo test
```
Expected: all existing + new tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "feat: group unidentified files by parent folder"
```

---

## Task 3: Local Thumbnail Generation During Scan

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`

### Context

After `scan_library` upserts videos, it now generates thumbnails for videos that don't have one. Uses the FFmpeg functions from Task 1. Multipart videos use `files[0]`.

### Step-by-step

- [ ] **Step 1: Add DB helper functions in db.rs**

Add to `src-tauri/src/db.rs`:

```rust
/// Get (id, first_file_path) for videos without a thumbnail.
pub fn get_videos_without_thumbnail(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT v.id, vf.path FROM videos v
         JOIN video_files vf ON vf.video_id = v.id
         WHERE v.thumbnail_path IS NULL
         GROUP BY v.id
         ORDER BY vf.rowid ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

/// Set the thumbnail_path for a video.
pub fn set_thumbnail_path(conn: &Connection, video_id: &str, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE videos SET thumbnail_path = ?1 WHERE id = ?2",
        params![path, video_id],
    )?;
    Ok(())
}
```

- [ ] **Step 2: Integrate thumbnail generation into `scan_library`**

In `src-tauri/src/lib.rs`, modify the `scan_library` command. Add the FFmpeg state parameters and thumbnail generation after the orphan removal block:

```rust
#[tauri::command]
fn scan_library(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    ffmpeg_state: tauri::State<'_, FfmpegPath>,
    ffprobe_state: tauri::State<'_, FfprobePath>,
) -> Result<Vec<Video>, String> {
    tracing::info!("cmd: scan_library");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;

    // Remove orphaned videos (existing logic — keep as-is)
    let scanned_codes: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.code.clone()).collect();
    let all_db = db::get_all_video_id_codes(&conn).map_err(|e| e.to_string())?;
    let orphan_ids: Vec<String> = all_db
        .into_iter()
        .filter(|(_, code)| !scanned_codes.contains(code))
        .map(|(id, _)| id)
        .collect();
    if !orphan_ids.is_empty() {
        tracing::info!("scan_library: removing {} orphaned videos", orphan_ids.len());
        db::delete_videos(&conn, &orphan_ids).map_err(|e| e.to_string())?;
    }

    // Generate thumbnails for videos without one
    if let (Some(ffmpeg), Some(ffprobe)) = (&ffmpeg_state.0, &ffprobe_state.0) {
        let need_thumbs = db::get_videos_without_thumbnail(&conn).map_err(|e| e.to_string())?;
        if !need_thumbs.is_empty() {
            tracing::info!("scan_library: generating thumbnails for {} videos", need_thumbs.len());
        }
        for (video_id, file_path) in &need_thumbs {
            if let Some(thumb_path) = ffmpeg::extract_thumbnail(ffmpeg, ffprobe, file_path, video_id, &thumbnails.0) {
                let _ = db::set_thumbnail_path(&conn, video_id, &thumb_path);
                tracing::info!("scan_library: thumbnail generated for {}", video_id);
            }
        }
    }

    db::get_all_videos(&conn).map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Verify**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: generate local thumbnails during scan via FFmpeg"
```

---

## Task 4: Frontend — Unidentified Video UI

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/components/library/VideoCard.tsx`
- Modify: `src/components/detail/VideoMetadata.tsx`
- Modify: `src/components/library/FilterBar.tsx`
- Modify: `src/hooks/useFilteredVideos.ts`

### Context

Videos with `code.startsWith("?:")` or `code === "?"` are "unidentified". They get a grey badge in the card and metadata views. A new filter toggle lets users filter to unidentified-only. The code badge shows the folder name (extracted from `?:Folder_Name`) rather than the raw code.

### Step-by-step

- [ ] **Step 1: Add `SpriteInfo` type, `isUnidentified` helper, and filter state**

In `src/types/index.ts`:

After the `Video` interface, add:

```typescript
export interface SpriteInfo {
  url: string
  width: number
  height: number
  columns: number
  rows: number
  interval: number
  totalFrames: number
}

export function isUnidentified(video: Video): boolean {
  return video.code === '?' || video.code.startsWith('?:')
}

/** Extract display name from code. "?:Folder_Name" → "Folder_Name", others → code as-is */
export function displayCode(video: Video): string {
  if (video.code.startsWith('?:')) return video.code.slice(2)
  return video.code
}
```

Add `unidentifiedOnly` to `FilterState`:

```typescript
export interface FilterState {
  sortBy: 'addedAt' | 'releasedAt' | 'title'
  sortOrder: 'asc' | 'desc'
  watchedFilter: 'all' | 'watched' | 'unwatched'
  favoriteOnly: boolean
  tagFilter: TagFilter
  scrapeStatusFilter: ScrapeStatus | 'all'
  unidentifiedOnly: boolean
}
```

- [ ] **Step 2: Update libraryStore default filters**

In `src/stores/libraryStore.ts`, find the default `FilterState` and add `unidentifiedOnly: false` to it.

- [ ] **Step 3: Add "미식별" badge to VideoCard**

In `src/components/library/VideoCard.tsx`:

1. Add import: `import { isUnidentified, displayCode } from '@/types'`

2. Find the code badge element (the div showing `video.code` near the top-left of the card). Replace the code text with `displayCode(video)`.

3. After the code badge, add the unidentified badge (conditionally rendered):

```tsx
{isUnidentified(video) && (
  <span className="absolute top-8 left-1.5 bg-muted text-muted-foreground text-[10px] px-1.5 py-0.5 rounded font-medium z-10">
    미식별
  </span>
)}
```

The exact positioning (`top-8`) should be adjusted so it sits below the existing code badge. Check the existing badge's position and offset accordingly.

- [ ] **Step 4: Add "미식별" badge to VideoMetadata**

In `src/components/detail/VideoMetadata.tsx`:

1. Add import: `import { isUnidentified, displayCode } from '@/types'`

2. Find where the scrape status badge is rendered. After it, add:

```tsx
{isUnidentified(video) && (
  <span className="border border-muted-foreground/30 text-muted-foreground text-xs px-2 py-0.5 rounded">
    미식별
  </span>
)}
```

3. Replace the code display text with `displayCode(video)`.

- [ ] **Step 5: Add "미식별" filter toggle to FilterBar**

In `src/components/library/FilterBar.tsx`:

1. Add import: `import { isUnidentified } from '@/types'`

2. In the controls row, after the favorite toggle button, add:

```tsx
<button
  onClick={() =>
    setFilters({
      ...filters,
      unidentifiedOnly: !filters.unidentifiedOnly,
    })
  }
  className={cn(
    'text-xs px-2 py-1 rounded border transition-colors',
    filters.unidentifiedOnly
      ? 'bg-muted text-foreground border-muted-foreground/50'
      : 'text-muted-foreground border-border hover:border-muted-foreground/30'
  )}
>
  미식별
</button>
```

- [ ] **Step 6: Add unidentified filter logic to useFilteredVideos**

In `src/hooks/useFilteredVideos.ts`:

1. Add import: `import { isUnidentified } from '@/types'`

2. After the scrape status filter block, add:

```typescript
// 미식별 필터
if (filters.unidentifiedOnly) {
  result = result.filter((v) => isUnidentified(v))
}
```

- [ ] **Step 7: Verify**

```bash
pnpm tsc --noEmit
```

- [ ] **Step 8: Commit**

```bash
git add src/types/index.ts src/stores/libraryStore.ts src/components/library/VideoCard.tsx src/components/detail/VideoMetadata.tsx src/components/library/FilterBar.tsx src/hooks/useFilteredVideos.ts
git commit -m "feat: add unidentified video UI with badge and filter"
```

---

## Task 5: Manual Code Assignment

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/detail/VideoMetadata.tsx`

### Context

Unidentified videos can have a code manually assigned by the user. Backend merges with existing video if the code already exists. Frontend shows an input + button only for unidentified videos.

### Step-by-step

- [ ] **Step 1: Add `assign_code` to db.rs**

```rust
/// Assign a new code to a video. If a video with that code already exists,
/// merge files into the existing video and delete the old one.
/// Returns the final video ID.
pub fn assign_code(conn: &Connection, video_id: &str, new_code: &str) -> Result<String> {
    // Check if a video with the new code already exists
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id FROM videos WHERE code = ?1 AND id != ?2",
            params![new_code, video_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(target_id) = existing_id {
        // Merge: move files from old video to existing one
        conn.execute(
            "UPDATE video_files SET video_id = ?1 WHERE video_id = ?2",
            params![target_id, video_id],
        )?;
        // Delete the old video record (cascade doesn't apply to video_files since we moved them)
        conn.execute("DELETE FROM video_tags WHERE video_id = ?1", [video_id])?;
        conn.execute("DELETE FROM video_actors WHERE video_id = ?1", [video_id])?;
        conn.execute("DELETE FROM sample_images WHERE video_id = ?1", [video_id])?;
        conn.execute("DELETE FROM videos WHERE id = ?1", [video_id])?;
        Ok(target_id)
    } else {
        // No collision: update code and reset scrape status
        conn.execute(
            "UPDATE videos SET code = ?1, scrape_status = 'not_scraped', scraped_at = NULL, retry_count = 0 WHERE id = ?2",
            params![new_code, video_id],
        )?;
        Ok(video_id.to_string())
    }
}
```

- [ ] **Step 2: Add `assign_code` command to lib.rs**

```rust
#[tauri::command]
fn assign_code(
    db: tauri::State<'_, DbPath>,
    video_id: String,
    new_code: String,
) -> Result<Video, String> {
    tracing::info!("cmd: assign_code video_id={} new_code={}", video_id, new_code);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let final_id = db::assign_code(&conn, &video_id, &new_code).map_err(|e| e.to_string())?;
    db::get_video_by_id(&conn, &final_id).map_err(|e| e.to_string())
}
```

Add `assign_code` to the `invoke_handler` macro.

- [ ] **Step 3: Add manual code input to VideoMetadata**

In `src/components/detail/VideoMetadata.tsx`:

1. Add state for code input (inside the component):

```tsx
const [newCode, setNewCode] = useState('')
const [assigning, setAssigning] = useState(false)
```

2. Add a handler:

```tsx
const handleAssignCode = async () => {
  const trimmed = newCode.trim().toUpperCase()
  if (!trimmed) return
  setAssigning(true)
  try {
    const updated = await invoke<Video>('assign_code', {
      videoId: video.id,
      newCode: trimmed,
    })
    // Navigate to the updated video (ID may have changed due to merge)
    window.location.href = `/library/${updated.id}`
  } catch (e) {
    console.error('assign_code failed:', e)
  } finally {
    setAssigning(false)
  }
}
```

3. Below the "미식별" badge (added in Task 4), render the input — only for unidentified videos:

```tsx
{isUnidentified(video) && (
  <div className="flex items-center gap-2 mt-2">
    <input
      type="text"
      placeholder="코드 입력 (예: ABC-123)"
      value={newCode}
      onChange={(e) => setNewCode(e.target.value)}
      onKeyDown={(e) => e.key === 'Enter' && handleAssignCode()}
      className="text-xs border border-border rounded px-2 py-1 bg-background text-foreground w-40"
    />
    <button
      onClick={handleAssignCode}
      disabled={assigning || !newCode.trim()}
      className="text-xs px-2 py-1 rounded bg-primary text-primary-foreground disabled:opacity-50"
    >
      {assigning ? '...' : '할당'}
    </button>
  </div>
)}
```

You'll also need `import { invoke } from '@tauri-apps/api/core'` and `import { useState } from 'react'` and `import type { Video } from '@/types'` (check existing imports).

- [ ] **Step 4: Verify**

```bash
cd src-tauri && cargo check && cd .. && pnpm tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs src/components/detail/VideoMetadata.tsx
git commit -m "feat: add manual code assignment for unidentified videos"
```

---

## Task 6: Seek Bar Sprite Sheet Preview

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/detail/CinemaPlayer.tsx`
- Modify: `src/components/detail/PlayerControls.tsx`

### Context

When Cinema Mode opens, the frontend calls `get_or_generate_sprite` to get or lazily generate a sprite sheet for the current video file. If available, the seek bar tooltip shows a thumbnail from the sprite sheet above the time text.

### Step-by-step

- [ ] **Step 1: Add `get_or_generate_sprite` command to lib.rs**

```rust
#[tauri::command]
fn get_or_generate_sprite(
    ffmpeg_state: tauri::State<'_, FfmpegPath>,
    ffprobe_state: tauri::State<'_, FfprobePath>,
    sprites: tauri::State<'_, SpritesDir>,
    video_id: String,
    file_path: String,
    part_index: u32,
) -> Option<models::SpriteInfo> {
    tracing::info!("cmd: get_or_generate_sprite video_id={} part={}", video_id, part_index);
    let ffmpeg = ffmpeg_state.0.as_ref()?;
    let ffprobe = ffprobe_state.0.as_ref()?;
    ffmpeg::generate_sprite_sheet(ffmpeg, ffprobe, &file_path, &video_id, part_index, &sprites.0)
}
```

Add `get_or_generate_sprite` to the `invoke_handler` macro.

- [ ] **Step 2: Add sprite info prop to PlayerControls**

In `src/components/detail/PlayerControls.tsx`:

1. Add import: `import type { SpriteInfo } from '@/types'`
2. Add import: `import { assetUrl } from '@/lib/utils'`

3. Add prop:
```typescript
interface PlayerControlsProps {
  // ... existing props ...
  spriteInfo?: SpriteInfo | null
}
```

4. Destructure in component: `spriteInfo` from props.

5. Replace the hover tooltip `div` (the one that shows `formatTime(hoverTime.time)`) with:

```tsx
{hoverTime && (
  <div
    className="absolute -translate-x-1/2 pointer-events-none flex flex-col items-center"
    style={{ left: hoverTime.left, bottom: '100%', marginBottom: 4 }}
  >
    {spriteInfo && (
      <div
        className="border border-white/20 rounded overflow-hidden mb-1"
        style={{
          width: spriteInfo.width,
          height: spriteInfo.height,
          backgroundImage: `url(${assetUrl(spriteInfo.url)})`,
          backgroundPosition: (() => {
            const frameIndex = Math.min(
              Math.floor(hoverTime.time / spriteInfo.interval),
              spriteInfo.totalFrames - 1
            )
            const col = frameIndex % spriteInfo.columns
            const row = Math.floor(frameIndex / spriteInfo.columns)
            return `-${col * spriteInfo.width}px -${row * spriteInfo.height}px`
          })(),
          backgroundSize: `${spriteInfo.columns * spriteInfo.width}px ${spriteInfo.rows * spriteInfo.height}px`,
        }}
      />
    )}
    <span className="bg-black/80 text-white text-xs font-mono px-2 py-1 rounded">
      {formatTime(hoverTime.time)}
    </span>
  </div>
)}
```

This replaces the existing `-top-6` positioned tooltip. The new tooltip uses `bottom: '100%'` to sit above the seek bar padding area, with the sprite thumbnail above the time text.

- [ ] **Step 3: Fetch sprite info in CinemaPlayer**

In `src/components/detail/CinemaPlayer.tsx`:

1. Add imports:
```typescript
import type { SpriteInfo } from '@/types'
import { invoke } from '@tauri-apps/api/core'
```

2. Add state:
```typescript
const [spriteInfo, setSpriteInfo] = useState<SpriteInfo | null>(null)
```

3. Add useEffect to fetch sprite when part changes:
```typescript
useEffect(() => {
  setSpriteInfo(null)
  const file = files[currentPart]
  if (!file) return

  invoke<SpriteInfo | null>('get_or_generate_sprite', {
    videoId: videoCode,
    filePath: file.path,
    partIndex: currentPart,
  }).then((info) => {
    setSpriteInfo(info ?? null)
  }).catch(() => {
    // FFmpeg not available or generation failed — no sprite preview
  })
}, [currentPart, files, videoCode])
```

Here `currentPart` is the existing state variable tracking which part is playing. If the variable is named differently (e.g., `partIndex`), use that name.

4. Pass to PlayerControls:
```tsx
<PlayerControls
  // ... existing props ...
  spriteInfo={spriteInfo}
/>
```

- [ ] **Step 4: Verify**

```bash
cd src-tauri && cargo check && cd .. && pnpm tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src/components/detail/CinemaPlayer.tsx src/components/detail/PlayerControls.tsx
git commit -m "feat: add seek bar sprite sheet preview in Cinema Mode"
```

---

## Task 7: License Compliance Notice

**Files:**
- Modify: `src/pages/SettingsPage.tsx`

### Context

Display FFmpeg + LGPL notice in the settings page as required by the LGPL compliance checklist.

### Step-by-step

- [ ] **Step 1: Add FFmpeg notice section to SettingsPage**

In `src/pages/SettingsPage.tsx`, after the last existing section (before the closing container div), add:

```tsx
{/* FFmpeg License Notice */}
<div className="space-y-2">
  <h3 className="text-sm font-medium text-muted-foreground">오픈소스 라이선스</h3>
  <div className="text-xs text-muted-foreground space-y-1">
    <p>
      이 앱은 썸네일 생성 및 미리보기를 위해{' '}
      <a href="https://ffmpeg.org" target="_blank" rel="noopener noreferrer" className="underline">
        FFmpeg
      </a>
      를 사용합니다.
    </p>
    <p>
      FFmpeg is licensed under the{' '}
      <a
        href="https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html"
        target="_blank"
        rel="noopener noreferrer"
        className="underline"
      >
        GNU Lesser General Public License (LGPL) v2.1
      </a>
      .
    </p>
    <p>
      FFmpeg source code:{' '}
      <a href="https://ffmpeg.org/download.html" target="_blank" rel="noopener noreferrer" className="underline">
        https://ffmpeg.org/download.html
      </a>
    </p>
  </div>
</div>
```

- [ ] **Step 2: Verify**

```bash
pnpm tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/pages/SettingsPage.tsx
git commit -m "feat: add FFmpeg LGPL license notice to settings"
```

---

## Self-Review Checklist

### Spec coverage

| Spec Section | Task |
|---|---|
| 1. FFmpeg Sidecar 번들링 | Task 1 |
| 2. 스캔 시 로컬 썸네일 생성 | Task 3 |
| 3. 폴더 기준 그룹핑 | Task 2 |
| 4. "미식별" 상태 UI 표시 | Task 4 |
| 5. 수동 코드 할당 | Task 5 |
| 6. 시크바 썸네일 프리뷰 | Task 6 |
| LGPL 라이선스 고지 | Task 7 |

### Type consistency

- `SpriteInfo`: Rust struct (Task 1) matches TS interface (Task 4), fields: `url`, `width`, `height`, `columns`, `rows`, `interval`, `totalFrames`
- `isUnidentified()`: defined in Task 4, used in Tasks 4, 5
- `displayCode()`: defined in Task 4, used in Task 4
- `FfmpegPath` / `FfprobePath`: defined and managed in Task 1, used in Tasks 3, 6
- `SpritesDir`: defined in Task 1, used in Task 6
- `check_ffmpeg`, `assign_code`, `get_or_generate_sprite`: all added to `invoke_handler` in their respective tasks

### Placeholder scan

No TBDs, TODOs, or "implement later" found. All steps contain concrete code.
