import { Badge } from '@/components/ui/badge'
import { useLibraryStore } from '@/stores/libraryStore'

interface TagGridProps {
  tags: string[]
  onSelect: (tag: string) => void
}

export default function TagGrid({ tags, onSelect }: TagGridProps) {
  const { videos } = useLibraryStore()

  const tagCounts = tags
    .map((tag) => ({
      tag,
      count: videos.filter((v) => v.tags.includes(tag)).length,
    }))
    .sort((a, b) => b.count - a.count)

  return (
    <div className="p-6 flex flex-wrap gap-3">
      {tagCounts.map(({ tag, count }) => (
        <button
          key={tag}
          onClick={() => onSelect(tag)}
          className="flex items-center gap-2 px-4 py-2 rounded-full bg-card border border-border hover:border-primary/50 transition-colors"
        >
          <span className="text-sm text-foreground">{tag}</span>
          <Badge variant="secondary" className="text-xs h-5">
            {count}
          </Badge>
        </button>
      ))}
    </div>
  )
}
