use verci::app::settings::AppSettings;
use verci::app::state::AppState;
use verci::renderer::fisheye::FisheyeRenderer;
use verci::renderer::ship::ShipRenderer;
use verci::renderer::visual::VisualRenderer;

#[test]
fn test_symbol_mappings_lookup() {
    let settings = AppSettings::default();
    let state = AppState::new(settings);

    assert_eq!(state.lookup_symbol("love"), Some("❤️".to_string()));
    assert_eq!(state.lookup_symbol("LOVE!"), Some("❤️".to_string()));
    assert_eq!(state.lookup_symbol("fire"), Some("🔥".to_string()));
    assert_eq!(state.lookup_symbol("star"), Some("⭐".to_string()));
    assert_eq!(state.lookup_symbol("music"), Some("🎵".to_string()));

    // Fallback gracefully for unknown words
    assert_eq!(state.lookup_symbol("unmappedwordxyz"), None);
}

#[test]
fn test_fisheye_math() {
    let fisheye = FisheyeRenderer::new();
    let dist = fisheye.get_distortion_uniform(1.0);
    assert!((0.2..=0.95).contains(&dist));

    let mvp = fisheye.compute_mvp(16.0 / 9.0, 4.0);
    assert_eq!(mvp[15], 1.0);
}

#[test]
fn test_ship_3d_mvp_matrix() {
    let mut ship = ShipRenderer::new();
    let mvp = ship.compute_mvp(1.77, 4.0, 0.0, 1.0, 0.016);
    // Ensure projection matrix values are valid non-NaN floats
    for val in &mvp {
        assert!(!val.is_nan());
        assert!(!val.is_infinite());
    }

    // Multiply center vertex [0.0, 0.0, 0.0, 1.0] by MVP
    // In GLSL: gl_Position = u_mvp * vec4(a_pos, 1.0);
    // Where u_mvp is column-major:
    let x = mvp[0] * 0.0 + mvp[4] * 0.0 + mvp[8] * 0.0 + mvp[12] * 1.0;
    let y = mvp[1] * 0.0 + mvp[5] * 0.0 + mvp[9] * 0.0 + mvp[13] * 1.0;
    let z = mvp[2] * 0.0 + mvp[6] * 0.0 + mvp[10] * 0.0 + mvp[14] * 1.0;
    let w = mvp[3] * 0.0 + mvp[7] * 0.0 + mvp[11] * 0.0 + mvp[15] * 1.0;
    println!("Clip coords: x={}, y={}, z={}, w={}", x, y, z, w);
    let ndc_x = x / w;
    let ndc_y = y / w;
    let ndc_z = z / w;
    println!(
        "NDC coords: ndc_x={}, ndc_y={}, ndc_z={}",
        ndc_x, ndc_y, ndc_z
    );
    assert!(w > 0.0, "w must be positive in front of camera");
    assert!(
        (-1.0..=1.0).contains(&ndc_z),
        "ndc_z must be inside [-1, 1]"
    );
}

#[test]
fn test_visual_particle_lifecycle() {
    let mut visual = VisualRenderer::new();
    visual.trigger_symbol("🔥", 0, 10.0);
    assert!(visual.active_particle.is_some());

    // Update at same time
    visual.update(10.05, Some(0));
    let p = visual.active_particle.as_ref().unwrap();
    assert!(p.scale > 0.0);
    assert!(p.opacity > 0.0);

    // Active word advances to word 1
    visual.update(10.5, Some(1));
    // Particle for word 0 begins fading out
    visual.update(12.0, Some(1));
    assert!(visual.active_particle.is_none());
}

#[test]
fn test_cairo_pango_pixel_generation() {
    use cairo::{Context, Format, ImageSurface};
    use pangocairo::pango::{AttrColor, AttrList, FontDescription};

    let width = 800;
    let height = 200;
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let cr = Context::new(&surface).unwrap();

    // Clear surface to fully transparent
    cr.set_operator(cairo::Operator::Clear);
    cr.paint().unwrap();
    cr.set_operator(cairo::Operator::Over);

    let layout = pangocairo::functions::create_layout(&cr);
    let mut font_desc = FontDescription::from_string("sans-serif");
    font_desc.set_size(36 * pango::SCALE);
    layout.set_font_description(Some(&font_desc));
    layout.set_text("Hello World Lyric Test");

    let attr_list = AttrList::new();
    let mut fg = AttrColor::new_foreground(65535, 65535, 65535);
    fg.set_start_index(0);
    fg.set_end_index(5);
    attr_list.insert(fg);
    layout.set_attributes(Some(&attr_list));

    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.move_to(50.0, 50.0);
    pangocairo::functions::show_layout(&cr, &layout);
    drop(cr);

    surface.flush();
    let data = surface.data().unwrap();
    let non_zero_count = data.iter().filter(|&&b| b > 0).count();
    println!("Non-zero pixel bytes count: {}", non_zero_count);
    assert!(non_zero_count > 100);
}

#[test]
fn test_hindi_lyric_text_atlas() {
    use cairo::{Context, Format, ImageSurface};
    use pangocairo::pango::{AttrColor, AttrList, FontDescription};

    let width = 1200;
    let height = 260;
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let cr = Context::new(&surface).unwrap();

    cr.set_operator(cairo::Operator::Clear);
    cr.paint().unwrap();
    cr.set_operator(cairo::Operator::Over);
    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);

    let layout = pangocairo::functions::create_layout(&cr);
    let mut font_desc = FontDescription::from_string("sans-serif");
    font_desc.set_size(38 * pango::SCALE);
    layout.set_font_description(Some(&font_desc));

    let display_text = "तेरा यकीन क्यों";
    layout.set_text(display_text);

    let attr_list = AttrList::new();
    let mut fg = AttrColor::new_foreground(65535, 65535, 65535);
    fg.set_start_index(0);
    fg.set_end_index(display_text.len() as u32);
    attr_list.insert(fg);
    layout.set_attributes(Some(&attr_list));

    let (tw, th) = layout.pixel_size();
    println!("Hindi text dimensions: tw={}, th={}", tw, th);
    assert!(tw > 0 && th > 0);

    let pos_x = (width - tw).max(0) as f64 / 2.0;
    let pos_y = (height - th).max(0) as f64 / 2.0;
    cr.move_to(pos_x, pos_y);
    pangocairo::functions::show_layout(&cr, &layout);
    drop(cr);

    surface.flush();
    let data = surface.data().unwrap();
    let non_zero = data.iter().filter(|&&b| b > 0).count();
    println!("Hindi text non-zero bytes: {}", non_zero);
    assert!(non_zero > 100);
}
