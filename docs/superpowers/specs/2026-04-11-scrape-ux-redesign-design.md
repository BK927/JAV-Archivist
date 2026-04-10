# Scrape UX Redesign — Selection Mode + Floating Action Bar

## Problem

1. **재수집 불가**: `Complete`/`Partial` 상태 비디오의 메타데이터를 다시 수집할 방법이 없음. 유일한 방법이 전체 데이터 초기화.
2. **일괄 조작 불가**: 여러 비디오를 선택해서 한번에 재수집할 수 없음. 개별 상세 페이지에서 하나씩 처리해야 함.
3. **관심사 혼재**: FilterBar가 필터링과 스크래핑 액션을 동시에 담당. 스크래핑 시작 시 레이아웃이 변경됨.
4. **결과 피드백 부족**: 수집 중 성공/실패 구분 없이 숫자만 표시. 완료 후 결과 요약 없음.

## Design Principle

메타데이터는 정적 콘텐츠. 한번 성공적으로 수집되면 재수집할 필요가 거의 없다. 재수집은 실패/불완전 수습을 위한 **예외 처리**이지 정기 작업이 아니다.

## Solution: Selection Mode + Floating Action Bar

### 1. FilterBar 변경

**제거:**
- "메타데이터 수집 (N)" 버튼
- 인라인 프로그레스 바 + 취소 버튼

**추가:**
- `scrapeStatus` 필터 드롭다운: 전체 / 미수집 / 부분 수집 / 실패 / 완료
- "선택 모드" 토글 버튼 (우측, 총 개수 옆)

**유지:**
- 정렬, 시청 필터, 즐겨찾기, 태그 필터 — 변경 없음

### 2. 셀렉션 모드

**진입/해제:**
- FilterBar의 "선택 모드" 토글로 on/off
- on: 카드 좌상단에 체크박스 표시, 카드 클릭이 선택/해제 동작 (상세 페이지 이동 X)
- off: 체크박스 숨김, 선택 상태 초기화, 기존 카드 클릭 동작 복원

**카드 선택 표시:**
- 미선택: 빈 체크박스 (border만)
- 선택: 보라색 채운 체크박스 + 카드 border 하이라이트

### 3. 플로팅 액션바

그리드 하단에 고정. 1개 이상 선택 시에만 표시.

**내용:**
- "N개 선택됨" 텍스트
- "재수집" 버튼
- "상태 초기화" 버튼
- "전체 선택" 버튼 (현재 필터 결과 전체를 선택)

**"상태 초기화" 동작:**
- 선택된 비디오의 `scrape_status`를 `NotScraped`로 리셋
- 메타데이터(썸네일, 배우, 태그 등)는 삭제하지 않음 — 상태만 리셋
- 리셋 후 "미수집" 필터에 잡히므로 일반 흐름으로 재수집 가능

### 4. 스크래핑 진행 + 결과 피드백

**시작:**
- 액션바에서 "재수집" 클릭 → 선택된 비디오 ID 목록을 백엔드에 전달
- 셀렉션 모드 자동 해제

**프로그레스 표시:**
- 액션바가 프로그레스 모드로 전환: 프로그레스 바 + "✓ N / ✕ N / 전체" + 취소 버튼
- 개별 카드에도 상태 반영: 수집 중(파란 테두리 + 스피너), 성공(초록), 실패(빨강)

**완료:**
- 액션바에 결과 요약: "수집 완료 — ✓ 성공 N / ✕ 실패 N"
- 5초 후 액션바 자동 소멸
- 그리드 비디오 데이터 갱신

**기존 "미수집 전체 수집" 흐름 대체:**
- FilterBar의 "메타데이터 수집 (N)" 버튼은 제거됨
- 동일 동작: scrapeStatus 필터 "미수집" → 전체 선택 → 재수집
- 셀렉션 흐름으로 통합하여 일관성 확보

### 5. 백엔드 변경

**새 커맨드:**
- `scrape_videos(video_ids: Vec<String>)` — 지정된 비디오 목록을 수집. 기존 `scrape_all_new`의 루프 로직 재사용, 대상만 다름.
- `reset_scrape_status(video_ids: Vec<String>)` — 선택된 비디오의 `scrape_status`를 `NotScraped`로 리셋. 메타데이터 유지.

**제거:**
- `scrape_all_new` — 프론트엔드에서 "미수집 필터 → 전체 선택 → scrape_videos"로 대체.

**유지:**
- `scrape_video(video_id)` — VideoDetail 개별 수집용.

**DB 변경:**
- `reset_scrape_status(conn, video_ids)` 함수 추가: `UPDATE videos SET scrape_status = 'not_scraped' WHERE id IN (...)`
- `get_videos_to_scrape` — 프론트엔드가 ID 목록을 직접 전달하므로 더 이상 불필요. 유지는 하되 사용처 없음.

**이벤트:**
- 기존 `scrape-progress`, `scrape-complete` 이벤트 구조 활용
- `scrape-progress` payload에 성공/실패 구분 추가: `{ current, total, video, status: "complete" | "not_found" | "partial" }`

### 6. VideoDetail 페이지 변경

- `scrapeStatus`와 무관하게 "메타데이터 수집" / "재수집" 버튼 항상 표시
- `Complete` 상태일 때 라벨을 "재수집"으로 변경
- 상태 뱃지 표시: Complete(초록), Partial(주황), NotFound(빨강), NotScraped(회색)
- 기존 `scrape_video` 커맨드 호출 — 변경 없음

## State Flow

```
[NotScraped] --수집--> [Complete | Partial | NotFound]
[Any status] --상태 초기화--> [NotScraped]
[Any status] --재수집(셀렉션 or VideoDetail)--> [Complete | Partial | NotFound]
```

## Files Affected

**Frontend:**
- `src/components/library/FilterBar.tsx` — scrapeStatus 필터, 선택 모드 토글 추가, 스크래핑 버튼/프로그레스 제거
- `src/components/library/VideoCard.tsx` — 체크박스 오버레이, 스크래핑 상태 표시
- `src/components/library/VideoGrid.tsx` — 셀렉션 상태 관리, 플로팅 액션바 렌더링
- `src/pages/LibraryPage.tsx` — 셀렉션 모드 상태, scrape 커맨드 호출 로직 변경
- `src/pages/VideoDetail.tsx` — 재수집 버튼 항상 표시
- `src/stores/libraryStore.ts` — scrapeStatus 필터 추가

**Backend:**
- `src-tauri/src/lib.rs` — `scrape_videos`, `reset_scrape_status` 커맨드 추가, `scrape_all_new` 제거
- `src-tauri/src/db.rs` — `reset_scrape_status` 함수 추가
