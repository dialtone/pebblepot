//! Pebblepot: a household's family board for earning and spending pebbles. See GLOSSARY.md.

pub mod board;
pub mod clock;
pub mod config;
pub mod model;
pub mod rules;
pub mod seed;
pub mod store;
pub mod web;

use std::sync::Arc;

use clock::Clock;
use config::Config;
use store::Store;
use tokio_util::sync::CancellationToken;

/// Opens the database (seeding it on first start) and builds the app.
/// Cancelling `shutdown` ends every open live-update stream.
pub async fn build_app(
    config: &Config,
    clock: Arc<dyn Clock>,
    shutdown: CancellationToken,
) -> store::Result<axum::Router> {
    let store = Store::open(&config.data_dir).await?;
    seed::seed_if_empty(&store).await?;
    Ok(web::router(web::AppState {
        store,
        clock,
        time_zone: config.time_zone.clone(),
        version: tokio::sync::watch::Sender::new(0),
        shutdown,
    }))
}
