import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import ActorGrid from '@/components/actors/ActorGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Actor } from '@/types'

export default function ActorsPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [actors, setActors] = useState<Actor[]>([])

  useEffect(() => {
    run<Actor[]>('get_actors', {}, []).then(setActors)
  }, [run])

  const handleSelect = (actor: Actor) => {
    navigate(`/library?actor=${encodeURIComponent(actor.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <ActorGrid actors={actors} onSelect={handleSelect} />
    </div>
  )
}
