import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import FilterBar from '@/components/library/FilterBar'
import VideoGrid from '@/components/library/VideoGrid'
import VideoDetail from '@/components/detail/VideoDetail'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { useFilteredVideos } from '@/hooks/useFilteredVideos'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video, Tag } from '@/types'

export default function LibraryPage() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery, setVideos } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const { run } = useTauriCommand()
  const [searchParams, setSearchParams] = useSearchParams()
  const [isScraping, setIsScraping] = useState(false)
  const [scrapeProgress, setScrapeProgress] = useState<{ current: number; total: number } | null>(null)
  const [allTags, setAllTags] = useState<Tag[]>([])

  // URL query param filter (memoized to avoid unnecessary useMemo recalculations)
  const activeFilter = useMemo(() => {
    if (searchParams.get('actor')) return { type: '배우', value: searchParams.get('actor')! }
    if (searchParams.get('series')) return { type: '시리즈', value: searchParams.get('series')! }
    if (searchParams.get('maker')) return { type: '제작사', value: searchParams.get('maker')! }
    if (searchParams.get('tag')) return { type: '태그', value: searchParams.get('tag')! }
    return null
  }, [searchParams])

  const clearFilter = () => setSearchParams({})

  const filtered = useFilteredVideos(videos, filters, searchQuery, activeFilter)
  const unscrapedCount = videos.filter((v) => v.scrapeStatus === 'not_scraped' && v.code !== '?').length

  useEffect(() => {
    run<Video[]>('scan_library', {}, []).then(setVideos)
  }, [run, setVideos])

  // videos.length를 의존성으로 사용해 개별 비디오 업데이트 시 불필요한 재페칭 방지
  const videoCount = videos.length
  useEffect(() => {
    run<Tag[]>('get_tags', {}, []).then(setAllTags)
  }, [run, videoCount])

  // Listen for scrape events
  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        if (cancelled) return

        const u1 = await listen<{ current: number; total: number; video?: Video }>('scrape-progress', (e) => {
          setIsScraping(true)
          setScrapeProgress(e.payload)
          if (e.payload.video) {
            const current = useLibraryStore.getState().videos
            setVideos(current.map((v) => v.id === e.payload.video!.id ? e.payload.video! : v))
          }
        })
        if (cancelled) { u1(); return }

        const u2 = await listen('scrape-complete', () => {
          setIsScraping(false)
          setScrapeProgress(null)
          run<Video[]>('get_videos', {}, []).then(setVideos)
          run<Tag[]>('get_tags', {}, []).then(setAllTags)
        })
        if (cancelled) { u1(); u2(); return }

        unlisten = () => { u1(); u2() }
      } catch {
        // Not in Tauri env
      }
    }
    setup()
    return () => { cancelled = true; unlisten?.() }
  }, [run, setVideos])

  useEffect(() => {
    if (id) {
      const found = videos.find((v) => v.id === id)
      if (found) setCurrentVideo(found)
    } else {
      setCurrentVideo(null)
    }
  }, [id, videos, setCurrentVideo])

  const handleScrapeAll = async () => {
    setIsScraping(true)
    setScrapeProgress({ current: 0, total: unscrapedCount })
    try {
      await run('scrape_all_new', {}, undefined)
    } catch {
      setIsScraping(false)
      setScrapeProgress(null)
    }
  }

  const handleCancelScrape = async () => {
    await run('cancel_scrape', {}, undefined)
    setIsScraping(false)
    setScrapeProgress(null)
  }

  const handleSelect = (video: Video) => navigate(`/library/${video.id}`)
  const handleClose = () => navigate('/library')

  if (currentVideo) {
    return <VideoDetail video={currentVideo} onClose={handleClose} />
  }

  return (
    <div className="flex flex-col h-full">
      <FilterBar
        totalCount={filtered.length}
        tags={allTags}
        unscrapedCount={unscrapedCount}
        isScraping={isScraping}
        scrapeProgress={scrapeProgress}
        onScrapeAll={handleScrapeAll}
        onCancelScrape={handleCancelScrape}
        activeFilter={activeFilter}
        onClearFilter={clearFilter}
      />
      <div className="flex-1 overflow-auto">
        <VideoGrid videos={filtered} onSelect={handleSelect} />
      </div>
    </div>
  )
}
