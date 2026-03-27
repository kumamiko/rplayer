use std::collections::BTreeMap;
use std::time::Duration;

/// A single word/syllable with its start time (Enhanced LRC)
#[derive(Debug, Clone)]
pub struct LyricsWord {
    pub start: Duration,
    pub text: String,
}

/// A lyrics line, either timed (Enhanced LRC) or plain (standard LRC)
#[derive(Debug, Clone)]
pub enum LyricsLine {
    /// Enhanced LRC: each word has its own timestamp
    Timed { time: Duration, words: Vec<LyricsWord> },
    /// Standard LRC: whole line shares one timestamp, Vec<String> for bilingual support
    Plain { time: Duration, text: Vec<String> },
}

impl LyricsLine {
    pub fn time(&self) -> Duration {
        match self {
            LyricsLine::Timed { time, .. } => *time,
            LyricsLine::Plain { time, .. } => *time,
        }
    }


}

/// Parse LRC format lyrics, supporting both standard and Enhanced LRC
/// Standard:   [mm:ss.xx]lyrics text
/// Enhanced:   [mm:ss.xx]<mm:ss.xx>word1<mm:ss.xx>word2...
pub fn parse_lrc(content: &str) -> BTreeMap<Duration, LyricsLine> {
    let mut lyrics: BTreeMap<Duration, LyricsLine> = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('[') {
            continue;
        }

        // Parse leading timestamp tags
        let mut remaining = line;
        let mut timestamps = Vec::new();

        while remaining.starts_with('[') {
            if let Some(close_bracket) = remaining.find(']') {
                let tag = &remaining[1..close_bracket];
                remaining = &remaining[close_bracket + 1..];

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

        if timestamps.is_empty() {
            continue;
        }

        let text = remaining.trim();

        // Check if this line contains inline time tags (Enhanced LRC)
        if text.contains('<') {
            // Enhanced LRC: parse word-level timestamps
            let words = parse_timed_words(text);
            if !words.is_empty() {
                for ts in timestamps {
                    lyrics.insert(ts, LyricsLine::Timed {
                        time: ts,
                        words: words.clone(),
                    });
                }
            }
        } else {
            // Standard LRC: plain text line
            let text_str = text.to_string();
            for ts in timestamps {
                lyrics.entry(ts).or_insert_with(|| LyricsLine::Plain {
                    time: ts,
                    text: Vec::new(),
                });
                // Append to existing or new entry
                if let Some(LyricsLine::Plain { text, .. }) = lyrics.get_mut(&ts) {
                    text.push(text_str.clone());
                }
            }
        }
    }

    lyrics
}

/// Parse inline time tags: <mm:ss.xx>word1<mm:ss.xx>word2...
fn parse_timed_words(text: &str) -> Vec<LyricsWord> {
    let mut words = Vec::new();
    let mut remaining = text;
    let mut current_start: Option<Duration> = None;
    let mut current_text = String::new();

    while !remaining.is_empty() {
        if let Some(tag_start) = remaining.find('<') {
            // Save any text before the tag
            let before = &remaining[..tag_start];
            if !before.is_empty() {
                current_text.push_str(before);
            }

            // Look for closing '>'
            if let Some(tag_end) = remaining[tag_start..].find('>') {
                let tag_content = &remaining[tag_start + 1..tag_start + tag_end];
                remaining = &remaining[tag_start + tag_end + 1..];

                // If this tag contains a timestamp, finalize the previous word and start a new one
                if let Ok(ts) = parse_timestamp(tag_content) {
                    if !current_text.is_empty() {
                        if let Some(start) = current_start.take() {
                            words.push(LyricsWord {
                                start,
                                text: std::mem::take(&mut current_text),
                            });
                        }
                    }
                    current_start = Some(ts);
                } else {
                    // Not a timestamp tag, treat as literal text
                    current_text.push('<');
                    current_text.push_str(tag_content);
                    current_text.push('>');
                }
            } else {
                // Unclosed tag, treat rest as literal text
                current_text.push_str(&remaining[tag_start..]);
                break;
            }
        } else {
            current_text.push_str(remaining);
            break;
        }
    }

    // Don't forget the last word
    if !current_text.is_empty() {
        if let Some(start) = current_start {
            words.push(LyricsWord {
                start,
                text: current_text,
            });
        }
    }

    words
}

/// Parse timestamp in format mm:ss.xx or mm:ss:xx
fn parse_timestamp(s: &str) -> Result<Duration, ()> {
    let parts: Vec<&str> = s.split(&[':', '.'][..]).collect();

    if parts.len() < 2 || parts.len() > 3 {
        return Err(());
    }

    let minutes: u64 = parts[0].parse().map_err(|_| ())?;
    let seconds: u64 = parts[1].parse().map_err(|_| ())?;
    let centiseconds: u64 = if parts.len() == 3 {
        let cs_str = parts[2];
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
        match lyrics.get(&Duration::from_millis(12000)) {
            Some(LyricsLine::Plain { text, .. }) => {
                assert_eq!(text, &vec!["Line one".to_string()]);
            }
            _ => panic!("Expected Plain line"),
        }
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

    #[test]
    fn test_parse_bilingual_lrc() {
        let lrc = r#"
[00:14.71]甘い潮风がまた手招きしてる
[00:14.71]香甜的海风 又开始对我招手
[00:22.77]夕凪に响く 「待ってよ」
[00:22.77]在夕凪中回响著"等等我喔"
"#;
        let lyrics = parse_lrc(lrc);
        assert_eq!(lyrics.len(), 2);
        match lyrics.get(&Duration::from_millis(14710)) {
            Some(LyricsLine::Plain { text, .. }) => {
                assert_eq!(text.len(), 2);
                assert_eq!(text[0], "甘い潮风がまた手招きしてる");
                assert_eq!(text[1], "香甜的海风 又开始对我招手");
            }
            _ => panic!("Expected Plain line"),
        }
    }

    #[test]
    fn test_parse_enhanced_lrc() {
        let lrc = "[00:14.71]<00:14.71>甘<00:14.95>い<00:15.10>潮<00:15.30>风<00:15.55>が<00:15.80>ま<00:16.00>た<00:16.20>手<00:16.45>招<00:16.70>き<00:16.95>し<00:17.20>て<00:17.45>る";
        let lyrics = parse_lrc(lrc);
        assert_eq!(lyrics.len(), 1);

        match lyrics.get(&Duration::from_millis(14710)) {
            Some(LyricsLine::Timed { words, .. }) => {
                assert_eq!(words.len(), 13);
                assert_eq!(words[0].text, "甘");
                assert_eq!(words[0].start, Duration::from_millis(14710));
                assert_eq!(words[1].text, "い");
                assert_eq!(words[1].start, Duration::from_millis(14950));
                assert_eq!(words[12].text, "る");
                assert_eq!(words[12].start, Duration::from_millis(17450));
            }
            _ => panic!("Expected Timed line"),
        }
    }

    #[test]
    fn test_parse_enhanced_lrc_multiline() {
        let lrc = r#"
[00:14.71]<00:14.71>甘<00:14.95>い<00:15.10>潮风
[00:17.20]<00:17.20>夕凪に响く
[00:22.00]<00:22.00>待ってよ
"#;
        let lyrics = parse_lrc(lrc);
        assert_eq!(lyrics.len(), 3);

        assert!(lyrics.get(&Duration::from_millis(14710)).unwrap().is_timed());
        assert!(lyrics.get(&Duration::from_millis(17200)).unwrap().is_timed());
        assert!(lyrics.get(&Duration::from_millis(22000)).unwrap().is_timed());
    }

    #[test]
    fn test_parse_enhanced_lrc_words() {
        let lrc = "[00:10.00]<00:10.00>Hello <00:10.50>World<00:11.00>!";
        let lyrics = parse_lrc(lrc);
        match lyrics.get(&Duration::from_millis(10000)) {
            Some(LyricsLine::Timed { words, .. }) => {
                assert_eq!(words.len(), 3);
                assert_eq!(words[0].text, "Hello ");
                assert_eq!(words[0].start, Duration::from_millis(10000));
                assert_eq!(words[1].text, "World");
                assert_eq!(words[1].start, Duration::from_millis(10500));
                assert_eq!(words[2].text, "!");
                assert_eq!(words[2].start, Duration::from_millis(11000));
            }
            _ => panic!("Expected Timed line"),
        }
    }
}
