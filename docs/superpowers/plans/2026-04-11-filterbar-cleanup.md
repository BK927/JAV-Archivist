# FilterBar Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove quick tags from FilterBar, consolidate tag filtering into TagPopover, fix Select text clipping, and persist allTags in store so tag filters survive tab navigation.

**Architecture:** Move `allTags` from LibraryPage local state to libraryStore + AppShell fetch. Strip FilterBar down to filter controls + single "태그 필터" button. Simplify TagPopover to show all tags without the slice(0,20) limit.

**Tech Stack:** React 19, Zustand, base-ui Popover, Tailwind CSS

---

### Task 1: Add allTags to libraryStore

**Files:**
- Modify: `src/stores/libraryStore.ts:13-33` (interface + defaults)
- Modify: `src/stores/libraryStore.ts:44-71` (implementation)

- [ ] **Step 1: Add allTags state and setAllTags action to store**

In `src/stores/libraryStore.ts`, add to the `LibraryStore` interface:

```typescript
allTags: Tag[]
setAllTags: (tags: Tag[]) => void
```

Add the import for `Tag`:

```typescript
import type { Video, FilterState, Tag } from '@/types'
```

Add defaults and implementation:

```typescript
// in create callback:
allTags: [],
setAllTags: (allTags) => set({ allTags }),
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 3: Commit**

```bash
git add src/stores/libraryStore.ts
git commit -m "feat: add allTags state to libraryStore"
```

---

### Task 2: Move tag fetching from LibraryPage to AppShell

**Files:**
- Modify: `src/components/layout/AppShell.tsx:10-16` (add tags fetch to init + scrape-complete listener)
- Modify: `src/pages/LibraryPage.tsx:1-20,36-58` (remove local allTags state and fetch effects)

- [ ] **Step 1: Add tags fetch to AppShell init**

In `src/components/layout/AppShell.tsx`, import `Tag`:

```typescript
import type { Video, Tag } from '@/types'
```

In the existing `scan_library` effect, chain the tags fetch:

```typescript
useEffect(() => {
  run<Video[]>('scan_library', {}, []).then(setVideos)
  run<Tag[]>('get_tags', {}, []).then((tags) => {
    useLibraryStore.getState().setAllTags(tags)
  })
}, [run, setVideos])
```

- [ ] **Step 2: Add tags refresh to scrape-complete listener in AppShell**

In the existing `scrape-complete` listener (u4), add tags refresh after videos refresh:

```typescript
const u4 = await listen('scrape-complete', () => {
  useLibraryStore.getState().setScrapeMode('result')
  run<Video[]>('get_videos', {}, []).then((vids) => {
    useLibraryStore.getState().setVideos(vids)
  })
  run<Tag[]>('get_tags', {}, []).then((tags) => {
    useLibraryStore.getState().setAllTags(tags)
  })
})
```

- [ ] **Step 3: Remove allTags local state and fetch effects from LibraryPage**

In `src/pages/LibraryPage.tsx`:

1. Remove `useState` from import (keep `useEffect`, `useMemo`), remove `Tag` from type import
2. Remove: `const [allTags, setAllTags] = useState<Tag[]>([])`
3. Remove the `videoCount` + tags fetch effect (lines 36-39)
4. Remove the `scrape-complete` tags refresh effect (lines 42-58)
5. Read allTags from store: `const { videos, filters, searchQuery, allTags } = useLibraryStore()`
6. Update FilterBar prop: `tags={allTags}` (already this name, just source changes)

- [ ] **Step 4: Verify types compile**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 5: Commit**

```bash
git add src/components/layout/AppShell.tsx src/pages/LibraryPage.tsx
git commit -m "refactor: move allTags fetch from LibraryPage to AppShell + store"
```

---

### Task 3: Simplify FilterBar — remove quick tags, fix Select widths

**Files:**
- Modify: `src/components/library/FilterBar.tsx` (full rewrite of template)

- [ ] **Step 1: Remove quick tag code and fix layout**

In `src/components/library/FilterBar.tsx`:

1. Remove `QUICK_TAG_COUNT` constant
2. Remove from props interface: `tags: Tag[]` — replace with nothing (tags come from store now via TagPopover)
3. Remove all quick tag computation: `quickTags`, `remainingCount`, `quickTagNames`, `extraSelected`, `toggleQuickTag`
4. Remove `Tag` from type import (only `FilterState` needed)

Replace the JSX with this cleaned layout:

```tsx
return (
  <div className="flex items-center gap-2 px-6 py-3 border-b border-border">
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
      <SelectTrigger className="w-auto h-7 text-xs bg-secondary border-border">
        <SelectValue>{SORT_LABELS[sortKey]}</SelectValue>
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
      <SelectTrigger className="w-auto h-7 text-xs bg-secondary border-border">
        <SelectValue>{WATCHED_LABELS[filters.watchedFilter]}</SelectValue>
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
      <SelectTrigger className="w-auto h-7 text-xs bg-secondary border-border">
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

    {/* 태그 필터 */}
    <TagPopover />

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
```

Key changes from current code:
- Container: `overflow-hidden` removed
- All `SelectTrigger`: fixed widths (`w-36`, `w-24`, `w-28`) → `w-auto`
- Quick tag badges, extraSelected badges, quick tag separator: all removed
- `TagPopover` rendered with no props (it reads from store)
- Tag popover conditional (`tags.length > QUICK_TAG_COUNT`) removed — always shown

2. Update props interface — remove `tags`:

```typescript
interface FilterBarProps {
  totalCount: number
  activeFilter: { type: string; value: string } | null
  onClearFilter: () => void
}
```

3. Update component signature:

```typescript
export default function FilterBar({
  totalCount,
  activeFilter,
  onClearFilter,
}: FilterBarProps) {
```

- [ ] **Step 2: Update LibraryPage to not pass tags prop**

In `src/pages/LibraryPage.tsx`, remove `tags={allTags}` from FilterBar:

```tsx
<FilterBar
  totalCount={filtered.length}
  activeFilter={activeFilter}
  onClearFilter={clearFilter}
/>
```

Also clean up: if `allTags` is no longer used in LibraryPage at all, remove it from the store destructure.

- [ ] **Step 3: Verify types compile**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 4: Commit**

```bash
git add src/components/library/FilterBar.tsx src/pages/LibraryPage.tsx
git commit -m "feat: remove quick tags from FilterBar, fix Select widths"
```

---

### Task 4: Update TagPopover — external trigger, show all tags

**Files:**
- Modify: `src/components/library/TagPopover.tsx` (props, trigger, search results)

- [ ] **Step 1: Update TagPopover to read allTags from store and show all tags**

In `src/components/library/TagPopover.tsx`:

1. Remove `TagPopoverProps` interface entirely
2. Remove `allTags` and `remainingCount` from props — read from store:

```typescript
export default function TagPopover() {
  const { filters, setFilters, allTags } = useLibraryStore()
```

3. Remove the `useTauriCommand` import and `run` — co-occurrence fetch uses `run`, so keep it. Actually check: `run` is used for `get_tag_cooccurrence`. Keep it.

4. Fix search results — remove the 20-item limit:

```typescript
const searchResults = useMemo(() => {
  if (!search.trim()) return allTags
  const q = search.trim().toLowerCase()
  return allTags.filter((t) => t.name.toLowerCase().includes(q))
}, [search, allTags])
```

5. Update section header — no longer "인기 태그" vs "검색 결과", just "전체 태그" vs "검색 결과":

```tsx
<div className="px-2.5 pt-1.5 pb-0.5 text-[10px] font-bold text-muted-foreground uppercase tracking-wider">
  {search.trim() ? '검색 결과' : '전체 태그'}
</div>
```

6. Replace the trigger button to show selected tag count:

```tsx
<PopoverPrimitive.Trigger
  className="inline-flex items-center gap-1 h-7 px-3 rounded-md text-xs border border-border bg-secondary cursor-pointer hover:bg-secondary/80 transition-colors whitespace-nowrap"
>
  태그 필터{selectedTagSet.size > 0 && ` (${selectedTagSet.size})`} {open ? '▴' : '▾'}
</PopoverPrimitive.Trigger>
```

When tags are selected, style the trigger to indicate active state:

```tsx
<PopoverPrimitive.Trigger
  className={cn(
    'inline-flex items-center gap-1 h-7 px-3 rounded-md text-xs cursor-pointer transition-colors whitespace-nowrap',
    selectedTagSet.size > 0
      ? 'border border-primary text-primary bg-primary/10 hover:bg-primary/20'
      : 'border border-border bg-secondary hover:bg-secondary/80'
  )}
>
  태그 필터{selectedTagSet.size > 0 && ` (${selectedTagSet.size})`} {open ? '▴' : '▾'}
</PopoverPrimitive.Trigger>
```

Add `cn` import:

```typescript
import { cn } from '@/lib/utils'
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 3: Commit**

```bash
git add src/components/library/TagPopover.tsx
git commit -m "feat: TagPopover reads from store, shows all tags, styled trigger"
```

---

### Task 5: Final cleanup and type check

**Files:**
- Modify: `src/stores/__tests__/libraryStore.test.ts` (update beforeEach to include new fields)

- [ ] **Step 1: Update store test reset to include allTags**

In `src/stores/__tests__/libraryStore.test.ts`, update `beforeEach`:

```typescript
beforeEach(() => {
  useLibraryStore.setState({
    videos: [],
    filters: DEFAULT_FILTERS,
    searchQuery: '',
    isScanning: false,
    allTags: [],
  })
})
```

- [ ] **Step 2: Run full type check**

Run: `npx tsc --noEmit`
Expected: exit code 0

- [ ] **Step 3: Run tests**

Run: `npx vitest run`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/stores/__tests__/libraryStore.test.ts
git commit -m "test: update store test reset for allTags field"
```
