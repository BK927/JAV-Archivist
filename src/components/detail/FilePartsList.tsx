import { Play, Monitor } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { VideoFile } from '@/types'

interface FilePartsListProps {
  files: VideoFile[]
  onPlayCinema: (fileIndex: number) => void
  onPlayExternal: (filePath: string) => void
}

function formatSize(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

export default function FilePartsList({ files, onPlayCinema, onPlayExternal }: FilePartsListProps) {
  if (files.length === 0) return null

  const totalSize = files.reduce((sum, f) => sum + f.size, 0)

  return (
    <div className="bg-card border border-border rounded-lg p-3">
      <div className="flex justify-between items-center mb-2">
        <span className="text-sm text-foreground">파일</span>
        <span className="text-xs text-muted-foreground">
          {files.length > 1 ? `${files.length}파트 · ` : ''}{formatSize(totalSize)}
        </span>
      </div>
      <div className="space-y-1.5">
        {files.map((file, idx) => (
          <div
            key={file.path}
            className="flex items-center gap-3 px-2.5 py-2 bg-secondary/50 rounded-md"
          >
            {files.length > 1 && (
              <span className="text-primary font-bold text-sm w-4 text-center">{idx + 1}</span>
            )}
            <div className="flex-1 min-w-0">
              <p className="text-sm text-foreground truncate">{fileName(file.path)}</p>
              <p className="text-xs text-muted-foreground">{formatSize(file.size)}</p>
            </div>
            <Button size="xs" onClick={() => onPlayCinema(idx)}>
              <Play className="w-3 h-3 mr-1" />
              Cinema
            </Button>
            <Button size="xs" variant="outline" onClick={() => onPlayExternal(file.path)}>
              <Monitor className="w-3 h-3 mr-1" />
              External
            </Button>
          </div>
        ))}
      </div>
    </div>
  )
}
