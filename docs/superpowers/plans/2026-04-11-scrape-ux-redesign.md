# Scrape UX Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the inline FilterBar scraping UI with a selection mode + floating action bar pattern, enabling batch re-scrape and status management.

**Architecture:** Add `scrapeStatusFilter` to FilterState and selection mode state to libraryStore. Create `scrape_videos` and `reset_scrape_status` Tauri commands, remove `scrape_all_new`. Build a FloatingActionBar component that handles selection actions and shows scraping progress. VideoCard gains a checkbox overlay. VideoDetail always shows a scrape button.

**Tech Stack:** React 19, Zustand, Tailwind CSS, Tauri v2, Rust/SQLite

**Spec:** `docs/superpowers/specs/2026-04-11-scrape-ux-redesign-design.md`

---

### Task 1: Backend — `reset_scrape_status` DB function + command

**Files:**
- Modify: `src-tauri/src/db.rs` (add function after line 743)
- Modify: `src-tauri/src/lib.rs` (add command, register in handler)

- [ ] **Step 1: Add `reset_scrape_status` to db.rs**

Add after the `get_videos_to_scrape` function (line 743):

```rust
pub fn reset_scrape_status(conn: &Connection, video_ids: &[String]) -> Result<()> {
    if video_ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<&str> = video_ids.iter().map(|_| "?").collect();
    let sql = format!(
        "UPDATE videos SET scrape_status = 'not_scraped' WHERE id IN ({})",
        placeholders.join(",")
    );
    let params: Vec<&dyn rusqlite::types::ToSql> = video_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    conn.execute(&sql, params.as_slice())?;
    Ok(())
}
```

- [ ] **Step 2: Add `reset_scrape_status` Tauri command in lib.rs**

Add after the `cancel_scrape` command (line 386):

```rust
#[tauri::command]
fn reset_scrape_status(
    db: tauri::State<'_, DbPath>,
    video_ids: Vec<String>,
) -> Result<(), String> {
    tracing::info!("cmd: reset_scrape_status count={}", video_ids.len());
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::reset_scrape_status(&conn, &video_ids).map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 3: Register the command**

In the `generate_handler!` macro (line 466), add `reset_scrape_status`:

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
    scrape_all_new,       // will be replaced in Task 2
    cancel_scrape,
    reset_scrape_status,  // NEW
    reset_data,
    get_actors,
    get_series_list,
    get_tags,
    get_tag_cooccurrence,
    get_makers,
    get_sample_images,
])
```

- [ ] **Step 4: Build and verify**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: add reset_scrape_status command"
```

---

### Task 2: Backend — `scrape_videos` command, remove `scrape_all_new`

**Files:**
- Modify: `src-tauri/src/lib.rs` (replace `scrape_all_new` with `scrape_videos`, update handler)

- [ ] **Step 1: Replace `scrape_all_new` with `scrape_videos`**

Replace the `scrape_all_new` command (lines 280-379) with:

```rust
#[tauri::command]
async fn scrape_videos(
    db: tauri::State<'_, DbPath>,
    thumbnails: tauri::State<'_, ThumbnailsDir>,
    actors_state: tauri::State<'_, ActorsDir>,
    samples_state: tauri::State<'_, SamplesDir>,
    cancel: tauri::State<'_, ScrapeCancel>,
    app: tauri::AppHandle,
    video_ids: Vec<String>,
) -> Result<(), String> {
    tracing::info!("cmd: scrape_videos count={}", video_ids.len());
    let db_path = db.0.clone();
    let thumbnails_dir = thumbnails.0.clone();
    let actors_dir = actors_state.0.clone();
    let samples_dir = samples_state.0.clone();
    let cancel_flag = cancel.0.clone();

    cancel_flag.store(false, Ordering::SeqCst);

    // Fetch codes for the requested video IDs
    let ids = video_ids.clone();
    let to_scrape = tokio::task::spawn_blocking(move || {
        let conn = db::open(db_path.to_str().unwrap()).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for id in &ids {
            if let Ok(video) = db::get_video_by_id(&conn, id) {
                if video.code != "?" {
                    result.push((video.id, video.code));
                }
            }
        }
        Ok::<Vec<(String, String)>, String>(result)
    })
    .await
    .map_err(|e| e.to_string())??;

    let total = to_scrape.len();
    tracing::info!("scrape_videos: {} videos to scrape", total);
    let pipeline = scraper::ScrapePipeline::new(thumbnails_dir, actors_dir, samples_dir)?;

    for (i, (video_id, code)) in to_scrape.into_iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            tracing::info!("scrape_videos: cancelled at {}/{}", i + 1, total);
            break;
        }

        let result = pipeline.scrape_one(&code, &video_id).await;

        let db_path2 = db.0.clone();
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
        let meta = result.metadata;
        let cover_path = result.cover_path;
        let evt_status = result.status.clone();
        let status = result.status;

        let updated_video = tokio::task::spawn_blocking(move || {
            let conn = db::open(db_path2.to_str().unwrap())?;

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
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let _ = app.emit("scrape-progress", ScrapeProgressEvent {
            video_id,
            status: evt_status,
            current: i + 1,
            total,
            video: updated_video,
        });
    }

    tracing::info!("scrape_videos: complete, processed {}", total);
    let _ = app.emit("scrape-complete", total);
    Ok(())
}
```

- [ ] **Step 2: Update handler registration**

Replace `scrape_all_new` with `scrape_videos` in the `generate_handler!` macro:

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
    scrape_videos,         // CHANGED from scrape_all_new
    cancel_scrape,
    reset_scrape_status,
    reset_data,
    get_actors,
    get_series_list,
    get_tags,
    get_tag_cooccurrence,
    get_makers,
    get_sample_images,
])
```

- [ ] **Step 3: Build and verify**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors. No remaining references to `scrape_all_new`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: replace scrape_all_new with scrape_videos (accepts video IDs)"
```

---

### Task 3: Frontend — Add `scrapeStatusFilter` and selection state to store

**Files:**
- Modify: `src/types/index.ts` (add `scrapeStatusFilter` to `FilterState`)
- Modify: `src/stores/libraryStore.ts` (add selection state + actions)
- Modify: `src/hooks/useFilteredVideos.ts` (add scrapeStatus filtering)

- [ ] **Step 1: Add `scrapeStatusFilter` to FilterState type**

In `src/types/index.ts`, add a field to `FilterState` (line 77-83):

```typescript
export interface FilterState {
  sortBy: 'addedAt' | 'releasedAt' | 'title'
  sortOrder: 'asc' | 'desc'
  watchedFilter: 'all' | 'watched' | 'unwatched'
  favoriteOnly: boolean
  tagFilter: TagFilter
  scrapeStatusFilter: ScrapeStatus | 'all'  // NEW
}
```

- [ ] **Step 2: Add selection state to libraryStore**

Replace `src/stores/libraryStore.ts`:

```typescript
import { create } from 'zustand'
import type { Video, FilterState } from '@/types'

interface LibraryStore {
  videos: Video[]
  filters: FilterState
  searchQuery: string
  isScanning: boolean
  // Selection mode
  selectionMode: boolean
  selectedIds: Set<string>
  setVideos: (videos: Video[]) => void
  setFilters: (filters: Partial<FilterState>) => void
  setSearchQuery: (q: string) => void
  setScanning: (v: boolean) => void
  setSelectionMode: (v: boolean) => void
  toggleSelected: (id: string) => void
  selectAll: (ids: string[]) => void
  clearSelection: () => void
}

const DEFAULT_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
  scrapeStatusFilter: 'all',
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  videos: [],
  filters: DEFAULT_FILTERS,
  searchQuery: '',
  isScanning: false,
  selectionMode: false,
  selectedIds: new Set(),
  setVideos: (videos) => set({ videos }),
  setFilters: (partial) =>
    set({ filters: { ...get().filters, ...partial } }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setScanning: (isScanning) => set({ isScanning }),
  setSelectionMode: (selectionMode) =>
    set({ selectionMode, selectedIds: selectionMode ? get().selectedIds : new Set() }),
  toggleSelected: (id) => {
    const next = new Set(get().selectedIds)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    set({ selectedIds: next })
  },
  selectAll: (ids) => set({ selectedIds: new Set(ids) }),
  clearSelection: () => set({ selectedIds: new Set() }),
}))
```

- [ ] **Step 3: Add scrapeStatus filtering to useFilteredVideos**

Read `src/hooks/useFilteredVideos.ts` and add scrapeStatus filtering. Add this filter alongside the existing filters:

```typescript
// Inside the filtering logic, add:
if (filters.scrapeStatusFilter !== 'all') {
  result = result.filter((v) => v.scrapeStatus === filters.scrapeStatusFilter)
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add src/types/index.ts src/stores/libraryStore.ts src/hooks/useFilteredVideos.ts
git commit -m "feat: add scrapeStatusFilter and selection state to store"
```

---

### Task 4: Frontend — Update FilterBar (remove scraping UI, add status filter + selection toggle)

**Files:**
- Modify: `src/components/library/FilterBar.tsx`

- [ ] **Step 1: Simplify FilterBar props**

Remove scraping-related props. Add selection mode props. Replace the props interface (lines 18-28):

```typescript
interface FilterBarProps {
  totalCount: number
  tags: Tag[]
  activeFilter: { type: string; value: string } | null
  onClearFilter: () => void
}
```

- [ ] **Step 2: Add scrapeStatus filter dropdown and selection mode toggle**

Inside the FilterBar component, after the favorites badge and separator, add the scrapeStatus dropdown. Add a selection mode toggle button near the right side (before the total count). Remove the scrape button (lines 176-182) and the inline progress bar (lines 184-199).

```tsx
const SCRAPE_STATUS_LABELS: Record<string, string> = {
  all: '수집 상태: 전체',
  not_scraped: '미수집',
  partial: '부분 수집',
  not_found: '실패',
  complete: '완료',
}

// Inside the component, use useLibraryStore for selectionMode:
const { filters, setFilters, selectionMode, setSelectionMode } = useLibraryStore()

// Add the scrapeStatus dropdown after the favorites badge:
<Select
  value={filters.scrapeStatusFilter}
  onValueChange={(v) => setFilters({ scrapeStatusFilter: v as FilterState['scrapeStatusFilter'] })}
>
  <SelectTrigger className="w-28 h-7 text-xs bg-secondary border-border">
    <SelectValue>{SCRAPE_STATUS_LABELS[filters.scrapeStatusFilter]}</SelectValue>
  </SelectTrigger>
  <SelectContent>
    <SelectItem value="all">전체</SelectItem>
    <SelectItem value="not_scraped">미수집</SelectItem>
    <SelectItem value="partial">부분 수집</SelectItem>
    <SelectItem value="not_found">실패</SelectItem>
    <SelectItem value="complete">완료</SelectItem>
  </SelectContent>
</Select>

// Add selection mode toggle near the right side, before totalCount:
<Button
  variant={selectionMode ? 'default' : 'outline'}
  size="sm"
  className="h-7 text-xs shrink-0"
  onClick={() => setSelectionMode(!selectionMode)}
>
  {selectionMode ? '선택 해제' : '☑ 선택'}
</Button>
```

- [ ] **Step 3: Update FilterBar import to include FilterState type**

Add `FilterState` to the type imports if needed for the `scrapeStatusFilter` cast.

- [ ] **Step 4: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/library/FilterBar.tsx
git commit -m "feat: replace scrape button with status filter + selection toggle in FilterBar"
```

---

### Task 5: Frontend — Update VideoCard with checkbox overlay

**Files:**
- Modify: `src/components/library/VideoCard.tsx`

- [ ] **Step 1: Add selection props to VideoCard**

Update the props interface and add checkbox rendering:

```typescript
interface VideoCardProps {
  video: Video
  onClick: (video: Video) => void
  selectionMode: boolean
  selected: boolean
  onToggleSelect: (id: string) => void
}
```

- [ ] **Step 2: Add checkbox overlay and modify click behavior**

Update the component to show a checkbox in selection mode and change click behavior:

```tsx
export default function VideoCard({ video, onClick, selectionMode, selected, onToggleSelect }: VideoCardProps) {
  const [hovered, setHovered] = useState(false)

  const handleClick = () => {
    if (selectionMode) {
      onToggleSelect(video.id)
    } else {
      onClick(video)
    }
  }

  return (
    <button
      className={cn(
        'group relative w-full text-left rounded-md bg-card border transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        selected ? 'border-primary shadow-lg z-10' : hovered ? 'border-primary/50 shadow-lg z-10' : 'border-border z-0'
      )}
      onClick={handleClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* 체크박스 오버레이 — 셀렉션 모드일 때만 */}
      {selectionMode && (
        <div className={cn(
          'absolute top-1.5 left-1.5 z-[4] w-5 h-5 rounded border-2 flex items-center justify-center text-xs',
          selected
            ? 'bg-primary border-primary text-primary-foreground'
            : 'border-muted-foreground/50 bg-black/30'
        )}>
          {selected && '✓'}
        </div>
      )}

      {/* ... rest of component unchanged ... */}
    </button>
  )
}
```

Note: When `selectionMode` is true, the checkbox replaces the code badge position (top-left). The code badge (`top-1.5 left-1.5 z-[2]`) should be shifted right when in selection mode. Add a conditional class:

```tsx
<Badge
  className={cn(
    'absolute top-1.5 z-[2] bg-primary text-primary-foreground text-[10px] px-1.5 py-0.5 font-mono font-bold',
    selectionMode ? 'left-8' : 'left-1.5'
  )}
  variant="default"
>
  {video.code}
</Badge>
```

The hover play overlay should be suppressed in selection mode:

```tsx
{!selectionMode && (
  <div
    className={cn(
      'absolute inset-0 z-[3] bg-black/60 flex items-center justify-center transition-opacity',
      hovered ? 'opacity-100' : 'opacity-0'
    )}
  >
    <Play className="w-10 h-10 text-white" />
  </div>
)}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/library/VideoCard.tsx
git commit -m "feat: add checkbox overlay to VideoCard for selection mode"
```

---

### Task 6: Frontend — Update VideoGrid to pass selection props

**Files:**
- Modify: `src/components/library/VideoGrid.tsx`

- [ ] **Step 1: Update VideoGrid to pass selection props to VideoCard**

```tsx
import VideoCard from './VideoCard'
import type { Video } from '@/types'
import { useLibraryStore } from '@/stores/libraryStore'

interface VideoGridProps {
  videos: Video[]
  onSelect: (video: Video) => void
}

export default function VideoGrid({ videos, onSelect }: VideoGridProps) {
  const { selectionMode, selectedIds, toggleSelected } = useLibraryStore()

  if (videos.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground text-sm">
        영상이 없습니다
      </div>
    )
  }

  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))' }}
    >
      {videos.map((video) => (
        <VideoCard
          key={video.id}
          video={video}
          onClick={onSelect}
          selectionMode={selectionMode}
          selected={selectedIds.has(video.id)}
          onToggleSelect={toggleSelected}
        />
      ))}
    </div>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/library/VideoGrid.tsx
git commit -m "feat: pass selection props from VideoGrid to VideoCard"
```

---

### Task 7: Frontend — Create FloatingActionBar component

**Files:**
- Create: `src/components/library/FloatingActionBar.tsx`

- [ ] **Step 1: Create the FloatingActionBar component**

```tsx
import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { X } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video } from '@/types'

interface ScrapeProgress {
  current: number
  total: number
  success: number
  fail: number
}

type ActionBarMode = 'selection' | 'progress' | 'result'

export default function FloatingActionBar({ filteredIds }: { filteredIds: string[] }) {
  const { selectedIds, clearSelection, setSelectionMode, selectAll } = useLibraryStore()
  const { run } = useTauriCommand()
  const [mode, setMode] = useState<ActionBarMode>('selection')
  const [progress, setProgress] = useState<ScrapeProgress>({ current: 0, total: 0, success: 0, fail: 0 })

  // Listen for scrape events
  useEffect(() => {
    if (mode !== 'progress') return
    let unlisten: (() => void) | undefined
    let cancelled = false

    async function setup() {
      const { listen } = await import('@tauri-apps/api/event')
      if (cancelled) return

      const u1 = await listen<{ current: number; total: number; status: string; video?: Video }>(
        'scrape-progress',
        (e) => {
          const isSuccess = e.payload.status === 'complete' || e.payload.status === 'partial'
          setProgress((prev) => ({
            current: e.payload.current,
            total: e.payload.total,
            success: prev.success + (isSuccess ? 1 : 0),
            fail: prev.fail + (isSuccess ? 0 : 1),
          }))
          // Update video in store
          if (e.payload.video) {
            const videos = useLibraryStore.getState().videos
            useLibraryStore.getState().setVideos(
              videos.map((v) => v.id === e.payload.video!.id ? e.payload.video! : v)
            )
          }
        }
      )
      if (cancelled) { u1(); return }

      const u2 = await listen('scrape-complete', () => {
        setMode('result')
        // Refresh all videos and tags
        run<Video[]>('get_videos', {}, []).then((vids) => {
          useLibraryStore.getState().setVideos(vids)
        })
      })
      if (cancelled) { u1(); u2(); return }

      unlisten = () => { u1(); u2() }
    }
    setup()
    return () => { cancelled = true; unlisten?.() }
  }, [mode, run])

  // Auto-dismiss result after 5 seconds
  useEffect(() => {
    if (mode !== 'result') return
    const timer = setTimeout(() => {
      setMode('selection')
      setProgress({ current: 0, total: 0, success: 0, fail: 0 })
    }, 5000)
    return () => clearTimeout(timer)
  }, [mode])

  const handleScrape = async () => {
    const ids = [...selectedIds]
    setMode('progress')
    setProgress({ current: 0, total: ids.length, success: 0, fail: 0 })
    setSelectionMode(false)
    try {
      await run('scrape_videos', { videoIds: ids }, undefined)
    } catch {
      setMode('selection')
    }
  }

  const handleReset = async () => {
    const ids = [...selectedIds]
    await run('reset_scrape_status', { videoIds: ids }, undefined)
    // Refresh videos after reset
    const videos = await run<Video[]>('get_videos', {}, [])
    useLibraryStore.getState().setVideos(videos)
    clearSelection()
  }

  const handleCancel = async () => {
    await run('cancel_scrape', {}, undefined)
  }

  const handleSelectAll = () => {
    selectAll(filteredIds)
  }

  // Progress mode
  if (mode === 'progress') {
    const pct = progress.total > 0 ? (progress.current / progress.total) * 100 : 0
    return (
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-card border border-border rounded-lg shadow-xl px-4 py-3 flex items-center gap-3 z-50 min-w-[400px]">
        <span className="text-sm font-semibold text-primary whitespace-nowrap">수집 중...</span>
        <div className="flex-1 h-2 bg-secondary rounded-full overflow-hidden">
          <div
            className="h-full bg-primary transition-all"
            style={{ width: `${pct}%` }}
          />
        </div>
        <span className="text-xs text-green-400 whitespace-nowrap">✓ {progress.success}</span>
        {progress.fail > 0 && (
          <span className="text-xs text-red-400 whitespace-nowrap">✕ {progress.fail}</span>
        )}
        <span className="text-xs text-muted-foreground whitespace-nowrap">/ {progress.total}</span>
        <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={handleCancel}>
          <X className="w-3.5 h-3.5" />
        </Button>
      </div>
    )
  }

  // Result mode
  if (mode === 'result') {
    return (
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-green-950 border border-green-800 rounded-lg shadow-xl px-4 py-3 flex items-center gap-3 z-50 min-w-[300px]">
        <span className="text-sm font-semibold text-green-400">수집 완료</span>
        <span className="text-xs text-green-400">✓ 성공 {progress.success}</span>
        {progress.fail > 0 && (
          <span className="text-xs text-red-400">✕ 실패 {progress.fail}</span>
        )}
      </div>
    )
  }

  // Selection mode — only show when items are selected
  if (selectedIds.size === 0) return null

  return (
    <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-card border border-border rounded-lg shadow-xl px-4 py-3 flex items-center gap-3 z-50">
      <span className="text-sm font-semibold text-primary whitespace-nowrap">
        {selectedIds.size}개 선택됨
      </span>
      <div className="w-px h-4 bg-border" />
      <Button variant="outline" size="sm" className="h-7 text-xs" onClick={handleScrape}>
        재수집
      </Button>
      <Button variant="outline" size="sm" className="h-7 text-xs text-red-400 hover:text-red-300" onClick={handleReset}>
        상태 초기화
      </Button>
      <div className="w-px h-4 bg-border" />
      <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={handleSelectAll}>
        전체 선택
      </Button>
    </div>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/library/FloatingActionBar.tsx
git commit -m "feat: create FloatingActionBar component for batch scrape actions"
```

---

### Task 8: Frontend — Update LibraryPage to use new components

**Files:**
- Modify: `src/pages/LibraryPage.tsx`

- [ ] **Step 1: Remove old scraping state and add FloatingActionBar**

Replace `src/pages/LibraryPage.tsx`:

```tsx
import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import FilterBar from '@/components/library/FilterBar'
import VideoGrid from '@/components/library/VideoGrid'
import FloatingActionBar from '@/components/library/FloatingActionBar'
import VideoDetail from '@/components/detail/VideoDetail'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { useFilteredVideos } from '@/hooks/useFilteredVideos'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video, Tag } from '@/types'

export default function LibraryPage() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery, setVideos, selectionMode } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const { run } = useTauriCommand()
  const [searchParams, setSearchParams] = useSearchParams()
  const [allTags, setAllTags] = useState<Tag[]>([])

  const activeFilter = useMemo(() => {
    if (searchParams.get('actor')) return { type: '배우', value: searchParams.get('actor')! }
    if (searchParams.get('series')) return { type: '시리즈', value: searchParams.get('series')! }
    if (searchParams.get('maker')) return { type: '제작사', value: searchParams.get('maker')! }
    if (searchParams.get('tag')) return { type: '태그', value: searchParams.get('tag')! }
    return null
  }, [searchParams])

  const clearFilter = () => setSearchParams({})

  const filtered = useFilteredVideos(videos, filters, searchQuery, activeFilter)
  const filteredIds = useMemo(() => filtered.map((v) => v.id), [filtered])

  const videoCount = videos.length
  useEffect(() => {
    run<Tag[]>('get_tags', {}, []).then(setAllTags)
  }, [run, videoCount])

  useEffect(() => {
    if (id) {
      const found = videos.find((v) => v.id === id)
      if (found) setCurrentVideo(found)
    } else {
      setCurrentVideo(null)
    }
  }, [id, videos, setCurrentVideo])

  const handleSelect = (video: Video) => navigate(`/library/${video.id}`)
  const handleClose = () => navigate('/library')

  if (currentVideo) {
    return <VideoDetail video={currentVideo} onClose={handleClose} />
  }

  return (
    <div className="flex flex-col h-full">
      <FilterBar
        totalCount={filtered.length}
        tags={allTags}
        activeFilter={activeFilter}
        onClearFilter={clearFilter}
      />
      <div className="flex-1 overflow-auto">
        <VideoGrid videos={filtered} onSelect={handleSelect} />
      </div>
      {selectionMode && <FloatingActionBar filteredIds={filteredIds} />}
    </div>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/pages/LibraryPage.tsx
git commit -m "feat: integrate FloatingActionBar, remove old scraping state from LibraryPage"
```

---

### Task 9: Frontend — Update VideoDetail to always show scrape button

**Files:**
- Modify: `src/components/detail/VideoDetail.tsx`

- [ ] **Step 1: Remove scrapeStatus condition from scrape button and add status badge**

Replace the conditional block (lines 195-205) with a button that always shows, plus a status badge:

```tsx
{/* 상태 뱃지 */}
{video.scrapeStatus !== 'not_scraped' && (
  <Badge
    variant="outline"
    className={cn(
      'text-xs',
      video.scrapeStatus === 'complete' && 'border-green-600 text-green-400',
      video.scrapeStatus === 'partial' && 'border-orange-600 text-orange-400',
      video.scrapeStatus === 'not_found' && 'border-red-600 text-red-400',
    )}
  >
    {video.scrapeStatus === 'complete' ? '수집 완료' : video.scrapeStatus === 'partial' ? '부분 수집' : '실패'}
  </Badge>
)}
<Button
  variant="outline"
  size="sm"
  onClick={handleScrape}
  disabled={isScraping}
>
  <Download className={`w-4 h-4 mr-1 ${isScraping ? 'animate-spin' : ''}`} />
  {isScraping ? '수집 중...' : video.scrapeStatus === 'not_scraped' ? '메타데이터 수집' : '재수집'}
</Button>
```

Make sure `Badge` and `cn` are imported at the top of the file. Add `import { cn } from '@/lib/utils'` and `import { Badge } from '@/components/ui/badge'` if not already present.

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/VideoDetail.tsx
git commit -m "feat: always show scrape button in VideoDetail, label changes based on status"
```

---

### Task 10: Integration test — full flow verification

**Files:** (no new files)

- [ ] **Step 1: Build the full app**

Run: `cd src-tauri && cargo build`
Expected: compiles successfully.

- [ ] **Step 2: Verify frontend builds**

Run: `npx tsc --noEmit && npx vite build`
Expected: No errors.

- [ ] **Step 3: Manual verification checklist**

Run `cargo tauri dev` and verify:
1. FilterBar shows scrapeStatus dropdown (전체/미수집/부분 수집/실패/완료)
2. FilterBar shows "선택" toggle button
3. Clicking "선택" enables selection mode — cards show checkboxes
4. Clicking a card in selection mode toggles its checkbox (does NOT navigate)
5. Clicking a card outside selection mode navigates to detail (unchanged)
6. Selecting cards shows FloatingActionBar at bottom with "재수집", "상태 초기화", "전체 선택"
7. "전체 선택" selects all currently filtered videos
8. "상태 초기화" resets selected videos to NotScraped status
9. "재수집" starts scraping — action bar shows progress with success/fail counts
10. Cancel button stops scraping mid-progress
11. Completion shows result summary for 5 seconds then dismisses
12. VideoDetail always shows scrape button, label changes to "재수집" for already-scraped videos
13. "선택 해제" exits selection mode and clears selection

- [ ] **Step 4: Commit any fixes found during testing**

```bash
git add -A
git commit -m "fix: integration fixes from manual testing"
```
