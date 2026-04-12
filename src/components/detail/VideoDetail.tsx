import { useState, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import CoverImage from './CoverImage'
import CoverOverlay from './CoverOverlay'
import VideoMetadata from './VideoMetadata'
import FilePartsList from './FilePartsList'
import SampleImageGrid from './SampleImageGrid'
import MiniPreview from './MiniPreview'
import CinemaPlayer from './CinemaPlayer'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import type { Video, SampleImage } from '@/types'

interface VideoDetailProps {
  video: Video
  onClose: () => void
}

export default function VideoDetail({ video, onClose }: VideoDetailProps) {
  const [cinemaPartIndex, setCinemaPartIndex] = useState<number | null>(null)
  const [showCover, setShowCover] = useState(false)
  const [sampleImages, setSampleImages] = useState<SampleImage[]>([])
  const { run } = useTauriCommand()
  const { videos, setVideos } = useLibraryStore()

  useEffect(() => {
    run<SampleImage[]>('get_sample_images', { videoId: video.id }, []).then(setSampleImages)
  }, [video.id, run])

  const handlePlayExternal = async (filePath: string) => {
    await run('open_with_player', { filePath }, undefined)
    setVideos(videos.map((v) => v.id === video.id ? { ...v, watched: true } : v))
  }

  const handleEnterCinema = (fileIndex: number) => {
    setCinemaPartIndex(fileIndex)
  }

  // Cinema Mode
  if (cinemaPartIndex !== null) {
    return (
      <CinemaPlayer
        files={video.files}
        initialPartIndex={cinemaPartIndex}
        videoId={video.id}
        videoCode={video.code}
        videoTitle={video.title}
        onExit={() => setCinemaPartIndex(null)}
      />
    )
  }

  // Info Mode
  return (
    <div className="flex flex-col h-full overflow-auto p-6 space-y-6">
      {/* Back button */}
      <Button variant="ghost" size="sm" onClick={onClose} className="w-fit -ml-2">
        <ArrowLeft className="w-4 h-4 mr-1" />
        라이브러리
      </Button>

      {/* Cover + Metadata */}
      <div className="flex gap-6">
        <CoverImage
          thumbnailPath={video.thumbnailPath}
          code={video.code}
          onClick={() => video.thumbnailPath && setShowCover(true)}
        />
        <VideoMetadata video={video} />
      </div>

      {/* File Parts */}
      <FilePartsList
        files={video.files}
        onPlayCinema={handleEnterCinema}
        onPlayExternal={handlePlayExternal}
      />

      {/* Sample Images */}
      <SampleImageGrid images={sampleImages} />

      {/* Mini Preview */}
      <MiniPreview
        filePath={video.files[0]?.path}
        onEnterCinema={() => handleEnterCinema(0)}
      />

      {/* Cover Overlay */}
      {showCover && video.thumbnailPath && (
        <CoverOverlay
          thumbnailPath={video.thumbnailPath}
          onClose={() => setShowCover(false)}
        />
      )}
    </div>
  )
}
