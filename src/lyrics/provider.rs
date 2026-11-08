use crate::lyrics::model::SyncedLyrics;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LyricsError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Lyrics not found for artist='{0}', title='{1}'")]
    NotFound(String, String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Provider error: {0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait LyricsProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn get_lyrics(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<SyncedLyrics, LyricsError>;
}
