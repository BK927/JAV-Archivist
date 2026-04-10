# Local Media Rendering Design

## Overview

Tauri 앱에서 로컬 썸네일, 샘플 이미지, 인앱 프리뷰 영상을 안정적으로 렌더링하기 위한 규칙 정리.

이번 장애의 핵심은 "파일은 디스크에 존재하지만 Tauri asset protocol scope가 해당 경로를 허용하지 않아 `asset.localhost` 요청이 403으로 차단"된 점이었다.

## Incident Summary

### Symptoms

- 라이브러리 카드 썸네일이 엑박으로 표시됨
- 배우 사진, 시리즈 커버, 샘플 이미지가 간헐적으로 깨짐
- 인앱 프리뷰가 열려도 영상이 로드되지 않음

### Confirmed Runtime Evidence

- WebView 내부 `<img>`의 `src`는 `http://asset.localhost/...` 형태였음
- 동일 URL을 WebView 안에서 `fetch()` 했을 때 `403`이 반환됨
- 파일 자체는 `data/thumbnails`, `data/samples`, `data/actors`에 정상 존재했음
- 수정 후 동일 요청이 이미지 `200`, 비디오 `206`으로 응답함

## Root Cause

### 1. Static asset scope assumed `$EXE` would cover our media directory

기존 설정:

```json
"assetProtocol": {
  "enable": true,
  "scope": ["$EXE/data/**", "$VIDEO/**", "$DOCUMENT/**", "$DOWNLOAD/**"]
}
```

하지만 앱은 Windows에서 `current_exe().parent().join("data")`를 사용해 데이터를 저장했고, Tauri 쪽 static scope 해석과 실제 런타임 경로가 안정적으로 맞지 않았다. 그 결과 `convertFileSrc()`는 URL을 만들었지만, asset protocol은 해당 절대경로를 허용하지 않아 403을 반환했다.

### 2. Preview player bypassed the shared URL conversion path

`InAppPlayer`는 공용 `assetUrl()` helper를 쓰지 않고 `asset://localhost/...` 문자열을 직접 조합하고 있었다. 이 패턴은 이미지 렌더링 흐름과 검증 포인트를 분산시켜, 문제를 재현하고 수정하기 어렵게 만들었다.

### 3. Scan folders were not guaranteed to be asset-readable

인앱 프리뷰 대상 영상은 앱 data 디렉터리가 아니라 사용자의 스캔 폴더에 존재한다. 따라서 썸네일 scope만 맞춰도 프리뷰는 계속 실패할 수 있다. 미디어 렌더링에는 app data 디렉터리와 scan folders 둘 다 asset scope에 포함되어야 한다.

## Adopted Fix

### Backend rules

- 앱 시작 시 `data_dir`를 asset scope에 런타임 등록한다
- 설정 저장 시 최신 `scan_folders`를 asset scope에 런타임 등록한다
- 존재하지 않는 경로는 건너뛰되 warning 로그를 남긴다

구현 위치:

- `src-tauri/src/lib.rs`
  - `asset_scope_paths(...)`
  - `sync_asset_protocol_scope(...)`
  - `setup(...)`
  - `save_settings(...)`

### Frontend rules

- 로컬 파일 URL 생성은 반드시 `assetUrl()` 한 경로로만 처리한다
- 컴포넌트에서 `asset://localhost/...` 또는 `http://asset.localhost/...`를 직접 조합하지 않는다

구현 위치:

- `src/lib/utils.ts`
- `src/components/detail/InAppPlayer.tsx`

### Config rules

- 정적 config scope는 최소 안전 범위만 유지한다
- 런타임에서 실제 사용하는 디렉터리를 `allow_directory(..., true)`로 추가 허용한다

현재 기본값:

```json
"assetProtocol": {
  "enable": true,
  "scope": ["$APPLOCALDATA/**", "$VIDEO/**", "$DOCUMENT/**", "$DOWNLOAD/**"]
}
```

## Guardrails

앞으로 로컬 미디어 관련 기능을 추가하거나 수정할 때는 아래 규칙을 지킨다.

1. 로컬 파일 렌더링은 항상 `assetUrl()`을 통한다.
2. 새로운 저장 디렉터리를 도입하면 startup 시 asset scope 등록 로직도 함께 수정한다.
3. 설정으로 바뀌는 경로가 렌더링에 사용되면, settings 저장 시 asset scope 동기화도 같이 처리한다.
4. "파일이 존재한다"와 "웹뷰가 읽을 수 있다"는 다른 문제이므로 둘 다 확인한다.
5. 브라우저 dev 환경에서 정상 동작해도 Tauri WebView에서 다시 검증한다.

## Debugging Checklist

문제가 다시 생기면 아래 순서로 본다.

1. 파일이 실제로 존재하는지 확인한다.
2. DB에 저장된 경로가 절대경로인지, 예상 디렉터리인지 확인한다.
3. 렌더된 DOM의 `img.src` 또는 `video.src`가 무엇인지 확인한다.
4. WebView 내부에서 해당 `asset.localhost` URL을 직접 `fetch()` 해본다.
5. `403`이면 asset scope 문제, `404`면 파일 경로 문제, `200/206`인데도 안 보이면 렌더 타이밍 또는 컴포넌트 로직 문제로 본다.

## Test Strategy

- 프론트엔드 테스트: Tauri 환경에서 `convertFileSrc(..., "asset")`를 호출하는지 확인
- Rust 테스트: scope 동기화 대상 경로 목록이 data dir + deduped scan folders로 만들어지는지 확인
- 실제 앱 검증: Tauri 실행 후 WebView 내부에서 asset URL fetch 결과가 이미지 `200`, 비디오 `206`인지 확인

## Anti-Patterns

다음 패턴은 다시 도입하지 않는다.

- `asset://localhost/${path}` 직접 문자열 조합
- 새 미디어 저장 경로를 만들고 asset scope 등록을 빠뜨리는 구현
- 파일 존재 여부만 보고 렌더링 문제를 "다운로드 실패"로 오판하는 디버깅
