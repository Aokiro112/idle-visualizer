use crate::app::settings::AppSettings;
use crate::color::palette::LyricPalette;
use crate::lyrics::synchronizer::ActiveSyncState;
use crate::media::player::TrackState;
use crate::visualizer::RollingLyricEngine;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub media: Arc<RwLock<TrackState>>,
    pub lyrics: Arc<RwLock<ActiveSyncState>>,
    pub current_palette: Arc<RwLock<LyricPalette>>,
    pub target_palette: Arc<RwLock<LyricPalette>>,
    pub symbol_map: Arc<HashMap<String, String>>,
    pub rolling_engine: Arc<Mutex<RollingLyricEngine>>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        let palette = LyricPalette {
            background: settings.background_color.clone(),
            lyric: settings.lyric_color.clone(),
            highlight: settings.highlight_color.clone(),
            secondary: settings.secondary_color.clone(),
        };

        // Load symbol mappings from embedded or assets file
        let symbol_json = include_str!("../../assets/symbol-mappings.json");
        let symbol_map: HashMap<String, String> =
            serde_json::from_str(symbol_json).unwrap_or_default();

        Self {
            settings: Arc::new(RwLock::new(settings)),
            media: Arc::new(RwLock::new(TrackState::default())),
            lyrics: Arc::new(RwLock::new(ActiveSyncState::default())),
            current_palette: Arc::new(RwLock::new(palette.clone())),
            target_palette: Arc::new(RwLock::new(palette)),
            symbol_map: Arc::new(symbol_map),
            rolling_engine: Arc::new(Mutex::new(RollingLyricEngine::new())),
        }
    }

    pub fn lookup_symbol(&self, word: &str) -> Option<String> {
        let clean = word
            .trim_matches(|c: char| !c.is_alphabetic())
            .to_lowercase();
        if let Some(sym) = self.symbol_map.get(&clean) {
            return Some(sym.clone());
        }
        // Plural stemming fallback (e.g. "cars" -> "car", "eyes" -> "eye", "stars" -> "star")
        if clean.ends_with('s') && clean.len() > 3 {
            let stem = &clean[..clean.len() - 1];
            if let Some(sym) = self.symbol_map.get(stem) {
                return Some(sym.clone());
            }
        }
        // Plural ending in "es" fallback (e.g. "watches" -> "watch")
        if clean.ends_with("es") && clean.len() > 4 {
            let stem = &clean[..clean.len() - 2];
            if let Some(sym) = self.symbol_map.get(stem) {
                return Some(sym.clone());
            }
        }
        None
    }
}
