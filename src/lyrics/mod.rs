pub mod cache;
pub mod lrclib;
pub mod mock;
pub mod model;
pub mod parser;
pub mod provider;
pub mod sync;
pub mod synchronizer;

pub use cache::LyricsCache;
pub use model::{LyricLine, LyricWord, SyncedLyrics};
pub use parser::LyricsParser;
pub use provider::{LyricsError, LyricsProvider};
pub use synchronizer::{ActiveSyncState, LyricSynchronizer, SyncState, SyncStatus};
