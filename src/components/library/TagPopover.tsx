import { useState, useRef, useEffect, useCallback, useMemo } from 'react'
import { Popover as PopoverPrimitive } from '@base-ui/react/popover'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { X, Search, Plus } from 'lucide-react'
import { useLibraryStore } from '@/stores/libraryStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Tag, TagCooccurrence } from '@/types'

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
        <PopoverPrimitive.Positioner sideOffset={4} side="bottom" align="end">
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
