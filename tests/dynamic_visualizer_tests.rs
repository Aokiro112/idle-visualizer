use verci::app::settings::AppSettings;
use verci::app::state::AppState;
use verci::lyrics::lrclib::LrclibProvider;
use verci::lyrics::model::{LyricLine, LyricWord, SyncedLyrics};
use verci::visualizer::animation::{damped_spring, exp_decay, lerp, organic_drift, smoothstep};
use verci::visualizer::engine::{ElementLifeState, VisualLyricEngine};
use verci::visualizer::layout::{DynamicLayoutEngine, LayoutArchetype};

#[test]
fn test_dynamic_layout_generation_and_spatial_distribution() {
    let layout_engine = DynamicLayoutEngine::new();

    // 1. Test 3 words like in the reference style: "NOW", "when", "you"
    let words = vec![
        LyricWord::new("NOW", 1.0, Some(2.0)),
        LyricWord::new("when", 2.0, Some(2.5)),
        LyricWord::new("you", 2.5, Some(3.5)),
    ];

    let layout = layout_engine.generate_layout(0, &words, 1920, 1080);
    assert_eq!(layout.nodes.len(), 3);

    // Ensure non-NaN and finite coordinates
    for node in &layout.nodes {
        assert!(!node.base_x.is_nan());
        assert!(!node.base_y.is_nan());
        assert!(!node.base_scale.is_nan());
        assert!(!node.base_rotation.is_nan());

        // Bounded within cinematic canvas
        assert!(node.base_x >= -0.85 && node.base_x <= 0.85);
        assert!(node.base_y >= -0.65 && node.base_y <= 0.65);
        assert!(node.base_scale > 0.5 && node.base_scale < 3.0);
    }

    // Words should NOT have identical coordinates: they must be dynamically positioned
    let n0 = &layout.nodes[0];
    let n1 = &layout.nodes[1];
    let n2 = &layout.nodes[2];

    assert!((n0.base_x - n1.base_x).abs() > 0.1 || (n0.base_y - n1.base_y).abs() > 0.1);
    assert!((n1.base_x - n2.base_x).abs() > 0.1 || (n1.base_y - n2.base_y).abs() > 0.1);

    // Words have different sizes (scales)
    println!(
        "Scales: NOW={}, when={}, you={}",
        n0.base_scale, n1.base_scale, n2.base_scale
    );
    assert!((n0.base_scale - n1.base_scale).abs() > 0.05);
}

#[test]
fn test_dynamic_layout_archetypes_and_reproducibility() {
    let layout_engine = DynamicLayoutEngine::new();

    // Single word
    let single = vec![LyricWord::new("SURRENDER", 0.0, Some(1.0))];
    let l_single = layout_engine.generate_layout(0, &single, 1920, 1080);
    assert_eq!(l_single.archetype, LayoutArchetype::CenteredRhythm);
    assert_eq!(l_single.nodes.len(), 1);
    assert!(l_single.nodes[0].base_scale >= 1.5); // Large typography

    // Deterministic reproducibility: same input gives exact same coordinates
    let l_single_dup = layout_engine.generate_layout(0, &single, 1920, 1080);
    assert_eq!(l_single.nodes[0].base_x, l_single_dup.nodes[0].base_x);
    assert_eq!(l_single.nodes[0].base_y, l_single_dup.nodes[0].base_y);
    assert_eq!(
        l_single.nodes[0].base_scale,
        l_single_dup.nodes[0].base_scale
    );

    // Many words (cascading cluster)
    let many_words = vec![
        LyricWord::new("tu", 0.0, Some(0.5)),
        LyricWord::new("agar", 0.5, Some(1.0)),
        LyricWord::new("meri", 1.0, Some(1.5)),
        LyricWord::new("ye", 1.5, Some(2.0)),
        LyricWord::new("hawayein", 2.0, Some(3.0)),
        LyricWord::new("teri", 3.0, Some(4.0)),
    ];
    let l_many = layout_engine.generate_layout(1, &many_words, 1920, 1080);
    assert_eq!(l_many.nodes.len(), 6);
    for n in &l_many.nodes {
        assert!(!n.base_x.is_nan());
        assert!(!n.base_y.is_nan());
    }
}

#[test]
fn test_visual_engine_element_lifecycle_and_word_activation() {
    let mut engine = VisualLyricEngine::new();

    let lines = vec![
        LyricLine::new(
            vec![
                LyricWord::new("NOW", 10.0, Some(11.0)),
                LyricWord::new("WHEN", 11.0, Some(12.0)),
                LyricWord::new("YOU", 12.0, Some(13.0)),
            ],
            10.0,
            Some(13.5),
        ),
        LyricLine::new(
            vec![
                LyricWord::new("TOUCH", 14.0, Some(15.0)),
                LyricWord::new("ME", 15.0, Some(16.0)),
            ],
            14.0,
            Some(16.5),
        ),
    ];
    let lyrics = SyncedLyrics::new(lines, true);

    // At pos 10.5s: First line active, first word ("NOW") active
    engine.update(Some(&lyrics), 10.5, true, 0.016, 1920, 1080);

    let elems = engine.elements();
    assert_eq!(elems.len(), 3);

    let word0 = elems.iter().find(|e| e.text == "NOW").unwrap();
    let word1 = elems.iter().find(|e| e.text == "WHEN").unwrap();

    assert_eq!(word0.life_state, ElementLifeState::Active);
    assert!(word0.is_active);
    assert!(word0.target_opacity > 0.9);
    assert!(word0.target_glow > 1.0);
    assert!(word0.target_color_lerp > 0.9);

    assert_eq!(word1.life_state, ElementLifeState::Entering);
    assert!(!word1.is_active);
    assert!(word1.target_glow < 0.1);

    // Advance to pos 11.5s: word1 ("WHEN") becomes active, word0 ("NOW") lingers
    engine.update(Some(&lyrics), 11.5, true, 0.016, 1920, 1080);

    let elems = engine.elements();
    let word0 = elems.iter().find(|e| e.text == "NOW").unwrap();
    let word1 = elems.iter().find(|e| e.text == "WHEN").unwrap();

    assert_eq!(word0.life_state, ElementLifeState::Lingering);
    assert!(!word0.is_active);

    assert_eq!(word1.life_state, ElementLifeState::Active);
    assert!(word1.is_active);

    // Advance into line 1 at 14.5s: line 0 elements transition to Exiting
    engine.update(Some(&lyrics), 14.5, true, 0.016, 1920, 1080);
    let elems = engine.elements();

    let touch_elem = elems.iter().find(|e| e.text == "TOUCH").unwrap();
    assert_eq!(touch_elem.life_state, ElementLifeState::Active);

    // Seek backward to 10.2s: engine updates immediately to line 0
    engine.update(Some(&lyrics), 10.2, true, 0.016, 1920, 1080);
    let elems = engine.elements();
    assert!(elems.iter().any(|e| e.text == "NOW"));
}

#[test]
fn test_animation_math_and_organic_drift() {
    // Smoothstep
    assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
    assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-4);

    // Lerp
    assert!((lerp(10.0, 20.0, 0.5) - 15.0).abs() < 1e-4);

    // Exp decay
    let val = exp_decay(0.0, 10.0, 5.0, 0.1);
    assert!(val > 0.0 && val < 10.0);

    // Damped spring
    let mut vel = 0.0;
    let mut current = 0.0;
    for _ in 0..60 {
        current = damped_spring(current, 1.0, &mut vel, 100.0, 10.0, 0.016);
    }
    assert!((current - 1.0).abs() < 0.1);

    // Organic drift
    let (d1_x, d1_y) = organic_drift(1.0, 0.5, 0.8);
    let (d2_x, d2_y) = organic_drift(2.0, 0.5, 0.8);
    assert!(!d1_x.is_nan() && !d1_y.is_nan());
    assert!((d1_x - d2_x).abs() > 0.0 || (d1_y - d2_y).abs() > 0.0);
}

#[test]
fn test_youtube_music_title_and_artist_sanitization() {
    // 1. YouTube Music soundtrack / movie tags
    let title1 = "Gehra Hua (From \"Dhurandhar\")";
    assert_eq!(LrclibProvider::sanitize_title(title1), "Gehra Hua");

    let title2 = "Tum Hi Ho (From \"Aashiqui 2\")";
    assert_eq!(LrclibProvider::sanitize_title(title2), "Tum Hi Ho");

    // 2. Featured artists in title
    let title3 = "Starboy (feat. Daft Punk)";
    assert_eq!(LrclibProvider::sanitize_title(title3), "Starboy");

    let title4 = "Levitating [feat. DaBaby]";
    assert_eq!(LrclibProvider::sanitize_title(title4), "Levitating");

    // 3. YouTube pipe syntax: "Song | Artist | Movie"
    let title5 = "Kesariya | Brahmāstra | Ranbir Kapoor";
    assert_eq!(LrclibProvider::sanitize_title(title5), "Kesariya");

    // 4. Remastered / Video suffixes
    let title6 = "Bohemian Rhapsody - Remastered 2011";
    assert_eq!(LrclibProvider::sanitize_title(title6), "Bohemian Rhapsody");

    let title7 = "Chaleya (Official Music Video)";
    assert_eq!(LrclibProvider::sanitize_title(title7), "Chaleya");

    // 5. Multi-artist strings
    let multi_artist = "Shashwat Sachdev, Arijit Singh, Armaan Khan, & Irshad Kamil";
    assert_eq!(
        LrclibProvider::primary_artist(multi_artist),
        "Shashwat Sachdev"
    );

    let feat_artist = "The Weeknd feat. Daft Punk";
    assert_eq!(LrclibProvider::primary_artist(feat_artist), "The Weeknd");
}

#[test]
fn test_symbol_plural_stemming_and_monumental_stack() {
    let settings = AppSettings::default();
    let state = AppState::new(settings);

    // Exact word from reference screenshot: "cars" -> stemmed to "car" -> "🚗"
    let sym_cars = state.lookup_symbol("cars");
    assert_eq!(sym_cars.as_deref(), Some("🚗"));

    let sym_car = state.lookup_symbol("car");
    assert_eq!(sym_car.as_deref(), Some("🚗"));

    let sym_eyes = state.lookup_symbol("eyes");
    assert_eq!(sym_eyes.as_deref(), Some("👀"));

    let sym_stars = state.lookup_symbol("stars");
    assert!(sym_stars.is_some());

    // Monumental vertical stack layout test: "of", "my", "cars"
    let layout_engine = DynamicLayoutEngine::new();
    let words = vec![
        LyricWord::new("of", 0.0, Some(0.8)),
        LyricWord::new("my", 0.8, Some(1.6)),
        LyricWord::new("cars", 1.6, Some(2.5)),
    ];
    let layout = layout_engine.generate_layout(42, &words, 1920, 1080);
    assert_eq!(layout.archetype, LayoutArchetype::MonumentalStack);
    assert_eq!(layout.nodes.len(), 3);

    let n_of = &layout.nodes[0];
    let n_my = &layout.nodes[1];
    let n_cars = &layout.nodes[2];

    // Check vertical ordering: of on top, my in center, cars at bottom
    assert!(n_of.base_y > n_my.base_y);
    assert!(n_my.base_y > n_cars.base_y);

    // Check monumental scale hierarchy: "my" dominates canvas (>= 2.2x)
    assert!(n_my.is_peak_word);
    assert!(n_my.base_scale >= 2.2);
    assert!(n_of.base_scale < 1.3);
    assert!(n_cars.base_scale < 1.4);

    // Near-center alignment on X axis
    assert!(n_of.base_x.abs() < 0.05);
    assert!(n_my.base_x.abs() < 0.05);
    assert!(n_cars.base_x.abs() < 0.05);
}

#[test]
fn test_kinetic_highlight_tracking_and_interpolation() {
    let mut engine = VisualLyricEngine::new();

    let lines = vec![
        LyricLine::new(
            vec![
                LyricWord::new("I'M", 2.0, Some(3.0)),
                LyricWord::new("STUNNING", 3.0, Some(4.5)),
            ],
            2.0,
            Some(5.0),
        ),
    ];
    let lyrics = SyncedLyrics::new(lines, true);

    // Initial tick before music starts: highlight should be invisible
    engine.update(Some(&lyrics), 1.0, true, 0.016, 1920, 1080);
    assert_eq!(engine.highlight().target_opacity, 0.0);

    // At pos 2.5s: "I'M" is active! Highlight targets "I'M"
    engine.update(Some(&lyrics), 2.5, true, 0.016, 1920, 1080);
    let hl = engine.highlight();
    assert_eq!(hl.active_word.as_deref(), Some("I'M"));
    assert!(hl.target_opacity > 0.8);
    assert!(hl.target_w > 0.1);
    assert!(hl.target_h > 0.05);

    // Run a few animation frames: highlight position interpolates towards active word
    for _ in 0..10 {
        engine.update(Some(&lyrics), 2.6, true, 0.016, 1920, 1080);
    }
    assert!(engine.highlight().opacity > 0.5);

    // At pos 3.5s: "STUNNING" becomes active! Highlight targets "STUNNING"
    engine.update(Some(&lyrics), 3.5, true, 0.016, 1920, 1080);
    assert_eq!(engine.highlight().active_word.as_deref(), Some("STUNNING"));

    // Advance past line duration: highlight target opacity fades to 0
    engine.update(Some(&lyrics), 5.5, true, 0.016, 1920, 1080);
    assert_eq!(engine.highlight().target_opacity, 0.0);
}
