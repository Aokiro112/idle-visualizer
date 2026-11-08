use crate::lyrics::model::SyncedLyrics;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use tracing::{debug, error, info};

pub struct LyricsCache {
    memory_cache: RwLock<HashMap<String, SyncedLyrics>>,
    cache_dir: PathBuf,
}

impl LyricsCache {
    pub fn new() -> Self {
        let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache_dir.push("verci");
        cache_dir.push("lyrics");
        let _ = fs::create_dir_all(&cache_dir);

        Self {
            memory_cache: RwLock::new(HashMap::new()),
            cache_dir,
        }
    }

    fn make_key(artist: &str, title: &str) -> String {
        let clean_artist = artist.trim().to_lowercase();
        let clean_title = title.trim().to_lowercase();
        format!("{} - {}", clean_artist, clean_title)
    }

    fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    pub fn get(&self, artist: &str, title: &str) -> Option<SyncedLyrics> {
        let key = Self::make_key(artist, title);

        // Check memory cache
        if let Ok(mem) = self.memory_cache.read() {
            if let Some(lyrics) = mem.get(&key) {
                debug!("Memory cache hit for '{}'", key);
                return Some(lyrics.clone());
            }
        }

        // Check disk cache
        let filename = format!("{}.json", Self::sanitize_filename(&key));
        let path = self.cache_dir.join(filename);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(lyrics) = serde_json::from_str::<SyncedLyrics>(&content) {
                    debug!("Disk cache hit for '{}'", key);
                    if let Ok(mut mem) = self.memory_cache.write() {
                        mem.insert(key, lyrics.clone());
                    }
                    return Some(lyrics);
                }
            }
        }

        None
    }

    pub fn insert(&self, artist: &str, title: &str, lyrics: SyncedLyrics) {
        let key = Self::make_key(artist, title);

        // Store to memory
        if let Ok(mut mem) = self.memory_cache.write() {
            mem.insert(key.clone(), lyrics.clone());
        }

        // Store to disk
        let filename = format!("{}.json", Self::sanitize_filename(&key));
        let path = self.cache_dir.join(filename);
        if let Ok(json) = serde_json::to_string_pretty(&lyrics) {
            if let Err(e) = fs::write(&path, json) {
                error!("Failed to write lyrics cache to {:?}: {}", path, e);
            } else {
                info!("Cached lyrics to disk at {:?}", path);
            }
        }
    }
}

impl Default for LyricsCache {
    fn default() -> Self {
        Self::new()
    }
}
