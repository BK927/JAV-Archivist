# FilterBar 정리: 퀵태그 제거 + UI 버그 수정

## 배경

FilterBar에 퀵태그 8개 + 필터 컨트롤이 한 줄에 들어가면서 공간 경쟁 발생:
- Select 컴포넌트 텍스트 잘림 (고정 너비 부족)
- 태그 선택 시 TagPopover 트리거 버튼 텍스트 2줄 넘침
- 윈도우 크기에 따라 반복 발생하는 구조적 문제

추가로:
- TagPopover 검색에서 인기 태그 20개만 노출 (전체 태그 필요)
- 태그 선택이 탭 이동 후 UI에서 사라짐 (allTags가 로컬 state라 리마운트 시 빈 배열)

## 설계

### FilterBar 레이아웃

퀵태그를 전부 제거하고 한 줄로 정리:

```
[정렬 ▾] [시청 ▾] [★ 즐겨찾기] [수집상태 ▾] | [태그 필터 (3)] | [배우: xxx ✕] ── [☑ 선택] [1,234개]
```

- "태그 필터" 버튼 클릭 시 기존 TagPopover 열림
- 괄호 안 숫자는 현재 선택된 태그 수 (0이면 숫자 없이 "태그 필터"만)
- Select 컴포넌트 너비를 `w-auto`로 변경해서 텍스트 잘림 해결

### TagPopover 변경

- `remainingCount` prop 제거 — 전체 태그 팝오버로 변경
- 검색어 없을 때 `allTags.slice(0, 20)` 제한 제거 → 전체 태그 목록 표시
- 트리거: FilterBar에서 직접 렌더링하는 "태그 필터" 버튼으로 교체

### 태그 필터 탭 이동 시 유지

- `allTags`를 LibraryPage 로컬 state → libraryStore로 이동
- store의 `tagFilter`는 이미 전역이므로 선택 상태 유지됨
- `allTags`만 store에 올리면 LibraryPage 리마운트 시 UI 즉시 복원

## 변경 파일

- `src/stores/libraryStore.ts` — `allTags`, `setAllTags` 추가
- `src/components/library/FilterBar.tsx` — 퀵태그 제거, 태그 필터 버튼 추가, Select 너비 w-auto
- `src/components/library/TagPopover.tsx` — remainingCount 제거, slice(0,20) 제거, 트리거 외부화
- `src/pages/LibraryPage.tsx` — allTags를 store에서 읽기, 로컬 state 제거
- `src/components/layout/AppShell.tsx` — tags fetch를 AppShell로 이동 (또는 LibraryPage에서 store에 set)
