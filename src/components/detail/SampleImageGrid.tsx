import { useState } from 'react'
import { ImagePlus, Loader2, AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import ImageLightbox from './ImageLightbox'
import { assetUrl } from '@/lib/utils'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { SampleImage } from '@/types'

interface SampleImageGridProps {
  images: SampleImage[]
  videoId: string
  onImagesUpdated: (images: SampleImage[]) => void
}

export default function SampleImageGrid({ images, videoId, onImagesUpdated }: SampleImageGridProps) {
  const [lightboxIdx, setLightboxIdx] = useState<number | null>(null)
  const [extracting, setExtracting] = useState(false)
  const [error, setError] = useState(false)
  const { run } = useTauriCommand()

  const handleExtract = async () => {
    setExtracting(true)
    setError(false)
    try {
      const result = await run<SampleImage[]>('generate_local_samples', { videoId }, [])
      if (result.length === 0) {
        setError(true)
        setTimeout(() => setError(false), 3000)
      } else {
        onImagesUpdated(result)
      }
    } catch {
      setError(true)
      setTimeout(() => setError(false), 3000)
    } finally {
      setExtracting(false)
    }
  }

  return (
    <>
      <div className="space-y-2">
        <div className="flex justify-between items-center">
          <span className="text-sm text-foreground">샘플 이미지</span>
          <div className="flex items-center gap-2">
            {images.length > 0 && (
              <span className="text-xs text-muted-foreground">{images.length}장</span>
            )}
            <Button
              variant="ghost"
              size="sm"
              className={`h-6 text-xs gap-1 ${error ? 'text-destructive' : ''}`}
              onClick={handleExtract}
              disabled={extracting}
            >
              {extracting ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : error ? (
                <AlertCircle className="w-3 h-3" />
              ) : (
                <ImagePlus className="w-3 h-3" />
              )}
              {extracting ? '추출 중...' : error ? '추출 실패' : '로컬 추출'}
            </Button>
          </div>
        </div>
        {images.length > 0 && (
          <div className="grid grid-cols-5 gap-1.5">
            {images.map((img, idx) => (
              <button
                key={img.id}
                onClick={() => setLightboxIdx(idx)}
                className="aspect-video rounded overflow-hidden border border-border hover:border-primary/50 transition-colors"
              >
                <img
                  src={assetUrl(img.path)}
                  alt={`Sample ${idx + 1}`}
                  className="w-full h-full object-cover"
                  loading="lazy"
                />
              </button>
            ))}
          </div>
        )}
      </div>

      {lightboxIdx !== null && (
        <ImageLightbox
          images={images}
          currentIndex={lightboxIdx}
          onIndexChange={setLightboxIdx}
          onClose={() => setLightboxIdx(null)}
        />
      )}
    </>
  )
}
