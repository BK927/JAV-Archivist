# VideoDetail Redesign Spec

## Overview

VideoDetail 페이지를 정보 중심 + 온디맨드 플레이어의 Two-Mode 구조로 재설계한다. 커버 이미지 표시 개선, 멀티파트 파일 지원, 풀 커스텀 플레이어, 개선된 이미지 뷰어를 포함한다.

## Current State

- 좌측 커버(2:3 고정) + 우측 메타데이터의 단순 가로 배치
- InAppPlayer: HTML5 `<video>` 기반, 재생/정지만 가능
- 샘플 이미지: 가로 스크롤 썸네일, 클릭 시 오버레이 1장 표시 (네비게이션 없음)
- 멀티파트 파일: `files[0]`만 재생, 나머지 파트 무시
- 외부 플레이어만 `files[0]` 재생

## Design: Two-Mode Architecture

### Mode 1: Info Mode (Default)

페이지 진입 시 기본 상태. 세로 스크롤 단일 페이지.

**섹션 순서 (위→아래):**

1. **뒤로가기 버튼** — "라이브러리"로 복귀
2. **커버 + 메타데이터** (가로 배치)
3. **파일 파트 목록**
4. **샘플 이미지 그리드**
5. **미니 프리뷰 플레이어**

#### 2-1. Cover Image

- VideoCard와 동일한 표시 방식: blur 배경 + `object-contain` (잘림 없이 전체 표시)
- 컨테이너: `aspect-ratio: 2/3`, `width: 130px`
- 클릭 시 원본 크기 오버레이 표시 (배경 클릭 또는 X 버튼으로 닫기)
- 썸네일 없을 경우 Play 아이콘 fallback

#### 2-2. Metadata

현재와 동일한 정보 표시:
- 품번 (Badge), 스크래핑 상태 Badge
- 타이틀
- 배우 목록 (사진 + 한자 이름, 클릭 시 라이브러리 필터)
- 시리즈, 제작사 (클릭 시 라이브러리 필터), 출시일, 재생시간
- 태그 목록 (Badge)

#### 2-3. Action Buttons

- **즐겨��기** 토글
- **폴더 열기** — 비디오 파일이 위치한 폴더를 시스템 탐색기로 열기. Rust 백엔드에 `open_folder` 커맨드 추가 필요.
- **메타데이터 재수집** — 현재와 동일

#### 2-4. File Parts List

파일이 1개인 경우와 여러 개인 경우 모두 대응.

- 헤더: "Files" + 파트 수 + 총 용량
- 각 파트 행: 파트 번호, 파일명, 파일 크기, Cinema 재생 버튼, External 재생 버튼
- 파일 1개일 때도 동일 UI (일관성)
- Cinema 버튼: 해당 파트부터 Cinema Mode 진입
- External 버튼: 해당 파트를 외부 플레이어로 열기

#### 2-5. Sample Images Grid

- 그리드 레이아웃 (5열), `aspect-ratio: 16/9`
- 헤더: "Sample Images" + 이미지 수
- 클릭 시 Lightbox 열기 (해당 이미지 인덱스부터)

#### 2-6. Mini Preview Player

- 페이지 하단, 전체 너비
- `files[0]`을 자동 재생, 음소거 상태
- "Cinema Mode" 버튼: Cinema Mode 진입 (Part 1부터)
- 음소거 표시 아이콘

### Mode 2: Cinema Mode

사용자가 재생을 선택하면 페이지 전체가 플레이어로 전환된다. 같은 라우트, 같은 컴포넌트 내에서 상태로 전환.

#### Player Controls

- **시크바**: 드래그 및 클릭으로 탐색. 버퍼링 표시.
- **재생/일시정지**: 중앙 버튼 + Space 키
- **10초 앞/뒤 건너뛰기**: 버튼 + 좌/우 화살표 키
- **볼륨**: 슬라이더 + 위/아래 화살표 키. 음소거 토글.
- **재생 속도**: 0.5x, 1x, 1.5x, 2x 전환
- **전체화면**: 버튼 + F 키
- **시간 표시**: `현재 / 전체` 형식

#### Part Navigation

- **상단 바**: "Back to Info" 링크, 품번 + 타이틀, 파트 선택 탭
- **파트 탭**: 현재 파트 강조, 클릭으로 파트 전환
- **하단 컨트롤**: `Part 1/3` 표시
- **자동 연속 재생**: 현재 파트 종료 시 다음 파트 자동 재생. 마지막 파트 종료 시 정지.
- 파일이 1개인 경우 파트 UI 숨김

#### Top/Bottom Bar Auto-hide

- 마우스 이동 시 표시, 3초 무동작 후 자동 숨김
- 컨트롤 위에 마우스가 있으면 숨기지 않음

#### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | 재생/일시정지 |
| Left Arrow | 10초 ���로 |
| Right Arrow | 10초 앞으로 |
| Up Arrow | 볼륨 증가 |
| Down Arrow | 볼륨 감소 |
| F | 전체화면 토글 |
| M | 음소거 토글 |
| ESC | Cinema Mode 종료 → Info Mode 복귀 |

### Lightbox (Sample Image Viewer)

기존 "닫고 다시 열기" 방식 대신 연속 탐색이 가능한 라이트박스.

- **좌/우 네비게이션**: 화살표 버튼 + Left/Right 키
- **카운터**: 상단 중앙 `3 / 12` 형식
- **썸네일 스트립**: 하단에 가로 스크롤 썸네일 목록, 현재 이미지 강조 (primary border)
- **닫기**: X 버튼, ESC 키, 배경 클릭
- **이미지 표시**: `max-width: 90vw`, `max-height: 90vh`, `object-contain`

### Cover Overlay

커버 이미지를 원본 크기로 보기 위한 단순 오버레이.

- 배경: 반투명 검정 (`bg-black/85`)
- 원본 이미지: `max-width: 90vw`, `max-height: 90vh`, crop 없음
- 닫기: X 버튼, ESC 키, 배경 클릭

## New Backend Command

### `open_folder`

- Input: `filePath: String` (비디오 파일 경로)
- 파일의 부모 디렉토리를 시스템 탐색기로 연다
- Windows: `explorer /select,{path}` (파일 선택 상태로 열기)
- macOS: `open -R {path}`
- Linux: `xdg-open {parent_dir}`

## Component Structure

```
VideoDetail.tsx           — 메인 컴포넌트, Info/Cinema 모드 상태 관리
├── CoverImage.tsx        — 커버 표시 + 클릭 시 overlay
├── VideoMetadata.tsx     — 메타데이터 + 액션 버튼
├── FilePartsList.tsx     — 파트 목록 + 파트별 재생 ��튼
├── SampleImageGrid.tsx   — 샘플 이미지 그리드
├── ImageLightbox.tsx     — 라이트박스 (네비게이션 포함)
├── MiniPreview.tsx       — 음소거 자동재생 프리뷰
├── CinemaPlayer.tsx      — 풀 커스텀 플레이어
│   ├── PlayerControls.tsx — 시크바, 볼륨, 속도, 전체화면
│   └── PartSelector.tsx   — 파트 탭 + 자동 연속 재생
└── CoverOverlay.tsx      — 커버 원본 오버레이
```

## Scope Boundaries

**포함:**
- VideoDetail 페이지 재설계 (Info Mode + Cinema Mode)
- 커스텀 비디오 플레이어 (HTML5 `<video>` 기반, 커스텀 컨트롤)
- 이미지 라이트박스 개선
- 커버 이미지 표시 개선
- 멀티파트 파일 재생 지원
- `open_folder` 백엔드 커맨드
- 키보드 단축키

**미포함:**
- 자막 지원
- PIP (Picture-in-Picture)
- 비디오 코덱/포맷 변환
- 플레이리스트 (여러 비디오 연속 재생)
- 비디오 북마크/타임스탬프 저장
