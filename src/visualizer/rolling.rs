use crate::app::settings::ColorRgba;
use crate::lyrics::model::SyncedLyrics;
use crate::lyrics::synchronizer::ActiveSyncState;
use crate::media::player::TrackState;

#[derive(Debug, Clone)]
pub struct RollingWordItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub scale: f32,
    pub opacity: f32,
    pub is_active: bool,
    pub has_highlight: bool,
    pub highlight_opacity: f32,
    pub highlight_color: ColorRgba,
    pub text_color: ColorRgba,
    pub is_italic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedWord {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

pub const DEMO_WORDS: &[&str] = &[
    "me",
    "I'm",
    "stunning",
    "you",
    "when",
    "now",
    "see",
    "And",
    "Telling",
    "the",
    "chances",
    "blew",
    "up",
    "Or",
    "whatever",
];

pub struct RollingLyricEngine {
    // Demo words fallback
    pub words: Vec<String>,
    pub current_index: usize,
    pub time_in_step: f32,
    pub step_duration: f32,
    pub transition_duration: f32,
    pub is_playing: bool,
    pub drift_time: f32,

    // Real-time song lyrics from media player (e.g. YouTube Music Desktop)
    pub current_track_key: String,
    pub track_title: String,
    pub track_artist: String,
    pub song_words: Vec<TimedWord>,
    pub live_playback_pos: f64,
    pub has_live_lyrics: bool,
}

impl Default for RollingLyricEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RollingLyricEngine {
    pub fn new() -> Self {
        let words = DEMO_WORDS.iter().map(|s| s.to_string()).collect();

        Self {
            words,
            // Start at index 2 so "me" is top (slot 0), "I'm" is center (slot 1), "stunning" is active (slot 2)
            current_index: 2,
            time_in_step: 0.0,
            step_duration: 0.85,
            transition_duration: 0.38,
            is_playing: true,
            drift_time: 0.0,

            current_track_key: String::new(),
            track_title: String::new(),
            track_artist: String::new(),
            song_words: Vec::new(),
            live_playback_pos: 0.0,
            has_live_lyrics: false,
        }
    }

    pub fn toggle_play_pause(&mut self) -> bool {
        self.is_playing = !self.is_playing;
        self.is_playing
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.is_playing = playing;
    }

    /// Sync the rolling engine with real-time media player state and fetched lyrics
    pub fn sync_with_playback(
        &mut self,
        sync_state: &ActiveSyncState,
        media_state: &TrackState,
    ) {
        let has_metadata = media_state.metadata.is_some();

        if !has_metadata {
            self.has_live_lyrics = false;
            return;
        }

        self.is_playing = media_state.playback_state.is_playing();
        self.live_playback_pos = sync_state.playback_position;

        let meta = media_state.metadata.as_ref().unwrap();
        let track_key = format!("{} - {}", meta.artist, meta.title);
        self.track_title = crate::lyrics::lrclib::LrclibProvider::sanitize_title(&meta.title);
        self.track_artist = crate::lyrics::lrclib::LrclibProvider::primary_artist(&meta.artist);

        if let Some(ref lyrics) = sync_state.lyrics {
            if self.current_track_key != track_key || self.song_words.is_empty() {
                self.load_lyrics(&track_key, lyrics);
            }
            self.has_live_lyrics = !self.song_words.is_empty();
        } else {
            // Song is playing but lyrics are loading or not found
            if self.current_track_key != track_key {
                self.current_track_key = track_key.clone();
                let clean_title = crate::lyrics::lrclib::LrclibProvider::sanitize_title(&meta.title);
                let clean_artist = crate::lyrics::lrclib::LrclibProvider::primary_artist(&meta.artist);

                let mut placeholder = Vec::new();
                for w in clean_artist.split_whitespace() {
                    placeholder.push(w.to_string());
                }
                for w in clean_title.split_whitespace() {
                    placeholder.push(w.to_string());
                }
                if placeholder.len() >= 3 {
                    self.words = placeholder;
                    self.current_index = 1;
                    self.has_live_lyrics = false;
                }
            }
        }
    }

    /// Load and flatten synchronized lyrics into timed words
    pub fn load_lyrics(&mut self, track_key: &str, lyrics: &SyncedLyrics) {
        self.current_track_key = track_key.to_string();
        self.song_words.clear();

        for (line_idx, line) in lyrics.lines.iter().enumerate() {
            let next_line_start = lyrics.lines.get(line_idx + 1).map(|l| l.start);
            let line_end = line.end.or(next_line_start).unwrap_or(line.start + 3.5);
            let line_duration = (line_end - line.start).max(0.5);

            if lyrics.is_word_synced && !line.words.is_empty() {
                for (w_idx, word) in line.words.iter().enumerate() {
                    let w_start = word.start;
                    let w_end = word.end.unwrap_or_else(|| {
                        line.words
                            .get(w_idx + 1)
                            .map(|next_w| next_w.start)
                            .unwrap_or(w_start + 0.35)
                    });
                    self.song_words.push(TimedWord {
                        text: word.text.clone(),
                        start: w_start,
                        end: w_end,
                    });
                }
            } else if !line.words.is_empty() {
                // Line-synced: distribute words across active singing portion of the line
                let count = line.words.len();
                let active_duration = line_duration.min(4.8);
                let per_word = active_duration / count as f64;

                for (w_idx, word) in line.words.iter().enumerate() {
                    let w_start = line.start + (w_idx as f64) * per_word;
                    let w_end = w_start + per_word;
                    self.song_words.push(TimedWord {
                        text: word.text.clone(),
                        start: w_start,
                        end: w_end,
                    });
                }
            } else {
                let raw_words: Vec<&str> = line.raw_text.split_whitespace().collect();
                if !raw_words.is_empty() {
                    let count = raw_words.len();
                    let active_duration = line_duration.min(4.8);
                    let per_word = active_duration / count as f64;

                    for (w_idx, &w) in raw_words.iter().enumerate() {
                        let w_start = line.start + (w_idx as f64) * per_word;
                        let w_end = w_start + per_word;
                        self.song_words.push(TimedWord {
                            text: w.to_string(),
                            start: w_start,
                            end: w_end,
                        });
                    }
                }
            }
        }

        tracing::info!(
            "Loaded {} synced lyric words for '{}'",
            self.song_words.len(),
            track_key
        );
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_playing {
            return;
        }

        self.drift_time += dt;

        // When not in live song mode, roll demo words autonomously
        if !self.has_live_lyrics && !self.words.is_empty() {
            self.time_in_step += dt;
            while self.time_in_step >= self.step_duration {
                self.time_in_step -= self.step_duration;
                self.current_index = (self.current_index + 1) % self.words.len();
            }
        }
    }

    /// Calculate smooth rolling items currently visible in the vertical sequence
    pub fn get_rendered_words(&self) -> Vec<RollingWordItem> {
        if self.has_live_lyrics && !self.song_words.is_empty() {
            return self.get_rendered_song_words();
        }

        self.get_rendered_demo_words()
    }

    /// Render real song words based on exact playback position
    fn get_rendered_song_words(&self) -> Vec<RollingWordItem> {
        let pos = self.live_playback_pos;
        let n = self.song_words.len();
        if n == 0 {
            return Vec::new();
        }

        // 1. Find active word index
        let mut active_idx = 0;
        let mut found = false;

        if pos < self.song_words[0].start {
            active_idx = 0;
        } else {
            for i in 0..n {
                let w_start = self.song_words[i].start;
                let w_end = if i + 1 < n {
                    self.song_words[i + 1].start
                } else {
                    self.song_words[i].end + 2.0
                };

                if pos >= w_start && pos < w_end {
                    active_idx = i;
                    found = true;
                    break;
                }
            }
            if !found {
                active_idx = n - 1;
            }
        }

        let w = &self.song_words[active_idx];
        let next_start = if active_idx + 1 < n {
            self.song_words[active_idx + 1].start
        } else {
            w.end
        };
        let word_dur = (next_start - w.start).max(0.15);
        let trans_dur = (0.32f64).min(word_dur * 0.40);

        let is_in_intro = pos < self.song_words[0].start;

        let slot_shift = if is_in_intro {
            let time_to_start = self.song_words[0].start - pos;
            if time_to_start <= trans_dur {
                let trans_t = 1.0 - (time_to_start / trans_dur).clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - trans_t as f32).powi(3);
                1.0 - ease
            } else {
                0.0
            }
        } else {
            let elapsed = pos - w.start;
            if elapsed < trans_dur {
                let trans_t = (elapsed / trans_dur).clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - trans_t as f32).powi(3);
                1.0 - ease
            } else {
                0.0
            }
        };

        let mut items = Vec::new();

        // Window of words around active_idx: [-3 .. +2]
        for offset in -3..=2 {
            let target_idx = active_idx as isize + offset;
            let text = if target_idx < 0 {
                if is_in_intro && offset == -1 && !self.track_title.is_empty() {
                    self.track_title.clone()
                } else {
                    continue;
                }
            } else if target_idx >= n as isize {
                continue;
            } else {
                self.song_words[target_idx as usize].text.clone()
            };

            // Continuous slot coordinate:
            // S = 2.0 (Active slot) + offset + slot_shift
            let s = 2.0 + (offset as f32) + slot_shift;

            if s < -0.85 || s > 3.25 {
                continue;
            }

            let (y, z, scale, opacity, text_color, has_hl, hl_opacity, hl_color, is_italic) =
                Self::evaluate_slot(s);

            let effective_opacity = opacity.clamp(0.0, 1.0);
            if effective_opacity < 0.01 {
                continue;
            }

            let is_active = (s - 2.0).abs() < 0.35;

            items.push(RollingWordItem {
                text: text.clone(),
                x: 0.0,
                y,
                z,
                scale,
                opacity: effective_opacity,
                is_active,
                has_highlight: has_hl,
                highlight_opacity: hl_opacity,
                highlight_color: hl_color,
                text_color,
                is_italic,
            });
        }

        // Sort items by Z depth so front items draw on top
        items.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
        items
    }

    /// Render fallback demo words autonomously
    fn get_rendered_demo_words(&self) -> Vec<RollingWordItem> {
        if self.words.is_empty() {
            return Vec::new();
        }

        let n = self.words.len();
        let trans_t = (self.time_in_step / self.transition_duration).clamp(0.0, 1.0);
        // Smooth cubic ease-out for fast dynamic roll and gentle deceleration
        let ease = 1.0 - (1.0 - trans_t).powi(3);

        let mut items = Vec::new();

        // Examine window of words around current_index: [-3 .. +2]
        for offset in -3..=2 {
            let idx = ((self.current_index as isize + offset).rem_euclid(n as isize)) as usize;
            let text = &self.words[idx];

            // Continuous slot coordinate:
            // S = 2.0 (Active slot) + offset + (1.0 - ease)
            let s = 2.0 + (offset as f32) + (1.0 - ease);

            // Visible range: roughly S in [-0.85, 3.25]
            if s < -0.85 || s > 3.25 {
                continue;
            }

            let (y, z, scale, opacity, text_color, has_hl, hl_opacity, hl_color, is_italic) =
                Self::evaluate_slot(s);

            let effective_opacity = opacity.clamp(0.0, 1.0);
            if effective_opacity < 0.01 {
                continue;
            }

            let is_active = (s - 2.0).abs() < 0.35;

            items.push(RollingWordItem {
                text: text.clone(),
                x: 0.0,
                y,
                z,
                scale,
                opacity: effective_opacity,
                is_active,
                has_highlight: has_hl,
                highlight_opacity: hl_opacity,
                highlight_color: hl_color,
                text_color,
                is_italic,
            });
        }

        // Sort items by Z depth so front items draw on top
        items.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
        items
    }

    /// Evaluates continuous slot position S into exact geometric and photometric properties
    /// Slot reference:
    ///   S = -1.0: Exiting off the top
    ///   S =  0.0: Previous small word
    ///   S =  1.0: Monumental BIG WORD (center, visually dominant)
    ///   S =  2.0: Active word (with dusty pink rectangular highlight box, creamy ivory text)
    ///   S =  3.0: Upcoming word entering from bottom
    fn evaluate_slot(
        s: f32,
    ) -> (
        f32,       // y
        f32,       // z
        f32,       // scale
        f32,       // opacity
        ColorRgba, // text_color
        bool,      // has_highlight
        f32,       // highlight_opacity
        ColorRgba, // highlight_color
        bool,      // is_italic
    ) {
        // Muted warm beige / terracotta: #BA826C (186, 130, 108)
        let warm_beige = ColorRgba::new(0.729, 0.510, 0.424, 1.0);
        // Creamy off-white ivory for active text: #F5EFEB (245, 239, 235)
        let creamy_ivory = ColorRgba::new(0.961, 0.937, 0.922, 1.0);
        // Muted dusty pink / rose highlight: #B87483 (184, 116, 131)
        let dusty_rose = ColorRgba::new(0.722, 0.455, 0.514, 0.96);

        // Highlight box: specifically focused on the active word around S = 2.0
        let dist_from_active = (s - 2.0).abs();
        let hl_opacity = if dist_from_active < 0.42 {
            let norm = dist_from_active / 0.42;
            (1.0 - norm * norm).max(0.0)
        } else {
            0.0
        };
        let has_hl = hl_opacity > 0.02;

        let is_italic = hl_opacity > 0.30;
        let text_color = lerp_color(&warm_beige, &creamy_ivory, hl_opacity);

        if s <= 0.0 {
            // S in [-1.0 .. 0.0]: Top exiting region
            let t = ((s + 1.0) / 1.0).clamp(0.0, 1.0);
            let h = 3.0 * t * t - 2.0 * t * t * t;

            let y = lerp_f32(0.68, 0.32, h);
            let z = lerp_f32(-2.45, -2.38, h);
            let scale = lerp_f32(0.36, 0.52, h);
            let opacity = lerp_f32(0.0, 0.75, h);

            (
                y, z, scale, opacity, text_color, has_hl, hl_opacity, dusty_rose, is_italic,
            )
        } else if s <= 1.0 {
            // S in [0.0 .. 1.0]: Moving from Top (0.0) into Monumental Center BIG WORD (1.0)
            let t = s.clamp(0.0, 1.0);
            let h = 3.0 * t * t - 2.0 * t * t * t;

            let y = lerp_f32(0.32, 0.04, h);
            let z = lerp_f32(-2.38, -2.35, h);
            let scale = lerp_f32(0.52, 1.48, h); // Strong typographic scale hierarchy: expands to HUGE
            let opacity = lerp_f32(0.75, 1.00, h);

            (
                y, z, scale, opacity, text_color, has_hl, hl_opacity, dusty_rose, is_italic,
            )
        } else if s <= 2.0 {
            // S in [1.0 .. 2.0]: Moving from Active Slot (2.0) to Monumental Center BIG WORD (1.0)
            let t = (s - 1.0).clamp(0.0, 1.0);
            let h = 3.0 * t * t - 2.0 * t * t * t;

            let y = lerp_f32(0.04, -0.32, h);
            let z = lerp_f32(-2.35, -2.36, h);
            let scale = lerp_f32(1.48, 0.94, h);
            let opacity = 1.00;

            (
                y,
                z,
                scale,
                opacity,
                text_color,
                has_hl,
                hl_opacity,
                dusty_rose,
                is_italic,
            )
        } else {
            // S in [2.0 .. 3.0]: Upcoming word (3.0) entering into Active Slot (2.0)
            let t = ((s - 2.0) / 1.0).clamp(0.0, 1.0);
            let h = 3.0 * t * t - 2.0 * t * t * t;

            let y = lerp_f32(-0.32, -0.68, h);
            let z = lerp_f32(-2.36, -2.45, h);
            let scale = lerp_f32(0.94, 0.42, h);
            let opacity = lerp_f32(1.00, 0.00, h);

            (
                y,
                z,
                scale,
                opacity,
                text_color,
                has_hl,
                hl_opacity,
                dusty_rose,
                is_italic,
            )
        }
    }
}

#[inline]
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: &ColorRgba, b: &ColorRgba, t: f32) -> ColorRgba {
    ColorRgba::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initial_state() {
        let engine = RollingLyricEngine::new();
        assert!(engine.is_playing);
        assert_eq!(engine.words[0], "me");
        assert_eq!(engine.words[1], "I'm");
        assert_eq!(engine.words[2], "stunning");
    }

    #[test]
    fn test_play_pause_toggle() {
        let mut engine = RollingLyricEngine::new();
        assert!(engine.is_playing);

        // Pause
        let is_playing = engine.toggle_play_pause();
        assert!(!is_playing);
        assert!(!engine.is_playing);

        let t_before = engine.time_in_step;
        engine.update(0.5);
        // Time should not advance while paused
        assert_eq!(engine.time_in_step, t_before);

        // Resume
        let is_playing = engine.toggle_play_pause();
        assert!(is_playing);
        assert!(engine.is_playing);
        engine.update(0.5);
        assert!(engine.time_in_step > t_before);
    }

    #[test]
    fn test_slot_scale_hierarchy() {
        // Top slot (S = 0.0): small word
        let (_, _, scale_top, _, _, has_hl_top, hl_top, _, _) = RollingLyricEngine::evaluate_slot(0.0);
        assert!(scale_top < 0.60, "Top word must be small: got {}", scale_top);
        assert!(!has_hl_top, "Top word must not have highlight");
        assert_eq!(hl_top, 0.0);

        // Center slot (S = 1.0): Monumental HUGE word
        let (_, _, scale_center, _, _, has_hl_center, hl_center, _, _) =
            RollingLyricEngine::evaluate_slot(1.0);
        assert!(
            scale_center > 1.40,
            "Center word must be monumental HUGE: got {}",
            scale_center
        );
        assert!(!has_hl_center, "Center monumental word must not have highlight");
        assert_eq!(hl_center, 0.0);

        // Active slot (S = 2.0): Emphasized medium word with highlight box
        let (_, _, scale_active, _, _, has_hl_active, hl_active, _, is_italic) =
            RollingLyricEngine::evaluate_slot(2.0);
        assert!(
            scale_active > 0.85 && scale_active < 1.10,
            "Active word must be medium: got {}",
            scale_active
        );
        assert!(has_hl_active, "Active word must have highlight box");
        assert!(hl_active > 0.95, "Active word must have full highlight opacity");
        assert!(is_italic, "Active word must be styled italic");
    }

    #[test]
    fn test_highlight_strictly_local_to_active() {
        // At S = 1.5, highlight should be 0 or virtually gone
        let (_, _, _, _, _, has_hl_15, hl_15, _, _) = RollingLyricEngine::evaluate_slot(1.5);
        assert!(!has_hl_15);
        assert_eq!(hl_15, 0.0);

        // At S = 2.5, highlight should be 0
        let (_, _, _, _, _, has_hl_25, hl_25, _, _) = RollingLyricEngine::evaluate_slot(2.5);
        assert!(!has_hl_25);
        assert_eq!(hl_25, 0.0);
    }

    #[test]
    fn test_song_lyrics_loading_and_slot_sync() {
        let mut engine = RollingLyricEngine::new();
        assert!(!engine.has_live_lyrics);

        let lines = vec![
            crate::lyrics::model::LyricLine::new(
                vec![
                    crate::lyrics::model::LyricWord::new("Tu", 10.0, Some(11.0)),
                    crate::lyrics::model::LyricWord::new("Agar", 11.0, Some(12.0)),
                    crate::lyrics::model::LyricWord::new("Meri", 12.0, Some(13.0)),
                ],
                10.0,
                Some(13.0),
            ),
        ];
        let lyrics = SyncedLyrics::new(lines, true);
        engine.load_lyrics("Test Artist - Test Title", &lyrics);

        assert_eq!(engine.song_words.len(), 3);
        assert_eq!(engine.song_words[0].text, "Tu");
        assert_eq!(engine.song_words[1].text, "Agar");
        assert_eq!(engine.song_words[2].text, "Meri");

        engine.has_live_lyrics = true;
        engine.live_playback_pos = 11.5; // "Agar" is active

        let rendered = engine.get_rendered_words();
        assert!(!rendered.is_empty());

        // Find the active rendered word
        let active_word = rendered.iter().find(|w| w.is_active);
        assert!(active_word.is_some());
        assert_eq!(active_word.unwrap().text, "Agar");
    }
}
