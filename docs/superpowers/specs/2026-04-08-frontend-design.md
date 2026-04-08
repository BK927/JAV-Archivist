# JAV Archivist — 프론트엔드 설계 문서

**날짜:** 2026-04-08  
**상태:** 확정

---

## 개요

로컬에 저장된 JAV 파일들을 관리하고, 재생하고, 자동으로 메타데이터를 수집하는 데스크탑 애플리케이션의 프론트엔드 설계.

---

## 기술 스택

| 항목 | 선택 | 이유 |
|------|------|------|
| 앱 셸 | **Tauri 2** | Electron 대비 경량, Rust 백엔드로 파일 시스템 접근 |
| 프론트엔드 | **React 19 + TypeScript** | 생태계 최대, Tauri 공식 템플릿 지원 |
| 라우팅 | **React Router v7** | 탭 기반 라우팅에 충분, 간단함 |
| 상태관리 | **Zustand** | 이 앱 규모에 적합, 보일러플레이트 최소 |
| UI 컴포넌트 | **shadcn/ui** | Radix UI 기반 접근성 우수, Tailwind와 통합, 커스터마이징 자유 |
| 스타일링 | **Tailwind CSS** | shadcn/ui 내부 의존성 |
| 로컬 DB | **SQLite** (tauri-plugin-sql) | 메타데이터 영구 저장 |

---

## UI 디자인

### 색상 테마

- **배경:** `#0d0d0d` (메인), `#141414` (헤더/카드)
- **강조색:** `#e94560` (레드)
- **텍스트:** `#ffffff` (주), `#aaaaaa` (부), `#555555` (비활성)
- **테마:** 다크 모드 전용

### 레이아웃

상단 탭 네비게이션 구조. 모든 페이지에 공통 헤더가 존재하고, 헤더 하단 전체 영역이 콘텐츠.

```
┌─────────────────────────────────────────────────────┐
│  JAV Archivist  │ 라이브러리 │ 배우 │ 시리즈 │ 태그 │ 🔍 ⚙ │
├─────────────────────────────────────────────────────┤
│                                                     │
│                  콘텐츠 영역                         │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## 페이지 구조

### `/library` — 라이브러리 (기본 탭)

**컴포넌트:**
- `FilterBar` — 정렬(최근 추가/출시일/제목), 필터 드롭다운(미시청/즐겨찾기/태그), 총 영상 수 표시
- `VideoGrid` — 가상 스크롤 적용 카드 그리드
- `VideoCard` — 아래 참고

**VideoCard 구성:**
- 썸네일 (aspect-ratio: 2/3)
- 썸네일 좌상단: 품번 배지 (빨간 배경, 흰 텍스트, 예: `ABC-123`)
- 카드 하단: 제목 (2줄 클램프), 배우명 (회색)
- 호버 시: 재생 오버레이 (반투명 + ▶ 아이콘)
- 즐겨찾기된 영상: 우상단 ★ 배지

### `/library/:id` — 영상 상세

**컴포넌트:**
- 포스터 (좌측)
- 메타데이터 패널 (우측): 품번, 제목, 배우명, 시리즈, 출시일, 태그 목록
- 액션 버튼: `▶ 외부 재생` (primary), `프리뷰` (secondary), `★ 즐겨찾기` (secondary)
- `InAppPlayer` — 인앱 프리뷰 플레이어 (aspect-ratio: 16/9, `asset://` 프로토콜)

### `/actors` — 배우

- `ActorGrid` — 배우 카드 그리드 (프로필 사진, 이름, 보유 편수)
- 카드 클릭 시 → `/actors/:id` 로 이동, 해당 배우 영상만 필터된 VideoGrid

### `/series` — 시리즈

- `SeriesGrid` — 시리즈/레이블 카드 그리드 (커버, 이름, 편수)
- 카드 클릭 시 → `/series/:id` 로 이동, 해당 시리즈 VideoGrid

### `/tags` — 태그

- `TagCloud` 또는 `TagGrid` — 태그 목록 (편수 순 정렬)
- 태그 클릭 시 → 해당 태그 필터가 적용된 라이브러리 뷰로 이동

### `/settings` — 설정

- 스캔 폴더 경로 추가/제거
- 외부 플레이어 실행 경로 설정 (기본: MPV)
- 라이브러리 재스캔 트리거 버튼

---

## 상태 관리 (Zustand)

### `libraryStore`

```typescript
interface LibraryStore {
  videos: Video[]
  filters: {
    sortBy: 'addedAt' | 'releasedAt' | 'title'
    sortOrder: 'asc' | 'desc'
    watchedFilter: 'all' | 'watched' | 'unwatched'
    favoriteOnly: boolean
    tags: string[]
  }
  searchQuery: string
  isScanning: boolean
  setVideos: (videos: Video[]) => void
  setFilters: (filters: Partial<FilterState>) => void
  setSearchQuery: (q: string) => void
  setScanning: (v: boolean) => void
}
```

### `playerStore`

```typescript
interface PlayerStore {
  currentVideo: Video | null
  isPreviewOpen: boolean
  setCurrentVideo: (video: Video | null) => void
  setPreviewOpen: (open: boolean) => void
}
```

### `Video` 타입

```typescript
interface Video {
  id: string
  code: string           // 품번 (예: "ABC-123")
  title: string
  filePath: string
  thumbnailPath: string | null
  actors: string[]
  series: string | null
  tags: string[]
  duration: number       // 초 단위
  watched: boolean
  favorite: boolean
  addedAt: Date
  releasedAt: Date | null
}
```

---

## 데이터 흐름

```
앱 시작
  └─ invoke('scan_library') ──→ libraryStore.videos 채움

탭 이동 / 필터 변경
  └─ libraryStore.filters 업데이트
     → useMemo로 파생 목록 재계산 (Tauri 호출 없음)

영상 카드 클릭
  └─ playerStore.currentVideo 설정
     → /library/:id 로 라우팅

"외부 재생" 클릭
  └─ invoke('open_with_player', { filePath })
     → OS에서 설정된 외부 플레이어 실행
     → 완료 후 watched = true 마킹 (invoke('mark_watched'))

"프리뷰" 클릭
  └─ playerStore.isPreviewOpen = true
     → <video src="asset://..."> 로 인앱 스트리밍
```

---

## Tauri 커맨드 인터페이스 (프론트엔드 관점)

```typescript
'scan_library'           // 등록된 폴더 스캔, Video[] 반환
'open_with_player'       // 외부 플레이어로 파일 열기
'mark_watched'           // 시청 여부 업데이트
'toggle_favorite'        // 즐겨찾기 토글
'get_settings'           // 설정 불러오기
'save_settings'          // 설정 저장
```

---

## 컴포넌트 디렉토리 구조

```
src/
├── components/
│   ├── layout/
│   │   ├── AppShell.tsx
│   │   └── TopNav.tsx
│   ├── library/
│   │   ├── VideoGrid.tsx
│   │   ├── VideoCard.tsx
│   │   └── FilterBar.tsx
│   ├── detail/
│   │   ├── VideoDetail.tsx
│   │   └── InAppPlayer.tsx
│   ├── actors/
│   │   └── ActorGrid.tsx
│   ├── series/
│   │   └── SeriesGrid.tsx
│   └── tags/
│       └── TagGrid.tsx
├── pages/
│   ├── LibraryPage.tsx
│   ├── ActorsPage.tsx
│   ├── SeriesPage.tsx
│   ├── TagsPage.tsx
│   └── SettingsPage.tsx
├── stores/
│   ├── libraryStore.ts
│   └── playerStore.ts
├── hooks/
│   ├── useTauriCommand.ts
│   └── useFilteredVideos.ts
└── types/
    └── index.ts
```

---

## 범위 외 (이번 설계에서 제외)

- 메타데이터 자동 수집 (스크래퍼) — 별도 설계
- Rust 백엔드 상세 구현 — 별도 설계
- 인증/다중 사용자 — 해당 없음
