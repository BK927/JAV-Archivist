# "실패/부분 선택" 버튼 추가

## Problem

Partial/NotFound 상태인 비디오만 일괄 선택하려면 필터를 바꿔가며 수동으로 선택해야 함. 직관적인 단축 버튼이 필요.

## Solution

FloatingActionBar의 "전체 선택" 버튼 옆에 "실패/부분 선택" 버튼 추가.

**동작:**
- 현재 필터된 비디오 중 `scrapeStatus === 'not_found' || scrapeStatus === 'partial'`인 것만 선택
- 해당 비디오가 필터 결과에 없으면 버튼 숨김

**Props 변경:**
- FloatingActionBar가 `filteredIds: string[]` 대신 `filteredVideos: Video[]`를 받도록 변경
- 내부에서 ID 추출 및 status 기반 필터링 수행

## Files Affected

- `src/components/library/FloatingActionBar.tsx` — 버튼 추가, prop 타입 변경, 핸들러 추가
- `src/pages/LibraryPage.tsx` — `filteredIds` 대신 `filtered` 전달
