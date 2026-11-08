use crate::app::settings::ColorRgba;
use image::DynamicImage;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LyricPalette {
    pub background: ColorRgba,
    pub lyric: ColorRgba,
    pub highlight: ColorRgba,
    pub secondary: ColorRgba,
}

/// Canonical alias for color palette as defined in the architectural contract
pub type ColorPalette = LyricPalette;

impl Default for LyricPalette {
    fn default() -> Self {
        Self {
            background: ColorRgba::new(0.0, 0.0, 0.0, 1.0),
            lyric: ColorRgba::new(0.706, 0.478, 0.400, 1.0),
            highlight: ColorRgba::new(0.941, 0.886, 0.902, 1.0),
            secondary: ColorRgba::new(0.463, 0.337, 0.361, 0.85),
        }
    }
}

impl LyricPalette {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lerp_col = |a: &ColorRgba, b: &ColorRgba| ColorRgba {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        };

        Self {
            background: lerp_col(&self.background, &other.background),
            lyric: lerp_col(&self.lyric, &other.lyric),
            highlight: lerp_col(&self.highlight, &other.highlight),
            secondary: lerp_col(&self.secondary, &other.secondary),
        }
    }

    /// Extract a tasteful, high-contrast palette from an album art image
    pub fn extract_from_image(img: &DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        if width == 0 || height == 0 {
            return Self::default();
        }

        // Downsample sampling step for fast real-time color clustering
        let step_x = (width / 48).max(1);
        let step_y = (height / 48).max(1);

        let mut bucket_counts: HashMap<(u8, u8, u8), usize> = HashMap::new();

        for y in (0..height).step_by(step_y as usize) {
            for x in (0..width).step_by(step_x as usize) {
                let pixel = rgba.get_pixel(x, y);
                if pixel[3] < 128 {
                    continue; // Skip transparent pixels
                }
                // Quantize into 16 levels per channel to cluster nearby tones
                let r = (pixel[0] / 16) * 16;
                let g = (pixel[1] / 16) * 16;
                let b = (pixel[2] / 16) * 16;
                *bucket_counts.entry((r, g, b)).or_insert(0) += 1;
            }
        }

        if bucket_counts.is_empty() {
            return Self::default();
        }

        let mut sorted_buckets: Vec<((u8, u8, u8), usize)> = bucket_counts.into_iter().collect();
        sorted_buckets.sort_by_key(|a| std::cmp::Reverse(a.1));

        // Evaluate dominant colors with saturation and luminance
        struct ColorCandidate {
            r: f32,
            g: f32,
            b: f32,
            luminance: f32,
            saturation: f32,
        }

        let candidates: Vec<ColorCandidate> = sorted_buckets
            .iter()
            .take(15)
            .map(|&((r, g, b), _)| {
                let rf = r as f32 / 255.0;
                let gf = g as f32 / 255.0;
                let bf = b as f32 / 255.0;
                let max = rf.max(gf).max(bf);
                let min = rf.min(gf).min(bf);
                let lum = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
                let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
                ColorCandidate {
                    r: rf,
                    g: gf,
                    b: bf,
                    luminance: lum,
                    saturation: sat,
                }
            })
            .collect();

        // 1. Pick background: Pure minimal velvety black canvas (#000000)
        let background = ColorRgba::new(0.0, 0.0, 0.0, 1.0);
        let primary = &candidates[0];

        // 2. Pick highlight color: Vibrant, high-saturation or high-luminance accent
        let best_highlight = candidates
            .iter()
            .filter(|c| c.saturation > 0.25 || c.luminance > 0.4)
            .max_by(|a, b| {
                let score_a = a.saturation * 1.5 + a.luminance;
                let score_b = b.saturation * 1.5 + b.luminance;
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(primary);

        // Ensure highlight has sufficient brightness for active word contrast
        let mut hl_r = best_highlight.r;
        let mut hl_g = best_highlight.g;
        let mut hl_b = best_highlight.b;
        let mut hl_lum = 0.2126 * hl_r + 0.7152 * hl_g + 0.0722 * hl_b;
        if hl_lum < 0.55 {
            let boost = 0.60 / hl_lum.max(0.05);
            hl_r = (hl_r * boost).clamp(0.0, 1.0);
            hl_g = (hl_g * boost).clamp(0.0, 1.0);
            hl_b = (hl_b * boost).clamp(0.0, 1.0);
            hl_lum = 0.2126 * hl_r + 0.7152 * hl_g + 0.0722 * hl_b;

            if hl_lum < 0.55 {
                let add = (0.55 - hl_lum) / 0.93;
                hl_r = (hl_r + add).clamp(0.0, 1.0);
                hl_g = (hl_g + add).clamp(0.0, 1.0);
                hl_b = (hl_b + add).clamp(0.0, 1.0);
            }
        }
        let highlight = ColorRgba::new(hl_r, hl_g, hl_b, 1.0);

        // 3. Normal lyric color: Soft light neutral tinted slightly by the highlight
        let lyric = ColorRgba::new(
            0.75 + hl_r * 0.15,
            0.75 + hl_g * 0.15,
            0.75 + hl_b * 0.15,
            0.70,
        );

        // 4. Secondary color: Subdued for past/future words
        let secondary = ColorRgba::new(
            0.40 + hl_r * 0.10,
            0.40 + hl_g * 0.10,
            0.40 + hl_b * 0.10,
            0.38,
        );

        Self {
            background,
            lyric,
            highlight,
            secondary,
        }
    }
}
