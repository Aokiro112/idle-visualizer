use crate::lyrics::model::LyricWord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutArchetype {
    /// Monumental vertical stack (centered column)
    MonumentalStack,
    /// Asymmetric balanced wings
    AsymmetricSplit,
    /// Diagonal cascade
    DiagonalCascade,
    /// Overlapping kinetic cluster
    OverlappingCluster,
    /// Staggered tiers
    StaggeredTiers,
    /// Centered rhythmic cluster for short punchy lines
    CenteredRhythm,
    /// Floating editorial
    FloatingEditorial,
}

#[derive(Debug, Clone)]
pub struct WordLayoutNode {
    pub word_index: usize,
    pub text: String,
    pub start_time: f64,
    pub end_time: f64,
    pub base_x: f32,
    pub base_y: f32,
    pub base_z: f32,
    pub base_scale: f32,
    pub base_rotation: f32,
    pub is_peak_word: bool,
}

#[derive(Debug, Clone)]
pub struct PhraseLayout {
    pub line_index: usize,
    pub archetype: LayoutArchetype,
    pub nodes: Vec<WordLayoutNode>,
}

pub struct DynamicLayoutEngine;

impl Default for DynamicLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicLayoutEngine {
    pub fn new() -> Self {
        Self
    }

    /// Compute a fast, deterministic hash for stable layout pseudo-randomness
    fn compute_hash(line_index: usize, word_count: usize, first_word: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        h = (h ^ (line_index as u64)).wrapping_mul(0x100000001b3);
        h = (h ^ (word_count as u64)).wrapping_mul(0x100000001b3);
        for b in first_word.bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Evaluates if a word is an emphatic/peak word based on position or lyrical weight
    pub fn is_peak_word(word: &str, word_idx: usize, total_words: usize) -> bool {
        if total_words <= 2 {
            return word_idx == total_words.saturating_sub(1) || total_words == 1;
        }
        if total_words == 3 {
            return word_idx == 1;
        }
        if total_words == 4 {
            return word_idx == 1 || word_idx == 2;
        }
        let len = word.chars().count();
        word_idx == total_words / 2 || len >= 6
    }

    /// Generate a centered, layered, stacked kinetic typography layout
    /// Purely centered along X=0 with intentional compact vertical overlap
    pub fn generate_layout(
        &self,
        line_index: usize,
        words: &[LyricWord],
        _viewport_width: i32,
        _viewport_height: i32,
    ) -> PhraseLayout {
        if words.is_empty() {
            return PhraseLayout {
                line_index,
                archetype: LayoutArchetype::CenteredRhythm,
                nodes: Vec::new(),
            };
        }

        let first_word = words.first().map(|w| w.text.as_str()).unwrap_or("");
        let hash = Self::compute_hash(line_index, words.len(), first_word);
        let total_words = words.len();

        if total_words == 1 {
            let w = &words[0];
            return PhraseLayout {
                line_index,
                archetype: LayoutArchetype::CenteredRhythm,
                nodes: vec![WordLayoutNode {
                    word_index: 0,
                    text: w.text.clone(),
                    start_time: w.start,
                    end_time: w.end.unwrap_or(w.start + 0.8),
                    base_x: 0.0,
                    base_y: 0.0,
                    base_z: 0.02,
                    base_scale: 2.45,
                    base_rotation: -0.045,
                    is_peak_word: true,
                }],
            };
        }

        let peak_idx = match total_words {
            2 => 1,
            3 => 1,
            4 => 1,
            _ => total_words / 2,
        };

        // Determine scales for each word in the stack
        let mut scales = Vec::with_capacity(total_words);
        for i in 0..total_words {
            let is_peak = i == peak_idx;
            let s = if is_peak {
                2.45
            } else if i < peak_idx {
                if i + 1 == peak_idx {
                    1.12
                } else {
                    0.92
                }
            } else if i == peak_idx + 1 {
                1.18
            } else {
                0.95
            };
            scales.push(s);
        }

        // Calculate compact vertical stack with intentional typographic overlap (0.74 factor)
        let overlap = 0.74f32;
        let mut y_raw = vec![0.0f32; total_words];
        for i in 1..total_words {
            let h_prev = scales[i - 1] * 0.16;
            let h_curr = scales[i] * 0.16;
            y_raw[i] = y_raw[i - 1] - (h_prev + h_curr) * 0.5 * overlap;
        }

        let mid_y = (y_raw[0] + y_raw[total_words - 1]) * 0.5;

        let mut nodes = Vec::with_capacity(total_words);
        for (i, w) in words.iter().enumerate() {
            let is_peak = i == peak_idx;
            let word_hash = (hash.wrapping_add((i * 47) as u64) % 100) as f32 / 100.0;
            // Subtle micro-offset for optical balance, strictly near central axis
            let micro_x = (word_hash - 0.5) * 0.025;
            let base_y = y_raw[i] - mid_y;
            let rot = if is_peak { -0.045 } else { 0.0 };
            let base_z = if is_peak { 0.02 } else { -0.01 };

            nodes.push(WordLayoutNode {
                word_index: i,
                text: w.text.clone(),
                start_time: w.start,
                end_time: w.end.unwrap_or(w.start + 0.5),
                base_x: micro_x,
                base_y,
                base_z,
                base_scale: scales[i],
                base_rotation: rot,
                is_peak_word: is_peak,
            });
        }

        PhraseLayout {
            line_index,
            archetype: LayoutArchetype::MonumentalStack,
            nodes,
        }
    }
}
