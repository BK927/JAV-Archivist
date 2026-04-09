import { useEffect } from 'react'
import { Outlet } from 'react-router-dom'
import TopNav from './TopNav'
import { useLogStore, type LogEntry } from '@/stores/logStore'

export default function AppShell() {
  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        if (cancelled) return

        const u = await listen<LogEntry>('log-event', (e) => {
          useLogStore.getState().addEntry(e.payload)
        })
        if (cancelled) { u(); return }

        unlisten = u
      } catch {
        // Not in Tauri env
      }
    }
    setup()
    return () => { cancelled = true; unlisten?.() }
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
