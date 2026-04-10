use scraper::{Html, Selector};
use super::types::{ScrapedMetadata, ScrapedActor, ScrapeError};

pub(crate) fn parse_javbus_html(html: &str, code: &str) -> Result<ScrapedMetadata, ScrapeError> {
    let document = Html::parse_document(html);
    let mut meta = ScrapedMetadata::default();

    // 1. Title from <h3> — strip leading code prefix (e.g. "DLDSS-140 タイトル" → "タイトル")
    let h3_sel = Selector::parse(".container h3").unwrap();
    if let Some(el) = document.select(&h3_sel).next() {
        let raw = el.text().collect::<String>();
        let raw = raw.trim();
        meta.title = Some(
            raw.strip_prefix(code)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| raw.to_string()),
        );
    }

    // 2. Cover image
    let cover_sel = Selector::parse("a.bigImage img").unwrap();
    if let Some(el) = document.select(&cover_sel).next() {
        meta.cover_url = el
            .value()
            .attr("src")
            .and_then(super::normalize_media_url);
    }

    // 3. Info fields from span.header
    let header_sel = Selector::parse(".movie .info p").unwrap();
    let a_sel = Selector::parse("a").unwrap();
    for p in document.select(&header_sel) {
        let text = p.text().collect::<String>();
        let link_text = p.select(&a_sel).next().map(|a| a.text().collect::<String>());

        if text.contains("發行日期") {
            // "發行日期: 2022-09-22" → "2022-09-22"
            meta.released_at = text.split(':').nth(1).map(|s| s.trim().to_string());
        } else if text.contains("長度") {
            // "長度: 120分鐘" → 7200 (seconds)
            if let Some(mins_str) = text.split(':').nth(1) {
                let digits: String = mins_str.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(mins) = digits.parse::<u64>() {
                    meta.duration = Some(mins * 60);
                }
            }
        } else if text.contains("製作商") {
            meta.maker = link_text.map(|s| s.trim().to_string());
        } else if text.contains("系列") {
            meta.series = link_text.map(|s| s.trim().to_string());
        }
    }

    // 4. Actors from avatar-box
    let avatar_sel = Selector::parse("a.avatar-box").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let span_sel = Selector::parse("span").unwrap();
    for el in document.select(&avatar_sel) {
        let name = el
            .select(&span_sel)
            .next()
            .map(|s| s.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        // Photo: prefer img src if non-empty, else derive from star URL
        let img_src = el
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("src"))
            .filter(|s| !s.is_empty())
            .and_then(super::normalize_media_url);

        let photo_url = img_src.or_else(|| {
            el.value().attr("href").and_then(|href| {
                href.rsplit('/').next().map(|id| {
                    format!("https://www.javbus.com/pics/actress/{}_a.jpg", id)
                })
            })
        });

        meta.actors.push(name.clone());
        meta.actor_details.push(ScrapedActor {
            name,
            name_kanji: None,
            photo_url,
        });
    }

    // 5. Tags/genres
    let genre_sel = Selector::parse("span.genre a").unwrap();
    for el in document.select(&genre_sel) {
        let tag = el.text().collect::<String>().trim().to_string();
        if !tag.is_empty() {
            meta.tags.push(tag);
        }
    }

    // 6. Sample images
    let sample_sel = Selector::parse("#sample-waterfall a.sample-box").unwrap();
    for el in document.select(&sample_sel) {
        if let Some(href) = el.value().attr("href") {
            if let Some(url) = super::normalize_media_url(href) {
                meta.sample_image_urls.push(url);
            }
        }
    }

    if !meta.has_any_field() {
        return Err(ScrapeError::ParseError("no metadata found in HTML".to_string()));
    }

    Ok(meta)
}

pub async fn fetch(code: &str, client: &rquest::Client) -> Result<ScrapedMetadata, ScrapeError> {
    let url = format!("https://www.javbus.com/{}", code);
    tracing::debug!("javbus: fetching {}", url);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    let status = resp.status().as_u16();
    if status == 404 {
        tracing::debug!("javbus: not found for code={}", code);
        return Err(ScrapeError::NotFound);
    }
    if status == 403 || status == 429 {
        tracing::warn!("javbus: rate limited (HTTP {}) for code={}", status, code);
        return Err(ScrapeError::RateLimited);
    }
    if status == 301 || status == 302 {
        tracing::warn!("javbus: redirected (HTTP {}) for code={}", status, code);
        return Err(ScrapeError::NotFound);
    }
    if status != 200 {
        tracing::warn!("javbus: unexpected HTTP {} for code={}", status, code);
        return Err(ScrapeError::NetworkError(format!("HTTP {}", status)));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if body.len() < 1000 {
        tracing::warn!("javbus: response body too short ({} bytes) for code={}", body.len(), code);
        return Err(ScrapeError::NotFound);
    }

    match parse_javbus_html(&body, code) {
        Ok(meta) => {
            tracing::debug!("javbus: parsed metadata for code={} title={:?}", code, meta.title);
            Ok(meta)
        }
        Err(e) => {
            tracing::warn!("javbus: parse failed for code={}: {}", code, e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_javbus_html() {
        let html = include_str!("../../tests/fixtures/javbus_sample.html");
        let meta = parse_javbus_html(html, "DLDSS-140").unwrap();

        assert_eq!(meta.title.as_deref(), Some("テスト動画タイトル"));
        assert_eq!(
            meta.cover_url.as_deref(),
            Some("https://www.javbus.com/pics/cover/955q_b.jpg")
        );
        assert_eq!(meta.released_at.as_deref(), Some("2022-09-22"));
        assert_eq!(meta.duration, Some(7200)); // 120 mins * 60
        assert_eq!(meta.maker.as_deref(), Some("DAHLIA"));
        assert_eq!(meta.series.as_deref(), Some("テストシリーズ"));
        assert_eq!(meta.actors, vec!["水川潤", "テスト女優"]);
        assert_eq!(meta.actor_details.len(), 2);
        assert_eq!(meta.actor_details[0].name, "水川潤");
        assert_eq!(
            meta.actor_details[0].photo_url.as_deref(),
            Some("https://www.javbus.com/pics/actress/zl8_a.jpg")
        );
        // Second actor has empty src → derived from href
        assert_eq!(
            meta.actor_details[1].photo_url.as_deref(),
            Some("https://www.javbus.com/pics/actress/abc_a.jpg")
        );
        assert_eq!(meta.tags, vec!["巨乳", "単体作品", "デビュー作品"]);
        assert_eq!(meta.sample_image_urls.len(), 2);
        assert_eq!(
            meta.sample_image_urls[0],
            "https://www.javbus.com/pics/sample/955q_1.jpg"
        );
    }

    #[test]
    fn test_parse_javbus_html_empty() {
        let html = "<html><body></body></html>";
        let result = parse_javbus_html(html, "ABC-123");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_javbus_title_strips_code() {
        let html = r#"<div class="container"><h3>ABC-123 Some Title</h3></div>"#;
        let meta = parse_javbus_html(html, "ABC-123").unwrap();
        assert_eq!(meta.title.as_deref(), Some("Some Title"));
    }
}
