# JAV Archivist — Core Backend 설계 문서

**날짜:** 2026-04-09  
**상태:** 확정  
**범위:** 파일 스캔, DB 스토리지, 외부 플레이어, 설정 관리  
**범위 외:** 메타데이터 스크래핑 (별도 설계)

---

## 개요

Tauri 2 Rust 백엔드에서 로컬 영상 파일을 스캔하고, 품번을 추출하고, SQLite에 저장하고, 외부 플레이어로 재생하는 핵심 백엔드 로직.

---

## 기술 스택

| 항목 | 선택 | 이유 |
|------|------|------|
| DB | `rusqlite` (+ `bundled` feature) | Rust 네이티브, 타입 안전, 스크래퍼와 자연스러운 통합 |
| 폴더 탐색 | `walkdir` | 재귀적 디렉토리 순회, 안정적 |
| 품번 추출 | `regex` | 다양한 품번 형식 대응 |
| 파일 열기 | `open` | Windows/macOS/Linux 시스템 기본 프로그램 |
| 직렬화 | `serde` + `serde_json` | Tauri 커맨드 인터페이스 (이미 의존성) |

---

## 모듈 구조

```
src-tauri/src/
├── lib.rs          // Tauri 커맨드 정의 (얇은 진입점)
├── models.rs       // 공유 타입: Video, VideoFile, Settings
├── db.rs           // SQLite 초기화, 마이그레이션, CRUD
├── scanner.rs      // 폴더 스캔 + 품번 추출 (정규식)
└── player.rs       // 외부 플레이어 실행
```

`lib.rs`는 Tauri 커맨드만 정의하고, 실제 로직은 각 모듈 함수를 호출한다.

```rust
#[tauri::command]
fn scan_library(db: State<DbPath>) -> Result<Vec<Video>, String> {
    let conn = db::open(&db.0)?;
    let settings = db::get_settings(&conn)?;
    let videos = scanner::scan_folders(&settings.scan_folders)?;
    db::upsert_videos(&conn, &videos)?;
    db::get_all_videos(&conn)
}
```

---

## 데이터 모델 (`models.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,              // UUID
    pub code: String,            // 품번 ("ABC-123") 또는 "?"
    pub title: String,           // 스크래핑 전엔 파일명
    pub files: Vec<VideoFile>,   // 같은 품번 영상 여러 개 가능
    pub thumbnail_path: Option<String>,
    pub actors: Vec<String>,
    pub series: Option<String>,
    pub tags: Vec<String>,
    pub duration: Option<u64>,   // 초 단위, 스크래핑 전엔 None
    pub watched: bool,
    pub favorite: bool,
    pub added_at: String,        // ISO 8601
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub path: String,
    pub size: u64,               // 바이트
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub scan_folders: Vec<String>,
    pub player_path: Option<String>,  // None이면 시스템 기본
}
```

### 설계 포인트

- **`Video.files: Vec<VideoFile>`** — 같은 품번에 영상 여러 개를 담기 위해 1:N 관계. 프론트엔드 `Video.filePath`는 `files[0].path`로 매핑.
- **`code: "?"`** — 품번 추출 실패 시. 메타데이터 수집 대상에서 제외.
- **`title`** — 스크래핑 전엔 파일명(확장자 제외)을 기본값으로 사용.
- **`player_path: Option<String>`** — `None`이면 시스템 기본 프로그램, `Some`이면 해당 경로로 실행.

### 프론트엔드 타입 변경 필요

기존 프론트엔드 `Video` 타입의 `filePath: string`을 `files: VideoFile[]`로 변경해야 한다. 구현 단계에서 함께 처리.

---

## DB 스키마 (`db.rs`)

```sql
-- 영상 (품번 기준 단위)
CREATE TABLE videos (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    title TEXT NOT NULL,
    thumbnail_path TEXT,
    series TEXT,
    duration INTEGER,
    watched INTEGER DEFAULT 0,
    favorite INTEGER DEFAULT 0,
    added_at TEXT NOT NULL,
    released_at TEXT
);

CREATE UNIQUE INDEX idx_videos_code ON videos(code) WHERE code != '?';

-- 영상 파일 (1:N)
CREATE TABLE video_files (
    id TEXT PRIMARY KEY,
    video_id TEXT NOT NULL REFERENCES videos(id),
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL
);

-- 배우 (M:N)
CREATE TABLE actors (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    photo_path TEXT
);

CREATE TABLE video_actors (
    video_id TEXT REFERENCES videos(id),
    actor_id TEXT REFERENCES actors(id),
    PRIMARY KEY (video_id, actor_id)
);

-- 태그 (M:N)
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE video_tags (
    video_id TEXT REFERENCES videos(id),
    tag_id TEXT REFERENCES tags(id),
    PRIMARY KEY (video_id, tag_id)
);

-- 설정 (key-value)
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 설계 포인트

- **`code != '?'`에만 UNIQUE 인덱스** — 품번 불명 영상은 여러 개 존재 가능하므로 `?` 중복 허용.
- **배우/태그는 정규화** — M:N 조인 테이블. 배우별·태그별 필터링에 필요.
- **설정은 key-value** — `scan_folders`는 JSON 배열 문자열로 저장.
- **마이그레이션** — 앱 시작 시 `CREATE TABLE IF NOT EXISTS`로 처리.

### 주요 CRUD 함수

```rust
pub fn init_db(path: &str) -> Result<()>          // 테이블 생성
pub fn open(path: &str) -> Result<Connection>      // 커넥션 열기
pub fn upsert_videos(conn: &Connection, videos: &[Video]) -> Result<()>
pub fn get_all_videos(conn: &Connection) -> Result<Vec<Video>>
pub fn get_video_by_id(conn: &Connection, id: &str) -> Result<Video>
pub fn set_watched(conn: &Connection, id: &str, watched: bool) -> Result<()>
pub fn set_favorite(conn: &Connection, id: &str, favorite: bool) -> Result<()>
pub fn get_settings(conn: &Connection) -> Result<Settings>
pub fn save_settings(conn: &Connection, settings: &Settings) -> Result<()>
```

---

## 파일 스캐너 (`scanner.rs`)

### 지원 확장자

`.mp4`, `.mkv`, `.avi`, `.wmv`, `.flv`, `.mov`, `.ts`, `.m4v`

### 품번 추출 정규식

```rust
const CODE_PATTERNS: &[&str] = &[
    r"(?i)FC2[-\s]?PPV[-\s]?(\d+)",   // FC2 계열 (FC2-PPV-123, FC2PPV 123 등 모든 변형)
    r"(?i)([A-Z]{2,6})-(\d{3,5})",      // 일반 품번 (ABC-123, ABCD-12345)
];
```

- **대소문자 무시** — `abc-123.mp4`도 `ABC-123`으로 정규화
- **FC2 정규화** — 모든 변형을 `FC2-PPV-{숫자}` 정규형으로 변환

### 스캔 흐름

```
scan_folders(&[폴더 경로])
  ├─ walkdir로 재귀 탐색
  ├─ 확장자 필터
  ├─ 각 파일에 대해:
  │   ├─ 1) 파일명에서 품번 추출 시도
  │   ├─ 2) 실패 시 → 부모 폴더명에서 추출 시도
  │   └─ 3) 실패 시 → code = "?"
  └─ 같은 품번끼리 그룹핑 → Video { files: [...] }
```

### 핵심 함수

```rust
/// 폴더 목록을 스캔하여 Video 목록 반환
pub fn scan_folders(folders: &[String]) -> Result<Vec<Video>>

/// 문자열에서 품번 추출 (파일명, 폴더명 공용)
fn extract_code(text: &str) -> Option<String>

/// 파일 목록을 품번 기준으로 그룹핑
fn group_by_code(files: Vec<ScannedFile>) -> Vec<Video>
```

### upsert 전략

- DB에 이미 있는 품번 → `files`만 갱신 (메타데이터 덮어쓰지 않음)
- 새 품번 → 새 레코드 생성

---

## 외부 플레이어 (`player.rs`)

```rust
pub fn open_with_player(file_path: &str, player_path: Option<&str>) -> Result<()> {
    match player_path {
        Some(path) => {
            Command::new(path).arg(file_path).spawn()?;
        }
        None => {
            open::that(file_path)?;
        }
    }
    Ok(())
}
```

- `player_path`가 설정되어 있으면 해당 경로로 실행
- 없으면 `open` 크레이트로 시스템 기본 프로그램 사용

---

## Tauri 커맨드 인터페이스

| 커맨드 | 인자 | 반환 | 설명 |
|--------|------|------|------|
| `scan_library` | — | `Vec<Video>` | 폴더 스캔 → DB upsert → 전체 목록 반환 |
| `get_videos` | — | `Vec<Video>` | DB에서 전체 영상 목록 조회 (스캔 없이) |
| `get_video` | `id: String` | `Video` | 단일 영상 상세 조회 |
| `open_with_player` | `file_path: String` | `()` | 외부 플레이어 실행 |
| `mark_watched` | `id: String, watched: bool` | `()` | 시청 여부 업데이트 |
| `toggle_favorite` | `id: String` | `()` | 즐겨찾기 토글 |
| `get_settings` | — | `Settings` | 설정 조회 |
| `save_settings` | `settings: Settings` | `()` | 설정 저장 |

### 기존 대비 변경

- **`get_videos` 추가** — 탭 이동 시 스캔 없이 DB에서 바로 조회
- **`get_video` 추가** — 상세 페이지에서 단일 영상 조회
- **`mark_watched`에 `watched: bool` 추가** — 토글이 아니라 명시적 값 설정

---

## 앱 시작 흐름

```
앱 시작
  └─ Tauri setup 훅
      ├─ db::init_db()          // 테이블 생성 (IF NOT EXISTS)
      └─ 프론트엔드 로드

프론트엔드 마운트
  └─ invoke('scan_library')     // 자동 스캔
      ├─ scanner::scan_folders()
      ├─ db::upsert_videos()
      └─ db::get_all_videos() → 프론트엔드로 반환
```

### DB 연결 관리

```rust
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("library.db");
            db::init_db(&db_path)?;
            app.manage(DbPath(db_path));
            Ok(())
        })
        .invoke_handler(...)
        .run(tauri::generate_context!());
}
```

`Tauri::State<DbPath>`로 각 커맨드에서 DB 경로를 받아서 사용. 커넥션은 커맨드 호출마다 열고 닫음.

---

## 범위 외 (이번 설계에서 제외)

- 메타데이터 스크래핑 파이프라인 — 별도 설계
- 썸네일/포스터 다운로드 — 스크래핑 설계에 포함
- 실시간 파일 감시 (file watcher) — 추후 필요 시 추가
- 인앱 프리뷰 플레이어 — 프론트엔드에서 `asset://` 프로토콜로 처리 (이미 구현됨)
