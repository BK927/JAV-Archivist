import { Button } from '@/components/ui/button'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video } from '@/types'

export default function FloatingActionBar({ filteredVideos }: { filteredVideos: Video[] }) {
  const {
    selectedIds, clearSelection, setSelectionMode, selectAll,
    setScrapeMode, setScrapeProgress,
  } = useLibraryStore()
  const { run } = useTauriCommand()

  const handleScrape = async () => {
    const ids = [...selectedIds]
    setScrapeMode('progress')
    setScrapeProgress({ current: 0, total: ids.length, success: 0, fail: 0 })
    setSelectionMode(false)
    try {
      await run('scrape_videos', { videoIds: ids }, undefined)
    } catch {
      setScrapeMode('idle')
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

  // Only show when items are selected
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
