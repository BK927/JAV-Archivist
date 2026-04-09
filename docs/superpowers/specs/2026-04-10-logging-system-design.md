# Logging System Design

## Overview

디버깅용 로그 시스템. 설정에서 기본값은 off이며, 유저가 on으로 전환하면 파일 기록 + 앱 내 실시간 표시.

## Architecture

### Backend (Rust)

`tracing` facade를 사용하여 모든 백엔드 로그를 기록한다. 두 개의 subscriber layer로 구성:

1. **파일 layer**: `tracing-appender`의 `RollingFileAppender`로 일별 로테이션. `data/logs/` 디렉토리에 `app-YYYY-MM-DD.log` 형식으로 저장.
2. **이벤트 layer**: 커스텀 `tracing::Layer` 구현. 로그 발생 시 `Tauri emit("log-event")`으로 프론트엔드에 실시간 전달.

### Settings 연동

`Settings` struct에 두 필드 추가:
- `log_enabled: bool` — 기본값 `false`
- `log_level: String` — 기본값 `"info"`, 선택지: `"error"`, `"warn"`, `"info"`, `"debug"`

로그 off 시 subscriber를 등록하지 않아 오버헤드 제로. 로그 레벨 변경은 앱 재시작 필요 (subscriber는 시작 시 한 번 등록).

### Data Flow

```
[Rust 코드] --tracing 매크로--> [tracing subscriber]
                                    ├─> 파일 layer → data/logs/app-YYYY-MM-DD.log
                                    └─> 이벤트 layer → Tauri emit("log-event") → 프론트엔드
```

### Log Event Structure

```rust
struct LogEvent {
    timestamp: String,   // "2026-04-10 15:32:01"
    level: String,       // "ERROR" | "WARN" | "INFO" | "DEBUG"
    target: String,      // "jav_archivist::scraper" 등 모듈 경로
    message: String,
}
```

## Log File Management

- 저장 위치: `data/logs/`
- 파일명: `app-YYYY-MM-DD.log`
- 로테이션: 일별 자동 (tracing-appender 내장)
- 보관: 7일. 앱 시작 시 `data/logs/`에서 7일 이상 된 파일 자동 삭제.

## Frontend

### 로그 탭 (`/logs`)

TopNav에 "로그" 탭 추가. `LogPage` 컴포넌트:

- **상단 바**: 레벨 필터 드롭다운 (All / Error / Warn / Info / Debug) + "로그 지우기" 버튼
- **본문**: 모노스페이스 폰트 로그 목록, 자동 스크롤 (최신이 아래)
- **레벨 색상**: Error=빨강, Warn=노랑, Info=기본, Debug=회색
- **로그 off 시**: "설정에서 로그를 활성화하세요" 안내 표시

### 프론트엔드 버퍼

- 메모리에 최근 1000건 유지 (링 버퍼 패턴)
- 페이지 이탈 시 버퍼 유지 (Zustand store)
- 앱 재시작 시 초기화
- 과거 로그는 파일에서 확인

### 설정 페이지 추가

기존 설정 페이지에 로그 섹션 추가:
- 로그 활성화 토글 (on/off)
- 로그 레벨 선택 드롭다운 (Error / Warn / Info / Debug)
- "변경 시 앱 재시작 필요" 안내 텍스트

## Dependencies

### Rust (Cargo.toml)
- `tracing` — 로그 facade
- `tracing-subscriber` — subscriber 구성 (with `fmt`, `env-filter` features)
- `tracing-appender` — 파일 로테이션

### Frontend
- 추가 의존성 없음. Tauri 이벤트 listen + Zustand store로 구현.

## Scope

백엔드 전체 모듈에 tracing 매크로 삽입:
- `scraper` — HTTP 요청/응답, 파싱, 다운로드
- `db` — 쿼리 실행, 스키마 변경
- `scanner` — 폴더 스캔, 파일 발견
- `player` — 외부 플레이어 실행
- `lib.rs` — 커맨드 진입점, 설정 변경
