use std::sync::Arc;
use std::time::{Duration, Instant};
use verci::lyrics::cache::LyricsCache;
use verci::lyrics::synchronizer::LyricSynchronizer;
use verci::media::player::{PlaybackState, TrackMetadata, TrackState};

#[test]
fn test_track_state_monotonic_and_pause_behavior() {
    let mut state = TrackState {
        player_name: "spotify".to_string(),
        metadata: Some(TrackMetadata {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: None,
            art_url: None,
            duration_seconds: Some(180.0),
        }),
        playback_state: PlaybackState::Playing,
        base_position: 45.0,
        base_instant: Instant::now() - Duration::from_millis(500),
    };

    // While playing, current_position should reflect base + elapsed
    let pos_playing = state.current_position();
    assert!((45.45..=45.55).contains(&pos_playing));

    // Pause playback
    state.playback_state = PlaybackState::Paused;
    state.base_position = pos_playing;
    state.base_instant = Instant::now();

    // After pause, position should remain completely frozen
    std::thread::sleep(Duration::from_millis(50));
    let pos_paused = state.current_position();
    assert_eq!(pos_paused, state.base_position);
}

#[tokio::test]
async fn test_rapid_song_switching() {
    let cache = Arc::new(LyricsCache::new());
    let synchronizer = LyricSynchronizer::new(cache);

    // Simulate 10 rapid track changes
    for i in 0..10 {
        let state = TrackState {
            player_name: "spotify".to_string(),
            metadata: Some(TrackMetadata {
                title: format!("Rapid Song {}", i),
                artist: format!("Artist {}", i),
                album: None,
                art_url: None,
                duration_seconds: Some(200.0),
            }),
            playback_state: PlaybackState::Playing,
            base_position: 10.0,
            base_instant: Instant::now(),
        };

        synchronizer.update_playback(&state, 0, true).await;
    }

    // Verify state settled on the final track
    let active = synchronizer.state_handle().read().await.clone();
    assert_eq!(active.current_title, "Rapid Song 9");
    assert_eq!(active.current_artist, "Artist 9");
}

#[tokio::test]
async fn test_no_mock_lyrics_on_not_found() {
    let cache = Arc::new(LyricsCache::new());
    let synchronizer = LyricSynchronizer::new(cache.clone());

    let state = TrackState {
        player_name: "firefox".to_string(),
        metadata: Some(TrackMetadata {
            title: "Completely Nonexistent Song 999999".to_string(),
            artist: "Unknown Artist 888888".to_string(),
            album: None,
            art_url: None,
            duration_seconds: Some(180.0),
        }),
        playback_state: PlaybackState::Playing,
        base_position: 5.0,
        base_instant: Instant::now(),
    };

    // force_mock is false: should never inject mock lyrics
    synchronizer.update_playback(&state, 0, false).await;

    // Wait briefly for background task
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify cache has no mock lyrics saved
    assert!(cache
        .get(
            "Unknown Artist 888888",
            "Completely Nonexistent Song 999999"
        )
        .is_none());
}

#[test]
fn test_line_synced_word_interpolation() {
    use verci::lyrics::parser::LyricsParser;

    let lrc = "[00:10.00] Line one with five words\n[00:15.00] Line two with five words\n";
    let lyrics = LyricsParser::parse_lrc(lrc);

    assert!(!lyrics.is_word_synced);
    assert_eq!(lyrics.lines.len(), 2);

    // At 10.5s (10% into a 5-second line)
    let (line_idx, word_idx, progress) = lyrics.find_active(10.5).expect("Should find active");
    assert_eq!(line_idx, 0);
    assert_eq!(word_idx, Some(0));
    assert!(progress > 0.0);

    // At 14.0s (80% into the line)
    let (line_idx_late, word_idx_late, _) = lyrics.find_active(14.0).expect("Should find active");
    assert_eq!(line_idx_late, 0);
    assert_eq!(word_idx_late, Some(4)); // Last word "words"
}
