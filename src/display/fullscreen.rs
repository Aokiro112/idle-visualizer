use crate::app::state::AppState;
use crate::display::layer_shell::LayerShellHelper;
use crate::renderer::LyricRenderer;
use crate::ui::tray::TrayAction;
use async_channel::Sender;
use gtk4::gdk::Key;
use gtk4::prelude::*;
use gtk4::{Button, EventControllerKey, GLArea, GestureClick, Overlay, Window};
use libadwaita::Application;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::info;

pub struct VisualizerWindow {
    window: Window,
    pub gl_area: GLArea,
    is_layer_shell: bool,
}

impl VisualizerWindow {
    pub fn new(
        app: &Application,
        app_state: AppState,
        tray_action_tx: Option<Sender<TrayAction>>,
    ) -> Self {
        let (transparent, click_thru, fs) = {
            let s = app_state.settings.blocking_read();
            (s.transparent_background, s.click_through, s.fullscreen)
        };

        // Inject GTK CSS provider to enforce pitch-black #000000 background
        // and style the floating bottom-left pause pill button matching the reference video
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_string(
            "window.verci-visualizer, window.verci-visualizer glarea, .verci-visualizer { \
                background-color: #000000; \
                background-image: none; \
                box-shadow: none; \
                border-style: none; \
                outline-style: none; \
            } \
            .verci-pill-button { \
                background-color: rgba(20, 20, 20, 0.78); \
                color: #e6e6e6; \
                border: 1px solid rgba(255, 255, 255, 0.16); \
                border-radius: 9999px; \
                padding: 7px 18px; \
                font-size: 13px; \
                font-weight: 600; \
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; \
                letter-spacing: -0.01em; \
                box-shadow: 0 4px 14px rgba(0, 0, 0, 0.45); \
                transition: background-color 0.15s ease, border-color 0.15s ease, transform 0.1s ease; \
            } \
            .verci-pill-button:hover { \
                background-color: rgba(38, 38, 38, 0.90); \
                border-color: rgba(255, 255, 255, 0.30); \
                color: #ffffff; \
            } \
            .verci-pill-button:active { \
                background-color: rgba(12, 12, 12, 0.95); \
                transform: scale(0.96); \
            }",
        );
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        let window = Window::builder()
            .application(app)
            .title("Verci")
            .default_width(1280)
            .default_height(720)
            .decorated(true)
            .build();

        window.add_css_class("verci-visualizer");

        // Initialize Wayland Layer Shell if explicitly requested in settings
        let is_layer_shell = transparent && LayerShellHelper::is_supported();
        if is_layer_shell {
            LayerShellHelper::setup_overlay(&window, gtk4_layer_shell::Layer::Overlay);
            info!("Visualizer running as Wayland layer-shell overlay surface");
        } else if fs {
            window.fullscreen();
        }

        let gl_area = GLArea::builder()
            .has_depth_buffer(true)
            .has_stencil_buffer(false)
            .auto_render(true)
            .build();

        gl_area.add_css_class("verci-visualizer");

        let renderer = Rc::new(RefCell::new(LyricRenderer::new()));
        let state_rc = Rc::new(app_state.clone());

        // Connect GLArea render signal
        {
            let rend_ref = renderer.clone();
            let st_ref = state_rc.clone();
            gl_area.connect_render(move |gl_area, _| {
                let width = gl_area.width();
                let height = gl_area.height();
                rend_ref.borrow_mut().render(&st_ref, width, height);
                glib::Propagation::Stop
            });
        }

        // Overlay containing the OpenGL canvas and the floating bottom-left pause pill button
        let overlay = Overlay::new();
        overlay.set_child(Some(&gl_area));

        let pause_button = Button::builder()
            .label("❚❚  Pause")
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::End)
            .margin_start(24)
            .margin_bottom(24)
            .css_classes(["verci-pill-button"])
            .build();
        pause_button.set_cursor_from_name(Some("pointer"));

        // Connect button click to toggle playback in RollingLyricEngine
        {
            let st_click = state_rc.clone();
            let btn_ref = pause_button.clone();
            pause_button.connect_clicked(move |_| {
                if let Ok(mut engine) = st_click.rolling_engine.lock() {
                    let is_playing = engine.toggle_play_pause();
                    if is_playing {
                        btn_ref.set_label("❚❚  Pause");
                    } else {
                        btn_ref.set_label("▶  Play");
                    }
                }
                tokio::spawn(async {
                    let _ = crate::media::MprisMonitor::send_play_pause(Some("ytmdesktop")).await;
                });
            });
        }

        overlay.add_overlay(&pause_button);

        // Add tick callback to guarantee vsync rendering (60Hz / 120Hz ProMotion) and sync button state
        {
            let area_weak = gl_area.downgrade();
            let btn_weak = pause_button.downgrade();
            let st_ref2 = state_rc.clone();
            let last_play_state = std::cell::Cell::new(true);

            gl_area.add_tick_callback(move |_, _| {
                if let Some(area) = area_weak.upgrade() {
                    area.queue_render();
                }
                if let Some(btn) = btn_weak.upgrade() {
                    if let Ok(engine) = st_ref2.rolling_engine.try_lock() {
                        let playing = engine.is_playing;
                        if last_play_state.get() != playing {
                            last_play_state.set(playing);
                            if playing {
                                btn.set_label("❚❚  Pause");
                            } else {
                                btn.set_label("▶  Play");
                            }
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Input region handling for click-through mode
        if click_thru {
            window.connect_realize(|win| {
                if let Some(surface) = win.surface() {
                    let empty_region = cairo::Region::create();
                    surface.set_input_region(Some(&empty_region));
                    info!("Transparent visualizer configured for input pass-through");
                }
            });
        }

        // Mouse gestures: Right-click opens settings, Double-click toggles fullscreen
        if let Some(tx) = tray_action_tx {
            let gesture = GestureClick::new();
            let tx_clone = tx.clone();
            let win_weak = window.downgrade();

            gesture.connect_pressed(move |g, n_press, _x, _y| {
                let button = g.current_button();
                if button == gtk4::gdk::BUTTON_SECONDARY {
                    // Right-click: Open Settings
                    let _ = tx_clone.send_blocking(TrayAction::OpenSettings);
                } else if button == gtk4::gdk::BUTTON_PRIMARY && n_press == 2 {
                    // Double-click: Toggle Fullscreen
                    if let Some(win) = win_weak.upgrade() {
                        if win.is_fullscreen() {
                            win.unfullscreen();
                        } else {
                            win.fullscreen();
                        }
                    }
                }
            });
            window.add_controller(gesture);
        }

        // Key bindings: Space to Play/Pause, Escape to close, F11 for fullscreen
        {
            let win_weak = window.downgrade();
            let btn_key = pause_button.downgrade();
            let st_key = state_rc.clone();
            let key_controller = EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                match keyval {
                    Key::space => {
                        if let Ok(mut engine) = st_key.rolling_engine.lock() {
                            let is_playing = engine.toggle_play_pause();
                            if let Some(btn) = btn_key.upgrade() {
                                if is_playing {
                                    btn.set_label("❚❚  Pause");
                                } else {
                                    btn.set_label("▶  Play");
                                }
                            }
                        }
                        tokio::spawn(async {
                            let _ = crate::media::MprisMonitor::send_play_pause(Some("ytmdesktop")).await;
                        });
                        glib::Propagation::Stop
                    }
                    Key::Escape => {
                        if let Some(win) = win_weak.upgrade() {
                            if win.is_fullscreen() {
                                win.unfullscreen();
                            } else {
                                win.close();
                            }
                        }
                        glib::Propagation::Stop
                    }
                    Key::F11 => {
                        if let Some(win) = win_weak.upgrade() {
                            if win.is_fullscreen() {
                                win.unfullscreen();
                            } else {
                                win.fullscreen();
                            }
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            window.add_controller(key_controller);
        }

        window.set_child(Some(&overlay));

        Self {
            window,
            gl_area,
            is_layer_shell,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn toggle_fullscreen(&self) {
        if self.is_layer_shell {
            // Already full-coverage on layer shell
            return;
        }
        if self.window.is_fullscreen() {
            self.window.unfullscreen();
        } else {
            self.window.fullscreen();
        }
    }

    pub fn toggle_visibility(&self) {
        let is_vis = self.window.is_visible();
        self.window.set_visible(!is_vis);
        if !is_vis {
            self.window.present();
        }
    }
}
