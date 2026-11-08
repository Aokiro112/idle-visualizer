use crate::app::settings::VisualMode;
use crate::app::state::AppState;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use std::rc::Rc;

pub struct SettingsDialog {
    window: PreferencesWindow,
}

impl SettingsDialog {
    pub fn new(app_state: AppState) -> Self {
        let window = PreferencesWindow::builder()
            .title("Verci Settings")
            .default_width(620)
            .default_height(600)
            .modal(false)
            .build();

        let state_rc = Rc::new(app_state);

        // ---------------- PAGE 1: Visualizer & Typography ----------------
        let vis_page = PreferencesPage::builder()
            .title("Visualizer")
            .icon_name("video-display-symbolic")
            .build();

        let vis_group = PreferencesGroup::builder()
            .title("Display Mode & Style")
            .description("Configure the 3D and shader visual experience")
            .build();

        // Mode Combo Row
        let mode_model = gtk4::StringList::new(&[
            "Dynamic Verci (Expressive Typography)",
            "Ship (3D Spatial Wall)",
            "Fisheye (Curved Lens Shader)",
            "Fisheye Visual (Curved + Icons)",
            "Visual (Dynamic Animated Icons)",
        ]);
        let mode_row = ComboRow::builder()
            .title("Rendering Mode")
            .subtitle("Select your lyric visualizer engine")
            .model(&mode_model)
            .build();

        {
            let current_mode = state_rc.settings.blocking_read().visual_mode;
            mode_row.set_selected(match current_mode {
                VisualMode::Dynamic => 0,
                VisualMode::Ship => 1,
                VisualMode::Fisheye => 2,
                VisualMode::FisheyeVisual => 3,
                VisualMode::Visual => 4,
            });

            let st = state_rc.clone();
            mode_row.connect_selected_notify(move |row| {
                let mut settings = st.settings.blocking_write();
                settings.visual_mode = match row.selected() {
                    0 => VisualMode::Dynamic,
                    1 => VisualMode::Ship,
                    2 => VisualMode::Fisheye,
                    3 => VisualMode::FisheyeVisual,
                    4 => VisualMode::Visual,
                    _ => VisualMode::Dynamic,
                };
                let _ = settings.save();
            });
        }
        vis_group.add(&mode_row);

        // Transparent Background Switch
        let trans_row = ActionRow::builder()
            .title("Transparent Background")
            .subtitle("Desktop overlay mode (empty space reveals wallpaper/windows)")
            .build();
        let trans_switch = gtk4::Switch::new();
        {
            let active = state_rc.settings.blocking_read().transparent_background;
            trans_switch.set_active(active);
            let st = state_rc.clone();
            trans_switch.connect_active_notify(move |sw| {
                let mut settings = st.settings.blocking_write();
                settings.transparent_background = sw.is_active();
                let _ = settings.save();
            });
        }
        trans_row.add_suffix(&trans_switch);
        trans_row.set_activatable_widget(Some(&trans_switch));
        vis_group.add(&trans_row);

        // Multi-line Context Switch
        let ctx_row = ActionRow::builder()
            .title("Lyric Context Lines")
            .subtitle("Display previous and upcoming lines with subtle preview alpha")
            .build();
        let ctx_switch = gtk4::Switch::new();
        {
            let active = state_rc.settings.blocking_read().multi_line_context;
            ctx_switch.set_active(active);
            let st = state_rc.clone();
            ctx_switch.connect_active_notify(move |sw| {
                let mut settings = st.settings.blocking_write();
                settings.multi_line_context = sw.is_active();
                let _ = settings.save();
            });
        }
        ctx_row.add_suffix(&ctx_switch);
        ctx_row.set_activatable_widget(Some(&ctx_switch));
        vis_group.add(&ctx_row);

        // Animation Intensity Slider
        let intensity_row = ActionRow::builder()
            .title("Animation Intensity")
            .subtitle("Dynamic depth and distortion curvature")
            .build();
        let intensity_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.2, 2.0, 0.1);
        intensity_scale.set_width_request(180);
        {
            let val = state_rc.settings.blocking_read().animation_intensity as f64;
            intensity_scale.set_value(val);
            let st = state_rc.clone();
            intensity_scale.connect_value_changed(move |scale| {
                let mut settings = st.settings.blocking_write();
                settings.animation_intensity = scale.value() as f32;
                let _ = settings.save();
            });
        }
        intensity_row.add_suffix(&intensity_scale);
        vis_group.add(&intensity_row);

        // Motion Speed Slider
        let speed_row = ActionRow::builder()
            .title("Camera Drift Speed")
            .subtitle("Subtle 3D movement in Ship mode")
            .build();
        let speed_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.2, 3.0, 0.1);
        speed_scale.set_width_request(180);
        {
            let val = state_rc.settings.blocking_read().motion_speed as f64;
            speed_scale.set_value(val);
            let st = state_rc.clone();
            speed_scale.connect_value_changed(move |scale| {
                let mut settings = st.settings.blocking_write();
                settings.motion_speed = scale.value() as f32;
                let _ = settings.save();
            });
        }
        speed_row.add_suffix(&speed_scale);
        vis_group.add(&speed_row);

        // Font Size Slider
        let font_row = ActionRow::builder()
            .title("Font Size")
            .subtitle("Base typographic scale for lyric words")
            .build();
        let font_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 20.0, 64.0, 2.0);
        font_scale.set_width_request(180);
        {
            let val = state_rc.settings.blocking_read().font_size as f64;
            font_scale.set_value(val);
            let st = state_rc.clone();
            font_scale.connect_value_changed(move |scale| {
                let mut settings = st.settings.blocking_write();
                settings.font_size = scale.value() as f32;
                let _ = settings.save();
            });
        }
        font_row.add_suffix(&font_scale);
        vis_group.add(&font_row);

        vis_page.add(&vis_group);
        window.add(&vis_page);

        // ---------------- PAGE 2: Colors & Palette ----------------
        let col_page = PreferencesPage::builder()
            .title("Colors")
            .icon_name("applications-graphics-symbolic")
            .build();

        let col_group = PreferencesGroup::builder()
            .title("Color Generation & Styling")
            .description("Customize manual or automatic album art palettes")
            .build();

        // Auto Sync Palette Switch
        let auto_sync_row = ActionRow::builder()
            .title("Auto Sync Palette")
            .subtitle("Extract harmonic readable colors directly from album artwork")
            .build();
        let auto_sync_switch = gtk4::Switch::new();
        {
            let active = state_rc.settings.blocking_read().auto_sync_palette;
            auto_sync_switch.set_active(active);
            let st = state_rc.clone();
            auto_sync_switch.connect_active_notify(move |sw| {
                let mut settings = st.settings.blocking_write();
                settings.auto_sync_palette = sw.is_active();
                let _ = settings.save();
            });
        }
        auto_sync_row.add_suffix(&auto_sync_switch);
        auto_sync_row.set_activatable_widget(Some(&auto_sync_switch));
        col_group.add(&auto_sync_row);

        col_page.add(&col_group);
        window.add(&col_page);

        // ---------------- PAGE 3: Playback & Display ----------------
        let play_page = PreferencesPage::builder()
            .title("Playback & Display")
            .icon_name("media-playback-start-symbolic")
            .build();

        let play_group = PreferencesGroup::builder()
            .title("Synchronization & Power")
            .description("Manage MPRIS timing precision and power assertions")
            .build();

        // Click-through Switch
        let click_row = ActionRow::builder()
            .title("Click-Through Mode")
            .subtitle("Mouse clicks pass straight through to background applications")
            .build();
        let click_switch = gtk4::Switch::new();
        {
            let active = state_rc.settings.blocking_read().click_through;
            click_switch.set_active(active);
            let st = state_rc.clone();
            click_switch.connect_active_notify(move |sw| {
                let mut settings = st.settings.blocking_write();
                settings.click_through = sw.is_active();
                let _ = settings.save();
            });
        }
        click_row.add_suffix(&click_switch);
        click_row.set_activatable_widget(Some(&click_switch));
        play_group.add(&click_row);

        // Sync Offset Slider
        let offset_row = ActionRow::builder()
            .title("Lyric Offset (ms)")
            .subtitle("Fine-tune timing (+/- 500ms)")
            .build();
        let offset_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -500.0, 500.0, 25.0);
        offset_scale.set_width_request(180);
        {
            let val = state_rc.settings.blocking_read().sync_offset_ms as f64;
            offset_scale.set_value(val);
            let st = state_rc.clone();
            offset_scale.connect_value_changed(move |scale| {
                let mut settings = st.settings.blocking_write();
                settings.sync_offset_ms = scale.value() as i64;
                let _ = settings.save();
            });
        }
        offset_row.add_suffix(&offset_scale);
        play_group.add(&offset_row);

        // Prevent Display Sleep
        let sleep_row = ActionRow::builder()
            .title("Prevent Display Sleep")
            .subtitle("Hold display awake via D-Bus while music is playing")
            .build();
        let sleep_switch = gtk4::Switch::new();
        {
            let active = state_rc.settings.blocking_read().prevent_display_sleep;
            sleep_switch.set_active(active);
            let st = state_rc.clone();
            sleep_switch.connect_active_notify(move |sw| {
                let mut settings = st.settings.blocking_write();
                settings.prevent_display_sleep = sw.is_active();
                let _ = settings.save();
            });
        }
        sleep_row.add_suffix(&sleep_switch);
        sleep_row.set_activatable_widget(Some(&sleep_switch));
        play_group.add(&sleep_row);

        play_page.add(&play_group);
        window.add(&play_page);

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
