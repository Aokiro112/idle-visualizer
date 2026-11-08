use crate::lyrics::model::{LyricLine, LyricWord, SyncedLyrics};
use crate::lyrics::provider::{LyricsError, LyricsProvider};

pub struct MockLyricsProvider;

impl MockLyricsProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn sample_word_synced_lyrics() -> SyncedLyrics {
        let lines = vec![
            LyricLine::new(
                vec![
                    LyricWord::new("When", 1.00, Some(1.35)),
                    LyricWord::new("the", 1.35, Some(1.60)),
                    LyricWord::new("night", 1.60, Some(2.10)),
                    LyricWord::new("has", 2.10, Some(2.45)),
                    LyricWord::new("come", 2.45, Some(3.30)),
                ],
                1.00,
                Some(3.50),
            ),
            LyricLine::new(
                vec![
                    LyricWord::new("And", 3.80, Some(4.10)),
                    LyricWord::new("the", 4.10, Some(4.30)),
                    LyricWord::new("land", 4.30, Some(4.80)),
                    LyricWord::new("is", 4.80, Some(5.10)),
                    LyricWord::new("dark", 5.10, Some(6.20)),
                ],
                3.80,
                Some(6.50),
            ),
            LyricLine::new(
                vec![
                    LyricWord::new("And", 6.80, Some(7.10)),
                    LyricWord::new("the", 7.10, Some(7.30)),
                    LyricWord::new("moon", 7.30, Some(7.85)),
                    LyricWord::new("is", 7.85, Some(8.15)),
                    LyricWord::new("the", 8.15, Some(8.40)),
                    LyricWord::new("only", 8.40, Some(8.95)),
                    LyricWord::new("light", 8.95, Some(9.50)),
                    LyricWord::new("we'll", 9.50, Some(9.85)),
                    LyricWord::new("see", 9.85, Some(11.00)),
                ],
                6.80,
                Some(11.20),
            ),
            LyricLine::new(
                vec![
                    LyricWord::new("No", 12.00, Some(12.40)),
                    LyricWord::new("I", 12.40, Some(12.65)),
                    LyricWord::new("won't", 12.65, Some(13.10)),
                    LyricWord::new("be", 13.10, Some(13.40)),
                    LyricWord::new("afraid", 13.40, Some(14.50)),
                ],
                12.00,
                Some(15.00),
            ),
            LyricLine::new(
                vec![
                    LyricWord::new("Oh", 15.50, Some(16.00)),
                    LyricWord::new("I", 16.00, Some(16.20)),
                    LyricWord::new("won't", 16.20, Some(16.60)),
                    LyricWord::new("shed", 16.60, Some(17.10)),
                    LyricWord::new("a", 17.10, Some(17.30)),
                    LyricWord::new("tear", 17.30, Some(18.50)),
                ],
                15.50,
                Some(19.00),
            ),
            LyricLine::new(
                vec![
                    LyricWord::new("Just", 19.30, Some(19.65)),
                    LyricWord::new("as", 19.65, Some(19.90)),
                    LyricWord::new("long", 19.90, Some(20.40)),
                    LyricWord::new("as", 20.40, Some(20.70)),
                    LyricWord::new("you", 20.70, Some(21.10)),
                    LyricWord::new("stand", 21.10, Some(21.75)),
                    LyricWord::new("by", 21.75, Some(22.10)),
                    LyricWord::new("me", 22.10, Some(23.50)),
                ],
                19.30,
                Some(24.00),
            ),
        ];

        SyncedLyrics::new(lines, true)
    }
}

impl Default for MockLyricsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LyricsProvider for MockLyricsProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn get_lyrics(
        &self,
        _artist: &str,
        _title: &str,
        _album: Option<&str>,
    ) -> Result<SyncedLyrics, LyricsError> {
        Ok(Self::sample_word_synced_lyrics())
    }
}
