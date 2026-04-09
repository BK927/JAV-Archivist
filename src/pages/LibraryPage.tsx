import { useEffect, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import FilterBar from '@/components/library/FilterBar'
import VideoGrid from '@/components/library/VideoGrid'
import VideoDetail from '@/components/detail/VideoDetail'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { useFilteredVideos } from '@/hooks/useFilteredVideos'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video } from '@/types'

export default function LibraryPage() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery, setVideos } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const { run } = useTauriCommand()
  const [searchParams, setSearchParams] = useSearchParams()
  const [isScraping, setIsScraping] = useState(false)
  const [scrapeProgress, setScrapeProgress] = useState<{ current: number; total: number } | null>(null)

  // URL query param filter
  const activeFilter = searchParams.get('actor')
    ? { type: '배우', value: searchParams.get('actor')! }
    : searchParams.get('series')
    ? { type: '시리즈', value: searchParams.get('series')! }
    : searchParams.get('maker')
    ? { type: '제작사', value: searchParams.get('maker')! }
    : searchParams.get('tag')
    ? { type: '태그', value: searchParams.get('tag')! }
    : null

  const clearFilter = () => setSearchParams({})

  const filtered = useFilteredVideos(videos, filters, searchQuery, activeFilter)
  const allTags = [...new Set(videos.flatMap((v) => v.tags))]
  const unscrapedCount = videos.filter((v) => v.scrapeStatus === 'not_scraped' && v.code !== '?').length

  useEffect(() => {
    run<Video[]>('scan_library', {}, []).then(setVideos)
  }, [run, setVideos])

  // Listen for scrape events
  useEffect(() => {
    let unlisten: (() => void) | undefined

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        const u1 = await listen<{ current: number; total: number }>('scrape-progress', (e) => {
          setScrapeProgress(e.payload)
        })
        const u2 = await listen('scrape-complete', () => {
          setIsScraping(false)
          setScrapeProgress(null)
          run<Video[]>('get_videos', {}, []).then(setVideos)
        })
        unlisten = () => { u1(); u2() }
      } catch {
        // Not in Tauri env
      }
    }
    setup()
    return () => unlisten?.()
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
    await run('scrape_all_new', {}, undefined)
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
