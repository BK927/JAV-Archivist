# Scan Result Toast Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a toast notification after library scan with added/removed counts.

**Architecture:** Change `scan_library` return type from `Vec<Video>` to a struct with videos + counts. Install `sonner` for toast UI. Frontend shows toast when counts > 0.

**Tech Stack:** Rust (Tauri command), sonner (toast), shadcn, React

---

## File Map

| File | Role |
|------|------|
| `src-tauri/src/models.rs` | Add `ScanResult` struct |
| `src-tauri/src/db.rs` | Change `upsert_videos` to return added count |
| `src-tauri/src/lib.rs` | Change `scan_library` to return `ScanResult` |
| `src/types/index.ts` | Add `ScanResult` interface |
| `src/components/layout/AppShell.tsx` | Update scan call + show toast |
| `src/pages/SettingsPage.tsx` | Update scan call + show toast |

---

### Task 1: Backend — ScanResult struct + upsert_videos returns added count

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db.rs:232-241` (upsert_videos)
- Modify: `src-tauri/src/lib.rs:96-139` (scan_library)

- [ ] **Step 1: Add ScanResult to models.rs**

After the `SpriteInfo` struct at the end of the file, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub videos: Vec<Video>,
    pub added: u32,
    pub removed: u32,
}
```

- [ ] **Step 2: Change upsert_videos to return added count**

In `src-tauri/src/db.rs`, change the signature and body of `upsert_videos`:

```rust
pub fn upsert_videos(conn: &Connection, videos: &[Video]) -> Result<u32> {
    conn.execute_batch("BEGIN")?;
    let result = upsert_videos_inner(conn, videos);
    match result {
        Ok(added) => {
            conn.execute_batch("COMMIT")?;
            Ok(added)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}
```

Change `upsert_videos_inner` signature to return `Result<u32>` and count new inserts:

```rust
fn upsert_videos_inner(conn: &Connection, videos: &[Video]) -> Result<u32> {
    let mut added: u32 = 0;
    for video in videos {
        let existing_id: Option<String> = if video.code != "?" {
            // ... existing code unchanged ...
        };

        let video_id = match existing_id {
            Some(id) => {
                conn.execute("DELETE FROM video_files WHERE video_id = ?1", [&id])?;
                id
            }
            None => {
                added += 1;
                // ... existing INSERT code unchanged ...
                video.id.clone()
            }
        };
        // ... rest unchanged ...
    }
    // ... orphan cleanup unchanged ...
    Ok(added)
}
```

- [ ] **Step 3: Change scan_library to return ScanResult**

In `src-tauri/src/lib.rs`, change the command:

```rust
#[tauri::command]
fn scan_library(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    ffmpeg_state: tauri::State<'_, FfmpegPath>,
    ffprobe_state: tauri::State<'_, FfprobePath>,
) -> Result<ScanResult, String> {
    tracing::info!("cmd: scan_library");
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    let settings = db::get_settings(&conn).map_err(|e| e.to_string())?;
    let scanned = scanner::scan_folders(&settings.scan_folders)?;
    let added = db::upsert_videos(&conn, &scanned).map_err(|e| e.to_string())?;

    // Remove orphaned videos
    let scanned_codes: std::collections::HashSet<String> =
        scanned.iter().map(|v| v.code.clone()).collect();
    let all_db = db::get_all_video_id_codes(&conn).map_err(|e| e.to_string())?;
    let orphan_ids: Vec<String> = all_db
        .into_iter()
        .filter(|(_, code)| !scanned_codes.contains(code))
        .map(|(id, _)| id)
        .collect();
    let removed = orphan_ids.len() as u32;
    if !orphan_ids.is_empty() {
        tracing::info!("scan_library: removing {} orphaned videos", orphan_ids.len());
        db::delete_videos(&conn, &orphan_ids).map_err(|e| e.to_string())?;
    }

    // Generate thumbnails (unchanged)
    if let (Some(ffmpeg), Some(ffprobe)) = (&ffmpeg_state.0, &ffprobe_state.0) {
        let need_thumbs = db::get_videos_without_thumbnail(&conn).map_err(|e| e.to_string())?;
        if !need_thumbs.is_empty() {
            tracing::info!("scan_library: generating thumbnails for {} videos", need_thumbs.len());
        }
        for (video_id, file_path) in &need_thumbs {
            if let Some(thumb_path) = ffmpeg::extract_thumbnail(ffmpeg, ffprobe, file_path, video_id, &thumbnails.0) {
                let _ = db::set_thumbnail_path(&conn, video_id, &thumb_path);
            }
        }
    }

    let videos = db::get_all_videos(&conn).map_err(|e| e.to_string())?;
    Ok(ScanResult { videos, added, removed })
}
```

Add `ScanResult` to the `use crate::models::` import at the top of lib.rs.

- [ ] **Step 4: Verify**

Run: `cd src-tauri && cargo check`
Expected: Success with no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: scan_library returns ScanResult with added/removed counts"
```

---

### Task 2: Frontend — ScanResult type + sonner + toast on scan

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/components/layout/AppShell.tsx`
- Modify: `src/pages/SettingsPage.tsx`

- [ ] **Step 1: Install sonner**

```bash
npx shadcn@latest add sonner
```

This installs the `sonner` package and creates `src/components/ui/sonner.tsx`.

- [ ] **Step 2: Add ScanResult interface to types**

In `src/types/index.ts`, after the `SpriteInfo` interface, add:

```typescript
export interface ScanResult {
  videos: Video[]
  added: number
  removed: number
}
```

- [ ] **Step 3: Update AppShell — add Toaster + toast on scan**

In `src/components/layout/AppShell.tsx`:

Add imports:
```typescript
import { Toaster } from '@/components/ui/sonner'
import { toast } from 'sonner'
import type { Video, Tag, ScanResult } from '@/types'
```

Change the scan call in the first useEffect:
```typescript
run<ScanResult>('scan_library', {}, { videos: [], added: 0, removed: 0 }).then((result) => {
  setVideos(result.videos)
  const parts: string[] = []
  if (result.added > 0) parts.push(`${result.added}개 추가`)
  if (result.removed > 0) parts.push(`${result.removed}개 제거`)
  if (parts.length > 0) toast(parts.join(' · '))
})
```

Add `<Toaster />` inside the return JSX, after `<ScrapeProgressBar />`:
```tsx
<ScrapeProgressBar />
<Toaster position="bottom-right" duration={4000} />
```

- [ ] **Step 4: Update SettingsPage — toast on rescan**

In `src/pages/SettingsPage.tsx`:

Add imports:
```typescript
import { toast } from 'sonner'
import type { AppSettings, Video, ScanResult } from '@/types'
```

Change `handleRescan`:
```typescript
const handleRescan = async () => {
  setScanning(true)
  const result = await run<ScanResult>('scan_library', {}, { videos: [], added: 0, removed: 0 })
  useLibraryStore.getState().setVideos(result.videos)
  setScanning(false)
  const parts: string[] = []
  if (result.added > 0) parts.push(`${result.added}개 추가`)
  if (result.removed > 0) parts.push(`${result.removed}개 제거`)
  if (parts.length > 0) toast(parts.join(' · '))
}
```

Remove `Video` from the existing `type` import since it's no longer used directly (only via `ScanResult`). Keep it if other code still references it.

- [ ] **Step 5: Verify**

Run: `pnpm tsc --noEmit`
Expected: No type errors.

- [ ] **Step 6: Commit**

```bash
git add src/types/index.ts src/components/layout/AppShell.tsx src/pages/SettingsPage.tsx src/components/ui/sonner.tsx package.json pnpm-lock.yaml
git commit -m "feat: show toast notification with scan results"
```

---

## Sequencing

```
Task 1 (backend) → Task 2 (frontend)
```

Sequential — Task 2 depends on the new return type from Task 1.

## Verification

1. `cargo check` — no errors
2. `pnpm tsc --noEmit` — no type errors
3. App start with existing library (no changes) → no toast
4. Add a new video file to scan folder → rescan → toast shows "1개 추가"
5. Remove a video file → rescan → toast shows "1개 제거"
6. Toast auto-disappears after 4 seconds
