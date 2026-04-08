use regex::Regex;

/// Extract a video code from a text string (filename or folder name).
/// Returns the normalized code or None if no pattern matches.
pub fn extract_code(text: &str) -> Option<String> {
    // FC2 pattern: FC2-PPV-123, FC2PPV 123, FC2PPV123, etc.
    let fc2_re = Regex::new(r"(?i)FC2[-\s]?PPV[-\s]?(\d+)").unwrap();
    if let Some(caps) = fc2_re.captures(text) {
        let digits = &caps[1];
        return Some(format!("FC2-PPV-{}", digits));
    }

    // General pattern: ABC-123, ABCD-12345
    let general_re = Regex::new(r"(?i)([A-Z]{2,6})-(\d{3,5})").unwrap();
    if let Some(caps) = general_re.captures(text) {
        let prefix = caps[1].to_uppercase();
        let number = &caps[2];
        return Some(format!("{}-{}", prefix, number));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_code() {
        assert_eq!(extract_code("ABC-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("ABCD-12345"), Some("ABCD-12345".to_string()));
        assert_eq!(extract_code("SONE-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_case_insensitive() {
        assert_eq!(extract_code("abc-123"), Some("ABC-123".to_string()));
        assert_eq!(extract_code("sone-001"), Some("SONE-001".to_string()));
    }

    #[test]
    fn test_general_code_in_noisy_filename() {
        assert_eq!(
            extract_code("[1080p] ABC-123 actress_name"),
            Some("ABC-123".to_string())
        );
        assert_eq!(
            extract_code("some_prefix_MIDE-456_suffix"),
            Some("MIDE-456".to_string())
        );
    }

    #[test]
    fn test_fc2_canonical() {
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_no_hyphens() {
        assert_eq!(
            extract_code("FC2PPV1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_with_spaces() {
        assert_eq!(
            extract_code("FC2PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
        assert_eq!(
            extract_code("FC2 PPV 1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_case_insensitive() {
        assert_eq!(
            extract_code("fc2-ppv-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_fc2_takes_priority_over_general() {
        assert_eq!(
            extract_code("FC2-PPV-1234567"),
            Some("FC2-PPV-1234567".to_string())
        );
    }

    #[test]
    fn test_no_match() {
        assert_eq!(extract_code("random_video"), None);
        assert_eq!(extract_code("video_20240301"), None);
        assert_eq!(extract_code(""), None);
    }
}
