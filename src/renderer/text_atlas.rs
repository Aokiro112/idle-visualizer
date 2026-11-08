use crate::app::settings::ColorRgba;
use crate::lyrics::model::LyricLine;
use cairo::{Context, Format, ImageSurface, LineCap, LineJoin};
use pangocairo::pango::{
    self, AttrColor, AttrFloat, AttrInt, AttrList, FontDescription, Style, Weight,
};
use std::collections::HashMap;
use tracing::error;

pub struct GpuTexture {
    pub id: u32,
    pub width: i32,
    pub height: i32,
}

impl Drop for GpuTexture {
    fn drop(&mut self) {
        if self.id != 0 {
            unsafe {
                gl::DeleteTextures(1, &self.id);
            }
            self.id = 0;
        }
    }
}

impl GpuTexture {
    pub fn new() -> Self {
        Self {
            id: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn upload(&mut self, surface: &mut ImageSurface, scratch: &mut Vec<u8>) {
        surface.flush();
        let width = surface.width();
        let height = surface.height();
        let data = match surface.data() {
            Ok(d) => d,
            Err(e) => {
                error!("surface.data() failed: {:?}", e);
                return;
            }
        };

        let total_bytes = (width * height * 4) as usize;
        if scratch.len() != total_bytes {
            scratch.resize(total_bytes, 0);
        }

        // Swizzle Cairo ARgb32 (BGRA) to standard OpenGL RGBA and un-premultiply alpha
        let (src_chunks, _) = data.as_chunks::<4>();
        let (dst_chunks, _) = scratch.as_chunks_mut::<4>();
        for (src, dst) in src_chunks.iter().zip(dst_chunks.iter_mut()) {
            let b = src[0];
            let g = src[1];
            let r = src[2];
            let a = src[3];
            if a == 0 {
                dst[0] = 0;
                dst[1] = 0;
                dst[2] = 0;
                dst[3] = 0;
            } else if a == 255 {
                dst[0] = r;
                dst[1] = g;
                dst[2] = b;
                dst[3] = 255;
            } else {
                let alpha_f = a as f32 / 255.0;
                dst[0] = ((r as f32 / alpha_f).min(255.0)) as u8;
                dst[1] = ((g as f32 / alpha_f).min(255.0)) as u8;
                dst[2] = ((b as f32 / alpha_f).min(255.0)) as u8;
                dst[3] = a;
            }
        }

        unsafe {
            if self.id == 0 {
                gl::GenTextures(1, &mut self.id);
                gl::BindTexture(gl::TEXTURE_2D, self.id);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            } else {
                gl::BindTexture(gl::TEXTURE_2D, self.id);
            }
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);

            if self.width != width || self.height != height {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width,
                    height,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    scratch.as_ptr() as *const _,
                );
                self.width = width;
                self.height = height;
            } else {
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width,
                    height,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    scratch.as_ptr() as *const _,
                );
            }
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Default for GpuTexture {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ActiveLineRenderParams<'a> {
    pub line: &'a LyricLine,
    pub active_word_idx: Option<usize>,
    pub word_progress: f32,
    pub font_family: &'a str,
    pub font_size: f32,
    pub lyric_color: &'a ColorRgba,
    pub highlight_color: &'a ColorRgba,
    pub viewport_width: i32,
}

pub struct TrackCardRenderParams<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub status_label: &'a str,
    pub time_str: Option<&'a str>,
    pub font_family: &'a str,
    pub title_color: &'a ColorRgba,
    pub artist_color: &'a ColorRgba,
    pub status_color: &'a ColorRgba,
    pub viewport_width: i32,
}

pub struct CachedWordTexture {
    pub texture: GpuTexture,
    pub aspect_ratio: f32,
    pub pixel_width: i32,
    pub pixel_height: i32,
}

pub struct TextAtlas {
    pub white_texture: GpuTexture,
    pub active_line_texture: GpuTexture,
    pub line_cache: HashMap<usize, GpuTexture>,
    pub word_cache: HashMap<String, CachedWordTexture>,
    pub track_card_texture: GpuTexture,
    pub symbol_texture: GpuTexture,
    pub last_symbol_rendered: String,
    pub last_track_key: String,
    scratch_buffer: Vec<u8>,
}

impl TextAtlas {
    pub fn new() -> Self {
        Self {
            white_texture: GpuTexture::new(),
            active_line_texture: GpuTexture::new(),
            line_cache: HashMap::new(),
            word_cache: HashMap::new(),
            track_card_texture: GpuTexture::new(),
            symbol_texture: GpuTexture::new(),
            last_symbol_rendered: String::new(),
            last_track_key: String::new(),
            scratch_buffer: Vec::new(),
        }
    }

    pub fn ensure_gl_initialized(&mut self) {
        if self.white_texture.id == 0 {
            if let Ok(mut surf) = ImageSurface::create(Format::ARgb32, 2, 2) {
                if let Ok(cr) = Context::new(&surf) {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                    let _ = cr.paint();
                    drop(cr);
                    self.white_texture.upload(&mut surf, &mut self.scratch_buffer);
                }
            }
        }
    }

    pub fn clear_cache(&mut self) {
        self.line_cache.clear();
        self.word_cache.clear();
        self.last_track_key.clear();
    }

    /// Retrieve or rasterize an individual word into a cached GPU texture
    pub fn get_or_render_word(
        &mut self,
        text: &str,
        is_bold: bool,
        font_family: &str,
        font_size: f32,
    ) -> Option<&CachedWordTexture> {
        self.get_or_render_styled_word(text, is_bold, false, font_family, font_size)
    }

    /// Retrieve or rasterize a styled word (with weight and italic style support)
    pub fn get_or_render_styled_word(
        &mut self,
        text: &str,
        is_bold: bool,
        is_italic: bool,
        font_family: &str,
        font_size: f32,
    ) -> Option<&CachedWordTexture> {
        let key = format!(
            "sw:{}:{}:{}:{}",
            text,
            if is_bold { "b" } else { "r" },
            if is_italic { "i" } else { "n" },
            font_family
        );
        if !self.word_cache.contains_key(&key) {
            let (tex, aspect, pw, ph) = Self::rasterize_styled_word(
                text,
                is_bold,
                is_italic,
                font_family,
                font_size,
                &mut self.scratch_buffer,
            )?;
            self.word_cache.insert(
                key.clone(),
                CachedWordTexture {
                    texture: tex,
                    aspect_ratio: aspect,
                    pixel_width: pw,
                    pixel_height: ph,
                },
            );
        }
        self.word_cache.get(&key)
    }

    /// Retrieve or rasterize a perfectly proportioned antialiased rounded rectangle highlight badge
    pub fn get_or_render_rounded_box(&mut self, aspect_ratio: f32) -> Option<&CachedWordTexture> {
        let aspect_bucket = ((aspect_ratio * 20.0).round() / 20.0).max(0.5);
        let key = format!("rbox:{:.2}", aspect_bucket);
        if !self.word_cache.contains_key(&key) {
            let height_px = 128;
            let width_px = (height_px as f32 * aspect_bucket).round().max(64.0) as i32;
            let radius = 18.0f64;

            let mut surface = ImageSurface::create(Format::ARgb32, width_px, height_px).ok()?;
            let cr = Context::new(&surface).ok()?;
            cr.set_operator(cairo::Operator::Clear);
            let _ = cr.paint();
            cr.set_operator(cairo::Operator::Over);

            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            Self::draw_rounded_rect(&cr, 0.0, 0.0, width_px as f64, height_px as f64, radius);
            let _ = cr.fill();
            drop(cr);

            let mut tex = GpuTexture::new();
            tex.upload(&mut surface, &mut self.scratch_buffer);

            self.word_cache.insert(
                key.clone(),
                CachedWordTexture {
                    texture: tex,
                    aspect_ratio: aspect_bucket,
                    pixel_width: width_px,
                    pixel_height: height_px,
                },
            );
        }
        self.word_cache.get(&key)
    }

    fn rasterize_styled_word(
        text: &str,
        is_bold: bool,
        is_italic: bool,
        font_family: &str,
        font_size: f32,
        scratch: &mut Vec<u8>,
    ) -> Option<(GpuTexture, f32, i32, i32)> {
        if text.is_empty() {
            return None;
        }

        let render_pt = (font_size * 1.5).clamp(42.0, 72.0);

        let dummy_surface = ImageSurface::create(Format::ARgb32, 10, 10).ok()?;
        let dummy_cr = Context::new(&dummy_surface).ok()?;
        let layout = pangocairo::functions::create_layout(&dummy_cr);

        let mut font_desc = FontDescription::from_string(font_family);
        font_desc.set_weight(if is_bold {
            Weight::Heavy
        } else {
            Weight::Bold
        });
        font_desc.set_style(if is_italic {
            Style::Italic
        } else {
            Style::Normal
        });
        font_desc.set_size((render_pt * pango::SCALE as f32) as i32);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(text);

        let (tw, th) = layout.pixel_size();
        if tw <= 0 || th <= 0 {
            return None;
        }

        let pad_x = 16;
        let pad_y = 12;
        let surface_w = tw + pad_x * 2;
        let surface_h = th + pad_y * 2;

        let mut surface = ImageSurface::create(Format::ARgb32, surface_w, surface_h).ok()?;
        let cr = Context::new(&surface).ok()?;

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        cr.move_to(pad_x as f64, pad_y as f64);
        pangocairo::functions::show_layout(&cr, &layout);
        drop(cr);

        let mut tex = GpuTexture::new();
        tex.upload(&mut surface, scratch);

        let aspect = (surface_w as f32) / (surface_h.max(1) as f32);
        Some((tex, aspect, surface_w, surface_h))
    }

    /// Retrieve or rasterize an active singing word badge with optional contextual symbol
    pub fn get_or_render_active_badge(
        &mut self,
        text: &str,
        symbol: Option<&str>,
        font_family: &str,
        font_size: f32,
        text_color: &ColorRgba,
        badge_color: &ColorRgba,
    ) -> Option<&CachedWordTexture> {
        let key = format!("badge:{}:{}:{}", text, symbol.unwrap_or(""), font_family);
        if !self.word_cache.contains_key(&key) {
            let (tex, aspect, pw, ph) = Self::rasterize_active_badge(
                text,
                symbol,
                font_family,
                font_size,
                text_color,
                badge_color,
                &mut self.scratch_buffer,
            )?;
            self.word_cache.insert(
                key.clone(),
                CachedWordTexture {
                    texture: tex,
                    aspect_ratio: aspect,
                    pixel_width: pw,
                    pixel_height: ph,
                },
            );
        }
        self.word_cache.get(&key)
    }

    fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
        use std::f64::consts::PI;
        cr.new_sub_path();
        cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
        cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
        cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
        cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
        cr.close_path();
    }

    fn draw_vector_car(cr: &Context, x: f64, y: f64, size: f64, color: &ColorRgba) {
        let _ = cr.save();
        cr.translate(x, y);
        let s = size / 48.0;
        cr.scale(s, s);
        cr.set_source_rgba(
            color.r as f64,
            color.g as f64,
            color.b as f64,
            color.a as f64,
        );
        cr.set_line_width(3.6);
        cr.set_line_cap(LineCap::Round);
        cr.set_line_join(LineJoin::Round);

        // Body / roof outer path
        cr.move_to(12.0, 14.0);
        cr.curve_to(16.0, 6.0, 32.0, 6.0, 36.0, 14.0);
        cr.line_to(44.0, 22.0);
        cr.curve_to(46.0, 24.0, 46.0, 28.0, 44.0, 30.0);
        cr.line_to(44.0, 40.0);
        cr.curve_to(44.0, 42.0, 42.0, 44.0, 40.0, 44.0);
        cr.line_to(38.0, 44.0);
        cr.line_to(38.0, 40.0);
        cr.line_to(10.0, 40.0);
        cr.line_to(10.0, 44.0);
        cr.line_to(8.0, 44.0);
        cr.curve_to(6.0, 44.0, 4.0, 42.0, 4.0, 40.0);
        cr.line_to(4.0, 30.0);
        cr.curve_to(2.0, 28.0, 2.0, 24.0, 4.0, 22.0);
        cr.close_path();
        let _ = cr.stroke();

        // Windshield inner outline
        cr.move_to(13.0, 16.0);
        cr.curve_to(17.0, 10.0, 31.0, 10.0, 35.0, 16.0);
        cr.line_to(40.0, 24.0);
        cr.line_to(8.0, 24.0);
        cr.close_path();
        let _ = cr.stroke();

        // Headlights
        cr.arc(10.0, 31.0, 2.5, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();
        cr.arc(38.0, 31.0, 2.5, 0.0, 2.0 * std::f64::consts::PI);
        let _ = cr.fill();

        let _ = cr.restore();
    }

    fn rasterize_active_badge(
        text: &str,
        symbol: Option<&str>,
        font_family: &str,
        font_size: f32,
        text_color: &ColorRgba,
        badge_color: &ColorRgba,
        scratch: &mut Vec<u8>,
    ) -> Option<(GpuTexture, f32, i32, i32)> {
        if text.is_empty() {
            return None;
        }

        let render_pt = (font_size * 1.5).clamp(42.0, 72.0);

        let dummy_surface = ImageSurface::create(Format::ARgb32, 10, 10).ok()?;
        let dummy_cr = Context::new(&dummy_surface).ok()?;
        let layout = pangocairo::functions::create_layout(&dummy_cr);

        let mut font_desc = FontDescription::from_string(font_family);
        font_desc.set_weight(Weight::Bold);
        font_desc.set_style(Style::Italic);
        font_desc.set_size((render_pt * pango::SCALE as f32) as i32);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(text);

        let (tw, th) = layout.pixel_size();
        if tw <= 0 || th <= 0 {
            return None;
        }

        let pad_x = 24;
        let pad_y = 12;
        let badge_w = tw + pad_x * 2;
        let badge_h = th + pad_y * 2;
        let radius = 12.0f64;

        let has_symbol = symbol.is_some();
        let symbol_size = (badge_h as f64 * 0.85).clamp(36.0, 80.0);
        let symbol_gap = 18;
        let symbol_total_w = if has_symbol {
            (symbol_size as i32) + symbol_gap
        } else {
            0
        };

        let margin = 12;
        let surface_w = badge_w + symbol_total_w + margin * 2;
        let surface_h = badge_h + margin * 2;

        let mut surface = ImageSurface::create(Format::ARgb32, surface_w, surface_h).ok()?;
        let cr = Context::new(&surface).ok()?;

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        let symbol_x = margin as f64;
        let symbol_y = (surface_h as f64 - symbol_size) / 2.0;

        // Render symbol if present
        if let Some(sym) = symbol {
            if sym == "🚗" || sym == "car" {
                Self::draw_vector_car(&cr, symbol_x, symbol_y, symbol_size, badge_color);
            } else {
                let sym_layout = pangocairo::functions::create_layout(&cr);
                let mut sym_desc = FontDescription::from_string("sans-serif");
                sym_desc.set_size(((symbol_size * 0.75) * pango::SCALE as f64) as i32);
                sym_layout.set_font_description(Some(&sym_desc));
                sym_layout.set_text(sym);
                let (sw, sh) = sym_layout.pixel_size();
                let sx = symbol_x + (symbol_size - sw as f64) / 2.0;
                let sy = (surface_h as f64 - sh as f64) / 2.0;
                cr.move_to(sx, sy);
                cr.set_source_rgba(
                    badge_color.r as f64,
                    badge_color.g as f64,
                    badge_color.b as f64,
                    badge_color.a as f64,
                );
                pangocairo::functions::show_layout(&cr, &sym_layout);
            }
        }

        // Draw rounded badge rectangle
        let badge_x = (margin + symbol_total_w) as f64;
        let badge_y = margin as f64;
        Self::draw_rounded_rect(
            &cr,
            badge_x,
            badge_y,
            badge_w as f64,
            badge_h as f64,
            radius,
        );
        cr.set_source_rgba(
            badge_color.r as f64,
            badge_color.g as f64,
            badge_color.b as f64,
            badge_color.a as f64,
        );
        let _ = cr.fill();

        // Draw text inside badge
        cr.move_to(badge_x + pad_x as f64, badge_y + pad_y as f64);
        cr.set_source_rgba(
            text_color.r as f64,
            text_color.g as f64,
            text_color.b as f64,
            text_color.a as f64,
        );
        pangocairo::functions::show_layout(&cr, &layout);

        drop(cr);

        let mut tex = GpuTexture::new();
        tex.upload(&mut surface, scratch);

        let aspect = (surface_w as f32) / (surface_h.max(1) as f32);
        Some((tex, aspect, surface_w, surface_h))
    }

    pub fn get_or_render_symbol_texture(
        &mut self,
        symbol: &str,
        color: &ColorRgba,
    ) -> Option<&CachedWordTexture> {
        let key = format!("sym:{}:{}", symbol, color.to_hex());
        if !self.word_cache.contains_key(&key) {
            let (tex, aspect, pw, ph) =
                Self::rasterize_symbol(symbol, color, &mut self.scratch_buffer)?;
            self.word_cache.insert(
                key.clone(),
                CachedWordTexture {
                    texture: tex,
                    aspect_ratio: aspect,
                    pixel_width: pw,
                    pixel_height: ph,
                },
            );
        }
        self.word_cache.get(&key)
    }

    fn rasterize_symbol(
        symbol: &str,
        color: &ColorRgba,
        scratch: &mut Vec<u8>,
    ) -> Option<(GpuTexture, f32, i32, i32)> {
        let size = 120;
        let mut surface = ImageSurface::create(Format::ARgb32, size, size).ok()?;
        let cr = Context::new(&surface).ok()?;

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        if symbol == "🚗" || symbol == "car" {
            Self::draw_vector_car(&cr, 10.0, 10.0, 100.0, color);
        } else {
            let layout = pangocairo::functions::create_layout(&cr);
            let mut font_desc = FontDescription::from_string("sans-serif");
            font_desc.set_size((56.0 * pango::SCALE as f32) as i32);
            layout.set_font_description(Some(&font_desc));
            layout.set_text(symbol);
            let (tw, th) = layout.pixel_size();
            let pos_x = (size as f64 - tw as f64) / 2.0;
            let pos_y = (size as f64 - th as f64) / 2.0;
            cr.move_to(pos_x, pos_y);
            cr.set_source_rgba(
                color.r as f64,
                color.g as f64,
                color.b as f64,
                color.a as f64,
            );
            pangocairo::functions::show_layout(&cr, &layout);
        }

        drop(cr);
        let mut tex = GpuTexture::new();
        tex.upload(&mut surface, scratch);
        let aspect = 1.0;
        Some((tex, aspect, size, size))
    }

    #[allow(dead_code)]
    fn rasterize_word(
        text: &str,
        is_bold: bool,
        font_family: &str,
        font_size: f32,
        scratch: &mut Vec<u8>,
    ) -> Option<(GpuTexture, f32, i32, i32)> {
        if text.is_empty() {
            return None;
        }

        // High resolution rendering for sharp scaling on 4K / retina
        let render_pt = (font_size * 1.5).clamp(42.0, 72.0);

        let dummy_surface = ImageSurface::create(Format::ARgb32, 10, 10).ok()?;
        let dummy_cr = Context::new(&dummy_surface).ok()?;
        let layout = pangocairo::functions::create_layout(&dummy_cr);

        let mut font_desc = FontDescription::from_string(font_family);
        font_desc.set_weight(if is_bold {
            Weight::Heavy
        } else {
            Weight::Normal
        });
        font_desc.set_size((render_pt * pango::SCALE as f32) as i32);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(text);

        let (tw, th) = layout.pixel_size();
        if tw <= 0 || th <= 0 {
            return None;
        }

        let pad_x = 14;
        let pad_y = 12;
        let surface_w = tw + pad_x * 2;
        let surface_h = th + pad_y * 2;

        let mut surface = ImageSurface::create(Format::ARgb32, surface_w, surface_h).ok()?;
        let cr = Context::new(&surface).ok()?;

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        // Render in pure white so GPU shader uniform tinting u_color works cleanly
        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        cr.move_to(pad_x as f64, pad_y as f64);
        pangocairo::functions::show_layout(&cr, &layout);
        drop(cr);

        let mut tex = GpuTexture::new();
        tex.upload(&mut surface, scratch);

        let aspect = (surface_w as f32) / (surface_h.max(1) as f32);
        Some((tex, aspect, surface_w, surface_h))
    }

    /// Render active singing line with per-word animation and return the horizontal normalized
    /// X coordinate [-1.0, 1.0] of the active singing word (for symbol/particle alignment)
    pub fn render_active_line(&mut self, params: ActiveLineRenderParams<'_>) -> f32 {
        let ActiveLineRenderParams {
            line,
            active_word_idx,
            word_progress,
            font_family,
            font_size,
            lyric_color,
            highlight_color,
            viewport_width,
        } = params;

        let width = (viewport_width.max(1280) as f64 * 1.5) as i32;
        let height = 180;

        let mut surface = match ImageSurface::create(Format::ARgb32, width, height) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create cairo surface: {}", e);
                return 0.0;
            }
        };

        let cr = match Context::new(&surface) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create cairo context: {}", e);
                return 0.0;
            }
        };

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        let layout = pangocairo::functions::create_layout(&cr);
        let mut font_desc = FontDescription::from_string(font_family);
        font_desc.set_size((font_size * pango::SCALE as f32) as i32);
        font_desc.set_weight(Weight::Medium);
        layout.set_font_description(Some(&font_desc));

        let mut display_text = String::new();
        let mut word_ranges: Vec<(usize, usize)> = Vec::new();

        if line.words.is_empty() {
            display_text.push_str(&line.raw_text);
        } else {
            for (i, word) in line.words.iter().enumerate() {
                let start_b = display_text.len();
                display_text.push_str(&word.text);
                let end_b = display_text.len();
                word_ranges.push((start_b, end_b));

                if i + 1 < line.words.len() {
                    display_text.push(' ');
                }
            }
        }

        layout.set_text(&display_text);

        let attr_list = AttrList::new();
        let mut active_word_start_byte = None;

        if word_ranges.is_empty() {
            let mut fg = AttrColor::new_foreground(
                (highlight_color.r * 65535.0) as u16,
                (highlight_color.g * 65535.0) as u16,
                (highlight_color.b * 65535.0) as u16,
            );
            fg.set_start_index(0);
            fg.set_end_index(display_text.len() as u32);
            attr_list.insert(fg);
        } else {
            for (i, &(start_b, end_b)) in word_ranges.iter().enumerate() {
                let is_active = Some(i) == active_word_idx;
                let is_past = active_word_idx.map(|act| i < act).unwrap_or(false);

                if is_active {
                    active_word_start_byte = Some(start_b);
                }

                let (r, g, b, scale, weight) = if is_active {
                    let blend = 0.5 + 0.5 * word_progress.clamp(0.0, 1.0);
                    (
                        highlight_color.r * blend + lyric_color.r * (1.0 - blend),
                        highlight_color.g * blend + lyric_color.g * (1.0 - blend),
                        highlight_color.b * blend + lyric_color.b * (1.0 - blend),
                        1.15,
                        Weight::Bold,
                    )
                } else if is_past {
                    (
                        highlight_color.r * 0.90,
                        highlight_color.g * 0.90,
                        highlight_color.b * 0.90,
                        1.0,
                        Weight::Semibold,
                    )
                } else {
                    (
                        lyric_color.r * 0.72,
                        lyric_color.g * 0.72,
                        lyric_color.b * 0.72,
                        1.0,
                        Weight::Normal,
                    )
                };

                let mut fg = AttrColor::new_foreground(
                    (r.clamp(0.0, 1.0) * 65535.0) as u16,
                    (g.clamp(0.0, 1.0) * 65535.0) as u16,
                    (b.clamp(0.0, 1.0) * 65535.0) as u16,
                );
                fg.set_start_index(start_b as u32);
                fg.set_end_index(end_b as u32);
                attr_list.insert(fg);

                let mut w_attr = AttrInt::new_weight(weight);
                w_attr.set_start_index(start_b as u32);
                w_attr.set_end_index(end_b as u32);
                attr_list.insert(w_attr);

                if is_active {
                    let mut s_attr = AttrFloat::new_scale(scale);
                    s_attr.set_start_index(start_b as u32);
                    s_attr.set_end_index(end_b as u32);
                    attr_list.insert(s_attr);
                }
            }
        }

        layout.set_attributes(Some(&attr_list));
        layout.set_alignment(pango::Alignment::Center);

        let (tw, th) = layout.pixel_size();
        let pos_x = (width as f64 - tw as f64) / 2.0;
        let pos_y = (height as f64 - th as f64) / 2.0;

        cr.move_to(pos_x, pos_y);
        pangocairo::functions::show_layout(&cr, &layout);

        // Calculate normalized X position of the active word in the quad [-1, 1]
        let word_x_norm = if let Some(sb) = active_word_start_byte {
            let rect = layout.index_to_pos(sb as i32);
            let word_px = pos_x
                + (rect.x() as f64 / pango::SCALE as f64)
                + (rect.width() as f64 / pango::SCALE as f64 / 2.0);
            ((word_px / width as f64) * 2.0 - 1.0) as f32
        } else {
            0.0
        };

        drop(cr);
        self.active_line_texture
            .upload(&mut surface, &mut self.scratch_buffer);

        word_x_norm
    }

    /// Retrieve or render a context line into the texture cache
    pub fn get_or_render_context_line(
        &mut self,
        line_idx: usize,
        line: &LyricLine,
        font_family: &str,
        font_size: f32,
        secondary_color: &ColorRgba,
        viewport_width: i32,
    ) {
        if self.line_cache.contains_key(&line_idx) {
            return;
        }

        let width = (viewport_width.max(1280) as f64 * 1.5) as i32;
        let height = 150;

        let mut surface = match ImageSurface::create(Format::ARgb32, width, height) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create cairo surface: {}", e);
                return;
            }
        };

        let cr = match Context::new(&surface) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create cairo context: {}", e);
                return;
            }
        };

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        let layout = pangocairo::functions::create_layout(&cr);
        let mut font_desc = FontDescription::from_string(font_family);
        font_desc.set_size(((font_size * 0.85) * pango::SCALE as f32) as i32);
        font_desc.set_weight(Weight::Normal);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(&line.raw_text);
        layout.set_alignment(pango::Alignment::Center);

        let attr_list = AttrList::new();
        let mut fg = AttrColor::new_foreground(
            (secondary_color.r * 65535.0) as u16,
            (secondary_color.g * 65535.0) as u16,
            (secondary_color.b * 65535.0) as u16,
        );
        fg.set_start_index(0);
        fg.set_end_index(line.raw_text.len() as u32);
        attr_list.insert(fg);
        layout.set_attributes(Some(&attr_list));

        let (tw, th) = layout.pixel_size();
        let pos_x = (width as f64 - tw as f64) / 2.0;
        let pos_y = (height as f64 - th as f64) / 2.0;

        cr.move_to(pos_x, pos_y);
        pangocairo::functions::show_layout(&cr, &layout);
        drop(cr);

        let mut tex = GpuTexture::new();
        tex.upload(&mut surface, &mut self.scratch_buffer);
        self.line_cache.insert(line_idx, tex);
    }

    /// Render visual symbol particle (emoji / icon)
    pub fn render_symbol(&mut self, symbol: &str) {
        if self.last_symbol_rendered == symbol && self.symbol_texture.width > 0 {
            return;
        }

        let size = 160;
        let mut surface = match ImageSurface::create(Format::ARgb32, size, size) {
            Ok(s) => s,
            Err(_) => return,
        };

        let cr = match Context::new(&surface) {
            Ok(c) => c,
            Err(_) => return,
        };

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        let layout = pangocairo::functions::create_layout(&cr);
        let mut font_desc = FontDescription::from_string("sans-serif");
        font_desc.set_size((72.0 * pango::SCALE as f32) as i32);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(symbol);
        layout.set_alignment(pango::Alignment::Center);

        let (tw, th) = layout.pixel_size();
        let pos_x = (size as f64 - tw as f64) / 2.0;
        let pos_y = (size as f64 - th as f64) / 2.0;

        cr.move_to(pos_x, pos_y);
        pangocairo::functions::show_layout(&cr, &layout);
        drop(cr);

        self.symbol_texture
            .upload(&mut surface, &mut self.scratch_buffer);
        self.last_symbol_rendered = symbol.to_string();
    }

    /// Render track info card (when lyrics are loading, not found, or idle)
    pub fn render_track_card(&mut self, params: TrackCardRenderParams<'_>) {
        let TrackCardRenderParams {
            title,
            artist,
            status_label,
            time_str,
            font_family,
            title_color,
            artist_color,
            status_color,
            viewport_width,
        } = params;

        let width = (viewport_width.max(1280) as f64 * 1.5) as i32;
        let height = 360;

        let mut surface = match ImageSurface::create(Format::ARgb32, width, height) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create cairo surface: {}", e);
                return;
            }
        };

        let cr = match Context::new(&surface) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create cairo context: {}", e);
                return;
            }
        };

        cr.set_operator(cairo::Operator::Clear);
        let _ = cr.paint();
        cr.set_operator(cairo::Operator::Over);

        let center_x = width as f64 / 2.0;

        // 1. Title
        let title_layout = pangocairo::functions::create_layout(&cr);
        let mut title_desc = FontDescription::from_string(font_family);
        title_desc.set_size((46.0 * pango::SCALE as f32) as i32);
        title_desc.set_weight(Weight::Bold);
        title_layout.set_font_description(Some(&title_desc));
        title_layout.set_text(title);
        title_layout.set_alignment(pango::Alignment::Center);

        let title_attrs = AttrList::new();
        let mut fg_title = AttrColor::new_foreground(
            (title_color.r * 65535.0) as u16,
            (title_color.g * 65535.0) as u16,
            (title_color.b * 65535.0) as u16,
        );
        fg_title.set_start_index(0);
        fg_title.set_end_index(title.len() as u32);
        title_attrs.insert(fg_title);
        title_layout.set_attributes(Some(&title_attrs));

        let (tw, th) = title_layout.pixel_size();
        let title_y = 65.0;
        cr.move_to(center_x - (tw as f64 / 2.0), title_y);
        pangocairo::functions::show_layout(&cr, &title_layout);

        // 2. Artist
        let artist_layout = pangocairo::functions::create_layout(&cr);
        let mut artist_desc = FontDescription::from_string(font_family);
        artist_desc.set_size((28.0 * pango::SCALE as f32) as i32);
        artist_desc.set_weight(Weight::Medium);
        artist_layout.set_font_description(Some(&artist_desc));
        artist_layout.set_text(artist);
        artist_layout.set_alignment(pango::Alignment::Center);

        let artist_attrs = AttrList::new();
        let mut fg_artist = AttrColor::new_foreground(
            (artist_color.r * 65535.0) as u16,
            (artist_color.g * 65535.0) as u16,
            (artist_color.b * 65535.0) as u16,
        );
        fg_artist.set_start_index(0);
        fg_artist.set_end_index(artist.len() as u32);
        artist_attrs.insert(fg_artist);
        artist_layout.set_attributes(Some(&artist_attrs));

        let (aw, ah) = artist_layout.pixel_size();
        let artist_y = title_y + th as f64 + 14.0;
        cr.move_to(center_x - (aw as f64 / 2.0), artist_y);
        pangocairo::functions::show_layout(&cr, &artist_layout);

        // 3. Status label & time pill
        let sub_text = if let Some(t) = time_str {
            format!("{} • {}", status_label, t)
        } else {
            status_label.to_string()
        };

        let sub_layout = pangocairo::functions::create_layout(&cr);
        let mut sub_desc = FontDescription::from_string(font_family);
        sub_desc.set_size((18.0 * pango::SCALE as f32) as i32);
        sub_desc.set_weight(Weight::Normal);
        sub_layout.set_font_description(Some(&sub_desc));
        sub_layout.set_text(&sub_text);
        sub_layout.set_alignment(pango::Alignment::Center);

        let sub_attrs = AttrList::new();
        let mut fg_sub = AttrColor::new_foreground(
            (status_color.r * 65535.0) as u16,
            (status_color.g * 65535.0) as u16,
            (status_color.b * 65535.0) as u16,
        );
        fg_sub.set_start_index(0);
        fg_sub.set_end_index(sub_text.len() as u32);
        sub_attrs.insert(fg_sub);
        sub_layout.set_attributes(Some(&sub_attrs));

        let (sw, _sh) = sub_layout.pixel_size();
        let sub_y = artist_y + ah as f64 + 20.0;
        cr.move_to(center_x - (sw as f64 / 2.0), sub_y);
        pangocairo::functions::show_layout(&cr, &sub_layout);

        drop(cr);
        self.track_card_texture
            .upload(&mut surface, &mut self.scratch_buffer);
    }
}

impl Default for TextAtlas {
    fn default() -> Self {
        Self::new()
    }
}
