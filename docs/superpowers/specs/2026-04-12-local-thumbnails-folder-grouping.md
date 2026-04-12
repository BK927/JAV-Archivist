# Local Thumbnail Generation, Folder Grouping & Seek Bar Preview

## Overview

FFmpeg를 활용한 로컬 썸네일 생성, 코드 미식별 파일의 폴더 기준 그룹핑, Cinema Mode 시크바 썸네일 프리뷰를 추가한다.

## Goals

- 모든 비디오가 라이브러리에서 썸네일과 함께 탐색 가능하도록 함 (메타데이터 유무 무관)
- 코드 추출 실패(code="?") 파일들을 폴더 단위로 묶어 의미 있는 단위로 관리
- 미식별/스크래핑 실패 상태를 유저에게 명확히 전달
- Cinema Mode에서 시크바 호버 시 썸네일 프리뷰 제공

## Feature List

### 1. FFmpeg Sidecar 번들링

FFmpeg LGPL-only static build를 Tauri sidecar로 앱에 포함. 유저 설치 불필요.

**배포 방식:**
- Tauri의 `bundle.externalBin` 설정으로 플랫폼별 FFmpeg/FFprobe 바이너리를 앱에 포함
- 바이너리 이름: `ffmpeg-{target_triple}`, `ffprobe-{target_triple}` (Tauri 규칙)
- Rust에서 `app.shell().sidecar("ffmpeg")` 또는 sidecar 경로로 `Command::new()` 호출

**LGPL 준수:**
- LGPL-only 빌드 사용 (`--enable-gpl`, `--enable-nonfree` 없이 빌드된 것)
- GPL 라이브러리(libx264 등) 미포함 — 디코딩 전용이므로 불필요
- 앱 설정/About 화면에 FFmpeg + LGPL 고지문 표시
- 번들한 FFmpeg 버전에 대응하는 소스코드 링크 제공

**법적 근거:**
- GNU GPL FAQ: pipes, command-line arguments를 통한 통신은 "separate programs"
- LGPL 2.1 Section 2: mere aggregation은 다른 저작물을 LGPL 범위로 끌어오지 않음
- FFmpeg을 라이브러리로 링크하지 않고 외부 실행파일로 호출하므로 앱 자체에 LGPL 의무 없음

**확인 시점:** 앱 시작 시 + 스캔 시작 시

**확인 방법:**
- sidecar 경로의 `ffmpeg -version` 실행
- exit code 0이면 사용 가능 (번들 누락 등 예외 상황 대비)

**Tauri 커맨드:**
- `check_ffmpeg() -> bool` — FFmpeg 사용 가능 여부 반환

### 2. 스캔 시 로컬 썸네일 생성

**대상:** `thumbnail_path`가 NULL인 모든 비디오 (신규 + 기존 미생성분)

**알고리즘:**
1. `ffprobe -v error -show_entries format=duration -of csv=p=0 {file_path}` → 영상 길이(초)
2. 10% 지점 계산 → `ffmpeg -ss {timestamp} -i {file_path} -frames:v 1 -q:v 3 {output.jpg}` → JPEG 추출
3. 출력 파일 크기 확인: < 3KB이면 블랙 프레임으로 간주
4. 블랙 프레임이면 25% 지점 재시도 → 여전히 블랙이면 50% 지점 재시도
5. 3번 모두 블랙이면 마지막 결과물 그대로 사용 (빈 카드보단 어두운 썸네일이 나음)

**멀티파트 파일:** 첫 번째 파일(`files[0]`)에서 추출

**저장 경로:** `{thumbnails_dir}/{video_id}_local.jpg`
- 스크래핑으로 받은 외부 커버와 파일명 패턴으로 구분 (`_local` 접미사)

**스크래핑과의 관계:**
- 스크래핑 성공 시: 외부 커버가 `thumbnail_path`를 덮어씀 (로컬 파일은 삭제하지 않음, 다만 DB 참조가 바뀜)
- 스크래핑 실패/미시도 시: 로컬 썸네일 경로가 `thumbnail_path`에 유지

**성능:**
- 스캔 프로세스의 일부로 순차 실행
- 영상당 1-2초 소요 (SSD 기준)
- 이미 `thumbnail_path`가 있는 비디오는 스킵

**Tauri 커맨드:**
- 별도 커맨드 불필요 — `scan_library` 내부에서 호출
- 스캔 진행 이벤트에 썸네일 생성 상태 포함 가능 (optional)

### 3. 폴더 기준 그룹핑 (code="?" 파일)

**현재 동작:** code="?" 파일은 각각 개별 Video 레코드
**변경 후:** 같은 부모 폴더의 code="?" 파일들을 하나의 Video로 그룹핑

**scanner.rs 변경:**

`group_by_code()` 함수에서:
1. code가 정상인 파일들 → 기존과 동일 (코드 기준 그룹핑)
2. code="?" 파일들 → 부모 폴더 경로(`parent_dir`)로 2차 그룹핑
3. 같은 `parent_dir`의 code="?" 파일들 → 하나의 Video로 묶음

**그룹핑된 비디오의 필드값:**
- `code`: `"?:{folder_name}"` — 고유성 보장 + UI에서 식별 가능
- `title`: 폴더명 (예: `"My_Video_Collection"`)
- `files`: 해당 폴더의 모든 code="?" 비디오 파일

**예외 처리:**
- 스캔 루트 폴더 자체의 code="?" 파일: 각각 개별 Video 유지 (루트에는 관련 없는 파일이 섞일 수 있음)
- 폴더에 code="?" 파일이 1개만 있어도 동일 로직 적용 (일관성)

**예시:**
```
/media/
  My_Folder/
    part1.mp4  (code="?")
    part2.mp4  (code="?")
    ABC-123.mp4  (code="ABC-123")
```
→ `part1.mp4` + `part2.mp4` → Video(code="?:My_Folder", title="My_Folder", files=[part1, part2])
→ `ABC-123.mp4` → Video(code="ABC-123", files=[ABC-123.mp4]) (기존 로직)

### 4. "미식별" 상태 UI 표시

**새 DB 상태를 추가하지 않음.** code 문자열 패턴으로 프론트엔드에서 구분.

**판별 로직 (프론트엔드):**
```ts
const isUnidentified = (video: Video) => video.code.startsWith('?:') || video.code === '?'
```

**VideoCard 표시:**
- 미식별 비디오: 로컬 썸네일 표시 + "미식별" 배지 (회색 배경, `text-muted-foreground`)
- 제목 위치에 폴더명 (또는 파일명, code="?"인 단일 파일의 경우)
- 코드 배지: `?:Folder_Name` 대신 폴더명만 표시

**VideoMetadata 표시:**
- "미식별" 배지 (회색)
- 코드 위치에 폴더명
- "코드 입력" 버튼 표시 (정상 코드 비디오에는 없음)

**FilterBar:**
- 기존 scrapeStatus 필터와 별도로 "미식별" 필터 옵션 추가

### 5. 수동 코드 할당

미식별 비디오(code가 `?:` 또는 `?`)에 대해 유저가 품번을 직접 입력.

**UI:**
- VideoMetadata에 "코드 입력" 인풋 + 확인 버튼
- 미식별 비디오에서만 표시

**동작 흐름:**
1. 유저가 코드 입력 (예: "ABC-123")
2. 프론트엔드 → Tauri 커맨드 `assign_code(videoId, newCode)`
3. 백엔드:
   - DB에서 해당 Video의 code 업데이트
   - 동일 코드의 기존 Video가 있으면 files를 병합 (중복 방지)
   - `scrape_status`를 `not_scraped`로 설정
4. 자동으로 스크래핑 시작 (또는 유저가 수동으로 "수집" 클릭)
5. 스크래핑 성공 시 외부 메타데이터 + 커버로 교체

**Tauri 커맨드:**
- `assign_code(video_id: String, new_code: String) -> Video` — 코드 할당 + 업데이트된 Video 반환

### 6. 시크바 썸네일 프리뷰 (스프라이트 시트)

**생성 시점:** Cinema Mode 진입 시 (lazy generation)

**백엔드 — 스프라이트 시트 생성:**

커맨드: `get_or_generate_sprite(videoId: String, filePath: String) -> SpriteInfo | null`

**FFmpeg 명령:**
```
ffmpeg -i {file_path} -vf "fps=1/{interval},scale=160:-1,tile=10x10" -q:v 5 {output.jpg}
```
- interval = max(10, ceil(duration / 100)) — 프레임 총 수를 100개 이내로 유지
  - 16분 이하 영상: 10초 간격 (최대 ~96프레임)
  - 2시간 영상: 72초 간격 (~100프레임)
- 160px 너비로 축소 (높이 자동)
- 10x10 타일 그리드 (항상 1장의 스프라이트로 충분)

**저장:**
- 스프라이트 이미지: `{sprites_dir}/{video_id}_part{N}.jpg`
- 메타데이터: `{sprites_dir}/{video_id}_part{N}.json`

**SpriteInfo 타입:**
```ts
interface SpriteInfo {
  url: string           // asset:// URL for the sprite image
  width: number         // 단일 프레임 너비 (px)
  height: number        // 단일 프레임 높이 (px)
  columns: number       // 타일 열 수
  rows: number          // 타일 행 수
  interval: number      // 프레임 간격 (초)
  totalFrames: number   // 총 프레임 수
}
```

**Rust SpriteInfo 구조체:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteInfo {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub interval: u32,
    pub total_frames: u32,
}
```

**생성 로직:**
1. 스프라이트 파일 이미 존재하는지 확인 → 있으면 즉시 SpriteInfo 반환
2. 없으면 FFmpeg으로 생성 (비동기, 10-30초 소요)
3. 생성 완료 후 SpriteInfo 반환

**프론트엔드 — PlayerControls 통합:**

시크바 호버 tooltip 확장:
- SpriteInfo가 있으면: 시간 텍스트 위에 썸네일 이미지 표시
- SpriteInfo가 없으면 (생성 중 또는 실패): 시간 텍스트 tooltip만 표시 (현재와 동일)

**썸네일 위치 계산:**
```ts
const frameIndex = Math.floor(hoverTime / sprite.interval)
const col = frameIndex % sprite.columns
const row = Math.floor(frameIndex / sprite.columns)
const bgX = -(col * sprite.width)
const bgY = -(row * sprite.height)
```

**tooltip 구조:**
```
┌──────────────┐
│  [thumbnail] │  ← 160px 너비, 스프라이트에서 잘라서 표시
├──────────────┤
│   12:34      │  ← 시간 텍스트
└──────────────┘
```

## Architecture

```
스캔 시:
  scan_library()
  ├── 기존 로직: 파일 탐색 → 코드 추출 → 그룹핑
  ├── 신규: code="?" 파일 폴더 기준 2차 그룹핑
  ├── DB upsert
  └── 신규: thumbnail_path 없는 비디오 → FFmpeg 프레임 추출 → thumbnail_path 설정

Cinema Mode 진입 시:
  get_or_generate_sprite(videoId, filePath)
  ├── 캐시 확인 → 있으면 즉시 반환
  └── 없으면 → FFmpeg 스프라이트 생성 → SpriteInfo 반환

프론트엔드:
  VideoCard/VideoMetadata → isUnidentified() 판별 → 배지/UI 분기
  PlayerControls → SpriteInfo 있으면 썸네일 tooltip, 없으면 시간 텍스트만
```

## Scope Boundaries

**포함:**
- FFmpeg sidecar 번들링 (LGPL-only build) + 라이선스 고지
- FFmpeg 사용 가능 여부 확인 로직
- 스캔 시 로컬 썸네일 생성
- code="?" 파일 폴더 기준 그룹핑
- "미식별" UI 표시 (배지, 카드, 메타데이터)
- 수동 코드 할당 커맨드 + UI
- 시크바 스프라이트 시트 생성 (backend)
- 시크바 썸네일 프리뷰 표시 (frontend)

**미포함:**
- FFmpeg 자동 업데이트
- 비디오 트랜스코딩
- 자동 코드 추출 개선 (OCR 등)
- 스프라이트 시트 자동 갱신 (비디오 파일 변경 시)
