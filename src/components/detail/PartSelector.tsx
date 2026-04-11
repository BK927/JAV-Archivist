import { cn } from '@/lib/utils'

interface PartSelectorProps {
  totalParts: number
  currentPart: number
  onSelectPart: (index: number) => void
}

export default function PartSelector({ totalParts, currentPart, onSelectPart }: PartSelectorProps) {
  if (totalParts <= 1) return null

  return (
    <div className="flex gap-1">
      {Array.from({ length: totalParts }, (_, i) => (
        <button
          key={i}
          onClick={() => onSelectPart(i)}
          className={cn(
            'text-xs px-3 py-1 rounded transition-colors',
            i === currentPart
              ? 'bg-primary text-primary-foreground'
              : 'bg-white/10 text-white/60 hover:bg-white/20 hover:text-white/80'
          )}
        >
          Part {i + 1}
        </button>
      ))}
    </div>
  )
}
