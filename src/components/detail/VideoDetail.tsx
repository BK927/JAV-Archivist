import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { ArrowLeft, Play, Star, Monitor, User, Download } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import InAppPlayer from './InAppPlayer'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import type { Video, SampleImage, Actor } from '@/types'

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
  const navigate = useNavigate()
  const [actorDetails, setActorDetails] = useState<Actor[]>([])
  const [sampleImages, setSampleImages] = useState<SampleImage[]>([])
  const [lightboxIdx, setLightboxIdx] = useState<number | null>(null)
  const [isScraping, setIsScraping] = useState(false)

  useEffect(() => {
    run<Actor[]>('get_actors', {}, []).then((all) => {
      setActorDetails(all.filter((a) => video.actors.includes(a.name)))
    })
  }, [video.actors, run])

  useEffect(() => {
    run<SampleImage[]>('get_sample_images', { videoId: video.id }, []).then(setSampleImages)
  }, [video.id, run])

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

  const handleScrape = async () => {
    setIsScraping(true)
    try {
      const updated = await run<Video>('scrape_video', { videoId: video.id }, undefined)
      if (updated) {
        const newVideos = videos.map((v) => v.id === updated.id ? updated : v)
        setVideos(newVideos)
      }
    } finally {
      setIsScraping(false)
    }
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
            {/* 배우 — with photos and kanji */}
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
                            <img src={detail.photoPath} alt={name} className="w-full h-full object-cover" />
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
            {(video.scrapeStatus === 'not_scraped' || video.scrapeStatus === 'not_found') && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleScrape}
                disabled={isScraping}
              >
                <Download className={`w-4 h-4 mr-1 ${isScraping ? 'animate-spin' : ''}`} />
                {isScraping ? '수집 중...' : '메타데이터 수집'}
              </Button>
            )}
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

      {/* 샘플 이미지 갤러리 */}
      {sampleImages.length > 0 && (
        <div className="space-y-2">
          <span className="text-sm text-foreground">샘플 이미지</span>
          <div className="flex gap-2 overflow-x-auto pb-2">
            {sampleImages.map((img, idx) => (
              <button
                key={img.id}
                onClick={() => setLightboxIdx(idx)}
                className="shrink-0 w-24 h-16 rounded overflow-hidden border border-border hover:border-primary/50 transition-colors"
              >
                <img
                  src={img.path}
                  alt={`Sample ${idx + 1}`}
                  className="w-full h-full object-cover"
                />
              </button>
            ))}
          </div>
        </div>
      )}

      {/* 라이트박스 */}
      {lightboxIdx !== null && lightboxIdx < sampleImages.length && (
        <div
          className="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
          onClick={() => setLightboxIdx(null)}
        >
          <img
            src={sampleImages[lightboxIdx].path}
            alt="Sample"
            className="max-w-[90vw] max-h-[90vh] object-contain"
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
    </div>
  )
}
