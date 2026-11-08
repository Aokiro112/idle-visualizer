use verci::lyrics::cache::LyricsCache;
use verci::lyrics::model::{LyricLine, LyricWord, SyncedLyrics};
use verci::lyrics::parser::LyricsParser;

#[test]
fn test_parse_enhanced_lrc_word_timestamps() {
    let lrc = r#"
[00:01.00] <00:01.00> When <00:01.35> the <00:01.60> night <00:02.10> has <00:02.45> come
[00:03.50] <00:03.50> And <00:03.80> the <00:04.10> land <00:04.30> is <00:04.80> dark
"#;

    let lyrics = LyricsParser::parse_lrc(lrc);
    assert!(lyrics.is_word_synced);
    assert_eq!(lyrics.lines.len(), 2);

    let line1 = &lyrics.lines[0];
    assert_eq!(line1.words.len(), 5);
    assert_eq!(line1.words[0].text, "When");
    assert!((line1.words[0].start - 1.00).abs() < 0.001);
    assert_eq!(line1.words[0].end, Some(1.35));

    assert_eq!(line1.words[2].text, "night");
    assert!((line1.words[2].start - 1.60).abs() < 0.001);
    assert_eq!(line1.words[2].end, Some(2.10));
}

#[test]
fn test_parse_standard_lrc_fallback() {
    let lrc = r#"
[00:10.50] Hello darkness my old friend
[00:15.20] I've come to talk with you again
"#;

    let lyrics = LyricsParser::parse_lrc(lrc);
    assert!(!lyrics.is_word_synced);
    assert_eq!(lyrics.lines.len(), 2);
    assert_eq!(lyrics.lines[0].words.len(), 5);
}

#[test]
fn test_empty_and_corrupted_lyrics() {
    let empty = LyricsParser::parse_lrc("");
    assert_eq!(empty.lines.len(), 0);
    assert!(empty.find_active(5.0).is_none());

    let corrupted = LyricsParser::parse_lrc("[invalid] garbage [99:99:99]");
    assert_eq!(corrupted.lines.len(), 0);
}

#[test]
fn test_lyrics_with_many_words() {
    let mut words = Vec::new();
    for i in 0..600 {
        words.push(format!("<00:{:02}.{:02}> word{}", (i / 60), (i % 60), i));
    }
    let lrc = format!("[00:00.00] {}", words.join(" "));
    let lyrics = LyricsParser::parse_lrc(&lrc);
    assert!(lyrics.lines[0].words.len() >= 500);
}

#[test]
fn test_timing_follows_playback_position_not_elapsed_time() {
    let lines = vec![
        LyricLine::new(
            vec![
                LyricWord::new("First", 10.0, Some(11.0)),
                LyricWord::new("Second", 11.0, Some(12.0)),
            ],
            10.0,
            Some(12.5),
        ),
        LyricLine::new(
            vec![
                LyricWord::new("Third", 20.0, Some(21.0)),
                LyricWord::new("Fourth", 21.0, Some(22.0)),
            ],
            20.0,
            Some(23.0),
        ),
    ];
    let lyrics = SyncedLyrics::new(lines, true);

    // At position 10.5s -> line 0, word 0 ("First"), 50% progress
    let (line_idx, word_idx, progress) = lyrics.find_active(10.5).unwrap();
    assert_eq!(line_idx, 0);
    assert_eq!(word_idx, Some(0));
    assert!((progress - 0.5).abs() < 0.05);

    // At position 11.75s -> line 0, word 1 ("Second"), 75% progress
    let (line_idx, word_idx, progress) = lyrics.find_active(11.75).unwrap();
    assert_eq!(line_idx, 0);
    assert_eq!(word_idx, Some(1));
    assert!((progress - 0.75).abs() < 0.05);

    // Direct seek to 20.2s -> line 1, word 0 ("Third"), 20% progress
    let (line_idx, word_idx, progress) = lyrics.find_active(20.2).unwrap();
    assert_eq!(line_idx, 1);
    assert_eq!(word_idx, Some(0));
    assert!((progress - 0.2).abs() < 0.05);
}

#[test]
fn test_lyrics_cache_roundtrip() {
    let cache = LyricsCache::new();
    let sample = verci::lyrics::mock::MockLyricsProvider::sample_word_synced_lyrics();
    cache.insert("Test Artist", "Test Song", sample.clone());

    let retrieved = cache.get("Test Artist", "Test Song");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().lines.len(), sample.lines.len());
}

#[tokio::test]
async fn test_live_lrclib_retrieval_for_youtube_music_track() {
    use verci::lyrics::lrclib::LrclibProvider;
    use verci::lyrics::provider::LyricsProvider;

    let provider = LrclibProvider::new();
    let artist = "Shashwat Sachdev, Arijit Singh, Armaan Khan, & Irshad Kamil";
    let title = "Gehra Hua (From \"Dhurandhar\")";

    let result = provider.get_lyrics(artist, title, None).await;
    assert!(
        result.is_ok(),
        "Failed to retrieve lyrics: {:?}",
        result.err()
    );
    let lyrics = result.unwrap();
    assert!(!lyrics.lines.is_empty(), "Retrieved empty lyrics");
    println!(
        "Retrieved {} lines of synchronized lyrics for Gehra Hua!",
        lyrics.lines.len()
    );
}
