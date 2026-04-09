import { User } from 'lucide-react'
import type { Actor } from '@/types'

interface ActorGridProps {
  actors: Actor[]
  onSelect: (actor: Actor) => void
}

export default function ActorGrid({ actors, onSelect }: ActorGridProps) {
  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(110px, 1fr))' }}
    >
      {actors.map((actor) => (
        <button
          key={actor.id}
          onClick={() => onSelect(actor)}
          className="flex flex-col items-center gap-2 p-3 rounded-md bg-card border border-border hover:border-primary/50 transition-colors"
        >
          <div className="w-16 h-16 rounded-full bg-secondary flex items-center justify-center overflow-hidden">
            {actor.photoPath ? (
              <img src={actor.photoPath} alt={actor.name} className="w-full h-full object-cover" />
            ) : (
              <User className="w-7 h-7 text-muted-foreground/40" />
            )}
          </div>
          <span className="text-xs text-center text-foreground leading-snug line-clamp-2">
            {actor.name}
          </span>
          {actor.nameKanji && (
            <span className="text-[10px] text-center text-muted-foreground leading-snug line-clamp-1">
              {actor.nameKanji}
            </span>
          )}
          <span className="text-[11px] text-muted-foreground">{actor.videoCount}편</span>
        </button>
      ))}
    </div>
  )
}
