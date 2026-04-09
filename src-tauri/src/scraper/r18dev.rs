use serde::Deserialize;
use super::types::{ScrapedMetadata, ScrapedActor, ScrapeError};

#[derive(Deserialize)]
struct R18Response {
    title_ja: Option<String>,
    title_en: Option<String>,
    jacket_full_url: Option<String>,
    actresses: Option<Vec<R18Actress>>,
    categories: Option<Vec<R18Category>>,
    series_name_en: Option<String>,
    maker_name_en: Option<String>,
    runtime_mins: Option<u64>,
    release_date: Option<String>,
}

#[derive(Deserialize)]
struct R18Actress {
    name_romaji: Option<String>,
    name_kanji: Option<String>,
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct R18Category {
    name_ja: Option<String>,
    name_en: Option<String>,
}

pub(crate) fn parse_r18_json(json: &str) -> Result<ScrapedMetadata, ScrapeError> {
    let resp: R18Response = serde_json::from_str(json)
        .map_err(|e| ScrapeError::ParseError(e.to_string()))?;

    let actress_list = resp.actresses.unwrap_or_default();

    let actors: Vec<String> = actress_list.iter()
        .filter_map(|a| a.name_romaji.clone())
        .collect();

    let actor_details: Vec<ScrapedActor> = actress_list.into_iter()
        .filter_map(|a| {
            a.name_romaji.map(|name| ScrapedActor {
                name,
                name_kanji: a.name_kanji,
                photo_url: a.image_url,
            })
        })
        .collect();

    let tags = resp
        .categories
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.name_ja.or(c.name_en))
        .collect();

    Ok(ScrapedMetadata {
        title: resp.title_ja.or(resp.title_en),
        cover_url: resp.jacket_full_url,
        actors,
        actor_details,
        tags,
        series: resp.series_name_en,
        maker: resp.maker_name_en,
        duration: resp.runtime_mins.map(|m| m * 60),
        released_at: resp.release_date,
        ..Default::default()
    })
}

/// Normalize video code for r18.dev URL: "ABC-123" → "abc123"
fn normalize_code(code: &str) -> String {
    code.to_lowercase().replace('-', "")
}

pub async fn fetch(code: &str, client: &rquest::Client) -> Result<ScrapedMetadata, ScrapeError> {
    let normalized = normalize_code(code);
    let url = format!(
        "https://r18.dev/videos/vod/movies/detail/-/combined={}/json",
        normalized
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    let status = resp.status().as_u16();
    if status == 404 {
        return Err(ScrapeError::NotFound);
    }
    if status == 403 || status == 429 {
        return Err(ScrapeError::RateLimited);
    }
    if status != 200 {
        return Err(ScrapeError::NetworkError(format!("HTTP {}", status)));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    parse_r18_json(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_r18_json() {
        let json = include_str!("../../tests/fixtures/r18_sample.json");
        let meta = parse_r18_json(json).unwrap();

        assert_eq!(meta.title.as_deref(), Some("三田真鈴 初体験スペシャル"));
        assert_eq!(
            meta.cover_url.as_deref(),
            Some("https://pics.dmm.co.jp/mono/movie/adult/sone001/sone001pl.jpg")
        );
        assert_eq!(meta.actors, vec!["Marin Mita", "Test Actress"]);
        assert_eq!(meta.actor_details.len(), 2);
        assert_eq!(meta.actor_details[0].name, "Marin Mita");
        assert_eq!(meta.actor_details[0].name_kanji.as_deref(), Some("三田真鈴"));
        assert_eq!(meta.actor_details[0].photo_url.as_deref(), Some("https://pics.dmm.co.jp/mono/actjpgs/mita_marin.jpg"));
        assert_eq!(meta.tags, vec!["巨乳", "デビュー作品"]);
        assert_eq!(meta.series.as_deref(), Some("First Experience Special"));
        assert_eq!(meta.maker.as_deref(), Some("S1 NO.1 STYLE"));
        assert_eq!(meta.duration, Some(9000)); // 150 mins * 60
        assert_eq!(meta.released_at.as_deref(), Some("2023-12-12"));
    }

    #[test]
    fn test_parse_r18_json_empty_response() {
        let json = "{}";
        let meta = parse_r18_json(json).unwrap();
        assert!(meta.title.is_none());
        assert!(meta.actors.is_empty());
    }

    #[test]
    fn test_normalize_code() {
        assert_eq!(normalize_code("SONE-001"), "sone001");
        assert_eq!(normalize_code("ABW-100"), "abw100");
    }
}
