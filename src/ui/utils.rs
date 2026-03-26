use std::time::Duration;
use unicode_width::UnicodeWidthStr;

/// Format duration as "M:SS" (statusbar style)
pub fn format_duration_compact(d: Duration) -> String {
    let total_secs = d.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Format duration as " M:SS" (playlist style, right-aligned minutes)
pub fn format_duration_wide(d: Duration) -> String {
    let total_secs = d.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:2}:{:02}", mins, secs)
}

/// Truncate string by display width, safe for UTF-8. Appends "..." if truncated.
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let mut width = 0;
    let mut result = String::new();

    for ch in s.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + ch_width > max_width - 3 {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    if result.len() < s.len() {
        format!("{}...", result)
    } else {
        result
    }
}
