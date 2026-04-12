import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useFilteredVideos } from '../useFilteredVideos'
import { MOCK_VIDEOS } from '@/lib/mockData'
import type { FilterState } from '@/types'

const BASE_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
  scrapeStatusFilter: 'all',
  unidentifiedOnly: false,
}

describe('useFilteredVideos', () => {
  it('watchedFilter=unwatched는 미시청 영상만 반환한다', () => {
    const { result } = renderHook(() =>
      useFilteredVideos(MOCK_VIDEOS, { ...BASE_FILTERS, watchedFilter: 'unwatched' }, '', null)
    )
    expect(result.current.every((v) => !v.watched)).toBe(true)
  })

  it('favoriteOnly=true는 즐겨찾기만 반환한다', () => {
    const { result } = renderHook(() =>
      useFilteredVideos(MOCK_VIDEOS, { ...BASE_FILTERS, favoriteOnly: true }, '', null)
    )
    expect(result.current.every((v) => v.favorite)).toBe(true)
  })

  it('tags 필터는 해당 태그를 포함하는 영상만 반환한다', () => {
    const { result } = renderHook(() =>
      useFilteredVideos(MOCK_VIDEOS, { ...BASE_FILTERS, tagFilter: { groups: [{ id: 'test-1', tags: ['직장물'] }], groupOperator: 'AND' } }, '', null)
    )
    expect(result.current.every((v) => v.tags.includes('직장물'))).toBe(true)
  })

  it('searchQuery는 품번과 제목을 검색한다', () => {
    const { result } = renderHook(() =>
      useFilteredVideos(MOCK_VIDEOS, BASE_FILTERS, 'SONE', null)
    )
    expect(result.current.length).toBeGreaterThan(0)
    expect(result.current.every((v) => v.code.includes('SONE') || v.title.includes('SONE'))).toBe(true)
  })

  it('sortBy=title + sortOrder=asc는 제목 오름차순 정렬', () => {
    const { result } = renderHook(() =>
      useFilteredVideos(MOCK_VIDEOS, { ...BASE_FILTERS, sortBy: 'title', sortOrder: 'asc' }, '', null)
    )
    const titles = result.current.map((v) => v.title)
    expect(titles).toEqual([...titles].sort())
  })
})
