import { create } from 'zustand'

export interface LogEntry {
  timestamp: string
  level: string
  target: string
  message: string
}

interface LogStore {
  entries: LogEntry[]
  filterLevel: 'ALL' | 'ERROR' | 'WARN' | 'INFO' | 'DEBUG'
  addEntry: (entry: LogEntry) => void
  clear: () => void
  setFilterLevel: (level: LogStore['filterLevel']) => void
}

export const useLogStore = create<LogStore>((set) => ({
  entries: [],
  filterLevel: 'ALL',
  addEntry: (entry: LogEntry) =>
    set((state) => ({
      entries: [...state.entries, entry].slice(-1000),
    })),
  clear: () =>
    set({
      entries: [],
    }),
  setFilterLevel: (level) =>
    set({
      filterLevel: level,
    }),
}))
