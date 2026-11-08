use crate::lyrics::cache::LyricsCache;
use crate::lyrics::lrclib::LrclibProvider;
use crate::lyrics::mock::MockLyricsProvider;
use crate::lyrics::model::SyncedLyrics;
use crate::lyrics::provider::LyricsProvider;
use crate::media::player::TrackState;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Idle,
    Loading(String, String),
    Ready(String, String),
    NotFound(String, String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ActiveSyncState {
    pub lyrics: Option<SyncedLyrics>,
    pub status: SyncStatus,
    pub current_title: String,
    pub current_artist: String,
    pub active_line_index: Option<usize>,
    pub active_word_index: Option<usize>,
    pub word_progress: f32,
    pub playback_position: f64,
    pub is_playing: bool,
}

/// Canonical alias for synchronization state as defined in the architectural contract
pub type SyncState = ActiveSyncState;

impl Default for ActiveSyncState {
    fn default() -> Self {
        Self {
            lyrics: None,
            status: SyncStatus::Idle,
            current_title: String::new(),
            current_artist: String::new(),
            active_line_index: None,
            active_word_index: None,
            word_progress: 0.0,
            playback_position: 0.0,
            is_playing: false,
        }
    }
}

pub struct LyricSynchronizer {
    cache: Arc<LyricsCache>,
    pub providers: Vec<Box<dyn LyricsProvider>>,
    state: Arc<RwLock<ActiveSyncState>>,
}

impl LyricSynchronizer {
    pub fn new(cache: Arc<LyricsCache>) -> Self {
        let providers: Vec<Box<dyn LyricsProvider>> = vec![
            Box::new(LrclibProvider::new()),
            Box::new(MockLyricsProvider::new()),
        ];

        Self {
            cache,
            providers,
            state: Arc::new(RwLock::new(ActiveSyncState::default())),
        }
    }

    pub fn state_handle(&self) -> Arc<RwLock<ActiveSyncState>> {
        self.state.clone()
    }

    /// Update synchronization against current media state
    pub async fn update_playback(
        &self,
        media_state: &TrackState,
        sync_offset_ms: i64,
        force_mock: bool,
    ) {
        let meta_opt = media_state.metadata.clone();
        let is_playing = media_state.playback_state.is_playing();
        let raw_pos = media_state.current_position();
        let effective_pos = (raw_pos + (sync_offset_ms as f64 / 1000.0)).max(0.0);

        if let Some(meta) = meta_opt {
            let title = meta.title.clone();
            let artist = meta.artist.clone();
            let album = meta.album.clone();

            let mut state_lock = self.state.write().await;
            let song_changed =
                state_lock.current_title != title || state_lock.current_artist != artist;

            if song_changed {
                info!("Track changed to: '{}' - '{}'", artist, title);
                state_lock.current_title = title.clone();
                state_lock.current_artist = artist.clone();
                state_lock.lyrics = None;
                state_lock.status = SyncStatus::Loading(artist.clone(), title.clone());
                state_lock.active_line_index = None;
                state_lock.active_word_index = None;
                state_lock.word_progress = 0.0;

                // Spawn lyrics fetch in background
                let cache = self.cache.clone();
                let state_ref = self.state.clone();
                let artist_fetch = artist.clone();
                let title_fetch = title.clone();
                let album_fetch = album.clone();

                tokio::spawn(async move {
                    // Check cache first
                    if let Some(cached) = cache.get(&artist_fetch, &title_fetch) {
                        info!(
                            "Using cached lyrics for '{}' - '{}'",
                            artist_fetch, title_fetch
                        );
                        let mut lock = state_ref.write().await;
                        if lock.current_title == title_fetch && lock.current_artist == artist_fetch
                        {
                            lock.lyrics = Some(cached);
                            lock.status = SyncStatus::Ready(artist_fetch, title_fetch);
                        }
                        return;
                    }

                    if force_mock {
                        let mock = MockLyricsProvider::sample_word_synced_lyrics();
                        cache.insert(&artist_fetch, &title_fetch, mock.clone());
                        let mut lock = state_ref.write().await;
                        if lock.current_title == title_fetch && lock.current_artist == artist_fetch
                        {
                            lock.lyrics = Some(mock);
                            lock.status = SyncStatus::Ready(artist_fetch, title_fetch);
                        }
                        return;
                    }

                    // Query remote provider
                    let lrclib = LrclibProvider::new();
                    match lrclib
                        .get_lyrics(&artist_fetch, &title_fetch, album_fetch.as_deref())
                        .await
                    {
                        Ok(lyrics) => {
                            cache.insert(&artist_fetch, &title_fetch, lyrics.clone());
                            let mut lock = state_ref.write().await;
                            if lock.current_title == title_fetch
                                && lock.current_artist == artist_fetch
                            {
                                lock.lyrics = Some(lyrics);
                                lock.status = SyncStatus::Ready(artist_fetch, title_fetch);
                            }
                        }
                        Err(e) => {
                            info!(
                                "Lyrics not found for '{}' - '{}': {}",
                                artist_fetch, title_fetch, e
                            );
                            let mut lock = state_ref.write().await;
                            if lock.current_title == title_fetch
                                && lock.current_artist == artist_fetch
                            {
                                // Do not display random mock lyrics; visualizer renders floating 3D track card
                                lock.lyrics = None;
                                lock.status = SyncStatus::NotFound(artist_fetch, title_fetch);
                            }
                        }
                    }
                });
            }

            state_lock.playback_position = effective_pos;
            state_lock.is_playing = is_playing;

            // Compute active word and line from lyrics
            if let Some(ref lyrics) = state_lock.lyrics {
                if let Some((line_idx, word_idx, progress)) = lyrics.find_active(effective_pos) {
                    state_lock.active_line_index = Some(line_idx);
                    state_lock.active_word_index = word_idx;
                    state_lock.word_progress = progress;
                }
            }
        } else {
            let mut state_lock = self.state.write().await;
            state_lock.is_playing = false;
            state_lock.playback_position = 0.0;
            state_lock.status = SyncStatus::Idle;
        }
    }
}
