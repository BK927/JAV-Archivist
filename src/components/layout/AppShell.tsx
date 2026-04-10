import { useEffect } from 'react'
import { Outlet } from 'react-router-dom'
import TopNav from './TopNav'
import { useLogStore, type LogEntry } from '@/stores/logStore'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video } from '@/types'

export default function AppShell() {
  const { setVideos } = useLibraryStore()
  const { run } = useTauriCommand()

  // 앱 시작 시 1회 스캔
  useEffect(() => {
    run<Video[]>('scan_library', {}, []).then(setVideos)
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

        unlisten = [u1, u2]
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
    </div>
  )
}
