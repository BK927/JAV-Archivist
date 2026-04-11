# 자동 메타데이터 수집 + 파일 삭제 감지

## 배경

현재 새 영상이 추가되면 watcher가 자동으로 라이브러리를 재스캔하지만, 메타데이터 수집은 사용자가 선택 모드에서 수동으로 해야 한다. 메타데이터는 정적 공개 정보이므로 첫 수집은 항상 원하는 동작이다. 또한 파일 삭제 시 DB에 고아 항목이 남는 문제도 있다.

## 설계

### 1. 자동 수집

두 시점에서 `not_scraped` 상태 영상을 자동 수집한다:

**앱 시작 시:**
- `scan_library` 후 `not_scraped && retry_count < 3`인 영상을 자동 `scrape_videos` 호출

**watcher 파일 감지 시:**
- `trigger_scan` 후 새로 추가된 `not_scraped` 영상을 자동 수집
- watcher의 `trigger_scan`이 완료된 뒤, `not_scraped` 영상을 조회하여 scrape 시작

기존 `ScrapeProgressBar`(AppShell 레벨)가 진행 상황을 보여준다.

### 2. 에러 분류: 재시도 가능 vs 영구 실패

모든 에러를 원인별로 열거하지 않고, **재시도 가능 여부**로 이분한다.

**영구 실패 (화이트리스트):**
- `ScrapeError::NotFound` — 소스가 "없음"을 명확히 응답

**재시도 가능 (그 외 전부, 기본값):**
- `ScrapeError::NetworkError` — 네트워크/서버 오류
- `ScrapeError::ParseError` — 파싱 실패
- `ScrapeError::RateLimited` — 레이트 리밋

구현:
- `scrape_one`의 최종 상태 결정 로직에서, `has_any_field()`가 false이고 `failed_sources`에 `NotFound`만 있으면 → `NotFound`. 그 외(NetworkError, ParseError, RateLimited 등이 하나라도 포함) → `NotScraped` 유지 (재시도 대상)
- 이렇게 하면 `scrape_one`의 caller(lib.rs)는 기존과 동일하게 `result.status`를 DB에 저장하면 됨

### 3. 재시도 상한

DB `videos` 테이블에 `retry_count INTEGER DEFAULT 0` 컬럼 추가.

- 자동 수집 시 `not_scraped && retry_count < 3`인 영상만 대상
- 수집 실패 후 `not_scraped`로 남으면 `retry_count += 1`
- 수집 성공(`complete`/`partial`) 또는 영구 실패(`not_found`) 시 `retry_count`는 그대로 (의미 없어짐)
- `retry_count >= 3`이면 자동 수집 대상에서 제외되지만, 사용자가 "상태 초기화" 시 `retry_count = 0`으로 리셋
- 수동 "재수집"은 `retry_count`와 무관하게 항상 동작

### 4. 파일 삭제 감지

watcher가 이미 파일 변경을 감지하고 `trigger_scan`을 호출한다. 현재 `scanner::scan_folders`가 파일 시스템의 영상만 반환하고, `db::upsert_videos`가 이를 DB와 동기화한다.

변경: `upsert_videos` 후 DB에는 있지만 스캔 결과에 없는 영상을 삭제한다.

- `trigger_scan`에서 스캔 결과의 video ID set과 DB의 전체 video ID set을 비교
- DB에만 있는 항목 → `db::delete_videos(conn, &orphan_ids)` 호출
- `delete_videos`는 관련 actor 매핑, tag 매핑, sample_images, video_files도 cascade 삭제
- 삭제 후 `library-changed` 이벤트에 갱신된 전체 목록 전송 (기존과 동일)

앱 시작 시 `scan_library`에서도 동일하게 고아 정리를 수행한다.

### 5. 프론트엔드 변경

**변경 없음.** 자동 수집은 백엔드에서 기존 `scrape-progress`/`scrape-complete` 이벤트를 그대로 emit하므로, AppShell의 `ScrapeProgressBar`가 자동으로 표시한다.

유일한 추가: AppShell의 `scrape-complete` 리스너에서 `allTags` 갱신은 이미 구현되어 있음.

## 변경 파일

### Rust (백엔드)
- `src-tauri/src/db.rs` — `retry_count` 컬럼 마이그레이션, `delete_videos()`, `get_unscraped_for_auto()` (not_scraped + retry_count < 3), `increment_retry_count()`, `reset_scrape_status`에 retry_count 리셋 추가
- `src-tauri/src/scraper/mod.rs` — `scrape_one`에서 영구 실패 vs 재시도 가능 구분 로직
- `src-tauri/src/watcher.rs` — `trigger_scan` 후 고아 삭제 + 자동 수집 트리거 (이벤트 emit)
- `src-tauri/src/lib.rs` — 앱 시작 시 자동 수집 호출, `scrape_videos`에서 실패 시 `retry_count` 증가

### TypeScript (프론트엔드)
- 변경 없음 (기존 이벤트 파이프라인 재활용)
