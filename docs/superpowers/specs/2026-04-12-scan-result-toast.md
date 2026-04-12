# Scan Result Toast Notification

## Goal

라이브러리 스캔 후 추가/제거된 항목 수를 토스트 알림으로 사용자에게 알린다. 변경이 없으면 아무것도 표시하지 않는다.

## Architecture

### Backend

`scan_library` 커맨드의 반환 타입을 변경:

```rust
// 현재: Vec<Video>
// 변경: ScanResult { videos: Vec<Video>, added: u32, removed: u32 }
```

- `added`: upsert 시 새로 INSERT된 비디오 수 (DB에 없던 code)
- `removed`: orphan 제거된 비디오 수 (이미 계산하고 있음: `orphan_ids.len()`)

### Frontend

- `sonner` 설치 (`npx shadcn add sonner`)
- `<Toaster />` 를 AppShell에 배치
- 스캔 완료 후 `added > 0 || removed > 0`이면 `toast()` 호출

### Toast Format

| 상황 | 메시지 |
|------|--------|
| 추가+제거 | `"3개 추가 · 1개 제거"` |
| 추가만 | `"3개 추가"` |
| 제거만 | `"1개 제거"` |
| 변경 없음 | 토스트 없음 |

- 자동 소멸: 4초
- 위치: 하단 우측 (sonner 기본값)

## Scope

- `scan_library` 반환 타입 변경 (Rust struct + TS interface)
- `upsert_videos`에서 added count 반환
- 프론트엔드 호출부 2곳 수정 (AppShell, SettingsPage)
- sonner 설치 + Toaster 컴포넌트 배치

## Non-Goals

- 세부 목록(어떤 파일이 추가/제거됐는지) 표시하지 않음 — 로그에서 확인 가능
- 토스트 클릭 시 동작 없음 (확장/네비게이션 없음)
