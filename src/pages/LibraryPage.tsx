import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import FilterBar from '@/components/library/FilterBar'
import VideoGrid from '@/components/library/VideoGrid'
import FloatingActionBar from '@/components/library/FloatingActionBar'
import VideoDetail from '@/components/detail/VideoDetail'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { useFilteredVideos } from '@/hooks/useFilteredVideos'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Video, Tag } from '@/types'

export default function LibraryPage() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const { run } = useTauriCommand()
  const [searchParams, setSearchParams] = useSearchParams()
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
  const filteredIds = useMemo(() => filtered.map((v) => v.id), [filtered])

  // videos.length를 의존성으로 사용해 개별 비디오 업데이트 시 불필요한 재페칭 방지
  const videoCount = videos.length
  useEffect(() => {
    run<Tag[]>('get_tags', {}, []).then(setAllTags)
  }, [run, videoCount])

  // Refresh tags after scraping completes (scraping updates tags but not video count)
  useEffect(() => {
    let unlisten: (() => void) | undefined
    let cancelled = false
    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event')
        if (cancelled) return
        const u = await listen('scrape-complete', () => {
          run<Tag[]>('get_tags', {}, []).then(setAllTags)
        })
        if (cancelled) { u(); return }
        unlisten = u
      } catch { /* not in Tauri env */ }
    }
    setup()
    return () => { cancelled = true; unlisten?.() }
  }, [run])

  useEffect(() => {
    if (id) {
      const found = videos.find((v) => v.id === id)
      if (found) setCurrentVideo(found)
    } else {
      setCurrentVideo(null)
    }
  }, [id, videos, setCurrentVideo])

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
        activeFilter={activeFilter}
        onClearFilter={clearFilter}
      />
      <div className="flex-1 overflow-auto">
        <VideoGrid videos={filtered} onSelect={handleSelect} />
      </div>
      <FloatingActionBar filteredIds={filteredIds} />
    </div>
  )
}
