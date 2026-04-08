import { useState } from 'react'
import { ArrowLeft, Play, Star, Monitor } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import InAppPlayer from './InAppPlayer'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import type { Video } from '@/types'

interface VideoDetailProps {
  video: Video
  onClose: () => void
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return h > 0 ? `${h}시간 ${m}분` : `${m}분`
}

export default function VideoDetail({ video, onClose }: VideoDetailProps) {
  const [showPlayer, setShowPlayer] = useState(false)
  const { run } = useTauriCommand()
  const { videos, setVideos } = useLibraryStore()

  const handleExternalPlay = async () => {
    await run('open_with_player', { filePath: video.files[0]?.path ?? '' }, undefined)
    const updated = videos.map((v) =>
      v.id === video.id ? { ...v, watched: true } : v
    )
    setVideos(updated)
  }

  const handleToggleFavorite = async () => {
    await run('toggle_favorite', { id: video.id }, undefined)
    const updated = videos.map((v) =>
      v.id === video.id ? { ...v, favorite: !v.favorite } : v
    )
    setVideos(updated)
  }

  return (
    <div className="flex flex-col h-full overflow-auto p-6 space-y-6">
      {/* 뒤로가기 */}
      <Button variant="ghost" size="sm" onClick={onClose} className="w-fit -ml-2">
        <ArrowLeft className="w-4 h-4 mr-1" />
        라이브러리
      </Button>

      <div className="flex gap-6">
        {/* 포스터 */}
        <div className="w-36 shrink-0">
          <div className="aspect-[2/3] bg-muted rounded-md flex items-center justify-center">
            {video.thumbnailPath ? (
              <img
                src={video.thumbnailPath}
                alt={video.code}
                className="w-full h-full object-cover rounded-md"
              />
            ) : (
              <Play className="w-8 h-8 text-muted-foreground/30" />
            )}
          </div>
        </div>

        {/* 메타데이터 */}
        <div className="flex-1 space-y-3">
          <div>
            <Badge className="bg-primary text-primary-foreground font-mono mb-1">
              {video.code}
            </Badge>
            <h1 className="text-lg font-semibold leading-snug">{video.title}</h1>
          </div>

          <div className="space-y-1 text-sm text-muted-foreground">
            <p><span className="text-foreground">배우</span>: {video.actors.join(', ')}</p>
            {video.series && (
              <p><span className="text-foreground">시리즈</span>: {video.series}</p>
            )}
            {video.releasedAt && (
              <p><span className="text-foreground">출시일</span>: {video.releasedAt}</p>
            )}
            <p><span className="text-foreground">재생시간</span>: {video.duration != null ? formatDuration(video.duration) : '-'}</p>
          </div>

          {video.tags.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {video.tags.map((tag) => (
                <Badge key={tag} variant="secondary" className="text-xs">
                  {tag}
                </Badge>
              ))}
            </div>
          )}

          <Separator />

          {/* 액션 버튼 */}
          <div className="flex gap-2">
            <Button onClick={handleExternalPlay} size="sm">
              <Monitor className="w-4 h-4 mr-1" />
              외부 재생
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowPlayer(!showPlayer)}
            >
              <Play className="w-4 h-4 mr-1" />
              프리뷰
            </Button>
            <Button
              variant={video.favorite ? 'default' : 'outline'}
              size="sm"
              onClick={handleToggleFavorite}
            >
              <Star
                className={`w-4 h-4 mr-1 ${video.favorite ? 'fill-current' : ''}`}
              />
              즐겨찾기
            </Button>
          </div>
        </div>
      </div>

      {/* 인앱 플레이어 */}
      {showPlayer && (
        <InAppPlayer
          filePath={video.files[0]?.path}
          onClose={() => setShowPlayer(false)}
        />
      )}
    </div>
  )
}
