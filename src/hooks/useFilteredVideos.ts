import { useMemo } from 'react'
import type { Video, FilterState } from '@/types'

export function useFilteredVideos(
  videos: Video[],
  filters: FilterState,
  searchQuery: string
): Video[] {
  return useMemo(() => {
    let result = [...videos]

    // 검색
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase()
      result = result.filter(
        (v) =>
          v.code.toLowerCase().includes(q) ||
          v.title.toLowerCase().includes(q) ||
          v.actors.some((a) => a.toLowerCase().includes(q))
      )
    }

    // 시청 여부
    if (filters.watchedFilter === 'watched') {
      result = result.filter((v) => v.watched)
    } else if (filters.watchedFilter === 'unwatched') {
      result = result.filter((v) => !v.watched)
    }

    // 즐겨찾기
    if (filters.favoriteOnly) {
      result = result.filter((v) => v.favorite)
    }

    // 태그
    if (filters.tags.length > 0) {
      result = result.filter((v) =>
        filters.tags.every((tag) => v.tags.includes(tag))
      )
    }

    // 정렬
    result.sort((a, b) => {
      let cmp = 0
      if (filters.sortBy === 'title') {
        cmp = a.title.localeCompare(b.title, 'ja')
      } else if (filters.sortBy === 'releasedAt') {
        cmp = (a.releasedAt ?? '').localeCompare(b.releasedAt ?? '')
      } else {
        cmp = a.addedAt.localeCompare(b.addedAt)
      }
      return filters.sortOrder === 'asc' ? cmp : -cmp
    })

    return result
  }, [videos, filters, searchQuery])
}
