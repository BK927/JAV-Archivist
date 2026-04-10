import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi, afterEach } from 'vitest'
import SeriesGrid from '../SeriesGrid'
import type { Series } from '@/types'

describe('SeriesGrid', () => {
  afterEach(() => {
    delete (window as any).__TAURI_INTERNALS__
  })

  it('시리즈 커버를 Tauri asset URL로 변환해 표시한다', () => {
    const convertFileSrc = vi.fn((filePath: string) => `asset://${filePath}`)
    ;(window as any).__TAURI_INTERNALS__ = { convertFileSrc }

    const series: Series[] = [
      {
        id: 'series-1',
        name: 'SONE',
        coverPath: 'C:/covers/sone001.jpg',
        videoCount: 4,
      },
    ]

    render(<SeriesGrid series={series} onSelect={() => {}} />)

    expect(convertFileSrc).toHaveBeenCalledWith('C:/covers/sone001.jpg', 'asset')
    expect(screen.getByAltText('SONE')).toHaveAttribute('src', 'asset://C:/covers/sone001.jpg')
  })
})
