use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackState {
    pub fn is_playing(&self) -> bool {
        matches!(self, PlaybackState::Playing)
    }
}

#[derive(Debug, Clone)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub art_url: Option<String>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TrackState {
    pub player_name: String,
    pub metadata: Option<TrackMetadata>,
    pub playback_state: PlaybackState,
    pub base_position: f64,
    pub base_instant: Instant,
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            metadata: None,
            playback_state: PlaybackState::Stopped,
            base_position: 0.0,
            base_instant: Instant::now(),
        }
    }
}

impl TrackState {
    /// Calculate the real-time monotonic playback position with sub-millisecond precision
    pub fn current_position(&self) -> f64 {
        match self.playback_state {
            PlaybackState::Playing => {
                let elapsed = self.base_instant.elapsed().as_secs_f64();
                let pos = self.base_position + elapsed;
                if let Some(meta) = &self.metadata {
                    if let Some(dur) = meta.duration_seconds {
                        return pos.min(dur);
                    }
                }
                pos
            }
            PlaybackState::Paused | PlaybackState::Stopped => self.base_position,
        }
    }

    /// Check if track matches another title/artist combination
    pub fn is_same_song(&self, other_title: &str, other_artist: &str) -> bool {
        if let Some(ref meta) = self.metadata {
            meta.title == other_title && meta.artist == other_artist
        } else {
            false
        }
    }
}

pub trait MediaPlayer {
    fn name(&self) -> &str;
    fn current_state(&self) -> Option<TrackState>;
}
