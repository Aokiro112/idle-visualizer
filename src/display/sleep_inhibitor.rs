use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, info};
use zbus::proxy;
use zbus::Connection;

#[proxy(
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver",
    interface = "org.freedesktop.ScreenSaver"
)]
trait ScreenSaverInhibitor {
    async fn inhibit(&self, application_name: &str, reason_for_inhibit: &str) -> zbus::Result<u32>;
    async fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;
}

pub struct SleepInhibitor {
    cookie: Arc<AtomicU32>,
}

impl SleepInhibitor {
    pub fn new() -> Self {
        Self {
            cookie: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn inhibit(&self) {
        let cookie_arc = self.cookie.clone();
        if cookie_arc.load(Ordering::SeqCst) != 0 {
            return; // Already inhibiting
        }

        tokio::spawn(async move {
            if let Ok(conn) = Connection::session().await {
                if let Ok(proxy) = ScreenSaverInhibitorProxy::new(&conn).await {
                    match proxy
                        .inhibit("Verci", "Synchronized music lyric visualizer is active")
                        .await
                    {
                        Ok(c) => {
                            info!("Display sleep successfully inhibited (cookie: {})", c);
                            cookie_arc.store(c, Ordering::SeqCst);
                        }
                        Err(e) => {
                            debug!("Could not inhibit screen sleep via D-Bus: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn un_inhibit(&self) {
        let cookie_arc = self.cookie.clone();
        let cookie = cookie_arc.swap(0, Ordering::SeqCst);
        if cookie == 0 {
            return;
        }

        tokio::spawn(async move {
            if let Ok(conn) = Connection::session().await {
                if let Ok(proxy) = ScreenSaverInhibitorProxy::new(&conn).await {
                    let _ = proxy.un_inhibit(cookie).await;
                    info!("Released display sleep inhibitor (cookie: {})", cookie);
                }
            }
        });
    }
}

impl Default for SleepInhibitor {
    fn default() -> Self {
        Self::new()
    }
}
