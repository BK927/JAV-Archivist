import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Download, X } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'

interface FilterBarProps {
  totalCount: number
  tags: string[]
  unscrapedCount: number
  isScraping: boolean
  scrapeProgress: { current: number; total: number } | null
  onScrapeAll: () => void
  onCancelScrape: () => void
  activeFilter: { type: string; value: string } | null
  onClearFilter: () => void
}

export default function FilterBar({
  totalCount,
  tags,
  unscrapedCount,
  isScraping,
  scrapeProgress,
  onScrapeAll,
  onCancelScrape,
  activeFilter,
  onClearFilter,
}: FilterBarProps) {
  const { filters, setFilters } = useLibraryStore()

  return (
    <div className="flex items-center gap-2 px-6 py-3 border-b border-border flex-wrap">
      {/* 정렬 */}
      <Select
        value={`${filters.sortBy}-${filters.sortOrder}`}
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
          <SelectValue />
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
          <SelectValue />
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
        className="cursor-pointer h-7 px-2 text-xs"
        onClick={() => setFilters({ favoriteOnly: !filters.favoriteOnly })}
      >
        ★ 즐겨찾기
      </Badge>

      {/* 태그 필터 */}
      {tags.map((tag) => (
        <Badge
          key={tag}
          variant={filters.tags.includes(tag) ? 'default' : 'outline'}
          className="cursor-pointer h-7 px-2 text-xs"
          onClick={() => {
            const next = filters.tags.includes(tag)
              ? filters.tags.filter((t) => t !== tag)
              : [...filters.tags, tag]
            setFilters({ tags: next })
          }}
        >
          {tag}
        </Badge>
      ))}

      {/* 스크래핑 버튼 */}
      {!isScraping && unscrapedCount > 0 && (
        <Button variant="outline" size="sm" className="h-7 text-xs" onClick={onScrapeAll}>
          <Download className="w-3 h-3 mr-1" />
          메타데이터 수집 ({unscrapedCount})
        </Button>
      )}

      {isScraping && scrapeProgress && (
        <div className="flex items-center gap-2">
          <div className="w-24 h-1.5 bg-secondary rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${(scrapeProgress.current / scrapeProgress.total) * 100}%` }}
            />
          </div>
          <span className="text-xs text-muted-foreground">
            {scrapeProgress.current}/{scrapeProgress.total}
          </span>
          <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onCancelScrape}>
            <X className="w-3 h-3" />
          </Button>
        </div>
      )}

      {/* 활성 필터 뱃지 */}
      {activeFilter && (
        <Badge variant="default" className="h-7 px-2 text-xs gap-1">
          {activeFilter.type}: {activeFilter.value}
          <button onClick={onClearFilter} className="ml-1">
            <X className="w-3 h-3" />
          </button>
        </Badge>
      )}

      <span className="ml-auto text-xs text-muted-foreground">
        {totalCount.toLocaleString()}개
      </span>
    </div>
  )
}
