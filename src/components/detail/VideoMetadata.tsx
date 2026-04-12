import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { Star, FolderOpen, Download, User } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import { isUnidentified, displayCode } from '@/types'
import type { Video, Actor } from '@/types'
import { assetUrl, cn } from '@/lib/utils'

interface VideoMetadataProps {
  video: Video
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return h > 0 ? `${h}시간 ${m}분` : `${m}분`
}

export default function VideoMetadata({ video }: VideoMetadataProps) {
  const { run } = useTauriCommand()
  const { videos, setVideos } = useLibraryStore()
  const navigate = useNavigate()
  const [actorDetails, setActorDetails] = useState<Actor[]>([])
  const [isScraping, setIsScraping] = useState(false)

  useEffect(() => {
    run<Actor[]>('get_actors', {}, []).then((all) => {
      setActorDetails(all.filter((a) => video.actors.includes(a.name)))
    })
  }, [video.actors, run])

  const handleToggleFavorite = async () => {
    await run('toggle_favorite', { id: video.id }, undefined)
    setVideos(videos.map((v) => v.id === video.id ? { ...v, favorite: !v.favorite } : v))
  }

  const handleOpenFolder = async () => {
    const filePath = video.files[0]?.path
    if (filePath) {
      await run('open_folder', { filePath }, undefined)
    }
  }

  const handleScrape = async () => {
    setIsScraping(true)
    try {
      const updated = await run<Video | null>('scrape_video', { videoId: video.id }, null)
      if (updated) {
        setVideos(videos.map((v) => v.id === updated.id ? updated : v))
      }
    } finally {
      setIsScraping(false)
    }
  }

  return (
    <div className="flex-1 space-y-3">
      {/* Code + scrape status */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <Badge className="bg-primary text-primary-foreground font-mono">{displayCode(video)}</Badge>
          {video.scrapeStatus !== 'not_scraped' && (
            <Badge
              variant="outline"
              className={cn(
                'text-xs',
                video.scrapeStatus === 'complete' && 'border-green-600 text-green-400',
                video.scrapeStatus === 'partial' && 'border-orange-600 text-orange-400',
                video.scrapeStatus === 'not_found' && 'border-red-600 text-red-400',
              )}
            >
              {video.scrapeStatus === 'complete' ? '수집 완료' : video.scrapeStatus === 'partial' ? '부분 수집' : '실패'}
            </Badge>
          )}
          {isUnidentified(video) && (
            <span className="border border-muted-foreground/30 text-muted-foreground text-xs px-2 py-0.5 rounded">
              미식별
            </span>
          )}
        </div>
        <h1 className="text-lg font-semibold leading-snug">{video.title}</h1>
      </div>

      {/* Metadata fields */}
      <div className="space-y-1 text-sm text-muted-foreground">
        {/* Actors */}
        {video.actors.length > 0 && (
          <div>
            <span className="text-foreground text-sm">배우</span>
            <div className="flex flex-wrap gap-3 mt-2">
              {video.actors.map((name) => {
                const detail = actorDetails.find((a) => a.name === name)
                return (
                  <button
                    key={name}
                    onClick={() => navigate(`/library?actor=${encodeURIComponent(name)}`)}
                    className="flex items-center gap-2 hover:bg-secondary/50 rounded px-2 py-1 transition-colors"
                  >
                    <div className="w-8 h-8 rounded-full bg-secondary flex items-center justify-center overflow-hidden shrink-0">
                      {detail?.photoPath ? (
                        <img src={assetUrl(detail.photoPath)} alt={name} className="w-full h-full object-cover" />
                      ) : (
                        <User className="w-4 h-4 text-muted-foreground/40" />
                      )}
                    </div>
                    <div className="text-left">
                      <p className="text-sm text-foreground leading-tight">{name}</p>
                      {detail?.nameKanji && (
                        <p className="text-[10px] text-muted-foreground leading-tight">{detail.nameKanji}</p>
                      )}
                    </div>
                  </button>
                )
              })}
            </div>
          </div>
        )}
        {video.series && (
          <p><span className="text-foreground">시리즈</span>: {video.series}</p>
        )}
        {video.makerName && (
          <p>
            <span className="text-foreground">제작사</span>:{' '}
            <button
              onClick={() => navigate(`/library?maker=${encodeURIComponent(video.makerName!)}`)}
              className="hover:text-foreground transition-colors underline"
            >
              {video.makerName}
            </button>
          </p>
        )}
        {video.releasedAt && (
          <p><span className="text-foreground">출시일</span>: {video.releasedAt}</p>
        )}
        <p><span className="text-foreground">재생시간</span>: {video.duration != null ? formatDuration(video.duration) : '-'}</p>
      </div>

      {/* Tags */}
      {video.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {video.tags.map((tag) => (
            <Badge key={tag} variant="secondary" className="text-xs">{tag}</Badge>
          ))}
        </div>
      )}

      <Separator />

      {/* Action buttons */}
      <div className="flex gap-2">
        <Button
          variant={video.favorite ? 'default' : 'outline'}
          size="sm"
          onClick={handleToggleFavorite}
        >
          <Star className={`w-4 h-4 mr-1 ${video.favorite ? 'fill-current' : ''}`} />
          즐겨찾기
        </Button>
        <Button variant="outline" size="sm" onClick={handleOpenFolder}>
          <FolderOpen className="w-4 h-4 mr-1" />
          폴더 열기
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={handleScrape}
          disabled={isScraping}
        >
          <Download className={`w-4 h-4 mr-1 ${isScraping ? 'animate-spin' : ''}`} />
          {isScraping ? '수집 중...' : video.scrapeStatus === 'not_scraped' ? '메타데이터 수집' : '재수집'}
        </Button>
      </div>
    </div>
  )
}
