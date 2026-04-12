import { describe, it, expect, beforeEach } from 'vitest'
import { useLibraryStore } from '../libraryStore'
import { MOCK_VIDEOS } from '@/lib/mockData'
import type { FilterState } from '@/types'

const DEFAULT_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
  scrapeStatusFilter: 'all',
  unidentifiedOnly: false,
}

beforeEach(() => {
  useLibraryStore.setState({
    videos: [],
    filters: DEFAULT_FILTERS,
    searchQuery: '',
    isScanning: false,
    allTags: [],
  })
})

describe('libraryStore', () => {
  it('setVideos가 영상 목록을 교체한다', () => {
    useLibraryStore.getState().setVideos(MOCK_VIDEOS)
    expect(useLibraryStore.getState().videos).toEqual(MOCK_VIDEOS)
  })

  it('setFilters가 부분 업데이트를 적용한다', () => {
    useLibraryStore.getState().setFilters({ sortBy: 'title' })
    expect(useLibraryStore.getState().filters.sortBy).toBe('title')
    expect(useLibraryStore.getState().filters.sortOrder).toBe('desc')
  })

  it('setSearchQuery가 검색어를 업데이트한다', () => {
    useLibraryStore.getState().setSearchQuery('SONE')
    expect(useLibraryStore.getState().searchQuery).toBe('SONE')
  })

  it('setScanning이 스캔 상태를 토글한다', () => {
    useLibraryStore.getState().setScanning(true)
    expect(useLibraryStore.getState().isScanning).toBe(true)
    useLibraryStore.getState().setScanning(false)
    expect(useLibraryStore.getState().isScanning).toBe(false)
  })
})
