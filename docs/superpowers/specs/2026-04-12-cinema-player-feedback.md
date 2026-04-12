# Cinema Player Visual Feedback & Controls Enhancement

## Overview

CinemaPlayer에 시각적 조작 피드백과 키보드/마우스 편의 기능을 추가한다.

## Goals

- 사용자가 조작(재생/일시정지, 볼륨, 속도, 시크)했을 때 즉각적인 시각적 피드백 제공
- 마우스/키보드 조작 편의성 향상

## Feature List

### 1. ActionFeedback 컴포넌트 (신규)

화면에 일시적 피드백을 표시하는 단일 컴포넌트. CinemaPlayer의 자식으로 렌더링.

**인터페이스:**

```ts
type FeedbackAction =
  | { type: 'play' | 'pause' | 'mute' | 'unmute' }
  | { type: 'volume'; value: number }   // 0~100
  | { type: 'speed'; value: number }    // 0.5, 1, 1.5, 2
```

**피드백 유형별 동작:**

| 트리거 | 표시 내용 | 위치 | 지속 시간 |
|--------|-----------|------|-----------|
| play | ▶ 아이콘, 반투명 원형 배경 | 화면 중앙 | 0.5s fade |
| pause | ⏸ 아이콘, 반투명 원형 배경 | 화면 중앙 | 0.5s fade |
| volume | 스피커 아이콘 + 바 + 퍼센트 | 상단 중앙 | 1s fade |
| speed | "1.5x" 텍스트 | 상단 중앙 | 0.8s fade |
| mute | 음소거 스피커 아이콘, 원형 배경 | 화면 중앙 | 0.5s fade |
| unmute | 스피커 아이콘, 원형 배경 | 화면 중앙 | 0.5s fade |

**동작 규칙:**
- 같은 타입의 피드백이 연속 트리거되면(예: 볼륨 반복 조절) 기존 타이머를 리셋하고 값만 업데이트
- CSS `opacity` transition으로 fade out 처리
- 트리거 시마다 key/counter를 증가시켜 React가 새 렌더를 보장

### 2. 시크 표시 (하단 바 통합)

PlayerControls 내부의 시간 표시 영역 옆에 "+10s" / "-10s" 텍스트를 표시.

- CinemaPlayer에서 시크 발생 시 PlayerControls에 `seekDelta` prop 전달 (예: `+10` 또는 `-10`)
- PlayerControls가 이 값을 수신하면 시간 텍스트 옆에 표시
- 1초 후 fade out
- 화면 오버레이가 아닌 하단 컨트롤 바 내에서만 표시

### 3. 시크바 시간 Tooltip

PlayerControls의 시크바에 호버/드래그 시 시간 tooltip 표시.

- 시크바 위에 마우스 호버 → 커서 x좌표에 맞춰 tooltip div가 시크바 위에 표시
- tooltip 내용: 해당 위치의 시간 (`formatTime` 유틸 사용)
- 계산: `(mouseX - barRect.left) / barRect.width * duration`
- 드래그 중에도 동일하게 표시
- 시크바 밖으로 마우스가 나가면 tooltip 숨김
- tooltip은 시크바 상단에 작은 말풍선 스타일

### 4. 더블클릭 풀스크린 토글

비디오 영역 더블클릭 시 풀스크린 전환.

- 싱글클릭(play/pause)과 구분 필요
- 구현: 싱글클릭 시 200ms 타이머 설정. 타이머 만료 전 두 번째 클릭이 오면 싱글클릭 취소 + 풀스크린 토글. 타이머 만료 시 play/pause 실행.
- 기존 `onClick` → 이 지연 로직으로 대체

### 5. 스크롤 휠 볼륨 조절

CinemaPlayer 컨테이너에 `onWheel` 핸들러 추가.

- deltaY < 0 (위로 스크롤): 볼륨 +5%
- deltaY > 0 (아래로 스크롤): 볼륨 -5%
- 볼륨 범위 0~1 클램프
- 볼륨 변경 시 ActionFeedback으로 volume 피드백 트리거
- `e.preventDefault()`로 페이지 스크롤 방지

### 6. `<` / `>` 키 속도 조절

기존 CinemaPlayer 키보드 핸들러에 추가.

- `,` (< 키): SPEEDS 배열에서 현재 인덱스 - 1 (최소 0)
- `.` (> 키): SPEEDS 배열에서 현재 인덱스 + 1 (최대 length-1)
- SPEEDS = [0.5, 1, 1.5, 2]
- 변경 시 ActionFeedback으로 speed 피드백 트리거
- PlayerControls의 속도 표시도 동기화 (현재 speedIndex 상태를 CinemaPlayer가 관리하도록 lift)

## Architecture

```
CinemaPlayer (상태 관리: feedback, speedIndex, seekDelta)
├── <video> (+ onDoubleClick, onWheel)
├── ActionFeedback (feedback state 수신 → 렌더 + auto fade)
└── PlayerControls (기존 + seekDelta prop, 시크바 tooltip, 속도 동기화)
```

### 상태 변경 요약

**CinemaPlayer로 lift되는 상태:**
- `speedIndex` — 현재 PlayerControls 내부 → CinemaPlayer로 이동 (키보드 `<`/`>`에서도 변경 필요)
- `feedback: FeedbackAction | null` — 신규
- `seekDelta: number | null` — 신규

**PlayerControls 새 props:**
- `speedIndex: number` — 외부에서 받음
- `onSpeedChange: (index: number) => void` — 속도 변경 콜백
- `seekDelta: number | null` — 시크 표시용

## Scope Boundaries

**포함:**
- ActionFeedback 컴포넌트 생성
- PlayerControls 시크바 tooltip 추가
- PlayerControls 시크 표시 텍스트 추가
- CinemaPlayer 더블클릭, 스크롤 휠, `<`/`>` 키 핸들러 추가
- speedIndex 상태 lift

**미포함:**
- 재생 위치 기억 (별도 기능)
- 자막/캡션 지원
- 구간 반복
- 모바일 제스처
