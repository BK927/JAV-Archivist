use scraper::{Html, Selector};
use super::types::{ScrapedMetadata, ScrapeError};

/// Parse ISO 8601 duration like "PT1H2M3S" to seconds
fn parse_iso_duration(s: &str) -> Option<u64> {
    let s = s.strip_prefix("PT")?;
    let mut total = 0u64;
    let mut buf = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            buf.push(c);
        } else {
            let n: u64 = buf.parse().ok()?;
            buf.clear();
            match c {
                'H' => total += n * 3600,
                'M' => total += n * 60,
                'S' => total += n,
                _ => {}
            }
        }
    }
    if total > 0 { Some(total) } else { None }
}

/// Decode a percent-encoded URL path segment (e.g. %E3%83%86 → UTF-8 "テ")
fn percent_decode(s: &str) -> String {
    let mut result: Vec<u8> = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Extract seller name from `<a href="/seller/{id}/{encoded-name}">` links
fn extract_seller(doc: &Html) -> Option<String> {
    let sel = Selector::parse("a[href*='/seller/']").ok()?;
    for el in doc.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            // e.g. /seller/6273/%E3%83%86%E3%82%B9...
            let parts: Vec<&str> = href.split('/').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 && parts[0] == "seller" {
                let name_part = parts[2];
                // Skip if purely numeric (it's an ID, not a name)
                if !name_part.chars().all(|c| c.is_ascii_digit()) {
                    let name = percent_decode(name_part);
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn parse_javten_page(html: &str) -> Result<ScrapedMetadata, ScrapeError> {
    let doc = Html::parse_document(html);
    let seller = extract_seller(&doc);

    let jsonld_sel = Selector::parse("script[type='application/ld+json']").unwrap();
    for el in doc.select(&jsonld_sel) {
        let json_text = el.inner_html();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_text) {
            if v.get("@type").and_then(|t| t.as_str()) != Some("Movie") {
                continue;
            }

            let mut meta = ScrapedMetadata::default();

            meta.title = v["name"].as_str().map(|s| s.trim().to_string());
            meta.cover_url = v["image"].as_str().map(|s| s.to_string());

            if let Some(genres) = v["genre"].as_array() {
                meta.tags = genres
                    .iter()
                    .filter_map(|g| g.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }

            if let Some(dur_str) = v["duration"].as_str() {
                meta.duration = parse_iso_duration(dur_str);
            }

            meta.released_at = v["datePublished"]
                .as_str()
                .or_else(|| v["dateCreated"].as_str())
                .map(|s| s.trim().to_string());

            meta.maker = seller.clone();

            if meta.has_any_field() {
                return Ok(meta);
            }
        }
    }

    Err(ScrapeError::ParseError("no Movie JSON-LD found".to_string()))
}

pub async fn fetch(code: &str, client: &rquest::Client) -> Result<ScrapedMetadata, ScrapeError> {
    let fc2_num = code.strip_prefix("FC2-PPV-").ok_or_else(|| {
        ScrapeError::ParseError(format!("not an FC2 code: {}", code))
    })?;

    tracing::debug!("javten: searching for code={}", code);

    // Step 1: Search → get 302 redirect Location
    let search_url = format!("https://javten.com/search?kw={}", fc2_num);
    let search_resp = client
        .get(&search_url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    let location = search_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            tracing::warn!("javten: no redirect for code={}", code);
            ScrapeError::NotFound
        })?;

    let _ = search_resp.bytes().await;

    // Step 2: Build HTTPS detail URL, strip language prefix
    // Location: http://javten.com/ko/video/{id}/id{fc2}/{slug}
    // Target:   https://javten.com/video/{id}/id{fc2}/{slug}
    let detail_url = location
        .replace("http://", "https://")
        .replace("https://javten.com/ko/", "https://javten.com/")
        .replace("https://javten.com/en/", "https://javten.com/");

    tracing::debug!("javten: fetching detail url={}", detail_url);

    // Step 3: Fetch detail page with up to 2 retries (server is occasionally unstable)
    let mut last_err = ScrapeError::NotFound;
    for attempt in 0..=2 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        let resp = match client.get(&detail_url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("javten: network error attempt={}: {}", attempt, e);
                last_err = ScrapeError::NetworkError(e.to_string());
                continue;
            }
        };

        let status = resp.status().as_u16();
        if status == 404 {
            return Err(ScrapeError::NotFound);
        }
        if status == 429 {
            return Err(ScrapeError::RateLimited);
        }
        if status != 200 {
            tracing::warn!("javten: HTTP {} attempt={} for code={}", status, attempt, code);
            last_err = ScrapeError::NetworkError(format!("HTTP {}", status));
            continue;
        }

        let body = resp
            .text()
            .await
            .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

        return match parse_javten_page(&body) {
            Ok(meta) => {
                tracing::debug!(
                    "javten: parsed metadata for code={} title={:?}",
                    code,
                    meta.title
                );
                Ok(meta)
            }
            Err(e) => {
                tracing::error!("javten: parse failed for code={}: {}", code, e);
                Err(e)
            }
        };
    }

    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_duration_minutes_seconds() {
        assert_eq!(parse_iso_duration("PT55M10S"), Some(55 * 60 + 10));
    }

    #[test]
    fn test_parse_iso_duration_hours_minutes() {
        assert_eq!(parse_iso_duration("PT1H30M"), Some(5400));
    }

    #[test]
    fn test_parse_iso_duration_seconds_only() {
        assert_eq!(parse_iso_duration("PT45S"), Some(45));
    }

    #[test]
    fn test_parse_iso_duration_hours_minutes_seconds() {
        assert_eq!(parse_iso_duration("PT1H2M3S"), Some(3723));
    }

    #[test]
    fn test_percent_decode_japanese() {
        assert_eq!(
            percent_decode("%E3%83%86%E3%82%B9%E3%83%88%E3%82%BB%E3%83%A9%E3%83%BC"),
            "テストセラー"
        );
    }

    #[test]
    fn test_parse_javten_page() {
        let html = include_str!("../../tests/fixtures/javten_sample.html");
        let meta = parse_javten_page(html).unwrap();
        assert_eq!(meta.title.as_deref(), Some("テスト動画タイトル"));
        assert_eq!(
            meta.cover_url.as_deref(),
            Some("https://cdn.fc2.com/test/thumbnail.jpg")
        );
        assert_eq!(meta.duration, Some(55 * 60 + 10));
        assert_eq!(meta.released_at.as_deref(), Some("2024-03-15"));
        assert_eq!(meta.tags, vec!["フェラ", "美人", "素人"]);
        assert_eq!(meta.maker.as_deref(), Some("テストセラー"));
    }

    #[test]
    fn test_parse_javten_page_no_jsonld() {
        let html = "<html><body></body></html>";
        assert!(parse_javten_page(html).is_err());
    }

    #[test]
    fn test_parse_javten_page_wrong_type() {
        let html = r#"<html><head><script type="application/ld+json">{"@type":"WebSite"}</script></head></html>"#;
        assert!(parse_javten_page(html).is_err());
    }
}
