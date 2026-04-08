import VideoCard from '@/components/library/VideoCard'
import { MOCK_VIDEOS } from '@/lib/mockData'

export default function LibraryPage() {
  return (
    <div className="p-6 grid gap-4" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 1fr))' }}>
      {MOCK_VIDEOS.map((v) => (
        <VideoCard key={v.id} video={v} onClick={(v) => console.log(v.code)} />
      ))}
    </div>
  )
}
