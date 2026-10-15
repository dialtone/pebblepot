use std::process::ExitCode;
use std::sync::Arc;

use pebblepot::clock::SystemClock;
use pebblepot::config::Config;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pebblepot: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> pebblepot::store::Result<()> {
    let config = Config::from_env()?;
    // One token for shutdown: SIGTERM/Ctrl-C cancels it, which stops the server and ends every
    // open event stream (axum would otherwise wait for them forever).
    let shutdown = CancellationToken::new();
    let app = pebblepot::build_app(&config, Arc::new(SystemClock), shutdown.clone()).await?;
    let listener = tokio::net::TcpListener::bind((config.listen_addr, config.port)).await?;
    eprintln!(
        "Pebblepot on http://{} (data in {}, time zone {})",
        listener.local_addr()?,
        config.data_dir.display(),
        config.time_zone.iana_name().unwrap_or("system"),
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(cancel_on_signal(shutdown))
        .await?;
    Ok(())
}

async fn cancel_on_signal(shutdown: CancellationToken) {
    shutdown_signal().await;
    shutdown.cancel();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
