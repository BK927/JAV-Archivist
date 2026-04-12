import { useEffect, useState } from 'react'
import { FolderOpen, Trash2, RefreshCw, AlertTriangle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { toast } from 'sonner'
import type { AppSettings, ScanResult } from '@/types'

const DEFAULT_SETTINGS: AppSettings = { scanFolders: [], playerPath: null, logEnabled: false, logLevel: 'info' }

const LOG_LEVEL_LABELS: Record<string, string> = { error: 'Error', warn: 'Warn', info: 'Info', debug: 'Debug' }

export default function SettingsPage() {
  const { run } = useTauriCommand()
  const { setScanning, isScanning } = useLibraryStore()
  const { setCurrentVideo } = usePlayerStore()
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS)
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [isResetting, setIsResetting] = useState(false)

  useEffect(() => {
    run<AppSettings>('get_settings', {}, DEFAULT_SETTINGS).then(setSettings)
  }, [run])

  const save = async (updated: AppSettings) => {
    setSettings(updated)
    await run('save_settings', { settings: updated }, undefined)
  }

  const addFolder = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ directory: true, title: '스캔 폴더 선택' })
      if (selected && !settings.scanFolders.includes(selected)) {
        save({ ...settings, scanFolders: [...settings.scanFolders, selected] })
      }
    } catch {
      // Not in Tauri env
    }
  }

  const removeFolder = (folder: string) => {
    save({ ...settings, scanFolders: settings.scanFolders.filter((f) => f !== folder) })
  }

  const handleRescan = async () => {
    setScanning(true)
    const result = await run<ScanResult>('scan_library', {}, { videos: [], added: [], removed: 0 })
    useLibraryStore.getState().setVideos(result.videos)
    if (result.added.length > 0) {
      useLibraryStore.getState().addNewVideoIds(result.added)
    }
    setScanning(false)
    const parts: string[] = []
    if (result.added.length > 0) parts.push(`${result.added.length}개 추가`)
    if (result.removed > 0) parts.push(`${result.removed}개 제거`)
    if (parts.length > 0) toast(parts.join(' · '))
  }

  return (
    <div className="max-w-xl p-8 space-y-8">
      <h1 className="text-lg font-semibold">설정</h1>

      {/* 스캔 폴더 */}
      <section className="space-y-3">
        <Label className="text-sm font-medium">스캔 폴더</Label>
        <div className="space-y-2">
          {settings.scanFolders.map((folder) => (
            <div key={folder} className="flex items-center gap-2">
              <span className="flex-1 text-sm text-muted-foreground truncate bg-secondary rounded px-3 py-1.5">
                {folder}
              </span>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 shrink-0"
                onClick={() => removeFolder(folder)}
              >
                <Trash2 className="w-3.5 h-3.5" />
              </Button>
            </div>
          ))}
        </div>
        <Button variant="secondary" size="sm" onClick={addFolder}>
          <FolderOpen className="w-3.5 h-3.5 mr-1" />
          폴더 추가
        </Button>
      </section>

      <Separator />

      {/* 외부 플레이어 */}
      <section className="space-y-3">
        <Label className="text-sm font-medium">외부 플레이어 경로</Label>
        <Input
          className="text-sm h-8 bg-secondary border-border"
          value={settings.playerPath ?? ''}
          onChange={(e) => save({ ...settings, playerPath: e.target.value || null })}
          placeholder="C:/Program Files/mpv/mpv.exe"
        />
      </section>

      <Separator />

      {/* 라이브러리 재스캔 */}
      <section className="space-y-3">
        <Label className="text-sm font-medium">라이브러리</Label>
        <Button
          variant="secondary"
          size="sm"
          onClick={handleRescan}
          disabled={isScanning}
        >
          <RefreshCw className={`w-3.5 h-3.5 mr-1 ${isScanning ? 'animate-spin' : ''}`} />
          {isScanning ? '스캔 중...' : '라이브러리 재스캔'}
        </Button>
      </section>

      <Separator />

      {/* 로그 */}
      <section className="space-y-3">
        <Label className="text-sm font-medium">로그</Label>
        <div className="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={settings.logEnabled}
            onClick={() => save({ ...settings, logEnabled: !settings.logEnabled })}
            className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors ${
              settings.logEnabled ? 'bg-primary' : 'bg-muted-foreground/30'
            }`}
          >
            <span
              className={`pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform mt-0.5 ${
                settings.logEnabled ? 'translate-x-[18px]' : 'translate-x-0.5'
              }`}
            />
          </button>
          <span className="text-sm">로그 활성화</span>
        </div>
        {settings.logEnabled && (
          <div className="space-y-2">
            <Label className="text-xs text-muted-foreground">로그 레벨</Label>
            <Select
              value={settings.logLevel}
              onValueChange={(v) => v && save({ ...settings, logLevel: v })}
            >
              <SelectTrigger className="w-32 h-8 text-sm bg-secondary border-border">
                <SelectValue>{LOG_LEVEL_LABELS[settings.logLevel] ?? settings.logLevel}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="error">Error</SelectItem>
                <SelectItem value="warn">Warn</SelectItem>
                <SelectItem value="info">Info</SelectItem>
                <SelectItem value="debug">Debug</SelectItem>
              </SelectContent>
            </Select>
          </div>
        )}
        <p className="text-xs text-muted-foreground">변경 시 앱 재시작 필요</p>
      </section>

      <Separator />

      {/* 데이터 초기화 */}
      <section className="space-y-3">
        <Label className="text-sm font-medium">데이터 관리</Label>
        <div className="space-y-2">
          <Button
            variant="destructive"
            size="sm"
            onClick={() => setShowResetConfirm(true)}
            disabled={isResetting}
          >
            <AlertTriangle className="w-3.5 h-3.5 mr-1" />
            {isResetting ? '초기화 중...' : '데이터 초기화'}
          </Button>
          <p className="text-xs text-muted-foreground">
            모든 비디오 메타데이터, 썸네일, 배우 사진을 삭제합니다. 설정과 스캔 폴더는 유지됩니다.
          </p>
        </div>
      </section>

      <Separator />

      {/* FFmpeg License Notice */}
      <div className="space-y-2">
        <h3 className="text-sm font-medium text-muted-foreground">오픈소스 라이선스</h3>
        <div className="text-xs text-muted-foreground space-y-1">
          <p>
            이 앱은 썸네일 생성 및 미리보기를 위해{' '}
            <a href="https://ffmpeg.org" target="_blank" rel="noopener noreferrer" className="underline">
              FFmpeg
            </a>
            를 사용합니다.
          </p>
          <p>
            FFmpeg is licensed under the{' '}
            <a
              href="https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html"
              target="_blank"
              rel="noopener noreferrer"
              className="underline"
            >
              GNU Lesser General Public License (LGPL) v2.1
            </a>
            .
          </p>
          <p>
            FFmpeg source code:{' '}
            <a href="https://ffmpeg.org/download.html" target="_blank" rel="noopener noreferrer" className="underline">
              https://ffmpeg.org/download.html
            </a>
          </p>
        </div>
      </div>

      {/* 초기화 확인 다이얼로그 */}
      {showResetConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div className="bg-background border border-border rounded-lg p-6 max-w-sm w-full mx-4 shadow-xl space-y-4">
            <div className="flex items-start gap-3">
              <AlertTriangle className="w-5 h-5 text-destructive shrink-0 mt-0.5" />
              <div className="space-y-2">
                <h3 className="text-sm font-semibold">데이터를 초기화하시겠습니까?</h3>
                <p className="text-xs text-muted-foreground">
                  모든 비디오 메타데이터, 썸네일, 배우 사진, 샘플 이미지가 삭제됩니다.
                  이 작업은 되돌릴 수 없습니다.
                </p>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowResetConfirm(false)}
              >
                취소
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={async () => {
                  setShowResetConfirm(false)
                  setIsResetting(true)
                  try {
                    await run('reset_data', {}, undefined)
                    useLibraryStore.getState().setVideos([])
                    setCurrentVideo(null)
                  } catch {
                    // error handled by tracing
                  } finally {
                    setIsResetting(false)
                  }
                }}
              >
                초기화
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
