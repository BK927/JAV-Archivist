import { useNavigate } from 'react-router-dom'
import TagGrid from '@/components/tags/TagGrid'
import { MOCK_TAGS } from '@/lib/mockData'

export default function TagsPage() {
  const navigate = useNavigate()

  const handleSelect = (tag: string) => {
    navigate(`/library?tag=${encodeURIComponent(tag)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <TagGrid tags={MOCK_TAGS} onSelect={handleSelect} />
    </div>
  )
}
