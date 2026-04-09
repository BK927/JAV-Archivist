import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import TagGrid from '@/components/tags/TagGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Tag } from '@/types'

export default function TagsPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [tags, setTags] = useState<Tag[]>([])

  useEffect(() => {
    run<Tag[]>('get_tags', {}, []).then(setTags)
  }, [run])

  const handleSelect = (tag: Tag) => {
    navigate(`/library?tag=${encodeURIComponent(tag.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <TagGrid tags={tags} onSelect={handleSelect} />
    </div>
  )
}
