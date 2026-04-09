import { Factory } from 'lucide-react'
import type { Maker } from '@/types'

interface MakerGridProps {
  makers: Maker[]
  onSelect: (maker: Maker) => void
}

export default function MakerGrid({ makers, onSelect }: MakerGridProps) {
  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))' }}
    >
      {makers.map((maker) => (
        <button
          key={maker.id}
          onClick={() => onSelect(maker)}
          className="flex flex-col rounded-md bg-card border border-border hover:border-primary/50 transition-colors overflow-hidden text-left"
        >
          <div className="aspect-video bg-secondary flex items-center justify-center">
            <Factory className="w-8 h-8 text-muted-foreground/30" />
          </div>
          <div className="p-2">
            <p className="text-xs font-medium text-foreground truncate">{maker.name}</p>
            <p className="text-[11px] text-muted-foreground">{maker.videoCount}편</p>
          </div>
        </button>
      ))}
    </div>
  )
}
