import { useNavigate } from 'react-router-dom'
import SeriesGrid from '@/components/series/SeriesGrid'
import { MOCK_SERIES } from '@/lib/mockData'
import type { Series } from '@/types'

export default function SeriesPage() {
  const navigate = useNavigate()

  const handleSelect = (series: Series) => {
    navigate(`/library?series=${encodeURIComponent(series.name)}`)
  }

  return (
    <div className="h-full overflow-auto">
      <SeriesGrid series={MOCK_SERIES} onSelect={handleSelect} />
    </div>
  )
}
