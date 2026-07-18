use std::time::{SystemTime, UNIX_EPOCH};

/// Current Unix timestamp as a string.
pub fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

/// Current Unix timestamp in nanoseconds (for unique IDs).
pub fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

/// Count words in a text string.
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Normalize text: strip CRLF, collapse excess blank lines, trim.
pub fn normalize_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut blank_lines = 0;
    let mut lines = Vec::new();
    for line in normalized.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.trim().is_empty() {
            blank_lines += 1;
            if blank_lines <= 2 {
                lines.push(String::new());
            }
        } else {
            blank_lines = 0;
            lines.push(trimmed_end.to_string());
        }
    }

    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_text_trims_crlf_and_extra_blank_lines() {
        assert_eq!(
            normalize_text(" hello \r\n\r\n\r\nworld  "),
            "hello\n\n\nworld"
        );
    }
}
