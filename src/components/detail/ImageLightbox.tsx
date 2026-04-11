import { useEffect, useRef, useCallback } from 'react'
import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { assetUrl } from '@/lib/utils'
import type { SampleImage } from '@/types'

interface ImageLightboxProps {
  images: SampleImage[]
  currentIndex: number
  onIndexChange: (index: number) => void
  onClose: () => void
}

export default function ImageLightbox({ images, currentIndex, onIndexChange, onClose }: ImageLightboxProps) {
  const stripRef = useRef<HTMLDivElement>(null)

  const goPrev = useCallback(() => {
    if (currentIndex > 0) onIndexChange(currentIndex - 1)
  }, [currentIndex, onIndexChange])

  const goNext = useCallback(() => {
    if (currentIndex < images.length - 1) onIndexChange(currentIndex + 1)
  }, [currentIndex, images.length, onIndexChange])

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
      else if (e.key === 'ArrowLeft') goPrev()
      else if (e.key === 'ArrowRight') goNext()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose, goPrev, goNext])

  // Auto-scroll thumbnail strip to keep current image visible
  useEffect(() => {
    const strip = stripRef.current
    if (!strip) return
    const thumb = strip.children[currentIndex] as HTMLElement | undefined
    if (thumb) {
      thumb.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' })
    }
  }, [currentIndex])

  if (currentIndex < 0 || currentIndex >= images.length) return null

  return (
    <div
      className="fixed inset-0 bg-black/90 flex flex-col items-center justify-center z-50"
      onClick={onClose}
    >
      {/* Close button */}
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-muted-foreground hover:text-foreground z-[51]"
      >
        <X className="w-6 h-6" />
      </button>

      {/* Counter */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 text-muted-foreground text-sm">
        {currentIndex + 1} / {images.length}
      </div>

      {/* Prev button */}
      {currentIndex > 0 && (
        <button
          onClick={(e) => { e.stopPropagation(); goPrev() }}
          className="absolute left-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white transition-colors"
        >
          <ChevronLeft className="w-5 h-5" />
        </button>
      )}

      {/* Next button */}
      {currentIndex < images.length - 1 && (
        <button
          onClick={(e) => { e.stopPropagation(); goNext() }}
          className="absolute right-4 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white transition-colors"
        >
          <ChevronRight className="w-5 h-5" />
        </button>
      )}

      {/* Main image */}
      <img
        src={assetUrl(images[currentIndex].path)}
        alt={`Sample ${currentIndex + 1}`}
        className="max-w-[90vw] max-h-[75vh] object-contain"
        onClick={(e) => e.stopPropagation()}
      />

      {/* Thumbnail strip */}
      <div
        ref={stripRef}
        className="flex gap-1.5 mt-4 overflow-x-auto max-w-[90vw] px-2 pb-1"
        onClick={(e) => e.stopPropagation()}
      >
        {images.map((img, idx) => (
          <button
            key={img.id}
            onClick={() => onIndexChange(idx)}
            className={`shrink-0 w-12 h-8 rounded overflow-hidden border-2 transition-colors ${
              idx === currentIndex ? 'border-primary' : 'border-transparent opacity-60 hover:opacity-100'
            }`}
          >
            <img
              src={assetUrl(img.path)}
              alt={`Thumb ${idx + 1}`}
              className="w-full h-full object-cover"
            />
          </button>
        ))}
      </div>
    </div>
  )
}
