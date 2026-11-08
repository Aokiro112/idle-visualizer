use image::{DynamicImage, RgbaImage};
use verci::app::settings::ColorRgba;
use verci::color::palette::LyricPalette;

#[test]
fn test_palette_extraction_from_image() {
    let mut img_buf = RgbaImage::new(100, 100);
    // Fill with deep blue/purple tone
    for pixel in img_buf.pixels_mut() {
        *pixel = image::Rgba([30, 20, 80, 255]);
    }
    let img = DynamicImage::ImageRgba8(img_buf);

    let palette = LyricPalette::extract_from_image(&img);

    // Background should be darkened atmospheric tone
    assert!(palette.background.r < 0.2);
    assert!(palette.background.g < 0.2);

    // Active highlight should maintain high luminance for readability
    let hl = &palette.highlight;
    let lum = 0.2126 * hl.r + 0.7152 * hl.g + 0.0722 * hl.b;
    assert!(
        lum >= 0.50,
        "Highlight luminance must be at least 0.50 for contrast, got {}",
        lum
    );
}

#[test]
fn test_palette_interpolation_lerp() {
    let pal_a = LyricPalette {
        background: ColorRgba::new(0.0, 0.0, 0.0, 1.0),
        lyric: ColorRgba::new(0.5, 0.5, 0.5, 1.0),
        highlight: ColorRgba::new(1.0, 1.0, 1.0, 1.0),
        secondary: ColorRgba::new(0.2, 0.2, 0.2, 1.0),
    };

    let pal_b = LyricPalette {
        background: ColorRgba::new(0.2, 0.2, 0.2, 1.0),
        lyric: ColorRgba::new(0.7, 0.7, 0.7, 1.0),
        highlight: ColorRgba::new(0.8, 0.8, 0.8, 1.0),
        secondary: ColorRgba::new(0.4, 0.4, 0.4, 1.0),
    };

    let mid = pal_a.lerp(&pal_b, 0.5);
    assert!((mid.background.r - 0.1).abs() < 0.01);
    assert!((mid.lyric.r - 0.6).abs() < 0.01);
    assert!((mid.highlight.r - 0.9).abs() < 0.01);
}

#[test]
fn test_palette_from_user_art() {
    let path = "/home/aokiro/.config/mozilla/firefox/firefox-mpris/18710_0.png";
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let pal = LyricPalette::extract_from_image(&img);
            println!("Background: {:?}", pal.background);
            println!("Highlight: {:?}", pal.highlight);
            println!("Lyric: {:?}", pal.lyric);
            println!("Secondary: {:?}", pal.secondary);
        }
    }
}
