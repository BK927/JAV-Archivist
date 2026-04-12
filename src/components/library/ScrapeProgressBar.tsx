import { useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { X } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'

export default function ScrapeProgressBar() {
  const { scrapeMode, scrapeProgress, setScrapeMode, setScrapeProgress } = useLibraryStore()
  const { run } = useTauriCommand()

  // Auto-dismiss result after 5 seconds
  useEffect(() => {
    if (scrapeMode !== 'result') return
    const timer = setTimeout(() => {
      setScrapeMode('idle')
      setScrapeProgress({ current: 0, total: 0, success: 0, fail: 0 })
    }, 5000)
    return () => clearTimeout(timer)
  }, [scrapeMode, setScrapeMode, setScrapeProgress])

  const handleCancel = async () => {
    await run('cancel_scrape', {}, undefined)
  }

  if (scrapeMode === 'progress') {
    const pct = scrapeProgress.total > 0 ? (scrapeProgress.current / scrapeProgress.total) * 100 : 0
    return (
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-card border border-border rounded-lg shadow-xl px-4 py-3 flex items-center gap-3 z-40 min-w-[400px]">
        <span className="text-sm font-semibold text-primary whitespace-nowrap">수집 중...</span>
        <div className="flex-1 h-2 bg-secondary rounded-full overflow-hidden">
          <div
            className="h-full bg-primary transition-all"
            style={{ width: `${pct}%` }}
          />
        </div>
        <span className="text-xs text-green-400 whitespace-nowrap">✓ {scrapeProgress.success}</span>
        {scrapeProgress.fail > 0 && (
          <span className="text-xs text-red-400 whitespace-nowrap">✕ {scrapeProgress.fail}</span>
        )}
        <span className="text-xs text-muted-foreground whitespace-nowrap">/ {scrapeProgress.total}</span>
        <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={handleCancel}>
          <X className="w-3.5 h-3.5" />
        </Button>
      </div>
    )
  }

  if (scrapeMode === 'result') {
    return (
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-green-950 border border-green-800 rounded-lg shadow-xl px-4 py-3 flex items-center gap-3 z-40 min-w-[300px]">
        <span className="text-sm font-semibold text-green-400">수집 완료</span>
        <span className="text-xs text-green-400">✓ 성공 {scrapeProgress.success}</span>
        {scrapeProgress.fail > 0 && (
          <span className="text-xs text-red-400">✕ 실패 {scrapeProgress.fail}</span>
        )}
      </div>
    )
  }

  return null
}
