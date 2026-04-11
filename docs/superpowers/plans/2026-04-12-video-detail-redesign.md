# VideoDetail Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign VideoDetail into a two-mode page (Info Mode + Cinema Mode) with custom video player, improved image lightbox, multi-part file support, and folder-open command.

**Architecture:** Same route (`/library/:id`) renders VideoDetail, which toggles between Info Mode (metadata-focused) and Cinema Mode (full player) via local state. Sub-components are small, focused files under `src/components/detail/`. A single new Rust command `open_folder` is added.

**Tech Stack:** React 19, TypeScript, Tailwind CSS, Zustand, Tauri v2, HTML5 `<video>` API, lucide-react icons

---

## File Structure

```
src/components/detail/
├── VideoDetail.tsx          — REWRITE: orchestrates Info/Cinema modes
├── CoverImage.tsx           — NEW: blur+contain cover with click-to-enlarge
├── CoverOverlay.tsx         — NEW: full-size cover overlay
├── VideoMetadata.tsx        — NEW: metadata display + action buttons
├── FilePartsList.tsx        — NEW: multi-part file list with play buttons
├── SampleImageGrid.tsx      — NEW: 5-column image grid
├── ImageLightbox.tsx        — NEW: navigable lightbox viewer
├── MiniPreview.tsx          — NEW: muted autoplay preview
├── CinemaPlayer.tsx         — NEW: full custom player with part support
├── PlayerControls.tsx       — NEW: seek, volume, speed, fullscreen controls
├── PartSelector.tsx         — NEW: part tabs for multi-file videos
├── InAppPlayer.tsx          — DELETE after Task 11

src-tauri/src/
├── lib.rs                   — MODIFY: add open_folder command
```

---

### Task 1: Backend `open_folder` Command

**Files:**
- Modify: `src-tauri/src/lib.rs`

This adds a Tauri command that opens the system file explorer at a video file's parent directory.

- [ ] **Step 1: Add the `open_folder` command**

In `src-tauri/src/lib.rs`, add the command function. Place it near the existing `open_with_player` command:

```rust
#[tauri::command]
fn open_folder(file_path: String) -> Result<(), String> {
    tracing::info!("cmd: open_folder path={}", file_path);
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&file_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

- [ ] **Step 2: Register the command in the invoke handler**

In the same file, find the `.invoke_handler(tauri::generate_handler![...])` call and add `open_folder` to the list:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    open_folder,
])
```

- [ ] **Step 3: Verify**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors, no warnings

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add open_folder command to open file explorer at video location"
```

---

### Task 2: CoverImage Component

**Files:**
- Create: `src/components/detail/CoverImage.tsx`

Displays the video cover with the same blur+contain pattern as VideoCard. Clicking opens the CoverOverlay (Task 3).

- [ ] **Step 1: Create CoverImage.tsx**

```tsx
import { Play } from 'lucide-react'
import { assetUrl } from '@/lib/utils'

interface CoverImageProps {
  thumbnailPath: string | null
  code: string
  onClick: () => void
}

export default function CoverImage({ thumbnailPath, code, onClick }: CoverImageProps) {
  return (
    <div className="w-[130px] shrink-0">
      <button
        onClick={onClick}
        className="w-full aspect-[2/3] bg-muted rounded-md overflow-hidden relative block"
      >
        {thumbnailPath ? (
          <>
            <img
              src={assetUrl(thumbnailPath)}
              alt=""
              aria-hidden
              className="absolute inset-0 w-full h-full object-cover blur-xl scale-110 opacity-50"
            />
            <img
              src={assetUrl(thumbnailPath)}
              alt={code}
              className="relative w-full h-full object-contain z-[1]"
            />
          </>
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-secondary">
            <Play className="w-8 h-8 text-muted-foreground/30" />
          </div>
        )}
      </button>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/CoverImage.tsx
git commit -m "feat: add CoverImage component with blur+contain display"
```

---

### Task 3: CoverOverlay Component

**Files:**
- Create: `src/components/detail/CoverOverlay.tsx`

Full-size cover overlay that appears when cover is clicked. Closes on background click, X button, or ESC.

- [ ] **Step 1: Create CoverOverlay.tsx**

```tsx
import { useEffect } from 'react'
import { X } from 'lucide-react'
import { assetUrl } from '@/lib/utils'

interface CoverOverlayProps {
  thumbnailPath: string
  onClose: () => void
}

export default function CoverOverlay({ thumbnailPath, onClose }: CoverOverlayProps) {
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose])

  return (
    <div
      className="fixed inset-0 bg-black/85 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-muted-foreground hover:text-foreground z-[51]"
      >
        <X className="w-6 h-6" />
      </button>
      <img
        src={assetUrl(thumbnailPath)}
        alt="Cover"
        className="max-w-[90vw] max-h-[90vh] object-contain"
        onClick={(e) => e.stopPropagation()}
      />
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/CoverOverlay.tsx
git commit -m "feat: add CoverOverlay for full-size cover image viewing"
```

---

### Task 4: ImageLightbox Component

**Files:**
- Create: `src/components/detail/ImageLightbox.tsx`

Navigable lightbox for sample images with arrow buttons, keyboard nav, thumbnail strip, and counter.

- [ ] **Step 1: Create ImageLightbox.tsx**

```tsx
import { useEffect, useRef, useCallback } from 'react'
import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { assetUrl } from '@/lib/utils'
import type { SampleImage } from '@/types'

interface ImageLightboxProps {
  images: SampleImage[]
  currentIndex: number
  onIndexChange: (index: number) => void
  onClose: () => void
}

export default function ImageLightbox({ images, currentIndex, onIndexChange, onClose }: ImageLightboxProps) {
  const stripRef = useRef<HTMLDivElement>(null)

  const goPrev = useCallback(() => {
    if (currentIndex > 0) onIndexChange(currentIndex - 1)
  }, [currentIndex, onIndexChange])

  const goNext = useCallback(() => {
    if (currentIndex < images.length - 1) onIndexChange(currentIndex + 1)
  }, [currentIndex, images.length, onIndexChange])

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
      else if (e.key === 'ArrowLeft') goPrev()
      else if (e.key === 'ArrowRight') goNext()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose, goPrev, goNext])

  // Auto-scroll thumbnail strip to keep current image visible
  useEffect(() => {
    const strip = stripRef.current
    if (!strip) return
    const thumb = strip.children[currentIndex] as HTMLElement | undefined
    if (thumb) {
      thumb.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' })
    }
  }, [currentIndex])

  if (currentIndex < 0 || currentIndex >= images.length) return null

  return (
    <div
      className="fixed inset-0 bg-black/90 flex flex-col items-center justify-center z-50"
      onClick={onClose}
    >
      {/* Close button */}
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-muted-foreground hover:text-foreground z-[51]"
      >
        <X className="w-6 h-6" />
      </button>

      {/* Counter */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 text-muted-foreground text-sm">
        {currentIndex + 1} / {images.length}
      </div>

      {/* Prev button */}
      {currentIndex > 0 && (
        <button
          onClick={(e) => { e.stopPropagation(); goPrev() }}
          className="absolute left-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white transition-colors"
        >
          <ChevronLeft className="w-5 h-5" />
        </button>
      )}

      {/* Next button */}
      {currentIndex < images.length - 1 && (
        <button
          onClick={(e) => { e.stopPropagation(); goNext() }}
          className="absolute right-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white transition-colors"
        >
          <ChevronRight className="w-5 h-5" />
        </button>
      )}

      {/* Main image */}
      <img
        src={assetUrl(images[currentIndex].path)}
        alt={`Sample ${currentIndex + 1}`}
        className="max-w-[90vw] max-h-[75vh] object-contain"
        onClick={(e) => e.stopPropagation()}
      />

      {/* Thumbnail strip */}
      <div
        ref={stripRef}
        className="flex gap-1.5 mt-4 overflow-x-auto max-w-[90vw] px-2 pb-1"
        onClick={(e) => e.stopPropagation()}
      >
        {images.map((img, idx) => (
          <button
            key={img.id}
            onClick={() => onIndexChange(idx)}
            className={`shrink-0 w-12 h-8 rounded overflow-hidden border-2 transition-colors ${
              idx === currentIndex ? 'border-primary' : 'border-transparent opacity-60 hover:opacity-100'
            }`}
          >
            <img
              src={assetUrl(img.path)}
              alt={`Thumb ${idx + 1}`}
              className="w-full h-full object-cover"
            />
          </button>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/ImageLightbox.tsx
git commit -m "feat: add ImageLightbox with arrow navigation and thumbnail strip"
```

---

### Task 5: SampleImageGrid Component

**Files:**
- Create: `src/components/detail/SampleImageGrid.tsx`

5-column grid of sample images. Clicking an image opens the ImageLightbox at that index.

- [ ] **Step 1: Create SampleImageGrid.tsx**

```tsx
import { useState } from 'react'
import ImageLightbox from './ImageLightbox'
import { assetUrl } from '@/lib/utils'
import type { SampleImage } from '@/types'

interface SampleImageGridProps {
  images: SampleImage[]
}

export default function SampleImageGrid({ images }: SampleImageGridProps) {
  const [lightboxIdx, setLightboxIdx] = useState<number | null>(null)

  if (images.length === 0) return null

  return (
    <>
      <div className="space-y-2">
        <div className="flex justify-between items-center">
          <span className="text-sm text-foreground">샘플 이미지</span>
          <span className="text-xs text-muted-foreground">{images.length}장</span>
        </div>
        <div className="grid grid-cols-5 gap-1.5">
          {images.map((img, idx) => (
            <button
              key={img.id}
              onClick={() => setLightboxIdx(idx)}
              className="aspect-video rounded overflow-hidden border border-border hover:border-primary/50 transition-colors"
            >
              <img
                src={assetUrl(img.path)}
                alt={`Sample ${idx + 1}`}
                className="w-full h-full object-cover"
                loading="lazy"
              />
            </button>
          ))}
        </div>
      </div>

      {lightboxIdx !== null && (
        <ImageLightbox
          images={images}
          currentIndex={lightboxIdx}
          onIndexChange={setLightboxIdx}
          onClose={() => setLightboxIdx(null)}
        />
      )}
    </>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/SampleImageGrid.tsx
git commit -m "feat: add SampleImageGrid with 5-column layout and lightbox integration"
```

---

### Task 6: FilePartsList Component

**Files:**
- Create: `src/components/detail/FilePartsList.tsx`

Displays video file parts with per-part Cinema and External play buttons.

- [ ] **Step 1: Create FilePartsList.tsx**

```tsx
import { Play, Monitor } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { VideoFile } from '@/types'

interface FilePartsListProps {
  files: VideoFile[]
  onPlayCinema: (fileIndex: number) => void
  onPlayExternal: (filePath: string) => void
}

function formatSize(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

export default function FilePartsList({ files, onPlayCinema, onPlayExternal }: FilePartsListProps) {
  if (files.length === 0) return null

  const totalSize = files.reduce((sum, f) => sum + f.size, 0)

  return (
    <div className="bg-card border border-border rounded-lg p-3">
      <div className="flex justify-between items-center mb-2">
        <span className="text-sm text-foreground">파일</span>
        <span className="text-xs text-muted-foreground">
          {files.length > 1 ? `${files.length}파트 · ` : ''}{formatSize(totalSize)}
        </span>
      </div>
      <div className="space-y-1.5">
        {files.map((file, idx) => (
          <div
            key={file.path}
            className="flex items-center gap-3 px-2.5 py-2 bg-secondary/50 rounded-md"
          >
            {files.length > 1 && (
              <span className="text-primary font-bold text-sm w-4 text-center">{idx + 1}</span>
            )}
            <div className="flex-1 min-w-0">
              <p className="text-sm text-foreground truncate">{fileName(file.path)}</p>
              <p className="text-xs text-muted-foreground">{formatSize(file.size)}</p>
            </div>
            <Button size="xs" onClick={() => onPlayCinema(idx)}>
              <Play className="w-3 h-3 mr-1" />
              Cinema
            </Button>
            <Button size="xs" variant="outline" onClick={() => onPlayExternal(file.path)}>
              <Monitor className="w-3 h-3 mr-1" />
              External
            </Button>
          </div>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors. If `size="xs"` is not defined on Button, check `src/components/ui/button.tsx` for available sizes and use the smallest available (likely `sm`). Adjust accordingly.

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/FilePartsList.tsx
git commit -m "feat: add FilePartsList with per-part cinema and external play buttons"
```

---

### Task 7: VideoMetadata Component

**Files:**
- Create: `src/components/detail/VideoMetadata.tsx`

Extracted metadata display from current VideoDetail: code badge, title, actors, series, maker, dates, tags, and action buttons (favorite, open folder, re-scrape).

- [ ] **Step 1: Create VideoMetadata.tsx**

```tsx
import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { Star, FolderOpen, Download, User } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import type { Video, Actor } from '@/types'
import { assetUrl, cn } from '@/lib/utils'

interface VideoMetadataProps {
  video: Video
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return h > 0 ? `${h}시간 ${m}분` : `${m}분`
}

export default function VideoMetadata({ video }: VideoMetadataProps) {
  const { run } = useTauriCommand()
  const { videos, setVideos } = useLibraryStore()
  const navigate = useNavigate()
  const [actorDetails, setActorDetails] = useState<Actor[]>([])
  const [isScraping, setIsScraping] = useState(false)

  useEffect(() => {
    run<Actor[]>('get_actors', {}, []).then((all) => {
      setActorDetails(all.filter((a) => video.actors.includes(a.name)))
    })
  }, [video.actors, run])

  const handleToggleFavorite = async () => {
    await run('toggle_favorite', { id: video.id }, undefined)
    setVideos(videos.map((v) => v.id === video.id ? { ...v, favorite: !v.favorite } : v))
  }

  const handleOpenFolder = async () => {
    const filePath = video.files[0]?.path
    if (filePath) {
      await run('open_folder', { filePath }, undefined)
    }
  }

  const handleScrape = async () => {
    setIsScraping(true)
    try {
      const updated = await run<Video>('scrape_video', { videoId: video.id }, undefined)
      if (updated) {
        setVideos(videos.map((v) => v.id === updated.id ? updated : v))
      }
    } finally {
      setIsScraping(false)
    }
  }

  return (
    <div className="flex-1 space-y-3">
      {/* Code + scrape status */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <Badge className="bg-primary text-primary-foreground font-mono">{video.code}</Badge>
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
        </div>
        <h1 className="text-lg font-semibold leading-snug">{video.title}</h1>
      </div>

      {/* Metadata fields */}
      <div className="space-y-1 text-sm text-muted-foreground">
        {/* Actors */}
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
                        <img src={assetUrl(detail.photoPath)} alt={name} className="w-full h-full object-cover" />
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
        {video.series && (
          <p><span className="text-foreground">시리즈</span>: {video.series}</p>
        )}
        {video.makerName && (
          <p>
            <span className="text-foreground">제작사</span>:{' '}
            <button
              onClick={() => navigate(`/library?maker=${encodeURIComponent(video.makerName!)}`)}
              className="hover:text-foreground transition-colors underline"
            >
              {video.makerName}
            </button>
          </p>
        )}
        {video.releasedAt && (
          <p><span className="text-foreground">출시일</span>: {video.releasedAt}</p>
        )}
        <p><span className="text-foreground">재생시간</span>: {video.duration != null ? formatDuration(video.duration) : '-'}</p>
      </div>

      {/* Tags */}
      {video.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {video.tags.map((tag) => (
            <Badge key={tag} variant="secondary" className="text-xs">{tag}</Badge>
          ))}
        </div>
      )}

      <Separator />

      {/* Action buttons */}
      <div className="flex gap-2">
        <Button
          variant={video.favorite ? 'default' : 'outline'}
          size="sm"
          onClick={handleToggleFavorite}
        >
          <Star className={`w-4 h-4 mr-1 ${video.favorite ? 'fill-current' : ''}`} />
          즐겨찾기
        </Button>
        <Button variant="outline" size="sm" onClick={handleOpenFolder}>
          <FolderOpen className="w-4 h-4 mr-1" />
          폴더 열기
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={handleScrape}
          disabled={isScraping}
        >
          <Download className={`w-4 h-4 mr-1 ${isScraping ? 'animate-spin' : ''}`} />
          {isScraping ? '수집 중...' : video.scrapeStatus === 'not_scraped' ? '메타데이터 수집' : '재수집'}
        </Button>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/VideoMetadata.tsx
git commit -m "feat: add VideoMetadata component with metadata display and action buttons"
```

---

### Task 8: MiniPreview Component

**Files:**
- Create: `src/components/detail/MiniPreview.tsx`

Muted auto-playing preview at the bottom of Info Mode. Has a "Cinema Mode" button to enter full player.

- [ ] **Step 1: Create MiniPreview.tsx**

```tsx
import { useRef, useEffect } from 'react'
import { Play, VolumeX } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { assetUrl } from '@/lib/utils'

interface MiniPreviewProps {
  filePath: string | undefined
  onEnterCinema: () => void
}

export default function MiniPreview({ filePath, onEnterCinema }: MiniPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    const video = videoRef.current
    if (!video || !filePath) return
    video.muted = true
    video.play().catch(() => {
      // Autoplay may be blocked, that's fine
    })
  }, [filePath])

  if (!filePath) return null

  const src = assetUrl(filePath)
  if (!src) return null

  return (
    <div className="relative rounded-lg overflow-hidden border border-border">
      <video
        ref={videoRef}
        src={src}
        muted
        loop
        className="w-full aspect-video bg-black"
      />
      <div className="absolute bottom-3 left-3 flex items-center gap-1.5 text-muted-foreground">
        <VolumeX className="w-4 h-4" />
        <span className="text-xs">음소거</span>
      </div>
      <Button
        size="sm"
        className="absolute bottom-3 right-3"
        onClick={onEnterCinema}
      >
        <Play className="w-3.5 h-3.5 mr-1" />
        Cinema Mode
      </Button>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/MiniPreview.tsx
git commit -m "feat: add MiniPreview with muted autoplay and cinema mode button"
```

---

### Task 9: PlayerControls Component

**Files:**
- Create: `src/components/detail/PlayerControls.tsx`

The full video player controls: seek bar, play/pause, skip forward/back, volume, speed, fullscreen, time display. This is the most complex component.

- [ ] **Step 1: Create PlayerControls.tsx**

```tsx
import { useState, useRef, useCallback, useEffect } from 'react'
import { Play, Pause, Volume2, VolumeX, Maximize, Minimize, SkipBack, SkipForward } from 'lucide-react'

interface PlayerControlsProps {
  videoRef: React.RefObject<HTMLVideoElement | null>
  isFullscreen: boolean
  onToggleFullscreen: () => void
  partLabel?: string // e.g., "Part 1/3"
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  return `${m}:${String(s).padStart(2, '0')}`
}

const SPEEDS = [0.5, 1, 1.5, 2]

export default function PlayerControls({ videoRef, isFullscreen, onToggleFullscreen, partLabel }: PlayerControlsProps) {
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [volume, setVolume] = useState(1)
  const [isMuted, setIsMuted] = useState(false)
  const [speedIndex, setSpeedIndex] = useState(1) // 1 = 1x
  const [isSeeking, setIsSeeking] = useState(false)
  const seekBarRef = useRef<HTMLDivElement>(null)

  // Sync state with video element
  useEffect(() => {
    const video = videoRef.current
    if (!video) return

    const onPlay = () => setIsPlaying(true)
    const onPause = () => setIsPlaying(false)
    const onTimeUpdate = () => { if (!isSeeking) setCurrentTime(video.currentTime) }
    const onLoaded = () => setDuration(video.duration)
    const onVolumeChange = () => {
      setVolume(video.volume)
      setIsMuted(video.muted)
    }

    video.addEventListener('play', onPlay)
    video.addEventListener('pause', onPause)
    video.addEventListener('timeupdate', onTimeUpdate)
    video.addEventListener('loadedmetadata', onLoaded)
    video.addEventListener('durationchange', onLoaded)
    video.addEventListener('volumechange', onVolumeChange)

    // Init state from current video
    if (video.duration) setDuration(video.duration)
    setCurrentTime(video.currentTime)
    setVolume(video.volume)
    setIsMuted(video.muted)
    setIsPlaying(!video.paused)

    return () => {
      video.removeEventListener('play', onPlay)
      video.removeEventListener('pause', onPause)
      video.removeEventListener('timeupdate', onTimeUpdate)
      video.removeEventListener('loadedmetadata', onLoaded)
      video.removeEventListener('durationchange', onLoaded)
      video.removeEventListener('volumechange', onVolumeChange)
    }
  }, [videoRef, isSeeking])

  const togglePlay = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    if (video.paused) video.play()
    else video.pause()
  }, [videoRef])

  const skip = useCallback((seconds: number) => {
    const video = videoRef.current
    if (!video) return
    video.currentTime = Math.max(0, Math.min(video.duration, video.currentTime + seconds))
  }, [videoRef])

  const toggleMute = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    video.muted = !video.muted
  }, [videoRef])

  const changeVolume = useCallback((delta: number) => {
    const video = videoRef.current
    if (!video) return
    video.volume = Math.max(0, Math.min(1, video.volume + delta))
    if (video.muted && delta > 0) video.muted = false
  }, [videoRef])

  const cycleSpeed = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    const next = (speedIndex + 1) % SPEEDS.length
    setSpeedIndex(next)
    video.playbackRate = SPEEDS[next]
  }, [videoRef, speedIndex])

  // Seek bar interaction
  const seekTo = useCallback((e: React.MouseEvent<HTMLDivElement> | MouseEvent) => {
    const bar = seekBarRef.current
    const video = videoRef.current
    if (!bar || !video || !duration) return
    const rect = bar.getBoundingClientRect()
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    video.currentTime = ratio * duration
    setCurrentTime(ratio * duration)
  }, [videoRef, duration])

  const handleSeekStart = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    setIsSeeking(true)
    seekTo(e)
    const handleMove = (ev: MouseEvent) => seekTo(ev)
    const handleUp = () => {
      setIsSeeking(false)
      window.removeEventListener('mousemove', handleMove)
      window.removeEventListener('mouseup', handleUp)
    }
    window.addEventListener('mousemove', handleMove)
    window.addEventListener('mouseup', handleUp)
  }, [seekTo])

  // Volume slider interaction
  const volumeBarRef = useRef<HTMLDivElement>(null)
  const setVolumeFromEvent = useCallback((e: React.MouseEvent<HTMLDivElement> | MouseEvent) => {
    const bar = volumeBarRef.current
    const video = videoRef.current
    if (!bar || !video) return
    const rect = bar.getBoundingClientRect()
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    video.volume = ratio
    if (video.muted && ratio > 0) video.muted = false
  }, [videoRef])

  const handleVolumeStart = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    setVolumeFromEvent(e)
    const handleMove = (ev: MouseEvent) => setVolumeFromEvent(ev)
    const handleUp = () => {
      window.removeEventListener('mousemove', handleMove)
      window.removeEventListener('mouseup', handleUp)
    }
    window.addEventListener('mousemove', handleMove)
    window.addEventListener('mouseup', handleUp)
  }, [setVolumeFromEvent])

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0

  return (
    <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/85 to-transparent pt-8 px-4 pb-4">
      {/* Seek bar */}
      <div
        ref={seekBarRef}
        className="h-1 bg-white/20 rounded-full mb-3 cursor-pointer group"
        onMouseDown={handleSeekStart}
      >
        <div
          className="h-full bg-primary rounded-full relative"
          style={{ width: `${progress}%` }}
        >
          <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
        </div>
      </div>

      {/* Controls row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {/* Skip back */}
          <button onClick={() => skip(-10)} className="text-white/80 hover:text-white transition-colors">
            <SkipBack className="w-4 h-4" />
          </button>

          {/* Play/Pause */}
          <button onClick={togglePlay} className="text-white hover:text-white/90 transition-colors">
            {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5" />}
          </button>

          {/* Skip forward */}
          <button onClick={() => skip(10)} className="text-white/80 hover:text-white transition-colors">
            <SkipForward className="w-4 h-4" />
          </button>

          {/* Volume */}
          <button onClick={toggleMute} className="text-white/80 hover:text-white transition-colors">
            {isMuted || volume === 0 ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
          </button>
          <div
            ref={volumeBarRef}
            className="w-16 h-1 bg-white/20 rounded-full cursor-pointer"
            onMouseDown={handleVolumeStart}
          >
            <div
              className="h-full bg-white/80 rounded-full"
              style={{ width: `${isMuted ? 0 : volume * 100}%` }}
            />
          </div>

          {/* Time */}
          <span className="text-white/60 text-xs font-mono">
            {formatTime(currentTime)} / {formatTime(duration)}
          </span>
        </div>

        <div className="flex items-center gap-3">
          {/* Speed */}
          <button onClick={cycleSpeed} className="text-white/60 hover:text-white text-xs font-mono transition-colors">
            {SPEEDS[speedIndex]}x
          </button>

          {/* Part label */}
          {partLabel && (
            <span className="text-white/60 text-xs">{partLabel}</span>
          )}

          {/* Fullscreen */}
          <button onClick={onToggleFullscreen} className="text-white/80 hover:text-white transition-colors">
            {isFullscreen ? <Minimize className="w-4 h-4" /> : <Maximize className="w-4 h-4" />}
          </button>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/PlayerControls.tsx
git commit -m "feat: add PlayerControls with seek, volume, speed, and fullscreen"
```

---

### Task 10: PartSelector Component

**Files:**
- Create: `src/components/detail/PartSelector.tsx`

Part tabs shown in Cinema Mode's top bar. Highlights current part, allows switching.

- [ ] **Step 1: Create PartSelector.tsx**

```tsx
import { cn } from '@/lib/utils'

interface PartSelectorProps {
  totalParts: number
  currentPart: number // 0-based index
  onSelectPart: (index: number) => void
}

export default function PartSelector({ totalParts, currentPart, onSelectPart }: PartSelectorProps) {
  if (totalParts <= 1) return null

  return (
    <div className="flex gap-1">
      {Array.from({ length: totalParts }, (_, i) => (
        <button
          key={i}
          onClick={() => onSelectPart(i)}
          className={cn(
            'text-xs px-3 py-1 rounded transition-colors',
            i === currentPart
              ? 'bg-primary text-primary-foreground'
              : 'bg-white/10 text-white/60 hover:bg-white/20 hover:text-white/80'
          )}
        >
          Part {i + 1}
        </button>
      ))}
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/PartSelector.tsx
git commit -m "feat: add PartSelector tabs for multi-part video navigation"
```

---

### Task 11: CinemaPlayer Component

**Files:**
- Create: `src/components/detail/CinemaPlayer.tsx`

Full-screen player that combines the video element, PlayerControls, and PartSelector. Handles auto-hide of controls, keyboard shortcuts, part auto-advance, and fullscreen.

- [ ] **Step 1: Create CinemaPlayer.tsx**

```tsx
import { useState, useRef, useCallback, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import PlayerControls from './PlayerControls'
import PartSelector from './PartSelector'
import { assetUrl } from '@/lib/utils'
import type { VideoFile } from '@/types'

interface CinemaPlayerProps {
  files: VideoFile[]
  initialPartIndex: number
  videoCode: string
  videoTitle: string
  onExit: () => void
}

export default function CinemaPlayer({ files, initialPartIndex, videoCode, videoTitle, onExit }: CinemaPlayerProps) {
  const [currentPart, setCurrentPart] = useState(initialPartIndex)
  const [showControls, setShowControls] = useState(true)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const videoRef = useRef<HTMLVideoElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const hideTimerRef = useRef<ReturnType<typeof setTimeout>>()

  const currentFile = files[currentPart]
  const src = currentFile ? assetUrl(currentFile.path) : undefined

  // Auto-hide controls after 3 seconds
  const resetHideTimer = useCallback(() => {
    setShowControls(true)
    if (hideTimerRef.current) clearTimeout(hideTimerRef.current)
    hideTimerRef.current = setTimeout(() => setShowControls(false), 3000)
  }, [])

  const handleMouseMove = useCallback(() => {
    resetHideTimer()
  }, [resetHideTimer])

  const handleMouseLeave = useCallback(() => {
    if (hideTimerRef.current) clearTimeout(hideTimerRef.current)
    hideTimerRef.current = setTimeout(() => setShowControls(false), 1000)
  }, [])

  // Show controls initially
  useEffect(() => {
    resetHideTimer()
    return () => {
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current)
    }
  }, [resetHideTimer])

  // Part auto-advance
  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    const handleEnded = () => {
      if (currentPart < files.length - 1) {
        setCurrentPart(currentPart + 1)
      }
    }
    video.addEventListener('ended', handleEnded)
    return () => video.removeEventListener('ended', handleEnded)
  }, [currentPart, files.length])

  // Auto-play when part changes
  useEffect(() => {
    const video = videoRef.current
    if (!video || !src) return
    video.load()
    video.play().catch(() => {})
  }, [src])

  // Fullscreen toggle
  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current
    if (!el) return
    if (document.fullscreenElement) {
      await document.exitFullscreen()
    } else {
      await el.requestFullscreen()
    }
  }, [])

  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement)
    document.addEventListener('fullscreenchange', handler)
    return () => document.removeEventListener('fullscreenchange', handler)
  }, [])

  // Keyboard shortcuts
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      const video = videoRef.current
      if (!video) return

      switch (e.key) {
        case 'Escape':
          if (document.fullscreenElement) document.exitFullscreen()
          else onExit()
          break
        case ' ':
          e.preventDefault()
          if (video.paused) video.play()
          else video.pause()
          break
        case 'ArrowLeft':
          e.preventDefault()
          video.currentTime = Math.max(0, video.currentTime - 10)
          break
        case 'ArrowRight':
          e.preventDefault()
          video.currentTime = Math.min(video.duration, video.currentTime + 10)
          break
        case 'ArrowUp':
          e.preventDefault()
          video.volume = Math.min(1, video.volume + 0.1)
          break
        case 'ArrowDown':
          e.preventDefault()
          video.volume = Math.max(0, video.volume - 0.1)
          break
        case 'f':
        case 'F':
          toggleFullscreen()
          break
        case 'm':
        case 'M':
          video.muted = !video.muted
          break
      }
      resetHideTimer()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onExit, toggleFullscreen, resetHideTimer])

  const partLabel = files.length > 1 ? `Part ${currentPart + 1}/${files.length}` : undefined

  return (
    <div
      ref={containerRef}
      className="fixed inset-0 bg-black z-50 flex items-center justify-center"
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {/* Video element */}
      <video
        ref={videoRef}
        src={src}
        className="w-full h-full object-contain"
        onClick={() => {
          const video = videoRef.current
          if (video) {
            if (video.paused) video.play()
            else video.pause()
          }
        }}
      />

      {/* Top bar */}
      <div
        className={`absolute top-0 left-0 right-0 bg-gradient-to-b from-black/70 to-transparent px-4 py-3 flex items-center justify-between transition-opacity duration-300 ${
          showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
      >
        <div className="flex items-center gap-3">
          <button onClick={onExit} className="text-white/80 hover:text-white flex items-center gap-1 transition-colors">
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm">Back to Info</span>
          </button>
          <span className="text-white/50 text-sm">{videoCode} — {videoTitle}</span>
        </div>
        <PartSelector
          totalParts={files.length}
          currentPart={currentPart}
          onSelectPart={setCurrentPart}
        />
      </div>

      {/* Bottom controls */}
      <div
        className={`transition-opacity duration-300 ${
          showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
      >
        <PlayerControls
          videoRef={videoRef}
          isFullscreen={isFullscreen}
          onToggleFullscreen={toggleFullscreen}
          partLabel={partLabel}
        />
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/CinemaPlayer.tsx
git commit -m "feat: add CinemaPlayer with keyboard shortcuts, part navigation, and auto-hide controls"
```

---

### Task 12: Rewrite VideoDetail

**Files:**
- Rewrite: `src/components/detail/VideoDetail.tsx`

Orchestrates Info Mode and Cinema Mode. Uses all the components created in Tasks 2-11.

- [ ] **Step 1: Rewrite VideoDetail.tsx**

Replace the entire file:

```tsx
import { useState, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import CoverImage from './CoverImage'
import CoverOverlay from './CoverOverlay'
import VideoMetadata from './VideoMetadata'
import FilePartsList from './FilePartsList'
import SampleImageGrid from './SampleImageGrid'
import MiniPreview from './MiniPreview'
import CinemaPlayer from './CinemaPlayer'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import type { Video, SampleImage } from '@/types'

interface VideoDetailProps {
  video: Video
  onClose: () => void
}

export default function VideoDetail({ video, onClose }: VideoDetailProps) {
  const [cinemaPartIndex, setCinemaPartIndex] = useState<number | null>(null)
  const [showCover, setShowCover] = useState(false)
  const [sampleImages, setSampleImages] = useState<SampleImage[]>([])
  const { run } = useTauriCommand()
  const { videos, setVideos } = useLibraryStore()

  useEffect(() => {
    run<SampleImage[]>('get_sample_images', { videoId: video.id }, []).then(setSampleImages)
  }, [video.id, run])

  const handlePlayExternal = async (filePath: string) => {
    await run('open_with_player', { filePath }, undefined)
    setVideos(videos.map((v) => v.id === video.id ? { ...v, watched: true } : v))
  }

  const handleEnterCinema = (fileIndex: number) => {
    setCinemaPartIndex(fileIndex)
  }

  // Cinema Mode
  if (cinemaPartIndex !== null) {
    return (
      <CinemaPlayer
        files={video.files}
        initialPartIndex={cinemaPartIndex}
        videoCode={video.code}
        videoTitle={video.title}
        onExit={() => setCinemaPartIndex(null)}
      />
    )
  }

  // Info Mode
  return (
    <div className="flex flex-col h-full overflow-auto p-6 space-y-6">
      {/* Back button */}
      <Button variant="ghost" size="sm" onClick={onClose} className="w-fit -ml-2">
        <ArrowLeft className="w-4 h-4 mr-1" />
        라이브러리
      </Button>

      {/* Cover + Metadata */}
      <div className="flex gap-6">
        <CoverImage
          thumbnailPath={video.thumbnailPath}
          code={video.code}
          onClick={() => video.thumbnailPath && setShowCover(true)}
        />
        <VideoMetadata video={video} />
      </div>

      {/* File Parts */}
      <FilePartsList
        files={video.files}
        onPlayCinema={handleEnterCinema}
        onPlayExternal={handlePlayExternal}
      />

      {/* Sample Images */}
      <SampleImageGrid images={sampleImages} />

      {/* Mini Preview */}
      <MiniPreview
        filePath={video.files[0]?.path}
        onEnterCinema={() => handleEnterCinema(0)}
      />

      {/* Cover Overlay */}
      {showCover && video.thumbnailPath && (
        <CoverOverlay
          thumbnailPath={video.thumbnailPath}
          onClose={() => setShowCover(false)}
        />
      )}
    </div>
  )
}
```

- [ ] **Step 2: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/VideoDetail.tsx
git commit -m "feat: rewrite VideoDetail with two-mode Info/Cinema architecture"
```

---

### Task 13: Cleanup

**Files:**
- Delete: `src/components/detail/InAppPlayer.tsx`
- Modify: `src/stores/playerStore.ts` (remove `isPreviewOpen` if unused)

Remove the old InAppPlayer component and any dead code.

- [ ] **Step 1: Delete InAppPlayer.tsx**

Delete the file `src/components/detail/InAppPlayer.tsx` — it is no longer imported by VideoDetail.

- [ ] **Step 2: Check for remaining references to InAppPlayer**

Run: `grep -r "InAppPlayer" src/` — should return zero results. If any remain, remove those imports.

- [ ] **Step 3: Check playerStore for dead code**

Read `src/stores/playerStore.ts`. If `isPreviewOpen` and `setPreviewOpen` are no longer used by any component (they were only used by the old VideoDetail's preview toggle), remove them from the store.

Run: `grep -r "isPreviewOpen\|setPreviewOpen" src/` — if only used in `playerStore.ts` definition, remove them.

- [ ] **Step 4: Verify**

Run: `pnpm tsc --noEmit`
Expected: no type errors

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors, no warnings

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove InAppPlayer and clean up dead code"
```

---

## Verification Checklist

After all tasks complete:

1. `cargo check --manifest-path src-tauri/Cargo.toml` — no errors, no warnings
2. `pnpm tsc --noEmit` — no type errors
3. App runs: `pnpm tauri dev`
4. Navigate to any video → Info Mode displays correctly:
   - Cover with blur+contain, click opens overlay
   - Metadata, actors, tags, action buttons all present
   - "폴더 열기" opens file explorer at correct location
   - File parts list shows all files with Cinema/External buttons
   - Sample images in 5-column grid, click opens lightbox
   - Lightbox: arrow nav works, thumbnail strip highlights current, ESC closes
   - Mini preview auto-plays muted at bottom
5. Click "Cinema Mode" or any Cinema button → Cinema Mode:
   - Full-screen player with video playing
   - Seek bar drag and click work
   - Play/Pause, skip 10s, volume slider all work
   - Speed cycles through 0.5x/1x/1.5x/2x
   - Part tabs show for multi-file videos, switching works
   - Auto-advance to next part on end
   - Keyboard shortcuts all work (Space, arrows, F, M, ESC)
   - Controls auto-hide after 3s, reappear on mouse move
   - "Back to Info" returns to Info Mode
6. Multi-part video: each part playable independently via Cinema or External
