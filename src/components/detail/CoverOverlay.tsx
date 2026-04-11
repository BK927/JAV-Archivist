import { useEffect } from 'react'
import { X } from 'lucide-react'
import { assetUrl } from '@/lib/utils'

interface CoverOverlayProps {
  thumbnailPath: string
  onClose: () => void
}

export default function CoverOverlay({ thumbnailPath, onClose }: CoverOverlayProps) {
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose])

  return (
    <div
      className="fixed inset-0 bg-black/85 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-muted-foreground hover:text-foreground z-[51]"
      >
        <X className="w-6 h-6" />
      </button>
      <img
        src={assetUrl(thumbnailPath)}
        alt="Cover"
        className="max-w-[90vw] max-h-[90vh] object-contain"
        onClick={(e) => e.stopPropagation()}
      />
    </div>
  )
}
