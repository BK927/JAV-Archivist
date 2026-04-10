import { useMemo } from 'react'
import type { Video, FilterState } from '@/types'

export function useFilteredVideos(
  videos: Video[],
  filters: FilterState,
  searchQuery: string,
  activeFilter: { type: string; value: string } | null
): Video[] {
  return useMemo(() => {
    let result = [...videos]

    // URL param filter
    if (activeFilter) {
      switch (activeFilter.type) {
        case '배우':
          result = result.filter((v) => v.actors.includes(activeFilter.value))
          break
        case '시리즈':
          result = result.filter((v) => v.series === activeFilter.value)
          break
        case '제작사':
          result = result.filter((v) => v.makerName === activeFilter.value)
          break
        case '태그':
          result = result.filter((v) => v.tags.includes(activeFilter.value))
          break
      }
    }

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

    // 태그 그룹 필터
    const { groups, groupOperator } = filters.tagFilter
    const activeGroups = groups.filter((g) => g.tags.length > 0)
    if (activeGroups.length > 0) {
      result = result.filter((v) => {
        const groupResults = activeGroups.map((g) =>
          g.tags.some((tag) => v.tags.includes(tag))
        )
        return groupOperator === 'AND'
          ? groupResults.every(Boolean)
          : groupResults.some(Boolean)
      })
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
  }, [videos, filters, searchQuery, activeFilter])
}
