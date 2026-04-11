import { useState } from 'react'
import ImageLightbox from './ImageLightbox'
import { assetUrl } from '@/lib/utils'
import type { SampleImage } from '@/types'

interface SampleImageGridProps {
  images: SampleImage[]
}

export default function SampleImageGrid({ images }: SampleImageGridProps) {
  const [lightboxIdx, setLightboxIdx] = useState<number | null>(null)

  if (images.length === 0) return null

  return (
    <>
      <div className="space-y-2">
        <div className="flex justify-between items-center">
          <span className="text-sm text-foreground">샘플 이미지</span>
          <span className="text-xs text-muted-foreground">{images.length}장</span>
        </div>
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
