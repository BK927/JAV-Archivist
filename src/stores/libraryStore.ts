import { create } from 'zustand'
import type { Video, FilterState } from '@/types'

interface LibraryStore {
  videos: Video[]
  filters: FilterState
  searchQuery: string
  isScanning: boolean
  setVideos: (videos: Video[]) => void
  setFilters: (filters: Partial<FilterState>) => void
  setSearchQuery: (q: string) => void
  setScanning: (v: boolean) => void
}

const DEFAULT_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  videos: [],
  filters: DEFAULT_FILTERS,
  searchQuery: '',
  isScanning: false,
  setVideos: (videos) => set({ videos }),
  setFilters: (partial) =>
    set({ filters: { ...get().filters, ...partial } }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setScanning: (isScanning) => set({ isScanning }),
}))
