import { useState, useRef, useCallback } from 'react'
import { Badge } from '@/components/ui/badge'
import { Play, Star } from 'lucide-react'
import type { Video } from '@/types'
import { cn, assetUrl } from '@/lib/utils'

interface VideoCardProps {
  video: Video
  onClick: (video: Video) => void
  selectionMode: boolean
  selected: boolean
  onToggleSelect: (id: string) => void
  onLongPress: (id: string) => void
}

export default function VideoCard({ video, onClick, selectionMode, selected, onToggleSelect, onLongPress }: VideoCardProps) {
  const [hovered, setHovered] = useState(false)
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const didLongPress = useRef(false)

  const handleClick = () => {
    if (didLongPress.current) return
    if (selectionMode) {
      onToggleSelect(video.id)
    } else {
      onClick(video)
    }
  }

  const handlePointerDown = useCallback(() => {
    didLongPress.current = false
    longPressTimer.current = setTimeout(() => {
      didLongPress.current = true
      onLongPress(video.id)
    }, 500)
  }, [video.id, onLongPress])

  const handlePointerUp = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current)
      longPressTimer.current = null
    }
  }, [])

  return (
    <button
      className={cn(
        'group relative w-full text-left rounded-md bg-card border transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        selected ? 'border-primary shadow-lg z-10' : hovered ? 'border-primary/50 shadow-lg z-10' : 'border-border z-0'
      )}
      onClick={handleClick}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
      onContextMenu={(e) => e.preventDefault()}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* 썸네일 영역 */}
      <div className="relative aspect-[800/538] bg-muted overflow-hidden rounded-t-md">
        {selectionMode && (
          <div className={cn(
            'absolute top-1.5 left-1.5 z-[4] w-5 h-5 rounded border-2 flex items-center justify-center text-xs',
            selected
              ? 'bg-primary border-primary text-primary-foreground'
              : 'border-muted-foreground/50 bg-black/30'
          )}>
            {selected && '✓'}
          </div>
        )}

        {video.thumbnailPath ? (
          <>
            {/* 블러 배경 레이어 — 비율 불일치 시 여백 채움 */}
            <img
              src={assetUrl(video.thumbnailPath)}
              alt=""
              aria-hidden
              className="absolute inset-0 w-full h-full object-cover blur-xl scale-110 opacity-50"
            />
            {/* 실제 이미지 — 잘림 없이 전체 표시 */}
            <img
              src={assetUrl(video.thumbnailPath)}
              alt={video.code}
              className="relative w-full h-full object-contain z-[1]"
              loading="lazy"
            />
          </>
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-secondary">
            <Play className="w-8 h-8 text-muted-foreground/30" />
          </div>
        )}

        {/* 품번 배지 - 좌상단 */}
        <Badge
          className={cn(
            'absolute top-1.5 z-[2] bg-primary text-primary-foreground text-[10px] px-1.5 py-0.5 font-mono font-bold',
            selectionMode ? 'left-8' : 'left-1.5'
          )}
          variant="default"
        >
          {video.code}
        </Badge>

        {/* 즐겨찾기 배지 - 우상단 */}
        {video.favorite && (
          <div className="absolute top-1.5 right-1.5 z-[2]">
            <Star className="w-3.5 h-3.5 fill-primary text-primary" />
          </div>
        )}

        {/* 호버 재생 오버레이 */}
        {!selectionMode && (
          <div
            className={cn(
              'absolute inset-0 z-[3] bg-black/60 flex items-center justify-center transition-opacity',
              hovered ? 'opacity-100' : 'opacity-0'
            )}
          >
            <Play className="w-10 h-10 text-white" />
          </div>
        )}
      </div>

      {/* 카드 하단 정보 — 호버 시 확장 */}
      <div
        className={cn(
          'p-2 space-y-0.5 bg-card rounded-b-md',
          hovered && 'absolute left-0 right-0 top-full -mt-[1px] border border-t-0 border-primary/50 rounded-t-none shadow-lg'
        )}
      >
        <p
          className={cn(
            'text-xs text-foreground leading-snug',
            !hovered && 'line-clamp-2'
          )}
        >
          {video.title}
        </p>
        <p
          className={cn(
            'text-[11px] text-muted-foreground',
            !hovered && 'truncate'
          )}
        >
          {video.actors.join(', ')}
        </p>
      </div>
    </button>
  )
}
