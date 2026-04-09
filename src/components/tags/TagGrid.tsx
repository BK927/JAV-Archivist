import { Badge } from '@/components/ui/badge'
import type { Tag } from '@/types'

interface TagGridProps {
  tags: Tag[]
  onSelect: (tag: Tag) => void
}

export default function TagGrid({ tags, onSelect }: TagGridProps) {
  return (
    <div className="p-6 flex flex-wrap gap-3">
      {tags.map((tag) => (
        <button
          key={tag.id}
          onClick={() => onSelect(tag)}
          className="flex items-center gap-2 px-4 py-2 rounded-full bg-card border border-border hover:border-primary/50 transition-colors"
        >
          <span className="text-sm text-foreground">{tag.name}</span>
          <Badge variant="secondary" className="text-xs h-5">
            {tag.videoCount}
          </Badge>
        </button>
      ))}
    </div>
  )
}
