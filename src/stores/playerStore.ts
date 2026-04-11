import { create } from 'zustand'
import type { Video } from '@/types'

interface PlayerStore {
  currentVideo: Video | null
  setCurrentVideo: (video: Video | null) => void
}

export const usePlayerStore = create<PlayerStore>((set) => ({
  currentVideo: null,
  setCurrentVideo: (currentVideo) => set({ currentVideo }),
}))
