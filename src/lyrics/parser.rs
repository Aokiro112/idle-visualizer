use crate::lyrics::model::{LyricLine, LyricWord, SyncedLyrics};
use regex::Regex;
use std::sync::LazyLock;

static LINE_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(\d{1,2}):(\d{2})(?:\.(\d{2,3}))?\]").expect("Invalid line regex")
});

static WORD_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(\d{1,2}):(\d{2})(?:\.(\d{2,3}))?>").expect("Invalid word regex")
});

pub fn parse_timestamp(min: &str, sec: &str, frac: Option<&str>) -> f64 {
    let minutes: f64 = min.parse().unwrap_or(0.0);
    let seconds: f64 = sec.parse().unwrap_or(0.0);
    let fraction: f64 = match frac {
        Some(f) => {
            let val: f64 = f.parse().unwrap_or(0.0);
            if f.len() == 2 {
                val / 100.0
            } else {
                val / 1000.0
            }
        }
        None => 0.0,
    };
    minutes * 60.0 + seconds + fraction
}

pub struct LyricsParser;

impl LyricsParser {
    /// Parse any LRC formatted string, automatically identifying if it has word-level timestamps
    pub fn parse_lrc(content: &str) -> SyncedLyrics {
        let has_word_tags = content.contains('<') && WORD_TIME_REGEX.is_match(content);

        if has_word_tags {
            Self::parse_enhanced_lrc(content)
        } else {
            Self::parse_standard_lrc(content)
        }
    }

    /// Parse enhanced LRC with word timestamps
    fn parse_enhanced_lrc(content: &str) -> SyncedLyrics {
        let mut lines: Vec<LyricLine> = Vec::new();

        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with("[ti:") || trimmed.starts_with("[ar:") {
                continue;
            }

            if let Some(caps) = LINE_TIME_REGEX.captures(trimmed) {
                let min = &caps[1];
                let sec = &caps[2];
                let frac = caps.get(3).map(|m| m.as_str());
                let line_start = parse_timestamp(min, sec, frac);

                // Strip the leading [mm:ss.xx]
                let after_tag = &trimmed[caps.get(0).unwrap().end()..];

                // Parse word tokens like `<00:12.34>word` or `word <00:12.80>`
                let mut words: Vec<LyricWord> = Vec::new();
                let mut last_pos = 0;
                let mut current_word_time = line_start;

                for m in WORD_TIME_REGEX.find_iter(after_tag) {
                    let text_segment = after_tag[last_pos..m.start()].trim();
                    if !text_segment.is_empty() {
                        words.push(LyricWord::new(text_segment, current_word_time, None));
                    }

                    // Extract the new word timestamp
                    if let Some(word_caps) = WORD_TIME_REGEX.captures(m.as_str()) {
                        let w_min = &word_caps[1];
                        let w_sec = &word_caps[2];
                        let w_frac = word_caps.get(3).map(|c| c.as_str());
                        current_word_time = parse_timestamp(w_min, w_sec, w_frac);
                    }
                    last_pos = m.end();
                }

                // Any trailing text after the last <mm:ss.xx>
                let trailing = after_tag[last_pos..].trim();
                if !trailing.is_empty() {
                    words.push(LyricWord::new(trailing, current_word_time, None));
                }

                if !words.is_empty() {
                    // Fill end times for each word from the start time of the next word
                    for i in 0..words.len().saturating_sub(1) {
                        let next_start = words[i + 1].start;
                        words[i].end = Some(next_start);
                    }
                    // Estimate the last word's end time
                    if let Some(last) = words.last_mut() {
                        if last.end.is_none() {
                            last.end = Some(last.start + 0.45);
                        }
                    }

                    lines.push(LyricLine::new(words, line_start, None));
                }
            }
        }

        // Fill line end times based on the next line start time
        for i in 0..lines.len().saturating_sub(1) {
            let next_start = lines[i + 1].start;
            lines[i].end = Some(next_start);
        }

        SyncedLyrics::new(lines, true)
    }

    /// Parse standard LRC (line-synced only)
    fn parse_standard_lrc(content: &str) -> SyncedLyrics {
        let mut lines: Vec<LyricLine> = Vec::new();

        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with("[ti:") || trimmed.starts_with("[ar:") {
                continue;
            }

            if let Some(caps) = LINE_TIME_REGEX.captures(trimmed) {
                let min = &caps[1];
                let sec = &caps[2];
                let frac = caps.get(3).map(|m| m.as_str());
                let line_start = parse_timestamp(min, sec, frac);

                let text = trimmed[caps.get(0).unwrap().end()..].trim();
                if !text.is_empty() {
                    let raw_words: Vec<&str> = text.split_whitespace().collect();
                    let word_count = raw_words.len().max(1);

                    // For line-level lyrics, create words with proportional spacing
                    let estimated_line_duration = 3.5;
                    let per_word = estimated_line_duration / word_count as f64;

                    let words: Vec<LyricWord> = raw_words
                        .iter()
                        .enumerate()
                        .map(|(i, &w)| {
                            let start = line_start + (i as f64) * per_word;
                            let end = start + per_word * 0.9;
                            LyricWord::new(w, start, Some(end))
                        })
                        .collect();

                    lines.push(LyricLine::new(words, line_start, None));
                }
            }
        }

        // Fill line end times
        for i in 0..lines.len().saturating_sub(1) {
            let next_start = lines[i + 1].start;
            lines[i].end = Some(next_start);
        }

        SyncedLyrics::new(lines, false)
    }

    /// Parse unsynced plain text
    pub fn parse_plain_text(content: &str) -> SyncedLyrics {
        let mut lines = Vec::new();
        let mut current_time = 0.0;

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let words = trimmed
                    .split_whitespace()
                    .map(|w| LyricWord::new(w, current_time, Some(current_time + 0.4)))
                    .collect();
                lines.push(LyricLine::new(
                    words,
                    current_time,
                    Some(current_time + 4.0),
                ));
                current_time += 4.0;
            }
        }

        SyncedLyrics::new(lines, false)
    }
}
