import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import MakerGrid from '@/components/makers/MakerGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Maker } from '@/types'

export default function MakersPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [makers, setMakers] = useState<Maker[]>([])

  useEffect(() => {
    run<Maker[]>('get_makers', {}, []).then(setMakers)
  }, [run])

  const handleSelect = (maker: Maker) => {
    navigate(`/library?maker=${encodeURIComponent(maker.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <MakerGrid makers={makers} onSelect={handleSelect} />
    </div>
  )
}
