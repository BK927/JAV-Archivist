import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import SeriesGrid from '@/components/series/SeriesGrid'
import { useTauriCommand } from '@/hooks/useTauriCommand'
import type { Series } from '@/types'

export default function SeriesPage() {
  const navigate = useNavigate()
  const { run } = useTauriCommand()
  const [series, setSeries] = useState<Series[]>([])

  useEffect(() => {
    run<Series[]>('get_series_list', {}, []).then(setSeries)
  }, [run])

  const handleSelect = (s: Series) => {
    navigate(`/library?series=${encodeURIComponent(s.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <SeriesGrid series={series} onSelect={handleSelect} />
    </div>
  )
}
