import { describe, it, expect, beforeEach } from 'vitest'
import { usePlayerStore } from '../playerStore'
import { MOCK_VIDEOS } from '@/lib/mockData'

beforeEach(() => {
  usePlayerStore.setState({ currentVideo: null })
})

describe('playerStore', () => {
  it('setCurrentVideo가 현재 영상을 설정한다', () => {
    usePlayerStore.getState().setCurrentVideo(MOCK_VIDEOS[0])
    expect(usePlayerStore.getState().currentVideo).toEqual(MOCK_VIDEOS[0])
  })

  it('setCurrentVideo(null)이 영상을 초기화한다', () => {
    usePlayerStore.getState().setCurrentVideo(MOCK_VIDEOS[0])
    usePlayerStore.getState().setCurrentVideo(null)
    expect(usePlayerStore.getState().currentVideo).toBeNull()
  })
})
