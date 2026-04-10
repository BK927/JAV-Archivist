# Tag Filter UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current unbounded tag badge list in FilterBar with a fixed 1-line layout: popular quick-tags + a popover with search/autocomplete, co-occurrence recommendations, and AND/OR tag groups.

**Architecture:** FilterBar stays 1 line with top-N popular tags visible. A popover (base-ui Popover) contains search input with autocomplete dropdown, co-occurrence suggestions, and tag group management. Filter state changes from `tags: string[]` to `tagFilter: { groups: TagGroup[], groupOperator }`. A new Rust command `get_tag_cooccurrence` provides co-occurrence data.

**Tech Stack:** React 19, @base-ui/react (Popover), Zustand, Tailwind CSS, Rust/SQLite (Tauri commands)

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `src/types/index.ts` | Add `TagGroup`, `TagFilter` types; update `FilterState` |
| Modify | `src/stores/libraryStore.ts` | Update default filters to use `tagFilter` |
| Modify | `src/hooks/useFilteredVideos.ts` | Group-based filtering logic |
| Create | `src/components/library/TagPopover.tsx` | Popover with search, autocomplete, groups |
| Modify | `src/components/library/FilterBar.tsx` | Quick tags + popover button layout |
| Modify | `src/pages/LibraryPage.tsx` | Fetch `Tag[]` via `get_tags`, pass to FilterBar |
| Modify | `src-tauri/src/db.rs` | Add `get_tag_cooccurrence()` function |
| Modify | `src-tauri/src/models.rs` | Add `TagCooccurrence` struct |
| Modify | `src-tauri/src/lib.rs` | Register `get_tag_cooccurrence` command |

---

### Task 1: Update TypeScript Types

**Files:**
- Modify: `src/types/index.ts:61-67`

- [ ] **Step 1: Add TagGroup and TagFilter types, update FilterState**

In `src/types/index.ts`, replace the `FilterState` interface and add new types before it:

```typescript
export interface TagGroup {
  id: string
  tags: string[]
}

export interface TagFilter {
  groups: TagGroup[]
  groupOperator: 'AND' | 'OR'
}

export interface FilterState {
  sortBy: 'addedAt' | 'releasedAt' | 'title'
  sortOrder: 'asc' | 'desc'
  watchedFilter: 'all' | 'watched' | 'unwatched'
  favoriteOnly: boolean
  tagFilter: TagFilter
}
```

- [ ] **Step 2: Verify types compile**

Run: `pnpm tsc --noEmit`
Expected: Errors in `libraryStore.ts`, `useFilteredVideos.ts`, `FilterBar.tsx` referencing old `tags` field. This is expected — we fix those in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add src/types/index.ts
git commit -m "refactor: replace tags array with TagFilter in FilterState type"
```

---

### Task 2: Update Store and Filter Hook

**Files:**
- Modify: `src/stores/libraryStore.ts:15-21`
- Modify: `src/hooks/useFilteredVideos.ts:54-59`

- [ ] **Step 1: Update libraryStore default filters**

In `src/stores/libraryStore.ts`, change the `DEFAULT_FILTERS`:

```typescript
const DEFAULT_FILTERS: FilterState = {
  sortBy: 'addedAt',
  sortOrder: 'desc',
  watchedFilter: 'all',
  favoriteOnly: false,
  tagFilter: { groups: [], groupOperator: 'AND' },
}
```

- [ ] **Step 2: Update useFilteredVideos tag filtering logic**

In `src/hooks/useFilteredVideos.ts`, replace lines 54-59 with:

```typescript
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
```

- [ ] **Step 3: Verify types compile**

Run: `pnpm tsc --noEmit`
Expected: Errors only in `FilterBar.tsx` (still references `filters.tags`). Store and hook should compile clean.

- [ ] **Step 4: Commit**

```bash
git add src/stores/libraryStore.ts src/hooks/useFilteredVideos.ts
git commit -m "refactor: update store and filter hook for tag groups"
```

---

### Task 3: Add Co-occurrence Rust Command

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add TagCooccurrence model**

In `src-tauri/src/models.rs`, add after the `Tag` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCooccurrence {
    pub tag_id: String,
    pub tag_name: String,
    pub co_count: u32,
}
```

- [ ] **Step 2: Add get_tag_cooccurrence function in db.rs**

In `src-tauri/src/db.rs`, add after the `get_tags` function (after line 437):

```rust
pub fn get_tag_cooccurrence(conn: &Connection, tag_id: &str) -> Result<Vec<crate::models::TagCooccurrence>> {
    let mut stmt = conn.prepare(
        "SELECT t2.id, t2.name, COUNT(*) as co_count
         FROM video_tags vt1
         JOIN video_tags vt2 ON vt1.video_id = vt2.video_id AND vt1.tag_id != vt2.tag_id
         JOIN tags t2 ON vt2.tag_id = t2.id
         WHERE vt1.tag_id = ?1
         GROUP BY t2.id
         ORDER BY co_count DESC
         LIMIT 10"
    )?;
    let rows = stmt.query_map([tag_id], |row| {
        Ok(crate::models::TagCooccurrence {
            tag_id: row.get(0)?,
            tag_name: row.get(1)?,
            co_count: row.get::<_, u32>(2)?,
        })
    })?;
    rows.collect()
}
```

- [ ] **Step 3: Add import and Tauri command in lib.rs**

In `src-tauri/src/lib.rs`, update the `use models::` line to include `TagCooccurrence`:

```rust
use models::{Settings, ScrapeStatus, Video, Actor, Maker, Series as SeriesModel, Tag, TagCooccurrence, SampleImage};
```

Add the command function (after `get_tags`):

```rust
#[tauri::command]
fn get_tag_cooccurrence(db: tauri::State<'_, DbPath>, tag_id: String) -> Result<Vec<TagCooccurrence>, String> {
    tracing::info!("cmd: get_tag_cooccurrence tag_id={}", tag_id);
    let conn = db::open(db.0.to_str().unwrap()).map_err(|e| e.to_string())?;
    db::get_tag_cooccurrence(&conn, &tag_id).map_err(|e| e.to_string())
}
```

Register in the `invoke_handler` macro (add after `get_tags,`):

```rust
get_tag_cooccurrence,
```

- [ ] **Step 4: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: add get_tag_cooccurrence Tauri command"
```

---

### Task 4: Add TagCooccurrence TypeScript type

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: Add TagCooccurrence interface**

In `src/types/index.ts`, add after the `Tag` interface:

```typescript
export interface TagCooccurrence {
  tagId: string
  tagName: string
  coCount: number
}
```

- [ ] **Step 2: Verify types compile**

Run: `pnpm tsc --noEmit`
Expected: Errors only in `FilterBar.tsx` (still not updated).

- [ ] **Step 3: Commit**

```bash
git add src/types/index.ts
git commit -m "feat: add TagCooccurrence TypeScript type"
```

---

### Task 5: Create TagPopover Component

**Files:**
- Create: `src/components/library/TagPopover.tsx`

This is the main new component containing: search input, autocomplete dropdown (search results + co-occurrence suggestions), and tag group management.

- [ ] **Step 1: Create TagPopover.tsx**

Create `src/components/library/TagPopover.tsx`:

```tsx
import { useState, useRef, useEffect, useCallback, useMemo } from 'react'
import { Popover as PopoverPrimitive } from '@base-ui/react/popover'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { X, Search, Plus } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Tag, TagGroup, TagCooccurrence } from '@/types'

interface TagPopoverProps {
  allTags: Tag[]
  remainingCount: number
}

export default function TagPopover({ allTags, remainingCount }: TagPopoverProps) {
  const { filters, setFilters } = useLibraryStore()
  const { tagFilter } = filters
  const { run } = useTauriCommand()

  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const [highlightIdx, setHighlightIdx] = useState(0)
  const [coTags, setCoTags] = useState<TagCooccurrence[]>([])
  const inputRef = useRef<HTMLInputElement>(null)

  // All currently selected tag names (across all groups)
  const selectedTags = useMemo(
    () => tagFilter.groups.flatMap((g) => g.tags),
    [tagFilter.groups]
  )

  // Search results: filter allTags by search text, sorted by videoCount
  const searchResults = useMemo(() => {
    if (!search.trim()) return allTags.slice(0, 20)
    const q = search.trim().toLowerCase()
    return allTags.filter((t) => t.name.toLowerCase().includes(q))
  }, [search, allTags])

  // Fetch co-occurrence when selected tags change
  useEffect(() => {
    if (selectedTags.length === 0) {
      setCoTags([])
      return
    }
    // Use the first selected tag for co-occurrence
    const firstTag = allTags.find((t) => t.name === selectedTags[0])
    if (!firstTag) return
    run<TagCooccurrence[]>('get_tag_cooccurrence', { tagId: firstTag.id }, []).then(setCoTags)
  }, [selectedTags, allTags, run])

  // Reset highlight when search changes
  useEffect(() => {
    setHighlightIdx(0)
  }, [search])

  // Focus input when popover opens
  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50)
    } else {
      setSearch('')
    }
  }, [open])

  const addTagToGroup = useCallback(
    (tagName: string, groupIdx: number = 0) => {
      if (selectedTags.includes(tagName)) return
      const groups = [...tagFilter.groups]
      if (groups.length === 0) {
        groups.push({ id: crypto.randomUUID(), tags: [tagName] })
      } else {
        const group = { ...groups[groupIdx], tags: [...groups[groupIdx].tags, tagName] }
        groups[groupIdx] = group
      }
      setFilters({ tagFilter: { ...tagFilter, groups } })
    },
    [tagFilter, selectedTags, setFilters]
  )

  const removeTagFromGroup = useCallback(
    (tagName: string, groupIdx: number) => {
      const groups = tagFilter.groups
        .map((g, i) =>
          i === groupIdx ? { ...g, tags: g.tags.filter((t) => t !== tagName) } : g
        )
        .filter((g) => g.tags.length > 0)
      setFilters({ tagFilter: { ...tagFilter, groups } })
    },
    [tagFilter, setFilters]
  )

  const addGroup = useCallback(() => {
    const groups = [...tagFilter.groups, { id: crypto.randomUUID(), tags: [] }]
    setFilters({ tagFilter: { ...tagFilter, groups } })
  }, [tagFilter, setFilters])

  const toggleGroupOperator = useCallback(() => {
    const next = tagFilter.groupOperator === 'AND' ? 'OR' : 'AND'
    setFilters({ tagFilter: { ...tagFilter, groupOperator: next } })
  }, [tagFilter, setFilters])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setHighlightIdx((i) => Math.min(i + 1, searchResults.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setHighlightIdx((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const tag = searchResults[highlightIdx]
      if (tag) {
        addTagToGroup(tag.name)
        setSearch('')
      }
    }
  }

  // Highlight matching text in tag name
  const highlightMatch = (name: string) => {
    if (!search.trim()) return name
    const q = search.trim().toLowerCase()
    const idx = name.toLowerCase().indexOf(q)
    if (idx === -1) return name
    return (
      <>
        {name.slice(0, idx)}
        <span className="text-primary font-semibold">{name.slice(idx, idx + q.length)}</span>
        {name.slice(idx + q.length)}
      </>
    )
  }

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen}>
      <PopoverPrimitive.Trigger
        className="inline-flex items-center gap-1 h-7 px-3 rounded-md text-xs border border-primary text-primary bg-primary/10 cursor-pointer hover:bg-primary/20 transition-colors"
      >
        +{remainingCount}개 {open ? '▴' : '▾'}
      </PopoverPrimitive.Trigger>

      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Positioner sideOffset={4} side="bottom" alignment="end">
          <PopoverPrimitive.Popup
            className="w-96 bg-popover border border-border rounded-lg shadow-xl z-50 p-3"
          >
            {/* Search input */}
            <div className="relative mb-2">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
              <Input
                ref={inputRef}
                value={search}
                onChange={(e) => setSearch((e.target as HTMLInputElement).value)}
                onKeyDown={handleKeyDown}
                placeholder="태그 검색..."
                className="pl-8 h-8 text-xs"
              />
            </div>

            {/* Autocomplete dropdown */}
            <div className="max-h-48 overflow-y-auto rounded-md border border-border mb-3">
              {/* Search results section */}
              <div className="px-2.5 pt-1.5 pb-0.5 text-[10px] font-bold text-muted-foreground uppercase tracking-wider">
                {search.trim() ? '검색 결과' : '인기 태그'}
              </div>
              {searchResults.length === 0 && (
                <div className="px-2.5 py-2 text-xs text-muted-foreground">결과 없음</div>
              )}
              {searchResults.map((tag, i) => (
                <button
                  key={tag.id}
                  className={`flex items-center justify-between w-full px-2.5 py-1.5 text-xs cursor-pointer hover:bg-muted/50 ${i === highlightIdx ? 'bg-muted/50' : ''}`}
                  onClick={() => {
                    addTagToGroup(tag.name)
                    setSearch('')
                  }}
                  onMouseEnter={() => setHighlightIdx(i)}
                >
                  <span>{highlightMatch(tag.name)}</span>
                  <span className="flex items-center gap-2">
                    {selectedTags.includes(tag.name) && (
                      <span className="text-[10px] text-primary bg-primary/15 px-1.5 py-0.5 rounded">선택됨</span>
                    )}
                    <span className="text-[10px] text-muted-foreground">{tag.videoCount}건</span>
                  </span>
                </button>
              ))}

              {/* Co-occurrence section */}
              {coTags.length > 0 && (
                <>
                  <Separator />
                  <div className="px-2.5 pt-1.5 pb-0.5 text-[10px] font-bold text-muted-foreground uppercase tracking-wider">
                    자주 같이 쓰는 태그
                  </div>
                  {coTags.map((ct) => (
                    <button
                      key={ct.tagId}
                      className="flex items-center justify-between w-full px-2.5 py-1.5 text-xs cursor-pointer hover:bg-muted/50"
                      onClick={() => {
                        addTagToGroup(ct.tagName)
                        setSearch('')
                      }}
                    >
                      <span>{ct.tagName}</span>
                      <span className="flex items-center gap-2">
                        {selectedTags.includes(ct.tagName) && (
                          <span className="text-[10px] text-primary bg-primary/15 px-1.5 py-0.5 rounded">선택됨</span>
                        )}
                        <span className="text-[10px] text-muted-foreground">{ct.coCount}건</span>
                      </span>
                    </button>
                  ))}
                </>
              )}
            </div>

            {/* Tag Groups */}
            {tagFilter.groups.length > 0 && (
              <div className="space-y-2">
                {tagFilter.groups.map((group, gi) => (
                  <div key={group.id}>
                    {/* Inter-group operator connector */}
                    {gi > 0 && (
                      <button
                        className="flex items-center justify-center w-full py-1 mb-2"
                        onClick={toggleGroupOperator}
                      >
                        <span className="text-[10px] text-muted-foreground bg-secondary px-3 py-0.5 rounded cursor-pointer hover:bg-secondary/80">
                          {tagFilter.groupOperator}
                        </span>
                      </button>
                    )}
                    <div className="border border-border rounded-lg p-2.5 bg-muted/20">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-[11px] font-semibold text-primary">
                          그룹 {gi + 1}
                        </span>
                        <span className="text-[9px] text-muted-foreground bg-secondary px-1.5 py-0.5 rounded">
                          OR
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {group.tags.map((tagName) => {
                          const tagData = allTags.find((t) => t.name === tagName)
                          return (
                            <Badge
                              key={tagName}
                              variant="default"
                              className="h-6 px-2 text-[11px] gap-1 cursor-pointer"
                              onClick={() => removeTagFromGroup(tagName, gi)}
                            >
                              {tagName}
                              <span className="text-[9px] opacity-50">
                                {tagData?.videoCount ?? 0}
                              </span>
                              <X className="w-2.5 h-2.5 opacity-60" />
                            </Badge>
                          )
                        })}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Add group button */}
            {tagFilter.groups.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                className="w-full mt-2 h-7 text-xs text-muted-foreground"
                onClick={addGroup}
              >
                <Plus className="w-3 h-3 mr-1" /> 새 그룹
              </Button>
            )}
          </PopoverPrimitive.Popup>
        </PopoverPrimitive.Positioner>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  )
}
```

- [ ] **Step 2: Verify types compile**

Run: `pnpm tsc --noEmit`
Expected: Errors only in `FilterBar.tsx`.

- [ ] **Step 3: Commit**

```bash
git add src/components/library/TagPopover.tsx
git commit -m "feat: create TagPopover component with search, autocomplete, and groups"
```

---

### Task 6: Update FilterBar

**Files:**
- Modify: `src/components/library/FilterBar.tsx`

- [ ] **Step 1: Rewrite FilterBar with quick tags + popover**

Replace the entire `src/components/library/FilterBar.tsx`:

```tsx
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
import { Download, X } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import TagPopover from '@/components/library/TagPopover'
import type { Tag } from '@/types'

const QUICK_TAG_COUNT = 8

interface FilterBarProps {
  totalCount: number
  tags: Tag[]
  unscrapedCount: number
  isScraping: boolean
  scrapeProgress: { current: number; total: number } | null
  onScrapeAll: () => void
  onCancelScrape: () => void
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
  const sortKey = `${filters.sortBy}-${filters.sortOrder}`

  // All selected tags across all groups
  const selectedTags = filters.tagFilter.groups.flatMap((g) => g.tags)

  // Quick tags: top N by videoCount (tags already sorted by videoCount DESC from backend)
  const quickTags = tags.slice(0, QUICK_TAG_COUNT)
  const remainingCount = Math.max(0, tags.length - QUICK_TAG_COUNT)

  // Selected tags not in quick tags (show them in FilterBar too)
  const quickTagNames = new Set(quickTags.map((t) => t.name))
  const extraSelected = selectedTags.filter((t) => !quickTagNames.has(t))

  // Toggle a tag in group 0 (quick tag click)
  const toggleQuickTag = (tagName: string) => {
    const { tagFilter } = filters
    if (selectedTags.includes(tagName)) {
      // Remove from whichever group it's in
      const groups = tagFilter.groups
        .map((g) => ({ ...g, tags: g.tags.filter((t) => t !== tagName) }))
        .filter((g) => g.tags.length > 0)
      setFilters({ tagFilter: { ...tagFilter, groups } })
    } else {
      // Add to group 0
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

      <Separator orientation="vertical" className="h-5" />

      {/* Quick tags */}
      {quickTags.map((tag) => (
        <Badge
          key={tag.id}
          variant={selectedTags.includes(tag.name) ? 'default' : 'outline'}
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

      {/* 스크래핑 버튼 */}
      {!isScraping && unscrapedCount > 0 && (
        <Button variant="outline" size="sm" className="h-7 text-xs shrink-0" onClick={onScrapeAll}>
          <Download className="w-3 h-3 mr-1" />
          메타데이터 수집 ({unscrapedCount})
        </Button>
      )}

      {isScraping && scrapeProgress && (
        <div className="flex items-center gap-2 shrink-0">
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
        <Badge variant="default" className="h-7 px-2 text-xs gap-1 shrink-0">
          {activeFilter.type}: {activeFilter.value}
          <button onClick={onClearFilter} className="ml-1">
            <X className="w-3 h-3" />
          </button>
        </Badge>
      )}

      <span className="ml-auto text-xs text-muted-foreground shrink-0">
        {totalCount.toLocaleString()}개
      </span>
    </div>
  )
}
```

Key changes from original:
- `tags` prop type: `string[]` → `Tag[]`
- Replaced `flex-wrap` with `overflow-hidden` (1 line)
- Only renders `QUICK_TAG_COUNT` tags inline
- Uses `TagPopover` for the rest
- `toggleQuickTag` uses new `tagFilter` structure instead of `filters.tags`

- [ ] **Step 2: Verify types compile**

Run: `pnpm tsc --noEmit`
Expected: Error in `LibraryPage.tsx` where `allTags` is `string[]` but FilterBar now expects `Tag[]`. Fixed in next task.

- [ ] **Step 3: Commit**

```bash
git add src/components/library/FilterBar.tsx
git commit -m "feat: update FilterBar with quick tags and popover button"
```

---

### Task 7: Update LibraryPage to Fetch Tag Objects

**Files:**
- Modify: `src/pages/LibraryPage.tsx`

- [ ] **Step 1: Replace allTags derivation with Tauri get_tags call**

In `src/pages/LibraryPage.tsx`:

1. Add `Tag` to the import from `@/types`:
```typescript
import type { Video, Tag } from '@/types'
```

2. Add state for tags (after the `scrapeProgress` state):
```typescript
const [allTags, setAllTags] = useState<Tag[]>([])
```

3. Remove the old derived `allTags` line:
```typescript
// DELETE: const allTags = [...new Set(videos.flatMap((v) => v.tags))]
```

4. Add a `useEffect` to fetch tags (after the `scan_library` effect):
```typescript
useEffect(() => {
  run<Tag[]>('get_tags', {}, []).then(setAllTags)
}, [run, videos])
```

Note: `videos` is in the dependency array so tags refresh when videos change (after scrape).

- [ ] **Step 2: Verify full project compiles**

Run: `pnpm tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/pages/LibraryPage.tsx
git commit -m "feat: fetch Tag[] from backend for FilterBar"
```

---

### Task 8: Verify and Fix Integration

**Files:** All modified files

- [ ] **Step 1: Run full type check**

Run: `pnpm tsc --noEmit`
Expected: No errors.

- [ ] **Step 2: Run Rust check**

Run: `cd src-tauri && cargo check`
Expected: No errors.

- [ ] **Step 3: Quick smoke test**

Run: `pnpm tauri dev`
Verify:
1. FilterBar shows max 8 tags in a single line
2. "+N개" button appears and opens popover
3. Tag search filters the list with red highlight on match
4. Clicking a tag adds it to Group 1
5. Co-occurrence suggestions appear below search results when a tag is selected
6. Groups display with OR internal and clickable AND/OR inter-group connector
7. Quick tags in FilterBar toggle correctly
8. Video grid filters according to selected tag groups

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: tag filter integration fixes"
```

---

## Sequencing

```
Task 1 (types) → Task 2 (store + hook) → Task 4 (TS co-occurrence type)
                                        ↘
Task 3 (Rust co-occurrence) ─────────────→ Task 5 (TagPopover) → Task 6 (FilterBar) → Task 7 (LibraryPage) → Task 8 (integration)
```

Tasks 3 and 4 can run in parallel. Tasks 5-7 are sequential.

## Verification

1. `pnpm tsc --noEmit` — no errors
2. `cargo check` — no errors
3. FilterBar always 1 line regardless of tag count
4. Popover search + autocomplete works with Japanese text
5. Co-occurrence recommendations show when tags are selected
6. Tag groups with AND/OR work correctly in filtering
7. Quick tag toggle in FilterBar updates groups properly
