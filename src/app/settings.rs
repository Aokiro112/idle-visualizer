use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VisualMode {
    #[default]
    Dynamic,
    Ship,
    Fisheye,
    FisheyeVisual,
    Visual,
}

/// Canonical alias for visualizer rendering mode as defined in the architectural contract
pub type VisualizerMode = VisualMode;

impl std::fmt::Display for VisualMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualMode::Dynamic => write!(f, "Dynamic Verci"),
            VisualMode::Ship => write!(f, "Ship"),
            VisualMode::Fisheye => write!(f, "Fisheye"),
            VisualMode::FisheyeVisual => write!(f, "Fisheye Visual"),
            VisualMode::Visual => write!(f, "Visual"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LyricsVisibility {
    #[default]
    Continuous,
    IdleOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.r * 255.0).clamp(0.0, 255.0) as u8,
            (self.g * 255.0).clamp(0.0, 255.0) as u8,
            (self.b * 255.0).clamp(0.0, 255.0) as u8
        )
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0
        } else {
            1.0
        };
        Some(Self { r, g, b, a })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub visual_mode: VisualMode,
    pub lyrics_visibility: LyricsVisibility,
    pub auto_sync_palette: bool,

    // Configurable Color System
    pub lyric_color: ColorRgba,
    pub highlight_color: ColorRgba,
    pub secondary_color: ColorRgba,
    pub background_color: ColorRgba,

    // Visual options
    pub font_family: String,
    pub font_size: f32,
    pub animation_intensity: f32,
    pub motion_speed: f32,
    pub background_blur: f32,

    // Playback & Sync
    pub preferred_player: Option<String>,
    pub sync_offset_ms: i64,
    pub lyrics_provider: String,

    // Display & Power
    pub fullscreen: bool,
    #[serde(default = "default_false")]
    pub transparent_background: bool,
    #[serde(default)]
    pub click_through: bool,
    #[serde(default = "default_true")]
    pub multi_line_context: bool,
    pub prevent_display_sleep: bool,
    pub target_fps: u32,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            visual_mode: VisualMode::Dynamic,
            lyrics_visibility: LyricsVisibility::Continuous,
            auto_sync_palette: true,

            // Clean typography defaults matching Verci aesthetic
            lyric_color: ColorRgba::new(0.706, 0.478, 0.400, 1.0), // Warm terracotta (#B47A66)
            highlight_color: ColorRgba::new(0.941, 0.886, 0.902, 1.0), // Light creamy off-white (#F0E2E6)
            secondary_color: ColorRgba::new(0.463, 0.337, 0.361, 0.85), // Dusty mauve badge (#76565C)
            background_color: ColorRgba::new(0.0, 0.0, 0.0, 1.0), // Pitch velvety black (#000000)

            font_family: "sans-serif".to_string(),
            font_size: 38.0,
            animation_intensity: 1.0,
            motion_speed: 1.0,
            background_blur: 0.8,

            preferred_player: None,
            sync_offset_ms: 0,
            lyrics_provider: "lrclib".to_string(),

            fullscreen: false,
            transparent_background: false,
            click_through: false,
            multi_line_context: true,
            prevent_display_sleep: true,
            target_fps: 60,
        }
    }
}

impl AppSettings {
    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("verci");
        let _ = fs::create_dir_all(&path);
        path.push("settings.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(settings) => {
                        info!("Loaded configuration from {:?}", path);
                        return settings;
                    }
                    Err(e) => {
                        error!("Failed to parse config from {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    error!("Failed to read config file {:?}: {}", path, e);
                }
            }
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
