use scraper::{Html, Selector};
use super::types::{ScrapedMetadata, ScrapeError};

/// Extract numeric ID from "FC2-PPV-1234567"
fn extract_fc2_id(code: &str) -> Option<&str> {
    code.strip_prefix("FC2-PPV-")
}

pub(crate) fn parse_fc2_html(html: &str) -> Result<ScrapedMetadata, ScrapeError> {
    let document = Html::parse_document(html);
    let mut meta = ScrapedMetadata::default();

    // 1. Parse JSON-LD for title, cover, seller
    let json_ld_sel = Selector::parse("script[type='application/ld+json']").unwrap();
    if let Some(el) = document.select(&json_ld_sel).next() {
        let json_text = el.inner_html();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_text) {
            meta.title = value["name"].as_str().map(|s| s.to_string());
            meta.cover_url = value["image"]["url"].as_str().map(|s| s.to_string());

            // Extract seller username from brand.url: ".../users/testuser/" → "testuser"
            if let Some(brand_url) = value["brand"]["url"].as_str() {
                if let Some(username) = brand_url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                {
                    if !username.is_empty() {
                        meta.maker = Some(username.to_string());
                    }
                }
            }
        }
    }

    // 2. Parse tags from data-tag attributes
    let tag_sel = Selector::parse(".items_article_TagArea a.tagTag[data-tag]").unwrap();
    for el in document.select(&tag_sel) {
        if let Some(tag) = el.value().attr("data-tag") {
            meta.tags.push(tag.to_string());
        }
    }

    // 3. Parse release date from "Sale Day : YYYY/MM/DD"
    let device_sel = Selector::parse(".items_article_softDevice > p").unwrap();
    for el in document.select(&device_sel) {
        let text = el.text().collect::<String>();
        if let Some(date_part) = text.strip_prefix("Sale Day : ") {
            let trimmed = date_part.trim();
            // Convert YYYY/MM/DD → YYYY-MM-DD
            meta.released_at = Some(trimmed.replace('/', "-"));
        }
    }

    if !meta.has_any_field() {
        return Err(ScrapeError::ParseError("no metadata found in HTML".to_string()));
    }

    Ok(meta)
}

pub async fn fetch(code: &str, client: &rquest::Client) -> Result<ScrapedMetadata, ScrapeError> {
    let id = extract_fc2_id(code).ok_or(ScrapeError::NotFound)?;
    let url = format!("https://adult.contents.fc2.com/article/{}/", id);
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

    // Short body likely means blocked or error page
    if body.len() < 1000 {
        return Err(ScrapeError::RateLimited);
    }

    parse_fc2_html(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_fc2_id() {
        assert_eq!(extract_fc2_id("FC2-PPV-1234567"), Some("1234567"));
        assert_eq!(extract_fc2_id("ABC-123"), None);
    }

    #[test]
    fn test_parse_fc2_html() {
        let html = include_str!("../../tests/fixtures/fc2_sample.html");
        let meta = parse_fc2_html(html).unwrap();

        assert_eq!(meta.title.as_deref(), Some("FC2テスト動画タイトル"));
        assert_eq!(
            meta.cover_url.as_deref(),
            Some("https://storage200000.contents.fc2.com/file/123/test_cover.jpg")
        );
        assert_eq!(meta.tags, vec!["射精", "素人", "中出し"]);
        assert_eq!(meta.released_at.as_deref(), Some("2025-06-03"));
        assert_eq!(meta.maker.as_deref(), Some("testuser"));
        assert!(meta.actors.is_empty()); // FC2 doesn't provide actors
        assert!(meta.duration.is_none()); // FC2 doesn't provide duration
    }

    #[test]
    fn test_parse_fc2_html_minimal() {
        let html = "<html><body></body></html>";
        let result = parse_fc2_html(html);
        assert!(result.is_err());
    }
}
