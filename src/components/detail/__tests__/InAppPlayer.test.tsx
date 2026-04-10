import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import InAppPlayer from '../InAppPlayer'

describe('InAppPlayer', () => {
  afterEach(() => {
    delete (window as any).__TAURI_INTERNALS__
  })

  it('Tauri 환경에서 공용 asset URL 변환기를 사용한다', () => {
    const convertFileSrc = vi.fn((filePath: string) => `http://asset.localhost/${encodeURIComponent(filePath)}`)
    ;(window as any).__TAURI_INTERNALS__ = { convertFileSrc }

    render(<InAppPlayer filePath={'C:\\library\\ABP-123.mp4'} onClose={() => {}} />)

    const video = document.querySelector('video')

    expect(convertFileSrc).toHaveBeenCalledWith('C:\\library\\ABP-123.mp4', 'asset')
    expect(video).toHaveAttribute('src', 'http://asset.localhost/C%3A%5Clibrary%5CABP-123.mp4')
    expect(screen.getAllByRole('button')).toHaveLength(2)
  })
})
