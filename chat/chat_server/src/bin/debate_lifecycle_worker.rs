use std::time::Duration;

use anyhow::Result;
use chat_server::{AppConfig, AppState};
use tokio::time::sleep;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load()?;
    if !config.worker_runtime.debate_lifecycle_worker_enabled {
        info!("debate_lifecycle_worker is disabled by config, exiting");
        return Ok(());
    }

    let interval_secs = config.worker_runtime.debate_lifecycle_interval_secs.max(1);
    let batch_size = config.worker_runtime.debate_lifecycle_batch_size.max(1);

    let state = AppState::try_new_for_standalone_worker(config).await?;
    info!(interval_secs, batch_size, "debate_lifecycle_worker started");

    loop {
        if let Err(err) = state.advance_debate_sessions(batch_size).await {
            warn!("debate_lifecycle_worker tick failed: {}", err);
        }
        sleep(Duration::from_secs(interval_secs)).await;
    }
}
