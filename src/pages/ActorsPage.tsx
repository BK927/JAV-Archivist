import { useNavigate } from 'react-router-dom'
import ActorGrid from '@/components/actors/ActorGrid'
import { MOCK_ACTORS } from '@/lib/mockData'
import type { Actor } from '@/types'

export default function ActorsPage() {
  const navigate = useNavigate()

  const handleSelect = (actor: Actor) => {
    navigate(`/library?actor=${encodeURIComponent(actor.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <ActorGrid actors={MOCK_ACTORS} onSelect={handleSelect} />
    </div>
  )
}
