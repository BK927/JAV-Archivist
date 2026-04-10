import { create } from 'zustand'
import type { Video, FilterState, Tag } from '@/types'

type ScrapeMode = 'idle' | 'progress' | 'result'

interface ScrapeProgress {
  current: number
  total: number
  success: number
  fail: number
}

interface LibraryStore {
  videos: Video[]
  filters: FilterState
  searchQuery: string
  isScanning: boolean
  selectionMode: boolean
  selectedIds: Set<string>
  allTags: Tag[]
  scrapeMode: ScrapeMode
  scrapeProgress: ScrapeProgress
  setVideos: (videos: Video[]) => void
  setFilters: (filters: Partial<FilterState>) => void
  setSearchQuery: (q: string) => void
  setScanning: (v: boolean) => void
  setSelectionMode: (v: boolean) => void
  toggleSelected: (id: string) => void
  selectAll: (ids: string[]) => void
  clearSelection: () => void
  setAllTags: (tags: Tag[]) => void
  setScrapeMode: (mode: ScrapeMode) => void
  setScrapeProgress: (p: ScrapeProgress) => void
  updateScrapeProgress: (updater: (prev: ScrapeProgress) => ScrapeProgress) => void
}

const DEFAULT_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
  scrapeStatusFilter: 'all',
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  videos: [],
  filters: DEFAULT_FILTERS,
  searchQuery: '',
  isScanning: false,
  selectionMode: false,
  selectedIds: new Set(),
  allTags: [],
  scrapeMode: 'idle',
  scrapeProgress: { current: 0, total: 0, success: 0, fail: 0 },
  setVideos: (videos) => set({ videos }),
  setFilters: (partial) =>
    set({ filters: { ...get().filters, ...partial } }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setScanning: (isScanning) => set({ isScanning }),
  setSelectionMode: (selectionMode) =>
    set({ selectionMode, selectedIds: selectionMode ? get().selectedIds : new Set() }),
  toggleSelected: (id) => {
    const next = new Set(get().selectedIds)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    set({ selectedIds: next })
  },
  selectAll: (ids) => set({ selectedIds: new Set(ids) }),
  clearSelection: () => set({ selectedIds: new Set() }),
  setAllTags: (allTags) => set({ allTags }),
  setScrapeMode: (scrapeMode) => set({ scrapeMode }),
  setScrapeProgress: (scrapeProgress) => set({ scrapeProgress }),
  updateScrapeProgress: (updater) => set({ scrapeProgress: updater(get().scrapeProgress) }),
}))
