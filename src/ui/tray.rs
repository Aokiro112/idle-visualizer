use crate::app::settings::VisualMode;
use crate::app::state::AppState;
use async_channel::Sender;
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem, SubMenu};
use ksni::{Tray, TrayMethods};
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    ToggleFullscreen,
    ToggleLyrics,
    OpenSettings,
    Quit,
}

pub struct VerciTray {
    app_state: AppState,
    action_sender: Sender<TrayAction>,
}

impl VerciTray {
    pub fn new(app_state: AppState, action_sender: Sender<TrayAction>) -> Self {
        Self {
            app_state,
            action_sender,
        }
    }

    pub fn start_service(self) {
        tokio::spawn(async move {
            if let Err(e) = TrayMethods::spawn(self).await {
                warn!("StatusNotifierItem tray registration failed (may not be supported by host compositor): {}", e);
            }
        });
    }
}

impl Tray for VerciTray {
    fn id(&self) -> String {
        "verci-lyrics".to_string()
    }

    fn title(&self) -> String {
        "Verci Lyric Visualizer".to_string()
    }

    fn icon_name(&self) -> String {
        "audio-x-generic".to_string()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let (title, artist, is_playing) = {
            if let Ok(m) = self.app_state.media.try_read() {
                let is_p = m.playback_state.is_playing();
                if let Some(ref meta) = m.metadata {
                    (meta.title.clone(), meta.artist.clone(), is_p)
                } else {
                    ("No music detected".to_string(), String::new(), false)
                }
            } else {
                ("Detecting music...".to_string(), String::new(), false)
            }
        };

        let current_mode = {
            if let Ok(s) = self.app_state.settings.try_read() {
                s.visual_mode
            } else {
                VisualMode::Ship
            }
        };

        let auto_color = {
            if let Ok(s) = self.app_state.settings.try_read() {
                s.auto_sync_palette
            } else {
                true
            }
        };

        let is_fullscreen = {
            if let Ok(s) = self.app_state.settings.try_read() {
                s.fullscreen
            } else {
                false
            }
        };

        let status_label = if is_playing {
            format!("▶ {} - {}", title, artist)
        } else if !title.is_empty() {
            format!("⏸ {} - {}", title, artist)
        } else {
            "No media playing".to_string()
        };

        let state_clone0 = self.app_state.clone();
        let state_clone1 = self.app_state.clone();
        let state_clone2 = self.app_state.clone();
        let state_clone3 = self.app_state.clone();
        let state_clone4 = self.app_state.clone();
        let state_clone_color = self.app_state.clone();

        let tx_fs = self.action_sender.clone();
        let tx_lyrics = self.action_sender.clone();
        let tx_settings = self.action_sender.clone();
        let tx_quit = self.action_sender.clone();

        vec![
            // Header: track status
            StandardItem {
                label: status_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Playback Controls
            StandardItem {
                label: if is_playing {
                    "Pause".to_string()
                } else {
                    "Play".to_string()
                },
                activate: Box::new(|_| {
                    tokio::spawn(async {
                        let _ = crate::media::MprisMonitor::send_play_pause(None).await;
                    });
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Next Track".to_string(),
                activate: Box::new(|_| {
                    tokio::spawn(async {
                        let _ = crate::media::MprisMonitor::send_next(None).await;
                    });
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Previous Track".to_string(),
                activate: Box::new(|_| {
                    tokio::spawn(async {
                        let _ = crate::media::MprisMonitor::send_previous(None).await;
                    });
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Visualizer Mode SubMenu
            SubMenu {
                label: "Visualizer Mode".to_string(),
                submenu: vec![
                    CheckmarkItem {
                        label: "Dynamic Verci (Expressive Typography)".to_string(),
                        checked: current_mode == VisualMode::Dynamic,
                        activate: Box::new(move |_| {
                            if let Ok(mut s) = state_clone0.settings.try_write() {
                                s.visual_mode = VisualMode::Dynamic;
                                let _ = s.save();
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                    CheckmarkItem {
                        label: "Ship (3D Spatial Wall)".to_string(),
                        checked: current_mode == VisualMode::Ship,
                        activate: Box::new(move |_| {
                            if let Ok(mut s) = state_clone1.settings.try_write() {
                                s.visual_mode = VisualMode::Ship;
                                let _ = s.save();
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                    CheckmarkItem {
                        label: "Fisheye (Curved Lens)".to_string(),
                        checked: current_mode == VisualMode::Fisheye,
                        activate: Box::new(move |_| {
                            if let Ok(mut s) = state_clone2.settings.try_write() {
                                s.visual_mode = VisualMode::Fisheye;
                                let _ = s.save();
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                    CheckmarkItem {
                        label: "Fisheye Visual (Icons + Curved)".to_string(),
                        checked: current_mode == VisualMode::FisheyeVisual,
                        activate: Box::new(move |_| {
                            if let Ok(mut s) = state_clone3.settings.try_write() {
                                s.visual_mode = VisualMode::FisheyeVisual;
                                let _ = s.save();
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                    CheckmarkItem {
                        label: "Visual (Dynamic Animated Icons)".to_string(),
                        checked: current_mode == VisualMode::Visual,
                        activate: Box::new(move |_| {
                            if let Ok(mut s) = state_clone4.settings.try_write() {
                                s.visual_mode = VisualMode::Visual;
                                let _ = s.save();
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            // Auto Color Palette
            CheckmarkItem {
                label: "Auto Color Palette (From Album Art)".to_string(),
                checked: auto_color,
                activate: Box::new(move |_| {
                    if let Ok(mut s) = state_clone_color.settings.try_write() {
                        s.auto_sync_palette = !s.auto_sync_palette;
                        let _ = s.save();
                    }
                }),
                ..Default::default()
            }
            .into(),
            // Fullscreen Toggle
            CheckmarkItem {
                label: "Fullscreen".to_string(),
                checked: is_fullscreen,
                activate: Box::new(move |_| {
                    let _ = tx_fs.send_blocking(TrayAction::ToggleFullscreen);
                }),
                ..Default::default()
            }
            .into(),
            // Show/Hide Lyrics
            StandardItem {
                label: "Toggle Lyrics Overlay".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_lyrics.send_blocking(TrayAction::ToggleLyrics);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Settings Window
            StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_settings.send_blocking(TrayAction::OpenSettings);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Quit
            StandardItem {
                label: "Quit Verci".to_string(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send_blocking(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
