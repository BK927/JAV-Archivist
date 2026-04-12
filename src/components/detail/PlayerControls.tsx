import { useState, useEffect, useCallback, useRef } from 'react'
import {
  Play,
  Pause,
  Volume2,
  VolumeX,
  Maximize,
  Minimize,
  SkipBack,
  SkipForward,
} from 'lucide-react'
import { formatTime } from '@/lib/utils'

interface PlayerControlsProps {
  videoRef: React.RefObject<HTMLVideoElement | null>
  isFullscreen: boolean
  onToggleFullscreen: () => void
  partLabel?: string
  speedIndex: number
  onSpeedChange: (index: number) => void
}

export const SPEEDS = [0.5, 1, 1.5, 2]

export default function PlayerControls({
  videoRef,
  isFullscreen,
  onToggleFullscreen,
  partLabel,
  speedIndex,
  onSpeedChange,
}: PlayerControlsProps) {
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [volume, setVolume] = useState(1)
  const [isMuted, setIsMuted] = useState(false)
  const [hoverTime, setHoverTime] = useState<{ time: number; left: number } | null>(null)
  const isSeekingRef = useRef(false)

  // Sync state from video element events
  useEffect(() => {
    const video = videoRef.current
    if (!video) return

    const onPlay = () => setIsPlaying(true)
    const onPause = () => setIsPlaying(false)
    const onTimeUpdate = () => {
      if (!isSeekingRef.current) {
        setCurrentTime(video.currentTime)
      }
    }
    const onDurationChange = () => setDuration(video.duration)
    const onLoadedMetadata = () => setDuration(video.duration)
    const onVolumeChange = () => {
      setVolume(video.volume)
      setIsMuted(video.muted)
    }

    video.addEventListener('play', onPlay)
    video.addEventListener('pause', onPause)
    video.addEventListener('timeupdate', onTimeUpdate)
    video.addEventListener('loadedmetadata', onLoadedMetadata)
    video.addEventListener('durationchange', onDurationChange)
    video.addEventListener('volumechange', onVolumeChange)

    return () => {
      video.removeEventListener('play', onPlay)
      video.removeEventListener('pause', onPause)
      video.removeEventListener('timeupdate', onTimeUpdate)
      video.removeEventListener('loadedmetadata', onLoadedMetadata)
      video.removeEventListener('durationchange', onDurationChange)
      video.removeEventListener('volumechange', onVolumeChange)
    }
  }, [videoRef])

  const togglePlay = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    if (video.paused) {
      video.play()
    } else {
      video.pause()
    }
  }, [videoRef])

  const skipBack = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    video.currentTime = Math.max(0, video.currentTime - 10)
  }, [videoRef])

  const skipForward = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    video.currentTime = Math.min(video.duration || 0, video.currentTime + 10)
  }, [videoRef])

  const toggleMute = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    video.muted = !video.muted
  }, [videoRef])

  const cycleSpeed = useCallback(() => {
    const next = (speedIndex + 1) % SPEEDS.length
    const video = videoRef.current
    if (video) video.playbackRate = SPEEDS[next]
    onSpeedChange(next)
  }, [videoRef, speedIndex, onSpeedChange])

  const handleSeekHover = useCallback(
    (e: React.MouseEvent) => {
      const bar = seekBarRef.current
      const video = videoRef.current
      if (!bar || !video || !isFinite(video.duration)) return
      const rect = bar.getBoundingClientRect()
      const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
      setHoverTime({ time: ratio * video.duration, left: e.clientX - rect.left })
    },
    [videoRef],
  )

  const handleSeekLeave = useCallback(() => {
    setHoverTime(null)
  }, [])

  // --- Seek bar drag ---
  const seekBarRef = useRef<HTMLDivElement>(null)

  const seekToPosition = useCallback(
    (clientX: number) => {
      const bar = seekBarRef.current
      const video = videoRef.current
      if (!bar || !video || !isFinite(video.duration)) return
      const rect = bar.getBoundingClientRect()
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
      const newTime = ratio * video.duration
      video.currentTime = newTime
      setCurrentTime(newTime)
    },
    [videoRef],
  )

  const handleSeekMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      isSeekingRef.current = true
      seekToPosition(e.clientX)

      const onMouseMove = (ev: MouseEvent) => seekToPosition(ev.clientX)
      const onMouseUp = (ev: MouseEvent) => {
        seekToPosition(ev.clientX)
        isSeekingRef.current = false
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [seekToPosition],
  )

  // --- Volume bar drag ---
  const volumeBarRef = useRef<HTMLDivElement>(null)

  const setVolumeFromPosition = useCallback(
    (clientX: number) => {
      const bar = volumeBarRef.current
      const video = videoRef.current
      if (!bar || !video) return
      const rect = bar.getBoundingClientRect()
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
      video.volume = ratio
      if (ratio > 0 && video.muted) video.muted = false
    },
    [videoRef],
  )

  const handleVolumeMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      setVolumeFromPosition(e.clientX)

      const onMouseMove = (ev: MouseEvent) => setVolumeFromPosition(ev.clientX)
      const onMouseUp = (ev: MouseEvent) => {
        setVolumeFromPosition(ev.clientX)
        window.removeEventListener('mousemove', onMouseMove)
        window.removeEventListener('mouseup', onMouseUp)
      }
      window.addEventListener('mousemove', onMouseMove)
      window.addEventListener('mouseup', onMouseUp)
    },
    [setVolumeFromPosition],
  )

  const seekPercent = duration > 0 ? (currentTime / duration) * 100 : 0
  const volumePercent = isMuted ? 0 : volume * 100

  return (
    <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/85 to-transparent pt-8 px-4 pb-4">
      {/* Seek bar */}
      <div
        ref={seekBarRef}
        className="group relative h-1 bg-white/20 rounded-full cursor-pointer mb-3"
        onMouseDown={handleSeekMouseDown}
        onMouseMove={handleSeekHover}
        onMouseLeave={handleSeekLeave}
      >
        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full"
          style={{ width: `${seekPercent}%` }}
        />
        <div
          className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
          style={{ left: `${seekPercent}%` }}
        />
        {hoverTime && (
          <div
            className="absolute -top-8 -translate-x-1/2 bg-black/80 text-white text-xs font-mono px-2 py-1 rounded pointer-events-none"
            style={{ left: hoverTime.left }}
          >
            {formatTime(hoverTime.time)}
          </div>
        )}
      </div>

      {/* Controls row */}
      <div className="flex items-center gap-2">
        {/* Playback controls */}
        <button
          onClick={skipBack}
          className="text-white/80 hover:text-white transition-colors p-1"
        >
          <SkipBack className="w-4 h-4" />
        </button>
        <button
          onClick={togglePlay}
          className="text-white/80 hover:text-white transition-colors p-1"
        >
          {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5" />}
        </button>
        <button
          onClick={skipForward}
          className="text-white/80 hover:text-white transition-colors p-1"
        >
          <SkipForward className="w-4 h-4" />
        </button>

        {/* Volume */}
        <button
          onClick={toggleMute}
          className="text-white/80 hover:text-white transition-colors p-1 ml-2"
        >
          {isMuted || volume === 0 ? (
            <VolumeX className="w-4 h-4" />
          ) : (
            <Volume2 className="w-4 h-4" />
          )}
        </button>
        <div
          ref={volumeBarRef}
          className="relative w-16 h-1 bg-white/20 rounded-full cursor-pointer"
          onMouseDown={handleVolumeMouseDown}
        >
          <div
            className="absolute inset-y-0 left-0 bg-white/80 rounded-full"
            style={{ width: `${volumePercent}%` }}
          />
        </div>

        {/* Time display */}
        <span className="text-white/60 text-xs font-mono ml-2">
          {formatTime(currentTime)} / {formatTime(duration)}
        </span>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Speed */}
        <button
          onClick={cycleSpeed}
          className="text-white/80 hover:text-white transition-colors text-xs font-mono px-1"
        >
          {SPEEDS[speedIndex]}x
        </button>

        {/* Part label */}
        {partLabel && (
          <span className="text-white/60 text-xs">{partLabel}</span>
        )}

        {/* Fullscreen */}
        <button
          onClick={onToggleFullscreen}
          className="text-white/80 hover:text-white transition-colors p-1"
        >
          {isFullscreen ? (
            <Minimize className="w-4 h-4" />
          ) : (
            <Maximize className="w-4 h-4" />
          )}
        </button>
      </div>
    </div>
  )
}
