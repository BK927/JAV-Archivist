# Metadata Enhancement & Frontend Integration Design

## Overview

JAV Archivist의 메타데이터 저장 확장 + 백엔드 쿼리 API 추가 + 프론트엔드 실데이터 연동.
기존 스크래퍼가 수집하지만 저장하지 않던 데이터를 DB에 반영하고, mock 데이터 기반 프론트엔드 페이지들을 실제 백엔드에 연결한다.

## Decisions

- **배우**: 전용 페이지 + 비디오 상세 양쪽 표시. 로마자 메인 + 일본어 서브텍스트.
- **제작사**: 전용 페이지 + 비디오 상세 양쪽 표시. 클릭 시 필터.
- **FC2 샘플 이미지**: 전부 다운로드 + 비디오 상세 갤러리. 커버 없으면 첫 장을 커버로 사용.
- **스크래핑 UI**: FilterBar에 전체 스크래핑 버튼 + 비디오 상세에 개별 버튼.
- **DB 접근**: 완전 정규화 (Approach A).
- **탭 구성**: 라이브러리 / 배우 / 시리즈 / 태그 / 제작사 (5탭).

---

## 1. DB Schema Changes

### 1.1 actors 테이블 확장

```sql
ALTER TABLE actors ADD COLUMN name_kanji TEXT;
```

기존: `id, name, photo_path`. 변경 후: `id, name, name_kanji, photo_path`.

### 1.2 makers 테이블 (신규)

```sql
CREATE TABLE IF NOT EXISTS makers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);
```

### 1.3 videos.maker_id (신규 컬럼)

```sql
ALTER TABLE videos ADD COLUMN maker_id TEXT REFERENCES makers(id);
```

한 영상 = 하나의 제작사. FK로 직접 참조.

### 1.4 series 테이블 (신규)

```sql
CREATE TABLE IF NOT EXISTS series (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    cover_path TEXT
);
```

### 1.5 videos.series_id (신규 FK + 마이그레이션)

```sql
ALTER TABLE videos ADD COLUMN series_id TEXT REFERENCES series(id);
```

마이그레이션 로직:
1. `SELECT DISTINCT series FROM videos WHERE series IS NOT NULL` 로 기존 시리즈명 수집
2. 각각 series 테이블에 INSERT
3. `UPDATE videos SET series_id = (SELECT id FROM series WHERE name = videos.series)` 로 FK 연결
4. `videos.series` 컬럼은 유지 (하위 호환, 점진적 제거)

### 1.6 sample_images 테이블 (신규)

```sql
CREATE TABLE IF NOT EXISTS sample_images (
    id TEXT PRIMARY KEY,
    video_id TEXT NOT NULL REFERENCES videos(id),
    path TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);
```

---

## 2. Backend API

### 2.1 Rust Models (추가/변경)

```rust
// 신규 모델
struct Maker {
    id: String,
    name: String,
    video_count: u32,
}

struct SampleImage {
    id: String,
    video_id: String,
    path: String,
    sort_order: u32,
}

// 기존 모델 변경
struct Actor {
    id: String,
    name: String,
    name_kanji: Option<String>,  // 추가
    photo_path: Option<String>,
    video_count: u32,
}

struct Series {
    id: String,
    name: String,
    cover_path: Option<String>,
    video_count: u32,
}

struct Tag {
    id: String,
    name: String,
    video_count: u32,            // 추가
}
```

### 2.2 신규 Tauri Commands

| 커맨드 | 시그니처 | 설명 |
|--------|----------|------|
| `get_actors` | `() -> Vec<Actor>` | 전체 배우, video_count DESC 정렬 |
| `get_series` | `() -> Vec<Series>` | 전체 시리즈, video_count DESC 정렬 |
| `get_tags` | `() -> Vec<Tag>` | 전체 태그, video_count DESC 정렬 |
| `get_makers` | `() -> Vec<Maker>` | 전체 제작사, video_count DESC 정렬 |
| `get_sample_images` | `(video_id: String) -> Vec<SampleImage>` | 특정 영상의 샘플 이미지, sort_order ASC |

### 2.3 DB Query Patterns

```sql
-- get_actors
SELECT a.id, a.name, a.name_kanji, a.photo_path, COUNT(va.video_id) as video_count
FROM actors a
LEFT JOIN video_actors va ON a.id = va.actor_id
GROUP BY a.id
ORDER BY video_count DESC;

-- get_series
SELECT s.id, s.name, s.cover_path, COUNT(v.id) as video_count
FROM series s
LEFT JOIN videos v ON v.series_id = s.id
GROUP BY s.id
ORDER BY video_count DESC;

-- get_tags
SELECT t.id, t.name, COUNT(vt.video_id) as video_count
FROM tags t
LEFT JOIN video_tags vt ON t.id = vt.tag_id
GROUP BY t.id
ORDER BY video_count DESC;

-- get_makers
SELECT m.id, m.name, COUNT(v.id) as video_count
FROM makers m
LEFT JOIN videos v ON v.maker_id = m.id
GROUP BY m.id
ORDER BY video_count DESC;

-- get_sample_images
SELECT id, video_id, path, sort_order
FROM sample_images
WHERE video_id = ?1
ORDER BY sort_order ASC;
```

### 2.4 update_video_metadata 변경

시그니처 확장:
```rust
pub fn update_video_metadata(
    conn: &Connection,
    video_id: &str,
    title: Option<&str>,
    thumbnail_path: Option<&str>,
    series: Option<&str>,          // series 테이블 upsert + series_id FK 연결로 변경
    duration: Option<u64>,
    released_at: Option<&str>,
    actor_details: &[ActorDetail], // (name, Option<name_kanji>) — 기존 actors: &[String] 대체
    tags: &[String],
    maker: Option<&str>,           // 신규: makers 테이블 upsert + videos.maker_id 연결
    sample_image_paths: &[String], // 신규: sample_images 테이블에 저장
    status: ScrapeStatus,
) -> Result<()>
```

처리 순서:
1. videos 테이블 UPDATE (기존 + maker_id, series_id)
2. maker upsert → makers 테이블에 INSERT OR IGNORE → id 조회 → videos.maker_id UPDATE
3. series upsert → series 테이블에 INSERT OR IGNORE → id 조회 → videos.series_id UPDATE
4. actor_details upsert → actors 테이블에 name + name_kanji INSERT/UPDATE → video_actors 연결
5. tags upsert (기존과 동일)
6. sample_images INSERT (기존 해당 video_id 행 DELETE 후 재삽입)

```rust
struct ActorDetail {
    name: String,
    name_kanji: Option<String>,
}
```

### 2.5 Video 반환 시 maker 포함

`get_all_videos`, `get_video_by_id`에서 makers JOIN으로 maker_name 포함:
```sql
SELECT v.*, m.name as maker_name FROM videos v LEFT JOIN makers m ON v.maker_id = m.id
```

Video 모델에 `maker_name: Option<String>` 추가.

---

## 3. Scraper Changes

### 3.1 ScrapedMetadata 확장

```rust
struct ScrapedMetadata {
    title: Option<String>,
    cover_url: Option<String>,
    actors: Vec<String>,               // 기존 유지 (하위 호환)
    actor_details: Vec<ScrapedActor>,  // 신규
    tags: Vec<String>,
    series: Option<String>,
    maker: Option<String>,             // 기존 (이번에 DB 연결)
    duration: Option<u64>,
    released_at: Option<String>,
    sample_image_urls: Vec<String>,    // 신규
}

struct ScrapedActor {
    name: String,              // romaji
    name_kanji: Option<String>,
    photo_url: Option<String>,
}
```

### 3.2 Source별 매핑

| 필드 | r18.dev | FC2 |
|------|---------|-----|
| actor_details.name | `actresses[].name_romaji` | - (FC2에 배우 정보 없음) |
| actor_details.name_kanji | `actresses[].name_kanji` | - |
| actor_details.photo_url | `actresses[].image_url` | - |
| maker | `maker_name_en` | JSON-LD `brand.name` |
| sample_image_urls | - (없음) | `.items_article_SampleImagesArea img[src]` |

### 3.3 이미지 다운로드 확장

`scraper/image.rs`에 추가:

```rust
/// 배우 사진 다운로드. {actors_dir}/{sanitized_name}.jpg
pub async fn download_actor_photo(
    client: &rquest::Client,
    url: &str,
    actors_dir: &Path,
    actor_name: &str,
) -> Result<PathBuf, ScrapeError>

/// 샘플 이미지 다운로드. {samples_dir}/{code}_01.jpg, _02.jpg, ...
pub async fn download_sample_images(
    client: &rquest::Client,
    urls: &[String],
    samples_dir: &Path,
    video_code: &str,
) -> Result<Vec<PathBuf>, ScrapeError>
```

### 3.4 ScrapePipeline.scrape_one() 변경

```
1. 소스별 fetch (기존)
2. merge metadata (기존)
3. download cover (기존)
4. download actor photos (신규) — actor_details에서 photo_url이 있는 경우
5. download sample images (신규) — sample_image_urls가 있는 경우
6. 반환값 확장: ScrapeResult {
       metadata: ScrapedMetadata,
       cover_path: Option<PathBuf>,
       actor_photo_paths: HashMap<String, PathBuf>,  // actor_name → path
       sample_image_paths: Vec<PathBuf>,
       status: ScrapeStatus,
   }
```

### 3.5 디렉토리 구조

```
{app_data}/
  thumbnails/     ← 기존 (커버 이미지)
  actors/         ← 신규 (배우 사진, {name}.jpg)
  samples/        ← 신규 (FC2 샘플, {code}_01.jpg, ...)
```

`ScrapePipeline::new()`에서 actors_dir, samples_dir 생성.

### 3.6 커버 이미지 폴백

FC2 영상에서 cover_url이 없고 sample_image_urls가 있는 경우:
- `sample_image_urls[0]`을 cover_url로 사용
- 이 로직은 fc2.rs parse 단계 또는 merge 단계에서 처리

---

## 4. Frontend Changes

### 4.1 TypeScript 타입 변경

```typescript
interface Actor {
  id: string
  name: string
  nameKanji: string | null
  photoPath: string | null
  videoCount: number
}

interface Maker {
  id: string
  name: string
  videoCount: number
}

interface Tag {
  id: string
  name: string
  videoCount: number
}

interface SampleImage {
  id: string
  videoId: string
  path: string
  sortOrder: number
}

interface Video {
  // 기존 필드 모두 유지
  makerName: string | null    // 신규
}
```

### 4.2 Mock 데이터 제거

`src/lib/mockData.ts`에서 `MOCK_ACTORS`, `MOCK_SERIES`, `MOCK_TAGS` 제거.
각 페이지가 Tauri 커맨드를 직접 호출하도록 변경.

### 4.3 페이지별 변경

**TopNav**: 5탭 (라이브러리 / 배우 / 시리즈 / 태그 / 제작사)

**ActorsPage**:
- `get_actors` Tauri 커맨드 호출
- ActorGrid 카드: 원형 사진 + 이름(로마자) + 서브텍스트(일본어) + 작품 수
- 클릭 → `/library?actor={name}` 필터

**SeriesPage**:
- `get_series` Tauri 커맨드 호출
- 기존 SeriesGrid 레이아웃 유지

**TagsPage**:
- `get_tags` Tauri 커맨드 호출
- 기존 TagGrid 레이아웃 유지, count를 서버에서 받음

**MakersPage** (신규):
- `get_makers` Tauri 커맨드 호출
- 그리드 레이아웃 (시리즈와 유사)
- 카드: 제작사명 + 작품 수
- 클릭 → `/library?maker={name}` 필터

**VideoDetail 변경**:
- 배우 섹션: 사진 + 로마자명 + 일본어명 (기존: 텍스트만)
- 제작사 표시: 클릭 시 `/library?maker={name}` 필터
- 샘플 이미지 갤러리: 커버 아래 가로 스크롤 썸네일 스트립, 클릭 시 라이트박스 확대
- 개별 스크래핑 버튼: scrapeStatus가 not_scraped/not_found일 때 표시

**FilterBar 변경**:
- "메타데이터 수집" 버튼 추가 (미수집 영상이 있을 때 활성화)
- 클릭 시 progress bar + 진행률 텍스트 (`3/15 수집 중...`)
- `scrape-progress` Tauri 이벤트 수신
- "취소" 버튼 (scrape_cancel 호출)
- 완료 시 라이브러리 자동 새로고침

### 4.4 라이브러리 필터링 확장

현재 FilterBar에서 지원하는 필터: sortBy, watchedFilter, favoriteOnly, tags.

추가: URL query param으로 `?actor=`, `?series=`, `?maker=` 필터 지원.
이 필터들은 각 전용 페이지에서 클릭 시 사용되며, FilterBar UI에는 노출하지 않음 (활성 필터 뱃지로만 표시 + 제거 가능).

---

## 5. Data Flow Summary

```
[스크래핑 트리거 (FilterBar 또는 VideoDetail)]
    ↓
[ScrapePipeline.scrape_one()]
    ↓ fetch metadata from r18.dev / FC2
    ↓ download cover, actor photos, sample images
    ↓
[update_video_metadata()] — DB에 모든 메타데이터 저장
    ↓
[scrape-progress 이벤트 → 프론트엔드]
    ↓
[프론트엔드 새로고침: get_all_videos, get_actors, etc.]
```
