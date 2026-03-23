use std::collections::BTreeMap;
use std::time::Duration;

/// Parse LRC format lyrics
/// Format: [mm:ss.xx]lyrics text
pub fn parse_lrc(content: &str) -> BTreeMap<Duration, String> {
    let mut lyrics = BTreeMap::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('[') {
            continue;
        }
        
        // Parse timestamp tags
        let mut remaining = line;
        let mut timestamps = Vec::new();
        
        while remaining.starts_with('[') {
            if let Some(close_bracket) = remaining.find(']') {
                let tag = &remaining[1..close_bracket];
                remaining = &remaining[close_bracket + 1..];
                
                // Check if it's a timestamp (not metadata like [ti:title])
                if let Ok(ts) = parse_timestamp(tag) {
                    timestamps.push(ts);
                } else {
                    // Metadata tag, skip
                    break;
                }
            } else {
                break;
            }
        }
        
        let text = remaining.trim().to_string();
        
        for ts in timestamps {
            lyrics.insert(ts, text.clone());
        }
    }
    
    lyrics
}

/// Parse timestamp in format mm:ss.xx or mm:ss:xx
fn parse_timestamp(s: &str) -> Result<Duration, ()> {
    // Handle format: mm:ss.xx or mm:ss:xx
    let parts: Vec<&str> = s.split(&[':', '.'][..]).collect();
    
    if parts.len() < 2 || parts.len() > 3 {
        return Err(());
    }
    
    let minutes: u64 = parts[0].parse().map_err(|_| ())?;
    let seconds: u64 = parts[1].parse().map_err(|_| ())?;
    let centiseconds: u64 = if parts.len() == 3 {
        let cs_str = parts[2];
        // Pad or truncate to 2 digits
        if cs_str.len() >= 2 {
            cs_str[..2].parse().unwrap_or(0)
        } else {
            cs_str.parse().unwrap_or(0) * 10
        }
    } else {
        0
    };
    
    Ok(Duration::from_millis(minutes * 60 * 1000 + seconds * 1000 + centiseconds * 10))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_lrc() {
        let lrc = r#"
[00:12.00]Line one
[00:17.20]Line two
[01:23.45]Line three
"#;
        let lyrics = parse_lrc(lrc);
        assert_eq!(lyrics.len(), 3);
        assert_eq!(lyrics.get(&Duration::from_millis(12000)), Some(&"Line one".to_string()));
    }
    
    #[test]
    fn test_parse_with_metadata() {
        let lrc = r#"
[ti:Song Title]
[ar:Artist]
[00:01.00]First line
"#;
        let lyrics = parse_lrc(lrc);
        assert_eq!(lyrics.len(), 1);
    }
}
