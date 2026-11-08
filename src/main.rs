use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use verci::app::VerciApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "verci=warn,error".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Verci Linux Lyric Visualizer...");

    // Initialize Tokio runtime and enter guard so all background tasks have an active reactor
    let runtime = Arc::new(Runtime::new()?);
    let _runtime_guard = runtime.enter();

    libadwaita::init()?;

    let application = VerciApp::new_with_runtime(runtime.clone())?;
    application.run();

    Ok(())
}
