import VideoCard from './VideoCard'
import type { Video } from '@/types'

interface VideoGridProps {
  videos: Video[]
  onSelect: (video: Video) => void
}

export default function VideoGrid({ videos, onSelect }: VideoGridProps) {
  if (videos.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground text-sm">
        영상이 없습니다
      </div>
    )
  }

  return (
    <div
      className="grid gap-4 p-6"
      style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))' }}
    >
      {videos.map((video) => (
        <VideoCard key={video.id} video={video} onClick={onSelect} />
      ))}
    </div>
  )
}
