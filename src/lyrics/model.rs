use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricWord {
    pub text: String,
    pub start: f64,
    pub end: Option<f64>,
}

impl LyricWord {
    pub fn new(text: impl Into<String>, start: f64, end: Option<f64>) -> Self {
        Self {
            text: text.into(),
            start,
            end,
        }
    }

    pub fn duration(&self) -> f64 {
        self.end.map(|e| (e - self.start).max(0.1)).unwrap_or(0.3)
    }

    pub fn progress_at(&self, position: f64) -> f32 {
        if position < self.start {
            return 0.0;
        }
        let end = self.end.unwrap_or(self.start + 0.35);
        if position >= end {
            return 1.0;
        }
        let dur = (end - self.start).max(0.01);
        ((position - self.start) / dur).clamp(0.0, 1.0) as f32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricLine {
    pub words: Vec<LyricWord>,
    pub start: f64,
    pub end: Option<f64>,
    pub raw_text: String,
}

impl LyricLine {
    pub fn new(words: Vec<LyricWord>, start: f64, end: Option<f64>) -> Self {
        let raw_text = words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            words,
            start,
            end,
            raw_text,
        }
    }

    pub fn duration(&self) -> f64 {
        self.end.map(|e| (e - self.start).max(0.5)).unwrap_or(3.0)
    }

    pub fn active_word_index_at(&self, position: f64) -> Option<usize> {
        if position < self.start {
            return None;
        }

        for (idx, word) in self.words.iter().enumerate() {
            let word_end = word.end.unwrap_or(word.start + 0.35);
            if position >= word.start && position < word_end {
                return Some(idx);
            }
        }

        // If after the last word but within line duration, point to the last word
        if let Some(last) = self.words.last() {
            if position >= last.start {
                return Some(self.words.len().saturating_sub(1));
            }
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncedLyrics {
    pub lines: Vec<LyricLine>,
    pub is_word_synced: bool,
}

impl SyncedLyrics {
    pub fn new(lines: Vec<LyricLine>, is_word_synced: bool) -> Self {
        Self {
            lines,
            is_word_synced,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Finds the active line index, active word index within that line, and word progress
    pub fn find_active(&self, position: f64) -> Option<(usize, Option<usize>, f32)> {
        if self.lines.is_empty() {
            return None;
        }

        // Find the line that covers the current position
        let mut active_line_idx = None;

        for (idx, line) in self.lines.iter().enumerate() {
            let next_start = self.lines.get(idx + 1).map(|l| l.start);
            let line_end = line.end.or(next_start).unwrap_or(line.start + 4.0);

            if position >= line.start && position < line_end {
                active_line_idx = Some(idx);
                break;
            }
        }

        // If before the first line
        if active_line_idx.is_none() && position < self.lines[0].start {
            return Some((0, None, 0.0));
        }

        // If after the last line
        if active_line_idx.is_none() {
            let last_idx = self.lines.len() - 1;
            return Some((
                last_idx,
                Some(self.lines[last_idx].words.len().saturating_sub(1)),
                1.0,
            ));
        }

        let line_idx = active_line_idx.unwrap();
        let line = &self.lines[line_idx];

        if !self.is_word_synced {
            // Line-only synchronization fallback: interpolate word progression across line duration
            let line_dur = line.duration();
            let progress = ((position - line.start) / line_dur).clamp(0.0, 1.0) as f32;

            if !line.words.is_empty() {
                let count = line.words.len();
                let word_float = progress * count as f32;
                let word_idx = (word_float as usize).min(count - 1);
                let word_prog = word_float.fract();
                return Some((line_idx, Some(word_idx), word_prog));
            }

            return Some((line_idx, None, progress));
        }

        let word_idx = line.active_word_index_at(position);
        let word_progress = if let Some(w_idx) = word_idx {
            line.words[w_idx].progress_at(position)
        } else {
            0.0
        };

        Some((line_idx, word_idx, word_progress))
    }
}
