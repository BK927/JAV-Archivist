import { create } from 'zustand'
import type { Video } from '@/types'

interface PlayerStore {
  currentVideo: Video | null
  isPreviewOpen: boolean
  setCurrentVideo: (video: Video | null) => void
  setPreviewOpen: (open: boolean) => void
}

export const usePlayerStore = create<PlayerStore>((set) => ({
  currentVideo: null,
  isPreviewOpen: false,
  setCurrentVideo: (currentVideo) => set({ currentVideo }),
  setPreviewOpen: (isPreviewOpen) => set({ isPreviewOpen }),
}))
