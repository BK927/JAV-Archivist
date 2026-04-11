import { useState, useRef, useCallback, useEffect } from 'react'
import { ArrowLeft } from 'lucide-react'
import PlayerControls from './PlayerControls'
import PartSelector from './PartSelector'
import { assetUrl } from '@/lib/utils'
import type { VideoFile } from '@/types'

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
  const mouseOverControlsRef = useRef(false)

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

  // --- Video click to toggle play/pause ---

  const handleVideoClick = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    if (video.paused) {
      video.play()
    } else {
      video.pause()
    }
  }, [])

  // --- Part navigation ---

  const handleSelectPart = useCallback((index: number) => {
    setCurrentPart(index)
  }, [])

  // Auto-play on part change
  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    video.load()
    video.play()
  }, [currentPart])

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

  // --- Fullscreen ---

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen()
    } else {
      containerRef.current?.requestFullscreen()
    }
  }, [])

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
            video.play()
          } else {
            video.pause()
          }
          break
        case 'ArrowLeft':
          e.preventDefault()
          video.currentTime = Math.max(0, video.currentTime - 10)
          break
        case 'ArrowRight':
          e.preventDefault()
          video.currentTime = Math.min(
            video.duration || 0,
            video.currentTime + 10,
          )
          break
        case 'ArrowUp':
          e.preventDefault()
          video.volume = Math.min(1, video.volume + 0.1)
          break
        case 'ArrowDown':
          e.preventDefault()
          video.volume = Math.max(0, video.volume - 0.1)
          break
        case 'f':
        case 'F':
          toggleFullscreen()
          break
        case 'm':
        case 'M':
          video.muted = !video.muted
          break
        case 'Escape':
          if (document.fullscreenElement) {
            document.exitFullscreen()
          } else {
            onExit()
          }
          break
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [toggleFullscreen, onExit])

  const partLabel =
    files.length > 1
      ? `Part ${currentPart + 1}/${files.length}`
      : undefined

  return (
    <div
      ref={containerRef}
      className="fixed inset-0 z-50 bg-black"
      onMouseMove={handleMouseMove}
    >
      {/* Video */}
      <video
        ref={videoRef}
        className="w-full h-full object-contain"
        src={assetUrl(files[currentPart].path)}
        autoPlay
        onClick={handleVideoClick}
      />

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
        />
      </div>
    </div>
  )
}
