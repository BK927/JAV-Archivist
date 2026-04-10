# Tag Filter UI Redesign

## Problem

FilterBar가 모든 태그를 `flex-wrap`으로 렌더링해서 태그 수(현재 279개)가 늘어날수록 topbar가 무한히 커진다. 태그 탐색, 검색, 조합 필터링 기능도 없다.

## Design

### 1. FilterBar (항상 1줄)

현재 `flex-wrap` 레이아웃을 `flex-nowrap overflow-hidden`으로 변경. 구성:

```
[정렬 Select] [시청 Select] [★ 즐겨찾기] | [인기태그1] [인기태그2] ... [인기태그N] | [+M개 ▾] ... [결과 수]
```

- **인기 태그**: DB에서 `video_count DESC` 순으로 상위 N개 표시 (N은 화면 너비에 따라 가변, 기본 ~8개)
- 클릭 시 즉시 필터 토글 (현재와 동일)
- 선택된 태그는 인기 태그 목록에 없더라도 FilterBar에 표시
- 구분선(`Separator`)으로 영역 분리

### 2. 태그 팝오버 ("+M개 더보기" 버튼)

Radix `Popover` 컴포넌트 사용. 버튼 클릭 시 열림.

#### 2-1. 검색 + 자동완성

- 상단 검색 `<input>` (디바운스 150ms)
- **빈 검색창**: 빈도순 전체 태그 목록 스크롤
- **텍스트 입력 시**: 자동완성 드롭다운

자동완성 드롭다운 구성:
1. **검색 결과** 섹션: 입력 문자열을 `includes()`로 매칭, 매칭 부분 빨간색 하이라이트, 빈도순 정렬
2. **추천 태그** 섹션 ("자주 같이 쓰는 태그"): 현재 선택된 태그 기준 co-occurrence 높은 태그. 이미 선택된 태그는 "선택됨" 뱃지 표시
3. 키보드: `↑↓` 이동, `Enter` 선택, `Esc` 닫기

#### 2-2. 태그 그룹 (AND/OR 조합)

팝오버 하단에 태그 그룹 영역:

- **그룹 내부**: 선택된 태그들은 **OR** 관계 (하나라도 매칭)
- **그룹 간**: 클릭 가능한 커넥터로 **AND ↔ OR** 토글
- **그룹 추가**: "+ 새 그룹" 버튼
- **그룹 삭제**: 그룹 내 모든 태그 제거 시 자동 삭제

예시:
```
그룹 1: [中出し] [素人]        ← OR (이 중 하나)
         ── AND ──              ← 클릭하면 OR로 전환
그룹 2: [個人撮影] [ハメ撮り]   ← OR (이 중 하나)
```
결과: `(中出し OR 素人) AND (個人撮影 OR ハメ撮り)`

### 3. Co-occurrence 계산

DB 쿼리로 계산. 특정 태그와 같은 비디오에 동시 등장하는 다른 태그의 빈도:

```sql
SELECT t2.name, COUNT(*) as co_count
FROM video_tags vt1
JOIN video_tags vt2 ON vt1.video_id = vt2.video_id AND vt1.tag_id != vt2.tag_id
JOIN tags t2 ON vt2.tag_id = t2.id
WHERE vt1.tag_id = ?1
GROUP BY t2.id
ORDER BY co_count DESC
LIMIT 10
```

Tauri command로 노출: `get_tag_cooccurrence(tag_id: String) -> Vec<TagCooccurrence>`

### 4. 필터 상태 모델 변경

현재 `FilterState.tags: string[]` (단순 AND)를 그룹 기반으로 변경:

```typescript
interface TagGroup {
  id: string
  tags: string[]       // 그룹 내 태그 (OR 관계)
}

interface TagFilter {
  groups: TagGroup[]
  groupOperator: 'AND' | 'OR'  // 그룹 간 연산자 (전체 동일)
}

interface FilterState {
  sortBy: 'addedAt' | 'releasedAt' | 'title'
  sortOrder: 'asc' | 'desc'
  watchedFilter: 'all' | 'watched' | 'unwatched'
  favoriteOnly: boolean
  tagFilter: TagFilter  // tags: string[] 대체
}
```

하위 호환: 그룹이 1개이고 `groupOperator`가 AND이면 현재와 동일한 동작. 태그를 FilterBar에서 직접 토글하면 그룹 1에 추가/제거.

### 5. 필터링 로직 변경

`useFilteredVideos.ts`의 태그 필터 부분:

```typescript
// 현재: filters.tags.every(tag => v.tags.includes(tag))
// 변경:
const { groups, groupOperator } = filters.tagFilter
if (groups.length > 0) {
  const groupResults = groups
    .filter(g => g.tags.length > 0)
    .map(g => g.tags.some(tag => v.tags.includes(tag)))  // 그룹 내 OR

  if (groupOperator === 'AND') {
    return groupResults.every(Boolean)
  } else {
    return groupResults.some(Boolean)
  }
}
```

### 6. 컴포넌트 구조

```
FilterBar.tsx (수정)
├── 정렬/시청/즐겨찾기 (기존 유지)
├── QuickTags (새 영역 - 인기 태그 뱃지들)
├── TagPopoverButton ("+M개 더보기")
│   └── TagPopover.tsx (새 파일)
│       ├── TagSearchInput
│       ├── TagAutocomplete (검색결과 + 추천)
│       └── TagGroups (그룹 목록 + AND/OR 커넥터)
├── 스크래핑 버튼 (기존 유지)
└── 결과 수 (기존 유지)
```

새 파일:
- `src/components/library/TagPopover.tsx` — 팝오버 전체 (검색, 자동완성, 그룹)

### 7. 데이터 흐름

1. `LibraryPage`가 `get_tags()` Tauri command 호출 → `Tag[]` (id, name, videoCount) 수신
2. FilterBar에 `Tag[]` 전달 (현재 `string[]` → `Tag[]`로 변경)
3. 인기 태그: `Tag[]`에서 상위 N개 슬라이스
4. 팝오버: 전체 `Tag[]` + 검색 필터링 + co-occurrence 추천
5. Co-occurrence: 팝오버에서 태그 검색 시 첫 번째 결과 또는 현재 선택된 태그 기준으로 `get_tag_cooccurrence()` 호출

### 8. 그룹 간 연산자

디자인 단순화를 위해 **전체 그룹 간 연산자를 하나로 통일**:
- 그룹 사이 커넥터를 클릭하면 **모든** 그룹 간 연산자가 동시에 AND ↔ OR 전환
- 그룹별 개별 연산자는 지원하지 않음 (복잡도 대비 실용성 낮음)

### 9. 마이그레이션

`FilterState.tags: string[]` → `FilterState.tagFilter: TagFilter` 변경 시:
- `libraryStore`의 `DEFAULT_FILTERS`에서 `tags: []`를 `tagFilter: { groups: [], groupOperator: 'AND' }`로 교체
- Zustand persist 사용 안 함 (현재 메모리 only) → 마이그레이션 불필요
