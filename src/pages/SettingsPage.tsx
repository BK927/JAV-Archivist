import { useEffect, useState } from 'react'
import { Plus, Trash2, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import { useLibraryStore } from '@/stores/libraryStore'
import { MOCK_SETTINGS, MOCK_VIDEOS } from '@/lib/mockData'
import type { AppSettings, Video } from '@/types'

export default function SettingsPage() {
  const { run } = useTauriCommand()
  const { setVideos, setScanning, isScanning } = useLibraryStore()
  const [settings, setSettings] = useState<AppSettings>(MOCK_SETTINGS)
  const [newFolder, setNewFolder] = useState('')

  useEffect(() => {
    run<AppSettings>('get_settings', {}, MOCK_SETTINGS).then(setSettings)
  }, [run])

  const save = async (updated: AppSettings) => {
    setSettings(updated)
    await run('save_settings', { settings: updated }, undefined)
  }

  const addFolder = () => {
    if (!newFolder.trim()) return
    save({ ...settings, scanFolders: [...settings.scanFolders, newFolder.trim()] })
    setNewFolder('')
  }

  const removeFolder = (folder: string) => {
    save({ ...settings, scanFolders: settings.scanFolders.filter((f) => f !== folder) })
  }

  const handleRescan = async () => {
    setScanning(true)
    const videos = await run<Video[]>('scan_library', {}, MOCK_VIDEOS)
    setVideos(videos)
    setScanning(false)
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
        <div className="flex gap-2">
          <Input
            className="text-sm h-8 bg-secondary border-border"
            placeholder="C:/Videos"
            value={newFolder}
            onChange={(e) => setNewFolder(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && addFolder()}
          />
          <Button variant="secondary" size="sm" onClick={addFolder}>
            <Plus className="w-3.5 h-3.5 mr-1" />
            추가
          </Button>
        </div>
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
    </div>
  )
}
