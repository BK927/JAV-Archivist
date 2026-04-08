import { useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Play, Star } from 'lucide-react'
import type { Video } from '@/types'
import { cn } from '@/lib/utils'

interface VideoCardProps {
  video: Video
  onClick: (video: Video) => void
}

export default function VideoCard({ video, onClick }: VideoCardProps) {
  const [hovered, setHovered] = useState(false)

  return (
    <button
      className="group relative w-full text-left rounded-md overflow-hidden bg-card border border-border hover:border-primary/50 transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      onClick={() => onClick(video)}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* 썸네일 영역 */}
      <div className="relative aspect-[2/3] bg-muted overflow-hidden">
        {video.thumbnailPath ? (
          <img
            src={video.thumbnailPath}
            alt={video.code}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-secondary">
            <Play className="w-8 h-8 text-muted-foreground/30" />
          </div>
        )}

        {/* 품번 배지 - 좌상단 */}
        <Badge
          className="absolute top-1.5 left-1.5 bg-primary text-primary-foreground text-[10px] px-1.5 py-0.5 font-mono font-bold"
          variant="default"
        >
          {video.code}
        </Badge>

        {/* 즐겨찾기 배지 - 우상단 */}
        {video.favorite && (
          <div className="absolute top-1.5 right-1.5">
            <Star className="w-3.5 h-3.5 fill-primary text-primary" />
          </div>
        )}

        {/* 호버 재생 오버레이 */}
        <div
          className={cn(
            'absolute inset-0 bg-black/60 flex items-center justify-center transition-opacity',
            hovered ? 'opacity-100' : 'opacity-0'
          )}
        >
          <Play className="w-10 h-10 text-white" />
        </div>
      </div>

      {/* 카드 하단 정보 */}
      <div className="p-2 space-y-0.5">
        <p className="text-xs text-foreground line-clamp-2 leading-snug">
          {video.title}
        </p>
        <p className="text-[11px] text-muted-foreground truncate">
          {video.actors.join(', ')}
        </p>
      </div>
    </button>
  )
}
