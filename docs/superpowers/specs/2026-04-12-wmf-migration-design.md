# FFmpeg → Windows Media Foundation 마이그레이션

## Goal

FFmpeg 외부 바이너리 의존을 완전히 제거하고 Windows Media Foundation(WMF)으로 교체한다. 라이선스 부담 제로, 유저 설치 허들 제로.

## Background

현재 앱은 FFmpeg/ffprobe를 subprocess로 호출하여 영상 길이 조회, 프레임 추출, 스프라이트 생성을 수행한다. FFmpeg는 LGPL/GPL 라이선스로, 번들링 시 소스 제공/고지문/EULA 요건이 따른다. 시스템 PATH 의존 방식은 일반 유저에게 설치 허들이 높다.

WMF는 Windows 내장 API로 H.264, H.265, VP9, AV1, MPEG-4 등 주요 코덱을 지원하며, 별도 바이너리 없이 프레임 추출과 메타데이터 조회가 가능하다.

## Dependencies

- `windows` crate (MIT/Apache 2.0) — WMF COM 바인딩 (`Media.MediaFoundation`, `Win32.Media.MediaFoundation`)
- `image` crate (MIT/Apache 2.0) — JPEG 인코딩, 스프라이트 리사이즈/타일링

## 교체 대상

### 제거

| 항목 | 위치 |
|------|------|
| `resolve_binary`, `check` | `ffmpeg.rs` |
| `get_duration` (ffprobe subprocess) | `ffmpeg.rs` |
| `extract_frame` (ffmpeg subprocess) | `ffmpeg.rs` |
| `extract_thumbnail` | `ffmpeg.rs` |
| `generate_sprite_sheet` | `ffmpeg.rs` |
| `extract_sample_images` | `ffmpeg.rs` |
| `FfmpegPath`, `FfprobePath` 구조체 | `lib.rs` |
| `check_ffmpeg` 커맨드 | `lib.rs` |
| FFmpeg 라이선스 섹션 | `SettingsPage.tsx` |
| `invoke_handler`의 `check_ffmpeg` | `lib.rs` |

### 유지 (WMF 무관)

| 항목 | 이유 |
|------|------|
| `image_dimensions` | JPEG SOF 파서, 외부 의존 없음 |
| `is_black_frame` | 파일 크기 체크만 수행 |
| `is_low_quality_image` | 위 두 함수 기반 |

### WMF로 교체

| 기존 함수 | WMF 구현 |
|-----------|----------|
| `get_duration(ffprobe, path)` | `IMFSourceReader` → presentation attribute에서 duration 읽기 |
| `extract_frame(ffmpeg, path, ts, out)` | `IMFSourceReader` → seek → `ReadSample` → RGB 픽셀 → `image` crate로 JPEG 인코딩 |
| `extract_thumbnail(ffmpeg, ffprobe, ...)` | 위 함수 조합 (기존 로직 동일) |
| `extract_sample_images(ffmpeg, ffprobe, ...)` | 위 함수 조합 (기존 로직 동일) |
| `generate_sprite_sheet(ffmpeg, ffprobe, ...)` | WMF 프레임 추출 + `image` crate로 resize/tile/JPEG |

## API 변경

기존 함수들에서 `ffmpeg_path`/`ffprobe_path` 파라미터 전부 제거:

```
// Before
pub fn extract_thumbnail(ffmpeg_path: &Path, ffprobe_path: &Path, file_path: &str, ...) -> Option<String>

// After
pub fn extract_thumbnail(file_path: &str, video_id: &str, thumbnails_dir: &Path) -> Option<String>
```

`lib.rs`의 호출부도 모두 정리. `FfmpegPath`/`FfprobePath` State 전부 삭제.

## UX: 실패 피드백

WMF는 Windows 내장이므로 "미설치" 상태가 없다. 하지만 특정 영상(희귀 코덱, 손상 파일, 권한 문제)에서 실패할 수 있다.

### 백그라운드 처리 실패

비디오 카드에 상태 표시. 썸네일 생성이 시도되었으나 실패한 영상에 "미리보기 생성 불가" 뱃지 표시.

구현: `videos` 테이블에 `thumbnail_failed: bool` 같은 플래그는 두지 않는다. 대신 기존 로직 그대로 — 썸네일이 없으면 플레이스홀더를 보여주고, 플레이스홀더 위에 작은 경고 아이콘을 표시한다. 스크래핑 성공한 영상(커버 이미지 있음)과 구분하기 위해, **로컬 썸네일도 없고 커버 이미지도 없는** 영상에만 표시.

### 수동 "로컬 추출" 실패

버튼 인라인 에러. 토스트가 아닌 버튼 자체가 에러 상태로 전환:
- 실패 시 버튼 텍스트가 "추출 실패"로 변경, 빨간색 표시
- 2-3초 후 원래 상태로 복귀

## 기존 데이터

이미 생성된 썸네일/샘플/스프라이트 파일은 그대로 유지. 신규 생성분만 WMF 경로를 탄다.

## Non-Goals

- 크로스 플랫폼 지원 (현재 Windows 전용)
- FFmpeg fallback 경로 유지
- 기존 생성물 재생성
