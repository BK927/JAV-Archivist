import { useEffect } from 'react'
import { Outlet } from 'react-router-dom'
import TopNav from './TopNav'
import ScrapeProgressBar from '@/components/library/ScrapeProgressBar'
import { useLogStore, type LogEntry } from '@/stores/logStore'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video, Tag } from '@/types'

export default function AppShell() {
  const { setVideos } = useLibraryStore()
  const { run } = useTauriCommand()

  // 앱 시작 시 1회 스캔 + 태그 로드
  useEffect(() => {
    run<Video[]>('scan_library', {}, []).then(setVideos)
    run<Tag[]>('get_tags', {}, []).then((tags) => {
      useLibraryStore.getState().setAllTags(tags)
    })
  }, [run, setVideos])

  // 이벤트 리스너: log-event, library-changed
  useEffect(() => {
    let unlisten: (() => void)[] = []
    let cancelled = false

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        if (cancelled) return

        const u1 = await listen<LogEntry>('log-event', (e) => {
          useLogStore.getState().addEntry(e.payload)
        })
        if (cancelled) { u1(); return }

        const u2 = await listen<Video[]>('library-changed', (e) => {
          useLibraryStore.getState().setVideos(e.payload)
        })
        if (cancelled) { u1(); u2(); return }

        const u3 = await listen<{ current: number; total: number; status: string; video?: Video }>(
          'scrape-progress',
          (e) => {
            let store = useLibraryStore.getState()
            // Auto-enter progress mode on first event (for auto-scrape)
            if (store.scrapeMode === 'idle') {
              store.setScrapeMode('progress')
              store.setScrapeProgress({ current: 0, total: e.payload.total, success: 0, fail: 0 })
              store = useLibraryStore.getState()
            }
            if (store.scrapeMode !== 'progress') return
            const isSuccess = e.payload.status === 'complete' || e.payload.status === 'partial'
            store.updateScrapeProgress((prev) => ({
              current: e.payload.current,
              total: e.payload.total,
              success: prev.success + (isSuccess ? 1 : 0),
              fail: prev.fail + (isSuccess ? 0 : 1),
            }))
            if (e.payload.video) {
              store.setVideos(
                store.videos.map((v) => v.id === e.payload.video!.id ? e.payload.video! : v)
              )
            }
          }
        )
        if (cancelled) { u1(); u2(); u3(); return }

        const u4 = await listen('scrape-complete', () => {
          useLibraryStore.getState().setScrapeMode('result')
          run<Video[]>('get_videos', {}, []).then((vids) => {
            useLibraryStore.getState().setVideos(vids)
          })
          run<Tag[]>('get_tags', {}, []).then((tags) => {
            useLibraryStore.getState().setAllTags(tags)
          })
        })
        if (cancelled) { u1(); u2(); u3(); u4(); return }

        unlisten = [u1, u2, u3, u4]
      } catch {
        // Not in Tauri env
      }
    }
    setup()
    return () => { cancelled = true; unlisten.forEach((u) => u()) }
  }, [])

  return (
    <div className="flex flex-col h-screen bg-background overflow-hidden">
      <TopNav />
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
      <ScrapeProgressBar />
    </div>
  )
}
