import { Film } from 'lucide-react'
import { assetUrl } from '@/lib/utils'
import type { Series } from '@/types'

interface SeriesGridProps {
  series: Series[]
  onSelect: (series: Series) => void
}

export default function SeriesGrid({ series, onSelect }: SeriesGridProps) {
  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))' }}
    >
      {series.map((s) => (
        <button
          key={s.id}
          onClick={() => onSelect(s)}
          className="flex flex-col rounded-md bg-card border border-border hover:border-primary/50 transition-colors overflow-hidden text-left"
        >
          <div className="aspect-video bg-secondary flex items-center justify-center">
            {s.coverPath ? (
              <img src={assetUrl(s.coverPath)} alt={s.name} className="w-full h-full object-cover" />
            ) : (
              <Film className="w-8 h-8 text-muted-foreground/30" />
            )}
          </div>
          <div className="p-2">
            <p className="text-xs font-medium text-foreground truncate">{s.name}</p>
            <p className="text-[11px] text-muted-foreground">{s.videoCount}편</p>
          </div>
        </button>
      ))}
    </div>
  )
}
