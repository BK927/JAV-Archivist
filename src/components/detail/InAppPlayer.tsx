import { useRef, useState } from 'react'
import { Play, Pause, X } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface InAppPlayerProps {
  filePath: string | undefined
  onClose: () => void
}

export default function InAppPlayer({ filePath, onClose }: InAppPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [playing, setPlaying] = useState(false)

  const toggle = () => {
    if (!videoRef.current) return
    if (playing) {
      videoRef.current.pause()
    } else {
      videoRef.current.play()
    }
    setPlaying(!playing)
  }

  // Tauri env: asset:// protocol; browser dev: empty src (video won't load, that's OK for mock)
  const src = filePath
    ? (window as any).__TAURI_INTERNALS__
      ? `asset://localhost/${filePath.replace(/\\/g, '/')}`
      : ''
    : ''

  return (
    <div className="relative bg-black rounded-md overflow-hidden">
      <video
        ref={videoRef}
        src={src}
        className="w-full aspect-video"
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
      />
      <div className="absolute top-2 right-2 flex gap-1">
        <Button size="icon" variant="ghost" className="h-7 w-7" onClick={toggle}>
          {playing ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
        </Button>
        <Button size="icon" variant="ghost" className="h-7 w-7" onClick={onClose}>
          <X className="w-4 h-4" />
        </Button>
      </div>
    </div>
  )
}
