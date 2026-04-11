import { useRef, useEffect } from 'react'
import { Play, VolumeX } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { assetUrl } from '@/lib/utils'

interface MiniPreviewProps {
  filePath: string | undefined
  onEnterCinema: () => void
}

export default function MiniPreview({ filePath, onEnterCinema }: MiniPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    const video = videoRef.current
    if (!video || !filePath) return
    video.muted = true
    video.play().catch(() => {})
  }, [filePath])

  if (!filePath) return null

  const src = assetUrl(filePath)
  if (!src) return null

  return (
    <div className="relative rounded-lg overflow-hidden border border-border">
      <video
        ref={videoRef}
        src={src}
        muted
        loop
        className="w-full aspect-video bg-black"
      />
      <div className="absolute bottom-3 left-3 flex items-center gap-1.5 text-muted-foreground">
        <VolumeX className="w-4 h-4" />
        <span className="text-xs">음소거</span>
      </div>
      <Button
        size="sm"
        className="absolute bottom-3 right-3"
        onClick={onEnterCinema}
      >
        <Play className="w-3.5 h-3.5 mr-1" />
        Cinema Mode
      </Button>
    </div>
  )
}
