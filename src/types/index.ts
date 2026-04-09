export interface VideoFile {
  path: string
  size: number
}

export type ScrapeStatus = 'not_scraped' | 'partial' | 'complete' | 'not_found'

export interface Video {
  id: string
  code: string            // 품번 (예: "ABC-123")
  title: string
  files: VideoFile[]
  thumbnailPath: string | null
  actors: string[]
  series: string | null
  tags: string[]
  duration: number | null // 초 단위
  watched: boolean
  favorite: boolean
  addedAt: string         // ISO 8601
  releasedAt: string | null
  scrapeStatus: ScrapeStatus
  scrapedAt: string | null
  makerName: string | null
}

export interface Actor {
  id: string
  name: string
  nameKanji: string | null
  photoPath: string | null
  videoCount: number
}

export interface Maker {
  id: string
  name: string
  videoCount: number
}

export interface Tag {
  id: string
  name: string
  videoCount: number
}

export interface SampleImage {
  id: string
  videoId: string
  path: string
  sortOrder: number
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
  playerPath: string | null
}
