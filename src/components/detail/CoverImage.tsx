import { Play } from 'lucide-react'
import { assetUrl } from '@/lib/utils'

interface CoverImageProps {
  thumbnailPath: string | null
  code: string
  onClick: () => void
}

export default function CoverImage({ thumbnailPath, code, onClick }: CoverImageProps) {
  return (
    <div className="w-[130px] shrink-0">
      <button
        onClick={onClick}
        className="w-full aspect-[2/3] bg-muted rounded-md overflow-hidden relative block"
      >
        {thumbnailPath ? (
          <>
            <img
              src={assetUrl(thumbnailPath)}
              alt=""
              aria-hidden
              className="absolute inset-0 w-full h-full object-cover blur-xl scale-110 opacity-50"
            />
            <img
              src={assetUrl(thumbnailPath)}
              alt={code}
              className="relative w-full h-full object-contain z-[1]"
            />
          </>
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-secondary">
            <Play className="w-8 h-8 text-muted-foreground/30" />
          </div>
        )}
      </button>
    </div>
  )
}
