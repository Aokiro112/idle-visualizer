use crate::lyrics::model::SyncedLyrics;
use crate::visualizer::animation::{damped_spring, exp_decay, lerp};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementLifeState {
    Entering,
    Active,
    Lingering,
    Exiting,
}

#[derive(Debug, Clone)]
pub struct VisualLyricElement {
    pub id: u64,
    pub text: String,
    pub line_index: usize,
    pub word_index: usize,
    pub global_index: usize,
    pub start_time: f64,
    pub end_time: f64,

    // Coordinates in normalized space (center 0, 0)
    pub base_x: f32,
    pub base_y: f32,
    pub base_z: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub current_z: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,

    // Scale and dynamics
    pub base_scale: f32,
    pub current_scale: f32,
    pub target_scale: f32,
    pub scale_velocity: f32,

    // Rotations (radians)
    pub base_rotation: f32,
    pub current_rotation: f32,
    pub target_rotation: f32,

    // Photometric properties
    pub opacity: f32,
    pub target_opacity: f32,
    pub glow: f32,
    pub target_glow: f32,
    pub color_lerp: f32, // 0.0 = secondary/terracotta, 1.0 = highlight/creamy white
    pub target_color_lerp: f32,

    // State tracking
    pub is_active: bool,
    pub is_next_word: bool, // The next upcoming word (receives the highlight box)
    pub progress: f32,
    pub life_state: ElementLifeState,
    pub is_peak_word: bool,
    pub aspect_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct KineticHighlight {
    pub current_x: f32,
    pub current_y: f32,
    pub current_z: f32,
    pub current_w: f32,
    pub current_h: f32,
    pub current_rotation: f32,
    pub opacity: f32,
    pub target_opacity: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub target_w: f32,
    pub target_h: f32,
    pub target_rotation: f32,
    pub active_word: Option<String>,
}

impl Default for KineticHighlight {
    fn default() -> Self {
        Self {
            current_x: 0.0,
            current_y: 0.0,
            current_z: 0.0,
            current_w: 0.22,
            current_h: 0.11,
            current_rotation: -0.048,
            opacity: 0.0,
            target_opacity: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            target_w: 0.22,
            target_h: 0.11,
            target_rotation: -0.048,
            active_word: None,
        }
    }
}

pub struct VisualLyricEngine {
    elements: Vec<VisualLyricElement>,
    highlight: KineticHighlight,
    current_line_idx: Option<usize>,
    last_playback_pos: f64,
    scroll_pos: f32,
    scroll_velocity: f32,
    next_id: u64,
}

impl Default for VisualLyricEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualLyricEngine {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            highlight: KineticHighlight::default(),
            current_line_idx: None,
            last_playback_pos: -1.0,
            scroll_pos: 0.0,
            scroll_velocity: 0.0,
            next_id: 1,
        }
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.current_line_idx = None;
        self.last_playback_pos = -1.0;
        self.scroll_pos = 0.0;
        self.scroll_velocity = 0.0;
        self.highlight.opacity = 0.0;
        self.highlight.target_opacity = 0.0;
    }

    pub fn elements(&self) -> &[VisualLyricElement] {
        &self.elements
    }

    pub fn highlight(&self) -> &KineticHighlight {
        &self.highlight
    }

    pub fn set_element_aspect(&mut self, text: &str, aspect: f32) {
        for elem in &mut self.elements {
            if elem.text == text {
                elem.aspect_ratio = aspect;
            }
        }
    }

    /// Fast-seek / scrub synchronization: instantly rolls reel to target position without spring lag
    pub fn seek_to(&mut self, pos: f64, lyrics: Option<&SyncedLyrics>, vp_w: i32, vp_h: i32) {
        self.elements.clear();
        self.last_playback_pos = pos;
        self.highlight.opacity = 0.0;
        self.highlight.target_opacity = 0.0;
        self.update(lyrics, pos, false, 0.016, vp_w, vp_h);

        for elem in &mut self.elements {
            elem.current_x = elem.target_x;
            elem.current_y = elem.target_y;
            elem.current_z = elem.target_z;
            elem.current_scale = elem.target_scale;
            elem.current_rotation = elem.target_rotation;
            elem.opacity = elem.target_opacity;
            elem.color_lerp = elem.target_color_lerp;
            elem.glow = elem.target_glow;
        }

        self.highlight.current_x = self.highlight.target_x;
        self.highlight.current_y = self.highlight.target_y;
        self.highlight.current_z = self.highlight.target_z;
        self.highlight.current_w = self.highlight.target_w;
        self.highlight.current_h = self.highlight.target_h;
        self.highlight.current_rotation = self.highlight.target_rotation;
        self.highlight.opacity = self.highlight.target_opacity;
    }

    /// Continuous frame tick: animates the vertical rolling reel of typography
    /// The lyric strip moves through a fixed central viewport along the central vertical axis
    pub fn update(
        &mut self,
        lyrics: Option<&SyncedLyrics>,
        pos: f64,
        _is_playing: bool,
        dt: f32,
        _vp_w: i32,
        _vp_h: i32,
    ) {
        let dt = dt.clamp(0.001, 0.05);

        let Some(lyrics) = lyrics else {
            self.elements.clear();
            self.current_line_idx = None;
            self.highlight.target_opacity = 0.0;
            self.highlight.opacity = exp_decay(self.highlight.opacity, 0.0, 14.0, dt);
            return;
        };

        if lyrics.lines.is_empty() {
            self.elements.clear();
            self.current_line_idx = None;
            self.highlight.target_opacity = 0.0;
            self.highlight.opacity = exp_decay(self.highlight.opacity, 0.0, 14.0, dt);
            return;
        }

        // 1. Flatten all words across lines into a sequential timed reel
        struct ReelWord<'a> {
            global_index: usize,
            line_index: usize,
            word_index: usize,
            text: &'a str,
            start: f64,
            end: f64,
        }

        let mut reel_words = Vec::new();
        let mut g_idx = 0;
        for (l_idx, line) in lyrics.lines.iter().enumerate() {
            for (w_idx, word) in line.words.iter().enumerate() {
                reel_words.push(ReelWord {
                    global_index: g_idx,
                    line_index: l_idx,
                    word_index: w_idx,
                    text: &word.text,
                    start: word.start,
                    end: word.end.unwrap_or(word.start + 0.6),
                });
                g_idx += 1;
            }
        }

        if reel_words.is_empty() {
            return;
        }

        let total_reel = reel_words.len();

        // 2. Identify the active focal word on the reel and calculate the target reel scroll position
        let mut active_global_idx = None;
        let mut target_scroll = 0.0f32;

        if pos < reel_words[0].start {
            target_scroll = 0.0;
        } else if pos > reel_words[total_reel - 1].end + 0.2 {
            target_scroll = (total_reel - 1) as f32;
        } else {
            // Find word that covers current timestamp
            for (i, rw) in reel_words.iter().enumerate() {
                if pos >= (rw.start - 0.05) && pos <= (rw.end + 0.12) {
                    active_global_idx = Some(i);
                    let dur = (rw.end - rw.start).max(0.1);
                    let prog = ((pos - rw.start) / dur).clamp(0.0, 1.0) as f32;
                    // Gentle upward pull as word finishes singing
                    target_scroll = (i as f32) + prog * 0.18;
                    break;
                }
            }

            // In between words or before next word starts
            if active_global_idx.is_none() {
                for (i, rw) in reel_words.iter().enumerate() {
                    if rw.start > pos {
                        let prev_idx = i.saturating_sub(1);
                        let prev_end = reel_words[prev_idx].end;
                        let gap = (rw.start - prev_end).max(0.05);
                        let gap_prog = ((pos - prev_end) / gap).clamp(0.0, 1.0) as f32;
                        target_scroll = (prev_idx as f32) + gap_prog;
                        break;
                    }
                }
            }
        }

        // Detect seek jump
        let seek_delta = (pos - self.last_playback_pos).abs();
        if self.last_playback_pos >= 0.0 && seek_delta > 0.65 {
            debug!("Visual lyric engine detected seek: jumping to {:.2}s", pos);
            self.scroll_pos = target_scroll;
            self.scroll_velocity = 0.0;
            self.elements.clear();
        }
        self.last_playback_pos = pos;

        // 3. Smooth physical spring rolling animation for the vertical reel
        self.scroll_pos = damped_spring(
            self.scroll_pos,
            target_scroll,
            &mut self.scroll_velocity,
            135.0,
            16.0,
            dt,
        );

        // Update current line index
        if let Some(act_idx) = active_global_idx {
            self.current_line_idx = Some(reel_words[act_idx].line_index);
        }

        // 4. Determine visible window of words rolling through the fixed central viewport
        // Show words within ~2 slots around focal center (prev, active, next + one more each side)
        let mut visible_indices = Vec::new();
        for (i, _) in reel_words.iter().enumerate() {
            let delta = (i as f32) - self.scroll_pos;
            if (-2.2..=2.2).contains(&delta) {
                visible_indices.push(i);
            }
        }

        // Find the index of the next upcoming word (the word that comes after the active one)
        let next_global_idx = active_global_idx.map(|a| a + 1).filter(|&n| n < total_reel);

        // 5. Update and spawn elements for the rolling typography strip
        for &i in &visible_indices {
            let rw = &reel_words[i];
            let delta = (i as f32) - self.scroll_pos;
            let abs_delta = delta.abs();
            let is_active_word = active_global_idx == Some(i);
            // The next word is the one immediately after the current active word
            let is_next_word = next_global_idx == Some(i);

            // Vertical position: past words roll UPWARD (+Y), upcoming from BELOW (-Y)
            // Spacing calibrated to Verci reference demo (top line ~ +0.34, active center 0.0, next ~ -0.34)
            let target_y = -delta * 0.34;
            let target_x = 0.0f32; // Centered strictly on central vertical axis

            // Scale envelope matching the Verci demo:
            //   Active (delta≈0): monumental peak scale (~2.2x to 2.6x)
            //   ±1 slot away: ~0.95x to 1.1x supporting words
            //   ±2 slots: ~0.60x to 0.70x, fading out
            let peak_scale = if rw.text.chars().count() <= 3 {
                2.65
            } else if rw.text.chars().count() <= 5 {
                2.45
            } else {
                2.25
            };

            let target_scale = if abs_delta <= 0.40 {
                // Focal active word — monumental
                peak_scale
            } else if abs_delta <= 1.0 {
                // Immediate neighbors (prev above, next below)
                let t = (abs_delta - 0.40) / 0.60;
                lerp(peak_scale, 0.98, t)
            } else {
                // Peripheral words fading out
                let t = ((abs_delta - 1.0) / 1.2).min(1.0);
                lerp(0.98, 0.60, t)
            };

            // Opacity: full 1.0 at center, ~0.68 for ±1 slot, fading smoothly to 0 at ±2.2 slots
            let target_opacity = if abs_delta <= 0.40 {
                1.0
            } else if abs_delta <= 1.1 {
                let t = (abs_delta - 0.40) / 0.70;
                lerp(1.0, 0.68, t)
            } else if abs_delta <= 2.2 {
                let t = (abs_delta - 1.1) / 1.1;
                lerp(0.68, 0.0, t)
            } else {
                0.0
            };

            // Active focal word receives signature Verci editorial tilt (-2.75 deg / -0.048 rad)
            let target_rotation = if abs_delta <= 0.45 { -0.048 } else { 0.0 };
            let target_color_lerp = if is_active_word { 1.0 } else { 0.0 };
            let target_glow = if is_active_word { 1.25 } else { 0.0 };
            let target_z = if abs_delta <= 0.40 { 0.02 } else { -0.01 };

            let life_state = if is_active_word {
                ElementLifeState::Active
            } else if delta < -0.3 {
                ElementLifeState::Lingering
            } else {
                ElementLifeState::Entering
            };

            let is_peak = target_scale > 2.0;

            let existing = self.elements.iter_mut().find(|e| {
                e.line_index == rw.line_index && e.word_index == rw.word_index && e.text == rw.text
            });

            if let Some(elem) = existing {
                elem.target_x = target_x;
                elem.target_y = target_y;
                elem.target_z = target_z;
                elem.target_scale = target_scale;
                elem.target_rotation = target_rotation;
                elem.target_opacity = target_opacity;
                elem.target_glow = target_glow;
                elem.target_color_lerp = target_color_lerp;
                elem.is_active = is_active_word;
                elem.is_next_word = is_next_word;
                elem.is_peak_word = is_peak;
                elem.life_state = life_state;
            } else {
                let id = self.next_id;
                self.next_id += 1;

                self.elements.push(VisualLyricElement {
                    id,
                    text: rw.text.to_string(),
                    line_index: rw.line_index,
                    word_index: rw.word_index,
                    global_index: rw.global_index,
                    start_time: rw.start,
                    end_time: rw.end,
                    base_x: target_x,
                    base_y: target_y,
                    base_z: target_z,
                    current_x: target_x,
                    current_y: target_y,
                    current_z: target_z,
                    target_x,
                    target_y,
                    target_z,
                    base_scale: target_scale,
                    current_scale: target_scale * 0.80,
                    target_scale,
                    scale_velocity: 0.0,
                    base_rotation: target_rotation,
                    current_rotation: target_rotation,
                    target_rotation,
                    opacity: 0.0,
                    target_opacity,
                    glow: 0.0,
                    target_glow,
                    color_lerp: target_color_lerp,
                    target_color_lerp,
                    is_active: is_active_word,
                    is_next_word,
                    progress: 0.0,
                    life_state,
                    is_peak_word: is_peak,
                    aspect_ratio: 1.25,
                });
            }
        }

        // Mark elements rolling out of viewport as Exiting
        for elem in &mut self.elements {
            if !visible_indices.contains(&elem.global_index) {
                elem.life_state = ElementLifeState::Exiting;
                elem.target_opacity = 0.0;
                elem.target_scale = elem.base_scale * 0.80;
                elem.is_active = false;
                elem.is_next_word = false;
            }
        }

        // 6. Smooth physical spring and exponential decay interpolation (pure vertical roll)
        for elem in &mut self.elements {
            elem.opacity = exp_decay(elem.opacity, elem.target_opacity, 14.0, dt);
            elem.color_lerp = exp_decay(elem.color_lerp, elem.target_color_lerp, 14.0, dt);
            elem.glow = exp_decay(elem.glow, elem.target_glow, 14.0, dt);
            elem.current_scale = damped_spring(
                elem.current_scale,
                elem.target_scale,
                &mut elem.scale_velocity,
                150.0,
                16.0,
                dt,
            );
            elem.current_x = exp_decay(elem.current_x, elem.target_x, 16.0, dt);
            elem.current_y = exp_decay(elem.current_y, elem.target_y, 16.0, dt);
            elem.current_z = exp_decay(elem.current_z, elem.target_z, 16.0, dt);
            elem.current_rotation =
                exp_decay(elem.current_rotation, elem.target_rotation, 16.0, dt);
        }

        // Retain active or lingering elements
        self.elements
            .retain(|elem| elem.life_state != ElementLifeState::Exiting || elem.opacity > 0.015);

        // 7. Update flat rectangular kinetic highlight layer (attached directly behind active word)
        let active_elem = self.elements.iter().find(|e| e.is_active);
        if let Some(elem) = active_elem {
            self.highlight.active_word = Some(elem.text.clone());
            self.highlight.target_x = elem.current_x;
            self.highlight.target_y = elem.current_y;
            self.highlight.target_z = elem.current_z - 0.005;
            self.highlight.target_rotation = elem.current_rotation;
            self.highlight.target_opacity = 0.90 * elem.target_opacity;

            let quad_h = elem.current_scale * 0.17;
            let quad_w = quad_h * elem.aspect_ratio;
            self.highlight.target_w = quad_w + 0.048 * elem.current_scale;
            self.highlight.target_h = quad_h + 0.024 * elem.current_scale;
        } else {
            self.highlight.target_opacity = 0.0;
        }

        self.highlight.current_x =
            exp_decay(self.highlight.current_x, self.highlight.target_x, 16.0, dt);
        self.highlight.current_y =
            exp_decay(self.highlight.current_y, self.highlight.target_y, 16.0, dt);
        self.highlight.current_z =
            exp_decay(self.highlight.current_z, self.highlight.target_z, 16.0, dt);
        self.highlight.current_w =
            exp_decay(self.highlight.current_w, self.highlight.target_w, 16.0, dt);
        self.highlight.current_h =
            exp_decay(self.highlight.current_h, self.highlight.target_h, 16.0, dt);
        self.highlight.current_rotation = exp_decay(
            self.highlight.current_rotation,
            self.highlight.target_rotation,
            16.0,
            dt,
        );
        self.highlight.opacity =
            exp_decay(self.highlight.opacity, self.highlight.target_opacity, 16.0, dt);
    }
}
