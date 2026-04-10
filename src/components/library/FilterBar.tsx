import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { X } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import TagPopover from '@/components/library/TagPopover'
import type { Tag, FilterState } from '@/types'

const QUICK_TAG_COUNT = 8

interface FilterBarProps {
  totalCount: number
  tags: Tag[]
  activeFilter: { type: string; value: string } | null
  onClearFilter: () => void
}

const SORT_LABELS: Record<string, string> = {
  'addedAt-desc': '최근 추가순',
  'addedAt-asc': '오래된 추가순',
  'releasedAt-desc': '출시일 최신순',
  'releasedAt-asc': '출시일 오래된순',
  'title-asc': '제목 오름차순',
  'title-desc': '제목 내림차순',
}

const WATCHED_LABELS: Record<string, string> = {
  all: '전체',
  unwatched: '미시청',
  watched: '시청함',
}

const SCRAPE_STATUS_LABELS: Record<string, string> = {
  all: '수집 상태: 전체',
  not_scraped: '미수집',
  partial: '부분 수집',
  not_found: '실패',
  complete: '완료',
}

export default function FilterBar({
  totalCount,
  tags,
  activeFilter,
  onClearFilter,
}: FilterBarProps) {
  const { filters, setFilters, selectionMode, setSelectionMode } = useLibraryStore()
  const sortKey = `${filters.sortBy}-${filters.sortOrder}`

  // All selected tags across all groups
  const selectedTagSet = new Set(filters.tagFilter.groups.flatMap((g) => g.tags))

  // Quick tags: top N by videoCount (tags already sorted by videoCount DESC from backend)
  const quickTags = tags.slice(0, QUICK_TAG_COUNT)
  const remainingCount = Math.max(0, tags.length - QUICK_TAG_COUNT)

  // Selected tags not in quick tags (show them in FilterBar too)
  const quickTagNames = new Set(quickTags.map((t) => t.name))
  const extraSelected = [...selectedTagSet].filter((t) => !quickTagNames.has(t))

  // Toggle a tag (quick tag click) — adds to first group, removes from any
  const toggleQuickTag = (tagName: string) => {
    const { tagFilter } = filters
    if (selectedTagSet.has(tagName)) {
      const groups = tagFilter.groups
        .map((g) => ({ ...g, tags: g.tags.filter((t) => t !== tagName) }))
        .filter((g) => g.tags.length > 0)
      setFilters({ tagFilter: { ...tagFilter, groups } })
    } else {
      const groups = [...tagFilter.groups]
      if (groups.length === 0) {
        groups.push({ id: crypto.randomUUID(), tags: [tagName] })
      } else {
        groups[0] = { ...groups[0], tags: [...groups[0].tags, tagName] }
      }
      setFilters({ tagFilter: { ...tagFilter, groups } })
    }
  }

  return (
    <div className="flex items-center gap-2 px-6 py-3 border-b border-border overflow-hidden">
      {/* 정렬 */}
      <Select
        value={sortKey}
        onValueChange={(value) => {
          if (!value) return
          const [sortBy, sortOrder] = value.split('-') as [
            typeof filters.sortBy,
            typeof filters.sortOrder,
          ]
          setFilters({ sortBy, sortOrder })
        }}
      >
        <SelectTrigger className="w-36 h-7 text-xs bg-secondary border-border">
          <SelectValue placeholder={SORT_LABELS[sortKey]}>{SORT_LABELS[sortKey]}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="addedAt-desc">최근 추가순</SelectItem>
          <SelectItem value="addedAt-asc">오래된 추가순</SelectItem>
          <SelectItem value="releasedAt-desc">출시일 최신순</SelectItem>
          <SelectItem value="releasedAt-asc">출시일 오래된순</SelectItem>
          <SelectItem value="title-asc">제목 오름차순</SelectItem>
          <SelectItem value="title-desc">제목 내림차순</SelectItem>
        </SelectContent>
      </Select>

      {/* 시청 필터 */}
      <Select
        value={filters.watchedFilter}
        onValueChange={(v) =>
          setFilters({ watchedFilter: v as typeof filters.watchedFilter })
        }
      >
        <SelectTrigger className="w-24 h-7 text-xs bg-secondary border-border">
          <SelectValue placeholder={WATCHED_LABELS[filters.watchedFilter]}>{WATCHED_LABELS[filters.watchedFilter]}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">전체</SelectItem>
          <SelectItem value="unwatched">미시청</SelectItem>
          <SelectItem value="watched">시청함</SelectItem>
        </SelectContent>
      </Select>

      {/* 즐겨찾기 토글 */}
      <Badge
        variant={filters.favoriteOnly ? 'default' : 'outline'}
        className="cursor-pointer h-7 px-2 text-xs shrink-0"
        onClick={() => setFilters({ favoriteOnly: !filters.favoriteOnly })}
      >
        ★ 즐겨찾기
      </Badge>

      {/* 수집 상태 필터 */}
      <Select
        value={filters.scrapeStatusFilter}
        onValueChange={(v) => setFilters({ scrapeStatusFilter: v as FilterState['scrapeStatusFilter'] })}
      >
        <SelectTrigger className="w-28 h-7 text-xs bg-secondary border-border">
          <SelectValue>{SCRAPE_STATUS_LABELS[filters.scrapeStatusFilter]}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">전체</SelectItem>
          <SelectItem value="not_scraped">미수집</SelectItem>
          <SelectItem value="partial">부분 수집</SelectItem>
          <SelectItem value="not_found">실패</SelectItem>
          <SelectItem value="complete">완료</SelectItem>
        </SelectContent>
      </Select>

      <Separator orientation="vertical" className="h-5" />

      {/* Quick tags */}
      {quickTags.map((tag) => (
        <Badge
          key={tag.id}
          variant={selectedTagSet.has(tag.name) ? 'default' : 'outline'}
          className="cursor-pointer h-7 px-2 text-xs shrink-0"
          onClick={() => toggleQuickTag(tag.name)}
        >
          {tag.name}
        </Badge>
      ))}

      {/* Extra selected tags (not in quick tags) */}
      {extraSelected.map((tagName) => (
        <Badge
          key={tagName}
          variant="default"
          className="cursor-pointer h-7 px-2 text-xs shrink-0"
          onClick={() => toggleQuickTag(tagName)}
        >
          {tagName}
        </Badge>
      ))}

      {/* Tag popover */}
      {tags.length > QUICK_TAG_COUNT && (
        <>
          <Separator orientation="vertical" className="h-5" />
          <TagPopover allTags={tags} remainingCount={remainingCount} />
        </>
      )}

      {/* 활성 필터 뱃지 */}
      {activeFilter && (
        <Badge variant="default" className="h-7 px-2 text-xs gap-1 shrink-0">
          {activeFilter.type}: {activeFilter.value}
          <button onClick={onClearFilter} className="ml-1">
            <X className="w-3 h-3" />
          </button>
        </Badge>
      )}

      {/* 선택 모드 토글 */}
      <Button
        variant={selectionMode ? 'default' : 'outline'}
        size="sm"
        className="ml-auto h-7 text-xs shrink-0"
        onClick={() => setSelectionMode(!selectionMode)}
      >
        {selectionMode ? '선택 해제' : '☑ 선택'}
      </Button>

      <span className="text-xs text-muted-foreground shrink-0">
        {totalCount.toLocaleString()}개
      </span>
    </div>
  )
}
