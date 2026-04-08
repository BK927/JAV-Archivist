export interface Video {
  id: string
  code: string            // 품번 (예: "ABC-123")
  title: string
  filePath: string
  thumbnailPath: string | null
  actors: string[]
  series: string | null
  tags: string[]
  duration: number        // 초 단위
  watched: boolean
  favorite: boolean
  addedAt: string         // ISO 8601
  releasedAt: string | null
}

export interface Actor {
  id: string
  name: string
  photoPath: string | null
  videoCount: number
}

export interface Series {
  id: string
  name: string
  coverPath: string | null
  videoCount: number
}

export interface FilterState {
  sortBy: 'addedAt' | 'releasedAt' | 'title'
  sortOrder: 'asc' | 'desc'
  watchedFilter: 'all' | 'watched' | 'unwatched'
  favoriteOnly: boolean
  tags: string[]
}

export interface AppSettings {
  scanFolders: string[]
  playerPath: string
}
