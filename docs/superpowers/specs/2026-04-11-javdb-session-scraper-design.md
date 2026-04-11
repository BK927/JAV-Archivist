# JavDB Session Scraper Design

## Overview

브라우저에서는 로그인 없이도 `JavDB` 검색과 상세 메타데이터 조회가 가능하지만, 현재 앱의 `JavDB` scraper는 stateless HTTP 요청만 수행해 production 환경에서 안정적으로 재현되지 않는다.

이번 설계의 목표는 `JavDB`를 "상태 있는 anonymous 세션 기반 메타데이터 소스"로 승격해서:

- 일반 품번 검색을 안정적으로 수행하고
- 품번 기준 상세 메타데이터를 가져오며
- 다른 컴퓨터에서도 동일하게 동작 가능한 bootstrap / retry / logging 흐름을 제공하는 것이다.

## Confirmed Evidence

- 브라우저에서는 `JavDB` 검색과 상세 페이지 접근이 회원가입 없이 가능하다.
- 현재 앱 구현은 [javdb.rs](/C:/Users/dead4/repo/JAV-Archivist/src-tauri/src/scraper/javdb.rs)에서 search/detail에 직접 단발 요청을 보낸다.
- 현재 앱에는 `JavDB` 전용 세션 bootstrap, cookie jar, 유효성 검사, 재초기화 로직이 없다.
- 현재 앱 설정 모델 [models.rs](/C:/Users/dead4/repo/JAV-Archivist/src-tauri/src/models.rs) 및 설정 UI [SettingsPage.tsx](/C:/Users/dead4/repo/JAV-Archivist/src/pages/SettingsPage.tsx)에는 `JavDB` 상태를 다룰 필드가 없다.
- 현재 환경에서 `https://javdb.com/search?q=ABP-001&f=all` 직접 요청은 HTTP 응답 전에 `ECONNRESET`로 종료된다.

이 조합은 "회원가입 필요"가 아니라 "브라우저는 통과하지만 현재 앱의 transport 방식은 차단된다"는 쪽을 root cause로 지지한다.

## Goals

- 일반 품번 경로에서 `JavDB`를 production-ready fallback source로 사용한다.
- anonymous 공개 세션을 앱이 자동으로 bootstrap 한다.
- 세션이 깨지거나 차단되면 자동 재초기화 후 1회 재시도한다.
- 실패를 `not found`와 `blocked/rate-limited`로 구분 가능한 로그로 남긴다.
- 다른 컴퓨터에서도 별도 수동 쿠키 입력 없이 재현 가능해야 한다.
- 설정 화면에서 `JavDB` 연결 상태를 점검할 수 있어야 한다.

## Non-Goals

- 브라우저 자동화 기반 scraping
- 사용자 로그인, 계정 저장, 자격증명 저장
- FC2 전용 `JavDB` 지원
- 기존 `R18Dev`, `JavBus`, `FC2`, `Javten` 흐름의 대규모 재설계

## Approaches Considered

### 1. Stateless HTTP fallback 강화

- 장점: 구현이 가장 빠르다.
- 단점: 현재 막히는 원인인 session/bootstrap 부재를 해결하지 못한다.

### 2. Stateful anonymous HTTP session manager

- 장점: 현재 Rust scraper 구조와 가장 잘 맞고, 다른 PC에서도 재현 가능하다.
- 장점: 브라우저 자동화 없이도 `JavDB`의 공개 접근 흐름을 흉내 낼 수 있다.
- 단점: transport와 scraper 경계, 설정, 테스트 범위가 조금 커진다.

### 3. Browser automation based source

- 장점: 실제 브라우저 동작과 가장 유사하다.
- 단점: Tauri 앱에 넣기엔 무겁고 배포/유지보수 비용이 크다.

### Recommendation

2번을 채택한다. 이번 기능은 `JavDB`를 일회성 parser가 아니라 "세션이 필요한 외부 메타데이터 소스"로 다루는 것이 핵심이다.

## 1. Architecture

`JavDB` 지원을 아래 3계층으로 나눈다.

### A. Session manager

새로운 `JavDbSessionManager`를 도입한다.

책임:

- `JavDB` anonymous 세션 bootstrap
- cookie jar 또는 이에 준하는 요청 상태 유지
- 세션 유효성 검사
- 차단 감지 시 재초기화
- 마지막 bootstrap 시각과 마지막 에러 상태 보관

이 매니저는 `ScrapePipeline`이 공유해서 사용한다. 즉 video마다 새 세션을 만들지 않고, 하나의 pipeline 실행 동안 재사용한다.

### B. Transport layer

`javdb.rs`는 더 이상 "URL 만들고 바로 GET"만 하는 모듈이 아니라:

- `search_by_code(code) -> detail_url`
- `fetch_detail(detail_url) -> html`
- `detect_blocked_response(resp/body) -> bool`

같은 transport helper를 갖는다.

transport는 session manager를 통해서만 요청한다.

### C. Parser layer

기존 parser 책임은 유지한다.

- `parse_javdb_search_results(html, code)`
- `parse_javdb_html(html, code)`

즉 "세션 준비"와 "HTML 파싱"을 분리해서, parser는 fixture 테스트로 안정화하고 transport는 mock server 테스트로 검증한다.

## 2. Session Bootstrap Flow

앱은 수동 쿠키 입력 없이 anonymous 세션을 매번 자동 bootstrap 한다.

초기 bootstrap 절차:

1. `https://javdb.com/` 진입
2. 리다이렉트 / age gate / locale 관련 응답을 따라가며 필요한 쿠키를 수집
3. `?locale=en` 또는 동등한 영어 locale 경로를 최종 기본값으로 고정
4. lightweight validation 요청으로 search page 접근 가능 여부 확인

validation 기준:

- search page HTML 안에 결과 카드 컨테이너 또는 no-result marker가 보이면 유효
- age gate marker, registration prompt, redirect loop, 비정상적으로 짧은 blocked body면 무효

세션 상태는 영구 저장하지 않는다.

이유:

- anonymous 공개 세션은 수명이 짧고 환경 의존적이다
- 영구 저장보다 앱 실행 시 재구성하는 편이 다른 PC에서 더 예측 가능하다
- 계정/비밀번호/개인 쿠키를 저장하지 않아도 된다

## 3. Retry and Failure Model

`JavDB` 전용 실패 모델은 내부적으로 아래를 구분하지만, pipeline에는 기존 타입과 호환되게 투영한다.

- `NotFound`: exact code 결과 없음
- `Blocked`: age gate, anti-bot, redirect loop, suspicious short body, `ECONNRESET`
- `Network`: 일시 네트워크 오류
- `Parse`: 구조 변경 또는 unexpected HTML

pipeline 투영 규칙:

- `NotFound` -> `ScrapeError::NotFound`
- `Blocked` -> `ScrapeError::RateLimited`
- `Network` / `Parse` -> 기존 `NetworkError` / `ParseError`

retry 규칙:

- search 단계 blocked -> session reset 후 1회 재시도
- detail 단계 blocked -> session reset 후 1회 재시도
- 두 번째도 blocked면 해당 source는 실패 처리하고 다음 source merge로 진행

이 설계는 현재 rate limiter와 자연스럽게 맞물린다.

## 4. Scrape Pipeline Integration

일반 품번 source 순서는 유지하되 `JavDB`를 마지막 fallback으로 둔다.

```
General code -> R18Dev -> JavBus -> JavDB
FC2 code     -> Fc2 -> Javten
```

`JavDB`가 성공하면 다음 필드를 merge 대상으로 사용한다.

- title
- cover
- released_at
- duration
- maker
- series
- actors / actor_details
- tags
- sample_image_urls

세션 bootstrap 실패가 있어도 전체 scrape job은 계속 진행한다. 즉 `JavDB`는 production source지만, 전체 파이프라인을 hard-fail 시키지는 않는다.

## 5. Settings and UI

이번 범위에서 settings는 최소 확장만 한다.

영구 저장 필드:

- `javdb_enabled: bool`

런타임 전용 상태:

- `javdb_connection_status: disabled | idle | checking | ok | blocked | error`
- `javdb_last_bootstrapped_at: string | null`
- `javdb_last_error: string | null`

이 중 영구 저장은 `enabled`만 한다. 나머지는 앱 실행 중 상태로만 관리한다.

Settings 페이지에는 `JavDB` 섹션을 추가한다.

- `JavDB 사용` 토글
- `연결 테스트` 버튼
- 최근 상태 표시
- 실패 시 짧은 원인 메시지 표시

`연결 테스트`는 임의 코드 scrape가 아니라, bootstrap + lightweight search probe만 수행한다. 실제 video scrape command와 책임을 분리해 디버깅 신호를 명확히 한다.

## 6. Commands and Interfaces

새 Tauri command를 추가한다.

- `test_javdb_connection() -> JavDbConnectionStatus`

새 응답 모델:

- `reachable: bool`
- `status: "disabled" | "ok" | "blocked" | "error"`
- `message: string | null`
- `bootstrappedAt: string | null`

scraper 내부 인터페이스:

- `JavDbSessionManager::ensure_ready()`
- `JavDbSessionManager::invalidate()`
- `JavDbSessionManager::search(code)`
- `JavDbSessionManager::fetch_detail(url)`

## 7. Logging

로그는 parser 실패와 transport 실패를 분리한다.

필수 로그:

- `javdb: bootstrap start`
- `javdb: bootstrap success`
- `javdb: bootstrap blocked`
- `javdb: search blocked, refreshing session`
- `javdb: detail blocked, refreshing session`
- `javdb: exact match found`
- `javdb: no exact match`
- `javdb: parse failed after successful transport`

이렇게 해야 나중에 "사이트 구조 변경"과 "세션/차단 문제"를 서로 다르게 분석할 수 있다.

## 8. Testing Strategy

### Parser unit tests

기존 fixture 기반 테스트 유지:

- exact code match
- detail metadata extraction
- no exact match

### Session / transport tests

mock HTTP server를 사용해 아래를 검증한다.

- bootstrap이 landing -> locale -> search probe 흐름을 따른다
- bootstrap에서 받은 cookie/state가 후속 search/detail에 재사용된다
- blocked search가 나오면 session invalidate + rebootstrap 후 재시도한다
- blocked detail도 동일하게 1회 재시도한다
- exact match 없음은 `NotFound`로 분류된다

### Settings / command tests

- `javdb_enabled` 저장/복원
- `test_javdb_connection` 응답 직렬화
- disabled일 때 connection test가 skip 또는 disabled 상태를 반환하는지 확인

## 9. Rollout

구현 순서는 아래로 나눈다.

1. settings 모델 및 UI에 `JavDB` 토글/연결 테스트 추가
2. `JavDbSessionManager`와 transport layer 도입
3. `javdb.rs`를 parser / transport 분리 구조로 리팩터링
4. mock 기반 세션 테스트 추가
5. scrape pipeline에 연결
6. 실제 로그를 켜고 production probe 수행

## 10. Guardrails

- `JavDB`는 계정 로그인이나 수동 쿠키 입력에 의존하지 않는다
- anonymous bootstrap이 실패하면 source를 soft-fail 시키고 다른 source merge는 계속한다
- session/state는 parser 로직에 섞지 않는다
- parser 테스트와 transport 테스트를 분리한다
- "브라우저에서 된다"를 재현하기 위해 브라우저 자동화를 바로 도입하지 않는다
