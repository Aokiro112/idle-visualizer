use crate::app::settings::AppSettings;
use crate::app::state::AppState;
use crate::color::palette::LyricPalette;
use crate::display::{SleepInhibitor, VisualizerWindow};
use crate::lyrics::cache::LyricsCache;
use crate::lyrics::synchronizer::LyricSynchronizer;
use crate::media::MprisMonitor;
use crate::ui::tray::{TrayAction, VerciTray};
use crate::ui::SettingsDialog;
use gtk4::prelude::*;
use libadwaita::Application;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::info;

pub struct VerciApp {
    app: Application,
    runtime: Arc<Runtime>,
    app_state: AppState,
    mpris_monitor: MprisMonitor,
    synchronizer: Arc<LyricSynchronizer>,
    sleep_inhibitor: Arc<SleepInhibitor>,
}

impl VerciApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = Arc::new(Runtime::new()?);
        Self::new_with_runtime(runtime)
    }

    pub fn new_with_runtime(runtime: Arc<Runtime>) -> Result<Self, Box<dyn std::error::Error>> {
        let settings = AppSettings::load();
        let app_state = AppState::new(settings.clone());

        let mpris_monitor = MprisMonitor::new(settings.preferred_player.clone());
        let lyrics_cache = Arc::new(LyricsCache::new());
        let synchronizer = Arc::new(LyricSynchronizer::new(lyrics_cache));
        let sleep_inhibitor = Arc::new(SleepInhibitor::new());

        let app = Application::builder()
            .application_id("xyz.verci.linux")
            .build();

        Ok(Self {
            app,
            runtime,
            app_state,
            mpris_monitor,
            synchronizer,
            sleep_inhibitor,
        })
    }

    pub fn run(self) {
        let _runtime_guard = self.runtime.enter();
        let runtime = self.runtime.clone();
        let app_state = self.app_state.clone();
        let mpris = self.mpris_monitor;
        let synchronizer = self.synchronizer.clone();
        let sleep_inhibitor = self.sleep_inhibitor.clone();

        // Start background media monitor
        mpris.start();
        let media_handle = mpris.state_handle();

        // High precision media & lyric synchronization loop
        {
            let st = app_state.clone();
            let sync_ref = synchronizer.clone();
            let sleep_ref = sleep_inhibitor.clone();
            let media_ref = media_handle.clone();

            runtime.spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
                let mut last_art_url: Option<String> = None;

                loop {
                    interval.tick().await;

                    let media_state = {
                        let m = media_ref.read().await;
                        m.clone()
                    };

                    let (offset, prevent_sleep, auto_color) = {
                        let s = st.settings.read().await;
                        (
                            s.sync_offset_ms,
                            s.prevent_display_sleep,
                            s.auto_sync_palette,
                        )
                    };

                    // Update lyrics synchronizer against MPRIS current playback state
                    sync_ref.update_playback(&media_state, offset, false).await;

                    // Sync state into app_state
                    {
                        let active_sync = sync_ref.state_handle().read().await.clone();
                        *st.lyrics.write().await = active_sync.clone();
                        *st.media.write().await = media_state.clone();

                        if let Ok(mut engine) = st.rolling_engine.try_lock() {
                            engine.sync_with_playback(&active_sync, &media_state);
                        }
                    }

                    // Handle display sleep assertions
                    if prevent_sleep && media_state.playback_state.is_playing() {
                        sleep_ref.inhibit();
                    } else {
                        sleep_ref.un_inhibit();
                    }

                    // Handle album art palette auto-sync
                    if auto_color {
                        if let Some(ref meta) = media_state.metadata {
                            if meta.art_url != last_art_url {
                                last_art_url = meta.art_url.clone();
                                if let Some(ref url) = last_art_url {
                                    let url_clone = url.clone();
                                    let target_pal_ref = st.target_palette.clone();
                                    tokio::spawn(async move {
                                        if let Ok(pal) = fetch_and_extract_palette(&url_clone).await
                                        {
                                            *target_pal_ref.write().await = pal;
                                            info!("Updated auto-sync palette from album art");
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            });
        }

        // Setup GTK application lifecycle
        let app = self.app;
        app.connect_activate(move |application| {
            info!("Verci application activate signal triggered");

            let (tx, rx) = async_channel::unbounded::<TrayAction>();

            let vis_window = Rc::new(VisualizerWindow::new(
                application,
                app_state.clone(),
                Some(tx.clone()),
            ));
            let settings_dialog = Rc::new(RefCell::new(None::<SettingsDialog>));

            let win_ref = vis_window.clone();
            let settings_ref = settings_dialog.clone();
            let state_ref = app_state.clone();
            let app_ref = application.clone();

            glib::spawn_future_local(async move {
                while let Ok(action) = rx.recv().await {
                    match action {
                        TrayAction::ToggleFullscreen => win_ref.toggle_fullscreen(),
                        TrayAction::ToggleLyrics => win_ref.toggle_visibility(),
                        TrayAction::OpenSettings => {
                            let mut holder = settings_ref.borrow_mut();
                            if holder.is_none() {
                                *holder = Some(SettingsDialog::new(state_ref.clone()));
                            }
                            if let Some(ref dialog) = *holder {
                                dialog.present();
                            }
                        }
                        TrayAction::Quit => app_ref.quit(),
                    }
                }
            });

            let tray = VerciTray::new(app_state.clone(), tx);
            tray.start_service();

            // Present visualizer window
            vis_window.window().present();
        });

        app.run();
    }
}

async fn fetch_and_extract_palette(
    art_url: &str,
) -> Result<LyricPalette, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = if art_url.starts_with("file://") {
        let path = art_url.strip_prefix("file://").unwrap();
        tokio::fs::read(path).await?
    } else if art_url.starts_with("http://") || art_url.starts_with("https://") {
        let resp = reqwest::get(art_url).await?;
        resp.bytes().await?.to_vec()
    } else {
        tokio::fs::read(art_url).await?
    };

    let img = image::load_from_memory(&bytes)?;
    Ok(LyricPalette::extract_from_image(&img))
}
