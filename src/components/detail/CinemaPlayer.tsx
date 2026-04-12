import { useState, useRef, useCallback, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import PlayerControls, { SPEEDS } from './PlayerControls'
import PartSelector from './PartSelector'
import { assetUrl } from '@/lib/utils'
import type { VideoFile, SpriteInfo } from '@/types'
import ActionFeedback, { type FeedbackAction } from './ActionFeedback'

interface CinemaPlayerProps {
  files: VideoFile[]
  initialPartIndex: number
  videoCode: string
  videoTitle: string
  onExit: () => void
}

const HIDE_DELAY = 3000
const HIDE_DELAY_SHORT = 800

export default function CinemaPlayer({
  files,
  initialPartIndex,
  videoCode,
  videoTitle,
  onExit,
}: CinemaPlayerProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [currentPart, setCurrentPart] = useState(initialPartIndex)
  const [controlsVisible, setControlsVisible] = useState(true)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [speedIndex, setSpeedIndex] = useState(1)
  const [feedback, setFeedback] = useState<FeedbackAction | null>(null)
  const [feedbackKey, setFeedbackKey] = useState(0)
  const [seekDelta, setSeekDelta] = useState<number | null>(null)
  const [seekDeltaKey, setSeekDeltaKey] = useState(0)
  const [spriteInfo, setSpriteInfo] = useState<SpriteInfo | null>(null)
  const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const mouseOverControlsRef = useRef(false)

  // --- Feedback helpers ---

  const triggerFeedback = useCallback((action: FeedbackAction) => {
    setFeedback(action)
    setFeedbackKey((k) => k + 1)
  }, [])

  const triggerSeekDelta = useCallback((delta: number) => {
    setSeekDelta(delta)
    setSeekDeltaKey((k) => k + 1)
  }, [])

  // --- Auto-hide controls ---

  const clearHideTimer = useCallback(() => {
    if (hideTimerRef.current !== null) {
      clearTimeout(hideTimerRef.current)
      hideTimerRef.current = null
    }
  }, [])

  const scheduleHide = useCallback(
    (delay: number = HIDE_DELAY) => {
      clearHideTimer()
      hideTimerRef.current = setTimeout(() => {
        if (!mouseOverControlsRef.current) {
          setControlsVisible(false)
        }
      }, delay)
    },
    [clearHideTimer],
  )

  const showControls = useCallback(() => {
    setControlsVisible(true)
    scheduleHide()
  }, [scheduleHide])

  const handleMouseMove = useCallback(() => {
    showControls()
  }, [showControls])

  const handleControlsMouseEnter = useCallback(() => {
    mouseOverControlsRef.current = true
    clearHideTimer()
    setControlsVisible(true)
  }, [clearHideTimer])

  const handleControlsMouseLeave = useCallback(() => {
    mouseOverControlsRef.current = false
    scheduleHide(HIDE_DELAY_SHORT)
  }, [scheduleHide])

  // Cleanup timer on unmount
  useEffect(() => {
    return clearHideTimer
  }, [clearHideTimer])

  // Initial hide schedule
  useEffect(() => {
    scheduleHide()
  }, [scheduleHide])

  // --- Fullscreen ---

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen()
    } else {
      containerRef.current?.requestFullscreen()
    }
  }, [])

  // --- Video click to toggle play/pause (double-click for fullscreen) ---

  const handleVideoClick = useCallback(() => {
    if (clickTimerRef.current) {
      // Second click within 200ms — double click
      clearTimeout(clickTimerRef.current)
      clickTimerRef.current = null
      toggleFullscreen()
      return
    }
    clickTimerRef.current = setTimeout(() => {
      clickTimerRef.current = null
      const video = videoRef.current
      if (!video) return
      if (video.paused) {
        video.play().catch(() => {})
        triggerFeedback({ type: 'play' })
      } else {
        video.pause()
        triggerFeedback({ type: 'pause' })
      }
    }, 200)
  }, [toggleFullscreen, triggerFeedback])

  // --- Scroll wheel volume ---

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault()
      const video = videoRef.current
      if (!video) return
      const delta = e.deltaY < 0 ? 0.05 : -0.05
      video.volume = Math.max(0, Math.min(1, video.volume + delta))
      if (video.volume > 0 && video.muted) video.muted = false
      triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
    },
    [triggerFeedback],
  )

  // --- Part navigation ---

  const handleSelectPart = useCallback((index: number) => {
    setCurrentPart(index)
  }, [])

  // Auto-play on part change
  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    video.load()
    video.play().catch(() => {})
  }, [currentPart])

  // Fetch sprite sheet for seek bar preview
  useEffect(() => {
    setSpriteInfo(null)
    const file = files[currentPart]
    if (!file) return

    invoke<SpriteInfo | null>('get_or_generate_sprite', {
      videoId: videoCode,
      filePath: file.path,
      partIndex: currentPart,
    }).then((info) => {
      setSpriteInfo(info ?? null)
    }).catch(() => {
      // FFmpeg not available or generation failed — no sprite preview
    })
  }, [currentPart, files, videoCode])

  // Auto-advance on ended
  useEffect(() => {
    const video = videoRef.current
    if (!video) return

    const onEnded = () => {
      if (currentPart < files.length - 1) {
        setCurrentPart((prev) => prev + 1)
      }
    }

    video.addEventListener('ended', onEnded)
    return () => video.removeEventListener('ended', onEnded)
  }, [currentPart, files.length])

  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement)
    }
    document.addEventListener('fullscreenchange', onFullscreenChange)
    return () =>
      document.removeEventListener('fullscreenchange', onFullscreenChange)
  }, [])

  // --- Keyboard shortcuts ---

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const video = videoRef.current
      if (!video) return

      switch (e.key) {
        case ' ':
          e.preventDefault()
          if (video.paused) {
            video.play().catch(() => {})
            triggerFeedback({ type: 'play' })
          } else {
            video.pause()
            triggerFeedback({ type: 'pause' })
          }
          break
        case 'ArrowLeft':
          e.preventDefault()
          video.currentTime = Math.max(0, video.currentTime - 10)
          triggerSeekDelta(-10)
          break
        case 'ArrowRight':
          e.preventDefault()
          video.currentTime = Math.min(video.duration || 0, video.currentTime + 10)
          triggerSeekDelta(10)
          break
        case 'ArrowUp':
          e.preventDefault()
          video.volume = Math.min(1, video.volume + 0.1)
          if (video.muted) video.muted = false
          triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
          break
        case 'ArrowDown':
          e.preventDefault()
          video.volume = Math.max(0, video.volume - 0.1)
          triggerFeedback({ type: 'volume', value: Math.round(video.volume * 100) })
          break
        case 'f':
        case 'F':
          toggleFullscreen()
          break
        case 'm':
        case 'M':
          video.muted = !video.muted
          triggerFeedback({ type: video.muted ? 'mute' : 'unmute' })
          break
        case ',':
          if (speedIndex > 0) {
            const newIndex = speedIndex - 1
            video.playbackRate = SPEEDS[newIndex]
            setSpeedIndex(newIndex)
            triggerFeedback({ type: 'speed', value: SPEEDS[newIndex] })
          }
          break
        case '.':
          if (speedIndex < SPEEDS.length - 1) {
            const newIndex = speedIndex + 1
            video.playbackRate = SPEEDS[newIndex]
            setSpeedIndex(newIndex)
            triggerFeedback({ type: 'speed', value: SPEEDS[newIndex] })
          }
          break
        case 'Escape':
          if (document.fullscreenElement) {
            document.exitFullscreen()
          } else {
            onExit()
          }
          break
      }
      showControls()
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [toggleFullscreen, onExit, showControls, triggerFeedback, triggerSeekDelta, speedIndex])

  // Cleanup clickTimerRef on unmount
  useEffect(() => {
    return () => {
      if (clickTimerRef.current) clearTimeout(clickTimerRef.current)
    }
  }, [])

  const partLabel =
    files.length > 1
      ? `Part ${currentPart + 1}/${files.length}`
      : undefined

  return (
    <div
      ref={containerRef}
      className="fixed inset-0 z-50 bg-black"
      onMouseMove={handleMouseMove}
      onWheel={handleWheel}
    >
      {/* Video */}
      <video
        ref={videoRef}
        className="w-full h-full object-contain"
        src={assetUrl(files[currentPart].path)}
        onClick={handleVideoClick}
      />

      <ActionFeedback action={feedback} triggerKey={feedbackKey} />

      {/* Top bar */}
      <div
        className={`absolute top-0 left-0 right-0 bg-gradient-to-b from-black/70 to-transparent px-4 py-3 flex items-center gap-3 transition-opacity duration-300 ${
          controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
        onMouseEnter={handleControlsMouseEnter}
        onMouseLeave={handleControlsMouseLeave}
      >
        <button
          onClick={onExit}
          className="text-white/80 hover:text-white transition-colors p-1"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <span className="text-white font-medium text-sm">{videoCode}</span>
        <span className="text-white/60 text-sm truncate">{videoTitle}</span>
        <div className="flex-1" />
        <PartSelector
          totalParts={files.length}
          currentPart={currentPart}
          onSelectPart={handleSelectPart}
        />
      </div>

      {/* Bottom controls */}
      <div
        className={`transition-opacity duration-300 ${
          controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
        onMouseEnter={handleControlsMouseEnter}
        onMouseLeave={handleControlsMouseLeave}
      >
        <PlayerControls
          videoRef={videoRef}
          isFullscreen={isFullscreen}
          onToggleFullscreen={toggleFullscreen}
          partLabel={partLabel}
          speedIndex={speedIndex}
          onSpeedChange={(index) => {
            setSpeedIndex(index)
            triggerFeedback({ type: 'speed', value: SPEEDS[index] })
          }}
          seekDelta={seekDelta}
          seekDeltaKey={seekDeltaKey}
          onPlayPause={() => {
            const video = videoRef.current
            if (video) triggerFeedback({ type: video.paused ? 'pause' : 'play' })
          }}
          onSkip={(delta) => triggerSeekDelta(delta)}
          onMuteToggle={() => {
            const video = videoRef.current
            if (video) triggerFeedback({ type: video.muted ? 'mute' : 'unmute' })
          }}
          spriteInfo={spriteInfo}
        />
      </div>
    </div>
  )
}
