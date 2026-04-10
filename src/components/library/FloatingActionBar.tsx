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

export default function FloatingActionBar({ filteredVideos }: { filteredVideos: Video[] }) {
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
    try {
      await run('reset_scrape_status', { videoIds: ids }, undefined)
      const videos = await run<Video[]>('get_videos', {}, [])
      useLibraryStore.getState().setVideos(videos)
      clearSelection()
    } catch {
      // keep selection on failure
    }
  }

  const handleCancel = async () => {
    await run('cancel_scrape', {}, undefined)
  }

  const filteredIds = filteredVideos.map((v) => v.id)

  const handleSelectAll = () => {
    selectAll(filteredIds)
  }

  const unscrapedIds = filteredVideos
    .filter((v) => v.scrapeStatus !== 'complete')
    .map((v) => v.id)

  const handleSelectUnscraped = () => {
    selectAll(unscrapedIds)
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
      {unscrapedIds.length > 0 && (
        <Button variant="ghost" size="sm" className="h-7 text-xs text-orange-400 hover:text-orange-300" onClick={handleSelectUnscraped}>
          미수집 선택
        </Button>
      )}
    </div>
  )
}
