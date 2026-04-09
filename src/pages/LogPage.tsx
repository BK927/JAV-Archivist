import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useLogStore } from '@/stores/logStore'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { AppSettings } from '@/types'

const DEFAULT_SETTINGS: AppSettings = { scanFolders: [], playerPath: null, logEnabled: false, logLevel: 'info' }

const LEVEL_LABELS: Record<string, string> = {
  ALL: '전체',
  ERROR: 'Error',
  WARN: 'Warn',
  INFO: 'Info',
  DEBUG: 'Debug',
}

const LEVEL_COLORS: Record<string, string> = {
  ERROR: 'text-red-500',
  WARN: 'text-yellow-500',
  INFO: 'text-foreground',
  DEBUG: 'text-muted-foreground',
}

export default function LogPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const { entries, filterLevel, clear, setFilterLevel } = useLogStore()
  const [logEnabled, setLogEnabled] = useState<boolean | null>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    run<AppSettings>('get_settings', {}, DEFAULT_SETTINGS).then((s) =>
      setLogEnabled(s.logEnabled)
    )
  }, [run])

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [entries])

  const filtered =
    filterLevel === 'ALL' ? entries : entries.filter((e) => e.level === filterLevel)

  if (logEnabled === null) return null

  if (!logEnabled) {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-sm text-muted-foreground">
          설정에서 로그를 활성화하세요{' '}
          <button
            onClick={() => navigate('/settings')}
            className="underline text-foreground hover:opacity-70"
          >
            설정으로 이동
          </button>
        </p>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Top bar */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-border shrink-0">
        <Select
          value={filterLevel}
          onValueChange={(v) => v && setFilterLevel(v as typeof filterLevel)}
        >
          <SelectTrigger className="w-28 h-8 text-sm bg-secondary border-border">
            <SelectValue>{LEVEL_LABELS[filterLevel]}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="ALL">전체</SelectItem>
            <SelectItem value="ERROR">Error</SelectItem>
            <SelectItem value="WARN">Warn</SelectItem>
            <SelectItem value="INFO">Info</SelectItem>
            <SelectItem value="DEBUG">Debug</SelectItem>
          </SelectContent>
        </Select>
        <Button variant="secondary" size="sm" onClick={clear}>
          로그 지우기
        </Button>
      </div>

      {/* Log list */}
      <div className="flex-1 overflow-y-auto px-4 py-2 font-mono text-xs">
        {filtered.map((entry, i) => (
          <div key={i} className="flex gap-2 py-0.5 leading-5">
            <span className="text-muted-foreground shrink-0">{entry.timestamp}</span>
            <span className={`shrink-0 w-12 ${LEVEL_COLORS[entry.level] ?? 'text-foreground'}`}>
              {entry.level}
            </span>
            <span className="text-muted-foreground shrink-0 truncate max-w-[160px]">
              {entry.target}
            </span>
            <span className="text-foreground break-all">{entry.message}</span>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  )
}
