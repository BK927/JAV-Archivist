import { useState, useEffect, useRef } from 'react'
import { Play, Pause, Volume2, VolumeX } from 'lucide-react'

export type FeedbackAction =
  | { type: 'play' | 'pause' | 'mute' | 'unmute' }
  | { type: 'volume'; value: number }
  | { type: 'speed'; value: number }

interface ActionFeedbackProps {
  action: FeedbackAction | null
  triggerKey: number
}

const DURATIONS: Record<string, number> = {
  play: 500,
  pause: 500,
  mute: 500,
  unmute: 500,
  volume: 1000,
  speed: 800,
}

export default function ActionFeedback({ action, triggerKey }: ActionFeedbackProps) {
  const [visible, setVisible] = useState(false)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!action) return

    // Show immediately
    setVisible(true)

    // Clear existing timer
    if (timerRef.current) clearTimeout(timerRef.current)

    // Schedule fade out
    timerRef.current = setTimeout(() => {
      setVisible(false)
    }, DURATIONS[action.type] ?? 500)

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [action, triggerKey])

  if (!action) return null

  const isCenterType = action.type === 'play' || action.type === 'pause'
    || action.type === 'mute' || action.type === 'unmute'

  return (
    <div className="pointer-events-none absolute inset-0 z-10">
      {isCenterType && (
        <div
          className={`absolute inset-0 flex items-center justify-center transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="w-16 h-16 bg-black/40 rounded-full flex items-center justify-center">
            {action.type === 'play' && <Play className="w-8 h-8 text-white fill-white" />}
            {action.type === 'pause' && <Pause className="w-8 h-8 text-white fill-white" />}
            {action.type === 'mute' && <VolumeX className="w-8 h-8 text-white" />}
            {action.type === 'unmute' && <Volume2 className="w-8 h-8 text-white" />}
          </div>
        </div>
      )}

      {action.type === 'volume' && (
        <div
          className={`absolute top-12 left-1/2 -translate-x-1/2 transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="flex items-center gap-3 bg-black/70 px-4 py-2 rounded-lg">
            {action.value === 0 ? (
              <VolumeX className="w-5 h-5 text-white" />
            ) : (
              <Volume2 className="w-5 h-5 text-white" />
            )}
            <div className="w-20 h-1 bg-white/20 rounded-full">
              <div
                className="h-full bg-white rounded-full transition-all"
                style={{ width: `${action.value}%` }}
              />
            </div>
            <span className="text-white text-xs font-mono w-8">{Math.round(action.value)}%</span>
          </div>
        </div>
      )}

      {action.type === 'speed' && (
        <div
          className={`absolute top-12 left-1/2 -translate-x-1/2 transition-opacity duration-300 ${
            visible ? 'opacity-100' : 'opacity-0'
          }`}
        >
          <div className="bg-black/70 px-4 py-2 rounded-lg">
            <span className="text-white text-lg font-bold font-mono">{action.value}x</span>
          </div>
        </div>
      )}
    </div>
  )
}
