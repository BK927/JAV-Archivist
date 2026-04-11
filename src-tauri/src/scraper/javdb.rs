use super::types::{ScrapeError, ScrapedActor, ScrapedMetadata};
use scraper::{Html, Selector};

fn normalize_detail_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut absolute = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://javdb.com{}", trimmed)
    };

    if !absolute.contains("locale=") {
        let separator = if absolute.contains('?') { '&' } else { '?' };
        absolute.push(separator);
        absolute.push_str("locale=en");
    }

    Some(absolute)
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_code_prefix(value: &str, code: &str) -> Option<String> {
    let trimmed = normalize_whitespace(value);
    if trimmed.is_empty() {
        return None;
    }

    Some(
        trimmed
            .strip_prefix(code)
            .map(|rest| rest.trim().to_string())
            .filter(|rest| !rest.is_empty())
            .unwrap_or(trimmed),
    )
}

fn normalize_date(value: &str) -> Option<String> {
    let normalized = value.trim().replace('/', "-");
    let parts: Vec<&str> = normalized.split('-').collect();
    if parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()))
    {
        Some(normalized)
    } else {
        None
    }
}

fn parse_minutes(value: &str) -> Option<u64> {
    let lowered = value.to_lowercase();
    if !(lowered.contains("minute") || lowered.contains('分')) {
        return None;
    }

    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    let minutes = digits.parse::<u64>().ok()?;
    Some(minutes * 60)
}

fn extract_title(document: &Html, code: &str) -> Option<String> {
    let h2_sel = Selector::parse("h2.title").ok()?;
    if let Some(el) = document.select(&h2_sel).next() {
        let text = el.text().collect::<String>();
        if let Some(title) = strip_code_prefix(&text, code) {
            return Some(title);
        }
    }

    let title_sel = Selector::parse("title").ok()?;
    document
        .select(&title_sel)
        .next()
        .and_then(|el| strip_code_prefix(&el.text().collect::<String>(), code))
        .map(|title| {
            title
                .split('|')
                .next()
                .map(str::trim)
                .unwrap_or(&title)
                .to_string()
        })
}

pub(crate) fn parse_javdb_search_results(html: &str, code: &str) -> Result<String, ScrapeError> {
    let document = Html::parse_document(html);
    let link_sel = Selector::parse("a[href]").unwrap();
    let uid_sel = Selector::parse(".uid").unwrap();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if !href.contains("/v/") {
            continue;
        }

        let found_code = link
            .select(&uid_sel)
            .next()
            .map(|el| normalize_whitespace(&el.text().collect::<String>()))
            .unwrap_or_default();
        if !found_code.eq_ignore_ascii_case(code) {
            continue;
        }

        if let Some(url) = normalize_detail_url(href) {
            return Ok(url);
        }
    }

    Err(ScrapeError::ParseError(
        "no exact search match found".to_string(),
    ))
}

pub(crate) fn parse_javdb_html(html: &str, code: &str) -> Result<ScrapedMetadata, ScrapeError> {
    let document = Html::parse_document(html);
    let mut meta = ScrapedMetadata {
        title: extract_title(&document, code),
        ..Default::default()
    };

    let cover_sel = Selector::parse("img.video-cover[src]").unwrap();
    meta.cover_url = document
        .select(&cover_sel)
        .next()
        .and_then(|el| el.value().attr("src"))
        .and_then(super::normalize_media_url);

    let value_sel = Selector::parse("span.value").unwrap();
    for el in document.select(&value_sel) {
        let text = normalize_whitespace(&el.text().collect::<String>());
        if meta.released_at.is_none() {
            meta.released_at = normalize_date(&text);
        }
        if meta.duration.is_none() {
            meta.duration = parse_minutes(&text);
        }
    }

    let maker_sel = Selector::parse("a[href*='/makers/']").unwrap();
    meta.maker = document
        .select(&maker_sel)
        .next()
        .map(|el| normalize_whitespace(&el.text().collect::<String>()))
        .filter(|value| !value.is_empty());

    let series_sel = Selector::parse("a[href*='/series/']").unwrap();
    meta.series = document
        .select(&series_sel)
        .next()
        .map(|el| normalize_whitespace(&el.text().collect::<String>()))
        .filter(|value| !value.is_empty());

    let actor_sel = Selector::parse("a[href*='/actors/']").unwrap();
    for el in document.select(&actor_sel) {
        let name = normalize_whitespace(&el.text().collect::<String>());
        if name.is_empty() || meta.actors.contains(&name) {
            continue;
        }

        meta.actors.push(name.clone());
        meta.actor_details.push(ScrapedActor {
            name,
            name_kanji: None,
            photo_url: None,
        });
    }

    let tag_sel = Selector::parse("a[href*='/tags/']").unwrap();
    for el in document.select(&tag_sel) {
        let tag = normalize_whitespace(&el.text().collect::<String>());
        if !tag.is_empty() && !meta.tags.contains(&tag) {
            meta.tags.push(tag);
        }
    }

    let sample_sel = Selector::parse("a.tile-item[data-fancybox='gallery'][href]").unwrap();
    for el in document.select(&sample_sel) {
        if let Some(href) = el.value().attr("href") {
            if let Some(url) = super::normalize_media_url(href) {
                meta.sample_image_urls.push(url);
            }
        }
    }

    if !meta.has_any_field() {
        return Err(ScrapeError::ParseError(
            "no metadata found in HTML".to_string(),
        ));
    }

    Ok(meta)
}

pub async fn fetch(code: &str, client: &rquest::Client) -> Result<ScrapedMetadata, ScrapeError> {
    let search_url = format!("https://javdb.com/search?q={}&f=all", code);
    tracing::debug!("javdb: searching {}", search_url);

    let search_resp = client
        .get(&search_url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    let search_status = search_resp.status().as_u16();
    if search_status == 404 {
        return Err(ScrapeError::NotFound);
    }
    if search_status == 403 || search_status == 429 {
        return Err(ScrapeError::RateLimited);
    }
    if search_status != 200 {
        return Err(ScrapeError::NetworkError(format!("HTTP {}", search_status)));
    }

    let search_body = search_resp
        .text()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;
    let detail_url = parse_javdb_search_results(&search_body, code)?;

    tracing::debug!("javdb: fetching detail {}", detail_url);

    let detail_resp = client
        .get(&detail_url)
        .send()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    let detail_status = detail_resp.status().as_u16();
    if detail_status == 404 {
        return Err(ScrapeError::NotFound);
    }
    if detail_status == 403 || detail_status == 429 {
        return Err(ScrapeError::RateLimited);
    }
    if detail_status != 200 {
        return Err(ScrapeError::NetworkError(format!("HTTP {}", detail_status)));
    }

    let detail_body = detail_resp
        .text()
        .await
        .map_err(|e| ScrapeError::NetworkError(e.to_string()))?;

    if detail_body.contains("activate_registration")
        || detail_body.contains("至少有 18 歲")
        || detail_body.contains("legal age")
    {
        return Err(ScrapeError::RateLimited);
    }

    parse_javdb_html(&detail_body, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_javdb_search_results_prefers_exact_code_match() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
  <div class="grid columns is-multiline">
    <a class="box" href="/v/wrong-match" title="Wrong Match">
      <div class="uid">ABPN-001</div>
      <div class="video-title">Wrong Match</div>
    </a>
    <a class="box" href="/v/right-match" title="Exact Match">
      <div class="uid">ABP-001</div>
      <div class="video-title">Exact Match</div>
    </a>
  </div>
</body>
</html>"#;

        let detail_url = parse_javdb_search_results(html, "ABP-001").unwrap();

        assert_eq!(detail_url, "https://javdb.com/v/right-match?locale=en");
    }

    #[test]
    fn test_parse_javdb_search_results_errors_without_exact_match() {
        let html = r#"<!DOCTYPE html>
<html>
<body>
  <a class="box" href="/v/wrong-match" title="Wrong Match">
    <div class="uid">ABPN-001</div>
    <div class="video-title">Wrong Match</div>
  </a>
</body>
</html>"#;

        assert!(parse_javdb_search_results(html, "ABP-001").is_err());
    }

    #[test]
    fn test_parse_javdb_html() {
        let html = include_str!("../../tests/fixtures/javdb_detail_sample.html");
        let meta = parse_javdb_html(html, "ABP-001").unwrap();

        assert_eq!(meta.title.as_deref(), Some("穏やかな午後のテスト作品"));
        assert_eq!(
            meta.cover_url.as_deref(),
            Some("https://images.javdb.test/covers/abp001.jpg")
        );
        assert_eq!(meta.released_at.as_deref(), Some("2024-05-31"));
        assert_eq!(meta.duration, Some(150 * 60));
        assert_eq!(meta.maker.as_deref(), Some("Idea Pocket"));
        assert_eq!(meta.series.as_deref(), Some("午後のテスト"));
        assert_eq!(meta.actors, vec!["三上悠亜", "テスト女優"]);
        assert_eq!(meta.actor_details.len(), 2);
        assert_eq!(meta.tags, vec!["單體作品", "高清", "企劃"]);
        assert_eq!(meta.sample_image_urls.len(), 2);
        assert_eq!(
            meta.sample_image_urls[0],
            "https://images.javdb.test/samples/abp001-1.jpg"
        );
    }
}
