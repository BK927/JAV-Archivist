# Cinema Player Feedback & Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add visual action feedback and keyboard/mouse convenience features to CinemaPlayer.

**Architecture:** New `ActionFeedback` component renders transient overlays (play/pause flash, volume/speed indicators). `PlayerControls` gains a seek bar tooltip and seek-delta display. `CinemaPlayer` orchestrates all new state (feedback, speedIndex lifted from PlayerControls, seekDelta) and adds double-click fullscreen, scroll-wheel volume, and `<`/`>` speed keys.

**Tech Stack:** React 19, TypeScript, Tailwind CSS, Lucide icons

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `src/components/detail/ActionFeedback.tsx` | Transient visual feedback overlay |
| Modify | `src/components/detail/PlayerControls.tsx` | Seek tooltip, seek-delta display, speed state lift |
| Modify | `src/components/detail/CinemaPlayer.tsx` | Feedback state, speedIndex lift, double-click, scroll wheel, `<`/`>` keys |

---

### Task 1: Create ActionFeedback component

**Files:**
- Create: `src/components/detail/ActionFeedback.tsx`

- [ ] **Step 1: Create the component file**

```tsx
// src/components/detail/ActionFeedback.tsx
import { useState, useEffect, useRef } from 'react'
import { Play, Pause, Volume2, VolumeX } from 'lucide-react'

export type FeedbackAction =
  | { type: 'play' | 'pause' | 'mute' | 'unmute' }
  | { type: 'volume'; value: number }
  | { type: 'speed'; value: number }

interface ActionFeedbackProps {
  action: FeedbackAction | null
  triggerKey: number
}

const DURATIONS: Record<string, number> = {
  play: 500,
  pause: 500,
  mute: 500,
  unmute: 500,
  volume: 1000,
  speed: 800,
}

export default function ActionFeedback({ action, triggerKey }: ActionFeedbackProps) {
  const [visible, setVisible] = useState(false)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!action) return

    // Show immediately
    setVisible(true)

    // Clear existing timer
    if (timerRef.current) clearTimeout(timerRef.current)

    // Schedule fade out
    timerRef.current = setTimeout(() => {
      setVisible(false)
    }, DURATIONS[action.type] ?? 500)

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [action, triggerKey])

  if (!action) return null

  const isCenterType = action.type === 'play' || action.type === 'pause'
    || action.type === 'mute' || action.type === 'unmute'

  return (
    <div className="pointer-events-none absolute inset-0 z-10">
      {isCenterType && (
        <div
          className={`absolute inset-0 flex items-center justify-center transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="w-16 h-16 bg-black/40 rounded-full flex items-center justify-center">
            {action.type === 'play' && <Play className="w-8 h-8 text-white fill-white" />}
            {action.type === 'pause' && <Pause className="w-8 h-8 text-white fill-white" />}
            {action.type === 'mute' && <VolumeX className="w-8 h-8 text-white" />}
            {action.type === 'unmute' && <Volume2 className="w-8 h-8 text-white" />}
          </div>
        </div>
      )}

      {action.type === 'volume' && (
        <div
          className={`absolute top-12 left-1/2 -translate-x-1/2 transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="flex items-center gap-3 bg-black/70 px-4 py-2 rounded-lg">
            {action.value === 0 ? (
              <VolumeX className="w-5 h-5 text-white" />
            ) : (
              <Volume2 className="w-5 h-5 text-white" />
            )}
            <div className="w-20 h-1 bg-white/20 rounded-full">
              <div
                className="h-full bg-white rounded-full transition-all"
                style={{ width: `${action.value}%` }}
              />
            </div>
            <span className="text-white text-xs font-mono w-8">{Math.round(action.value)}%</span>
          </div>
        </div>
      )}

      {action.type === 'speed' && (
        <div
          className={`absolute top-12 left-1/2 -translate-x-1/2 transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="bg-black/70 px-4 py-2 rounded-lg">
            <span className="text-white text-lg font-bold font-mono">{action.value}x</span>
          </div>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Verify no type errors**

Run: `pnpm tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/ActionFeedback.tsx
git commit -m "feat: add ActionFeedback component for transient visual feedback"
```

---

### Task 2: Lift speedIndex to CinemaPlayer and export SPEEDS

**Files:**
- Modify: `src/components/detail/PlayerControls.tsx`
- Modify: `src/components/detail/CinemaPlayer.tsx`

- [ ] **Step 1: Update PlayerControls interface — accept speedIndex and onSpeedChange as props, export SPEEDS**

In `PlayerControls.tsx`, change:

```tsx
// Export SPEEDS (line 21)
export const SPEEDS = [0.5, 1, 1.5, 2]

// Update interface (line 14-19)
interface PlayerControlsProps {
  videoRef: React.RefObject<HTMLVideoElement | null>
  isFullscreen: boolean
  onToggleFullscreen: () => void
  partLabel?: string
  speedIndex: number
  onSpeedChange: (index: number) => void
}
```

Remove internal `speedIndex` state (line 34):
```tsx
// DELETE: const [speedIndex, setSpeedIndex] = useState(1)
```

Replace `cycleSpeed` (lines 101-108) to use the prop callback:
```tsx
const cycleSpeed = useCallback(() => {
  const next = (speedIndex + 1) % SPEEDS.length
  const video = videoRef.current
  if (video) video.playbackRate = SPEEDS[next]
  onSpeedChange(next)
}, [videoRef, speedIndex, onSpeedChange])
```

Update the destructured props (line 23-28):
```tsx
export default function PlayerControls({
  videoRef,
  isFullscreen,
  onToggleFullscreen,
  partLabel,
  speedIndex,
  onSpeedChange,
}: PlayerControlsProps) {
```

- [ ] **Step 2: Add speedIndex state to CinemaPlayer and pass to PlayerControls**

In `CinemaPlayer.tsx`, add import and state:

```tsx
import { SPEEDS } from './PlayerControls'
```

Add state after existing state declarations (after line 32):
```tsx
const [speedIndex, setSpeedIndex] = useState(1)
```

Update the `<PlayerControls>` JSX (around line 254) to pass new props:
```tsx
<PlayerControls
  videoRef={videoRef}
  isFullscreen={isFullscreen}
  onToggleFullscreen={toggleFullscreen}
  partLabel={partLabel}
  speedIndex={speedIndex}
  onSpeedChange={setSpeedIndex}
/>
```

- [ ] **Step 3: Verify no type errors**

Run: `pnpm tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/components/detail/PlayerControls.tsx src/components/detail/CinemaPlayer.tsx
git commit -m "refactor: lift speedIndex state from PlayerControls to CinemaPlayer"
```

---

### Task 3: Add seek bar tooltip to PlayerControls

**Files:**
- Modify: `src/components/detail/PlayerControls.tsx`

- [ ] **Step 1: Add hover state and tooltip rendering**

Add new state after existing state declarations (after `isSeekingRef`):
```tsx
const [hoverTime, setHoverTime] = useState<{ time: number; left: number } | null>(null)
```

Add a `handleSeekHover` callback and `handleSeekLeave`:
```tsx
const handleSeekHover = useCallback(
  (e: React.MouseEvent) => {
    const bar = seekBarRef.current
    const video = videoRef.current
    if (!bar || !video || !isFinite(video.duration)) return
    const rect = bar.getBoundingClientRect()
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    setHoverTime({ time: ratio * video.duration, left: e.clientX - rect.left })
  },
  [videoRef],
)

const handleSeekLeave = useCallback(() => {
  setHoverTime(null)
}, [])
```

Update the seek bar `<div>` to include hover handlers and the tooltip (the div with `ref={seekBarRef}`):
```tsx
<div
  ref={seekBarRef}
  className="group relative h-1 bg-white/20 rounded-full cursor-pointer mb-3"
  onMouseDown={handleSeekMouseDown}
  onMouseMove={handleSeekHover}
  onMouseLeave={handleSeekLeave}
>
  <div
    className="absolute inset-y-0 left-0 bg-primary rounded-full"
    style={{ width: `${seekPercent}%` }}
  />
  <div
    className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
    style={{ left: `${seekPercent}%` }}
  />
  {/* Hover time tooltip */}
  {hoverTime && (
    <div
      className="absolute -top-8 -translate-x-1/2 bg-black/80 text-white text-xs font-mono px-2 py-1 rounded pointer-events-none"
      style={{ left: hoverTime.left }}
    >
      {formatTime(hoverTime.time)}
    </div>
  )}
</div>
```

- [ ] **Step 2: Verify no type errors**

Run: `pnpm tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/PlayerControls.tsx
git commit -m "feat: add seek bar hover/drag time tooltip"
```

---

### Task 4: Add seek-delta display to PlayerControls

**Files:**
- Modify: `src/components/detail/PlayerControls.tsx`

- [ ] **Step 1: Add seekDelta prop and fade logic**

Update interface:
```tsx
interface PlayerControlsProps {
  videoRef: React.RefObject<HTMLVideoElement | null>
  isFullscreen: boolean
  onToggleFullscreen: () => void
  partLabel?: string
  speedIndex: number
  onSpeedChange: (index: number) => void
  seekDelta: number | null
  seekDeltaKey: number
}
```

Update destructured props:
```tsx
export default function PlayerControls({
  videoRef,
  isFullscreen,
  onToggleFullscreen,
  partLabel,
  speedIndex,
  onSpeedChange,
  seekDelta,
  seekDeltaKey,
}: PlayerControlsProps) {
```

Add seek-delta visibility state and effect after existing state:
```tsx
const [seekDeltaVisible, setSeekDeltaVisible] = useState(false)
const seekDeltaTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

useEffect(() => {
  if (seekDelta === null) return
  setSeekDeltaVisible(true)
  if (seekDeltaTimerRef.current) clearTimeout(seekDeltaTimerRef.current)
  seekDeltaTimerRef.current = setTimeout(() => setSeekDeltaVisible(false), 1000)
  return () => {
    if (seekDeltaTimerRef.current) clearTimeout(seekDeltaTimerRef.current)
  }
}, [seekDelta, seekDeltaKey])
```

Add seek-delta display right after the time display span:
```tsx
{/* Time display */}
<span className="text-white/60 text-xs font-mono ml-2">
  {formatTime(currentTime)} / {formatTime(duration)}
</span>
{seekDelta !== null && (
  <span
    className={`text-primary text-xs font-mono font-bold transition-opacity duration-300 ${
      seekDeltaVisible ? 'opacity-100' : 'opacity-0'
    }`}
  >
    {seekDelta > 0 ? `+${seekDelta}s` : `${seekDelta}s`}
  </span>
)}
```

- [ ] **Step 2: Verify no type errors**

Run: `pnpm tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/PlayerControls.tsx
git commit -m "feat: add seek delta indicator to player controls bar"
```

---

### Task 5: Wire feedback, seekDelta, and new input handlers into CinemaPlayer

**Files:**
- Modify: `src/components/detail/CinemaPlayer.tsx`

- [ ] **Step 1: Add imports and state**

Update imports at top of file:
```tsx
import { useState, useRef, useCallback, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import PlayerControls from './PlayerControls'
import PartSelector from './PartSelector'
import ActionFeedback, { type FeedbackAction } from './ActionFeedback'
import { SPEEDS } from './PlayerControls'
import { assetUrl } from '@/lib/utils'
import type { VideoFile } from '@/types'
```

Add new state after existing state declarations (after `mouseOverControlsRef`):
```tsx
const [speedIndex, setSpeedIndex] = useState(1)
const [feedback, setFeedback] = useState<FeedbackAction | null>(null)
const [feedbackKey, setFeedbackKey] = useState(0)
const [seekDelta, setSeekDelta] = useState<number | null>(null)
const [seekDeltaKey, setSeekDeltaKey] = useState(0)
const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
```

Add a `triggerFeedback` helper after the state declarations:
```tsx
const triggerFeedback = useCallback((action: FeedbackAction) => {
  setFeedback(action)
  setFeedbackKey((k) => k + 1)
}, [])

const triggerSeekDelta = useCallback((delta: number) => {
  setSeekDelta(delta)
  setSeekDeltaKey((k) => k + 1)
}, [])
```

- [ ] **Step 2: Replace handleVideoClick with double-click aware handler**

Replace the existing `handleVideoClick` (lines 87-95) with:
```tsx
const handleVideoClick = useCallback(() => {
  if (clickTimerRef.current) {
    // Second click within 200ms — double click
    clearTimeout(clickTimerRef.current)
    clickTimerRef.current = null
    toggleFullscreen()
    return
  }
  clickTimerRef.current = setTimeout(() => {
    clickTimerRef.current = null
    const video = videoRef.current
    if (!video) return
    if (video.paused) {
      video.play().catch(() => {})
      triggerFeedback({ type: 'play' })
    } else {
      video.pause()
      triggerFeedback({ type: 'pause' })
    }
  }, 200)
}, [toggleFullscreen, triggerFeedback])
```

- [ ] **Step 3: Add scroll wheel handler**

Add after `handleVideoClick`:
```tsx
const handleWheel = useCallback(
  (e: React.WheelEvent) => {
    e.preventDefault()
    const video = videoRef.current
    if (!video) return
    const delta = e.deltaY < 0 ? 0.05 : -0.05
    video.volume = Math.max(0, Math.min(1, video.volume + delta))
    if (video.volume > 0 && video.muted) video.muted = false
    triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
  },
  [triggerFeedback],
)
```

- [ ] **Step 4: Update keyboard handler to add `<`/`>`, seek feedback, play/pause/mute/volume feedback**

Replace the entire `handleKeyDown` inside the keyboard useEffect (lines 148-197) with:
```tsx
const handleKeyDown = (e: KeyboardEvent) => {
  const video = videoRef.current
  if (!video) return

  switch (e.key) {
    case ' ':
      e.preventDefault()
      if (video.paused) {
        video.play().catch(() => {})
        triggerFeedback({ type: 'play' })
      } else {
        video.pause()
        triggerFeedback({ type: 'pause' })
      }
      break
    case 'ArrowLeft':
      e.preventDefault()
      video.currentTime = Math.max(0, video.currentTime - 10)
      triggerSeekDelta(-10)
      break
    case 'ArrowRight':
      e.preventDefault()
      video.currentTime = Math.min(video.duration || 0, video.currentTime + 10)
      triggerSeekDelta(10)
      break
    case 'ArrowUp':
      e.preventDefault()
      video.volume = Math.min(1, video.volume + 0.1)
      if (video.muted) video.muted = false
      triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
      break
    case 'ArrowDown':
      e.preventDefault()
      video.volume = Math.max(0, video.volume - 0.1)
      triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
      break
    case 'f':
    case 'F':
      toggleFullscreen()
      break
    case 'm':
    case 'M':
      video.muted = !video.muted
      triggerFeedback({ type: video.muted ? 'mute' : 'unmute' })
      break
    case ',':
      if (speedIndex > 0) {
        const newIndex = speedIndex - 1
        video.playbackRate = SPEEDS[newIndex]
        setSpeedIndex(newIndex)
        triggerFeedback({ type: 'speed', value: SPEEDS[newIndex] })
      }
      break
    case '.':
      if (speedIndex < SPEEDS.length - 1) {
        const newIndex = speedIndex + 1
        video.playbackRate = SPEEDS[newIndex]
        setSpeedIndex(newIndex)
        triggerFeedback({ type: 'speed', value: SPEEDS[newIndex] })
      }
      break
    case 'Escape':
      if (document.fullscreenElement) {
        document.exitFullscreen()
      } else {
        onExit()
      }
      break
  }
  showControls()
}
```

Update the useEffect dependency array to include the new deps:
```tsx
}, [toggleFullscreen, onExit, showControls, triggerFeedback, triggerSeekDelta, speedIndex])
```

- [ ] **Step 5: Update JSX — add ActionFeedback, onWheel, and new PlayerControls props**

Replace the return JSX with:
```tsx
return (
  <div
    ref={containerRef}
    className="fixed inset-0 z-50 bg-black"
    onMouseMove={handleMouseMove}
    onWheel={handleWheel}
  >
    {/* Video */}
    <video
      ref={videoRef}
      className="w-full h-full object-contain"
      src={assetUrl(files[currentPart].path)}
      onClick={handleVideoClick}
    />

    {/* Action feedback overlay */}
    <ActionFeedback action={feedback} triggerKey={feedbackKey} />

    {/* Top bar */}
    <div
      className={`absolute top-0 left-0 right-0 bg-gradient-to-b from-black/70 to-transparent px-4 py-3 flex items-center gap-3 transition-opacity duration-300 ${
        controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
      }`}
      onMouseEnter={handleControlsMouseEnter}
      onMouseLeave={handleControlsMouseLeave}
    >
      <button
        onClick={onExit}
        className="text-white/80 hover:text-white transition-colors p-1"
      >
        <ArrowLeft className="w-5 h-5" />
      </button>
      <span className="text-white font-medium text-sm">{videoCode}</span>
      <span className="text-white/60 text-sm truncate">{videoTitle}</span>
      <div className="flex-1" />
      <PartSelector
        totalParts={files.length}
        currentPart={currentPart}
        onSelectPart={handleSelectPart}
      />
    </div>

    {/* Bottom controls */}
    <div
      className={`transition-opacity duration-300 ${
        controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
      }`}
      onMouseEnter={handleControlsMouseEnter}
      onMouseLeave={handleControlsMouseLeave}
    >
      <PlayerControls
        videoRef={videoRef}
        isFullscreen={isFullscreen}
        onToggleFullscreen={toggleFullscreen}
        partLabel={partLabel}
        speedIndex={speedIndex}
        onSpeedChange={setSpeedIndex}
        seekDelta={seekDelta}
        seekDeltaKey={seekDeltaKey}
      />
    </div>
  </div>
)
```

- [ ] **Step 6: Clean up clickTimerRef on unmount**

Add to the existing cleanup useEffect (the one that returns `clearHideTimer`), or add a new one after it:
```tsx
useEffect(() => {
  return () => {
    if (clickTimerRef.current) clearTimeout(clickTimerRef.current)
  }
}, [])
```

- [ ] **Step 7: Verify no type errors**

Run: `pnpm tsc --noEmit`
Expected: no errors

- [ ] **Step 8: Manual verification**

Run: `pnpm tauri dev`

Verify each feature:
1. Click video → play/pause icon flashes center screen
2. Double-click video → fullscreen toggles (no play/pause flash)
3. Arrow left/right → "+10s"/"-10s" appears in bottom bar near time
4. Arrow up/down → volume overlay appears top-center
5. M key → mute/unmute icon flashes center
6. `,` and `.` keys → speed overlay appears top-center
7. Scroll wheel → volume overlay appears, volume changes
8. Hover seek bar → time tooltip follows cursor
9. Drag seek bar → time tooltip follows during drag

- [ ] **Step 9: Commit**

```bash
git add src/components/detail/CinemaPlayer.tsx
git commit -m "feat: wire action feedback, double-click fullscreen, scroll volume, speed keys"
```
