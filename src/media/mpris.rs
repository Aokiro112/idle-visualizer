use crate::media::player::{PlaybackState, TrackMetadata, TrackState};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use zbus::proxy;
use zbus::zvariant::Value;
use zbus::Connection;

#[proxy(
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    interface = "org.freedesktop.DBus"
)]
trait DBusDaemon {
    async fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer2Player {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, Value<'_>>>;

    fn play_pause(&self) -> zbus::Result<()>;
    fn play(&self) -> zbus::Result<()>;
    fn pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
    fn seek(&self, offset: i64) -> zbus::Result<()>;
}

pub struct MprisMonitor {
    current_state: Arc<RwLock<TrackState>>,
    preferred_player: Arc<RwLock<Option<String>>>,
}

impl MprisMonitor {
    pub fn new(preferred_player: Option<String>) -> Self {
        Self {
            current_state: Arc::new(RwLock::new(TrackState::default())),
            preferred_player: Arc::new(RwLock::new(preferred_player)),
        }
    }

    pub fn state_handle(&self) -> Arc<RwLock<TrackState>> {
        self.current_state.clone()
    }

    pub async fn set_preferred_player(&self, name: Option<String>) {
        *self.preferred_player.write().await = name;
    }

    pub async fn send_play_pause(
        preferred_player: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::session().await?;
        let service = if let Some(p) = preferred_player {
            if p.starts_with("org.mpris.MediaPlayer2.") {
                p.to_string()
            } else {
                format!("org.mpris.MediaPlayer2.{}", p)
            }
        } else {
            "org.mpris.MediaPlayer2.ytmdesktop".to_string()
        };
        let proxy = MediaPlayer2PlayerProxy::builder(&conn)
            .destination(service.as_str())?
            .build()
            .await?;
        proxy.play_pause().await?;
        Ok(())
    }

    pub async fn send_next(
        preferred_player: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::session().await?;
        let service = if let Some(p) = preferred_player {
            if p.starts_with("org.mpris.MediaPlayer2.") {
                p.to_string()
            } else {
                format!("org.mpris.MediaPlayer2.{}", p)
            }
        } else {
            "org.mpris.MediaPlayer2.ytmdesktop".to_string()
        };
        let proxy = MediaPlayer2PlayerProxy::builder(&conn)
            .destination(service.as_str())?
            .build()
            .await?;
        proxy.next().await?;
        Ok(())
    }

    pub async fn send_previous(
        preferred_player: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::session().await?;
        let service = if let Some(p) = preferred_player {
            if p.starts_with("org.mpris.MediaPlayer2.") {
                p.to_string()
            } else {
                format!("org.mpris.MediaPlayer2.{}", p)
            }
        } else {
            "org.mpris.MediaPlayer2.ytmdesktop".to_string()
        };
        let proxy = MediaPlayer2PlayerProxy::builder(&conn)
            .destination(service.as_str())?
            .build()
            .await?;
        proxy.previous().await?;
        Ok(())
    }

    pub fn ensure_youtube_music_running() {
        let ytm_path = std::path::Path::new("/opt/ytmdesktop/youtube-music-desktop-app");
        if !ytm_path.exists() {
            return;
        }

        let is_running = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("youtube-music-desktop-app")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);

        if !is_running {
            let _ = std::process::Command::new(ytm_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }

    /// Spawns the background MPRIS polling and event detection loop
    pub fn start(&self) {
        Self::ensure_youtube_music_running();

        let state_clone = self.current_state.clone();
        let pref_clone = self.preferred_player.clone();

        tokio::spawn(async move {
            info!("Starting MPRIS media monitor loop...");
            let connection = match Connection::session().await {
                Ok(conn) => conn,
                Err(err) => {
                    error!("Failed to connect to D-Bus session: {}", err);
                    return;
                }
            };

            let dbus_daemon = match DBusDaemonProxy::new(&connection).await {
                Ok(proxy) => proxy,
                Err(err) => {
                    error!("Failed to create DBus daemon proxy: {}", err);
                    return;
                }
            };

            let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
            let mut last_reported_pos: f64 = -1.0;

            loop {
                interval.tick().await;

                // Discover all MPRIS player services currently on D-Bus
                let names = match dbus_daemon.list_names().await {
                    Ok(names) => names,
                    Err(e) => {
                        debug!("Error listing D-Bus names: {}", e);
                        continue;
                    }
                };

                let mpris_players: Vec<String> = names
                    .into_iter()
                    .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
                    .collect();

                if mpris_players.is_empty() {
                    let mut lock = state_clone.write().await;
                    if lock.playback_state != PlaybackState::Stopped {
                        lock.playback_state = PlaybackState::Stopped;
                        lock.metadata = None;
                        last_reported_pos = -1.0;
                    }
                    continue;
                }

                // Choose player based on preferred player setting, or prioritize active 'Playing' player
                let preferred = pref_clone.read().await.clone();
                let mut chosen_player_info: Option<(String, TrackMetadata, PlaybackState, f64)> =
                    None;

                let mut sorted_players = mpris_players.clone();
                sorted_players.sort_by(|a, b| {
                    let a_pref = preferred.as_ref().map(|p| a.contains(p)).unwrap_or(false);
                    let b_pref = preferred.as_ref().map(|p| b.contains(p)).unwrap_or(false);
                    if a_pref != b_pref {
                        return b_pref.cmp(&a_pref);
                    }
                    // Prioritize YouTube Music Desktop
                    let a_ytm = a.contains("ytmdesktop") || a.contains("youtube");
                    let b_ytm = b.contains("ytmdesktop") || b.contains("youtube");
                    if a_ytm != b_ytm {
                        return b_ytm.cmp(&a_ytm);
                    }
                    // Demote playerctld proxy
                    let a_proxy = a.ends_with(".playerctld");
                    let b_proxy = b.ends_with(".playerctld");
                    if a_proxy != b_proxy {
                        return a_proxy.cmp(&b_proxy);
                    }
                    a.cmp(b)
                });

                for player_service in &sorted_players {
                    let short_name = player_service
                        .strip_prefix("org.mpris.MediaPlayer2.")
                        .unwrap_or(player_service)
                        .to_string();

                    if let Ok(player_proxy) = MediaPlayer2PlayerProxy::builder(&connection)
                        .destination(player_service.as_str())
                        .unwrap()
                        .build()
                        .await
                    {
                        let status_str = player_proxy.playback_status().await.unwrap_or_default();
                        let state = match status_str.as_str() {
                            "Playing" => PlaybackState::Playing,
                            "Paused" => PlaybackState::Paused,
                            _ => PlaybackState::Stopped,
                        };

                        let pos_us = player_proxy.position().await.unwrap_or(0);
                        let pos_sec = (pos_us as f64) / 1_000_000.0;

                        if let Ok(meta_map) = player_proxy.metadata().await {
                            let title = meta_map
                                .get("xesam:title")
                                .and_then(|v| match v {
                                    Value::Str(s) => Some(s.to_string()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let artist = meta_map
                                .get("xesam:artist")
                                .and_then(|v| match v {
                                    Value::Array(arr) => {
                                        let items: Vec<String> = arr
                                            .iter()
                                            .filter_map(|val| match val {
                                                Value::Str(s) => Some(s.to_string()),
                                                _ => None,
                                            })
                                            .collect();
                                        if items.is_empty() {
                                            None
                                        } else {
                                            Some(items.join(", "))
                                        }
                                    }
                                    Value::Str(s) => Some(s.to_string()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let album = meta_map.get("xesam:album").and_then(|v| match v {
                                Value::Str(s) => Some(s.to_string()),
                                _ => None,
                            });

                            let art_url = meta_map.get("mpris:artUrl").and_then(|v| match v {
                                Value::Str(s) => Some(s.to_string()),
                                _ => None,
                            });

                            let duration_seconds =
                                meta_map.get("mpris:length").and_then(|v| match v {
                                    Value::I64(l) => Some((*l as f64) / 1_000_000.0),
                                    Value::U64(l) => Some((*l as f64) / 1_000_000.0),
                                    _ => None,
                                });

                            if !title.is_empty() {
                                let meta = TrackMetadata {
                                    title,
                                    artist,
                                    album,
                                    art_url,
                                    duration_seconds,
                                };

                                let is_preferred = preferred
                                    .as_ref()
                                    .map(|p| {
                                        short_name.eq_ignore_ascii_case(p)
                                            || short_name
                                                .to_lowercase()
                                                .contains(&p.to_lowercase())
                                    })
                                    .unwrap_or(false);

                                let candidate = (short_name, meta, state, pos_sec);

                                if is_preferred || state == PlaybackState::Playing {
                                    chosen_player_info = Some(candidate);
                                    break;
                                } else if chosen_player_info.is_none() {
                                    chosen_player_info = Some(candidate);
                                }
                            }
                        }
                    }
                }

                let mut lock = state_clone.write().await;
                if let Some((player_name, meta, state, pos_sec)) = chosen_player_info {
                    let current_est = lock.current_position();
                    let title_changed = lock
                        .metadata
                        .as_ref()
                        .map(|m| m.title != meta.title || m.artist != meta.artist)
                        .unwrap_or(true);
                    let state_changed = lock.playback_state != state;

                    // Only evaluate seek if the player actually reported a changed position value
                    // Browsers (like Firefox on YouTube) frequently report constant 0 or microsecond noise
                    let pos_changed = (pos_sec - last_reported_pos).abs() > 0.05;
                    let pos_diff = (pos_sec - current_est).abs();
                    let seek_detected = pos_changed && pos_diff > 0.45;

                    if title_changed {
                        info!(
                            "Playback track changed: player='{}', title='{}', artist='{}', pos={:.2}s",
                            player_name, meta.title, meta.artist, pos_sec
                        );
                        lock.player_name = player_name;
                        lock.metadata = Some(meta);
                        lock.playback_state = state;
                        lock.base_position = if pos_sec > 0.05 { pos_sec } else { 0.0 };
                        lock.base_instant = Instant::now();
                        last_reported_pos = pos_sec;
                    } else if state_changed {
                        info!(
                            "Playback state changed from {:?} to {:?}",
                            lock.playback_state, state
                        );
                        if state == PlaybackState::Paused {
                            // Preserve monotonically estimated position if player reports 0
                            lock.base_position = if pos_sec > 0.05 { pos_sec } else { current_est };
                            lock.base_instant = Instant::now();
                        } else if state == PlaybackState::Playing {
                            // When resuming, if player reported a valid seek while paused, update base_position
                            if pos_sec > 0.05 && (pos_sec - lock.base_position).abs() > 1.0 {
                                lock.base_position = pos_sec;
                            }
                            lock.base_instant = Instant::now();
                        }
                        lock.playback_state = state;
                        last_reported_pos = pos_sec;
                    } else if seek_detected {
                        debug!(
                            "Seek detected: jumping from estimated {:.2}s to reported {:.2}s",
                            current_est, pos_sec
                        );
                        lock.base_position = pos_sec;
                        lock.base_instant = Instant::now();
                        last_reported_pos = pos_sec;
                    } else if pos_changed && pos_diff <= 1.5 && pos_sec > 0.05 {
                        // Minor periodic drift sync from reliable media players (Spotify, mpv, etc.)
                        last_reported_pos = pos_sec;
                    }
                } else if lock.playback_state != PlaybackState::Stopped {
                    lock.playback_state = PlaybackState::Stopped;
                    last_reported_pos = -1.0;
                }
            }
        });
    }
}
