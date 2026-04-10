import { useEffect, useMemo } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router-dom'
import FilterBar from '@/components/library/FilterBar'
import VideoGrid from '@/components/library/VideoGrid'
import FloatingActionBar from '@/components/library/FloatingActionBar'
import VideoDetail from '@/components/detail/VideoDetail'
import { useLibraryStore } from '@/stores/libraryStore'
import { usePlayerStore } from '@/stores/playerStore'
import { useFilteredVideos } from '@/hooks/useFilteredVideos'
import type { Video } from '@/types'

export default function LibraryPage() {
  const { id } = useParams()
  const navigate = useNavigate()
  const { videos, filters, searchQuery } = useLibraryStore()
  const { currentVideo, setCurrentVideo } = usePlayerStore()
  const [searchParams, setSearchParams] = useSearchParams()

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
        activeFilter={activeFilter}
        onClearFilter={clearFilter}
      />
      <div className="flex-1 overflow-auto">
        <VideoGrid videos={filtered} onSelect={handleSelect} />
      </div>
      <FloatingActionBar filteredVideos={filtered} />
    </div>
  )
}
