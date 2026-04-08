import { useCallback } from 'react'

// Tauri 환경 여부 확인
const isTauri = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core')
    return tauriInvoke<T>(command, args)
  }
  throw new Error(`[Dev] Tauri not available. Command: ${command}`)
}

export function useTauriCommand() {
  const run = useCallback(
    async <T>(
      command: string,
      args?: Record<string, unknown>,
      fallback?: T
    ): Promise<T> => {
      try {
        return await invoke<T>(command, args)
      } catch (err) {
        if (fallback !== undefined) {
          console.warn(`[useTauriCommand] fallback for "${command}":`, err)
          return fallback
        }
        throw err
      }
    },
    []
  )
  return { run }
}
