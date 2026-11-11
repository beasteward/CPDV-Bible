//! CPDV Bible API server (Axum). Drop-in replacement for the Node service:
//! same environment variables, same `/api/v1` contract.

mod app;
mod config;
mod cors;
mod docs;
mod qs;
mod ratelimit;
mod respond;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;

use crate::app::AppState;
use crate::config::Config;
use crate::ratelimit::RateLimiter;

fn panic_response(
    _: Box<dyn std::any::Any + Send + 'static>,
) -> axum::http::Response<axum::body::Body> {
    let mut r = axum::http::Response::new(axum::body::Body::from(
        r#"{"status":"error","error":"Internal Server Error"}"#,
    ));
    *r.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    r.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(respond::JSON_CT),
    );
    r
}

/// pino level names -> tracing filter.
fn log_filter(level: &str) -> String {
    match level {
        "silent" => "off".into(),
        "fatal" => "error".into(),
        l => l.into(),
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(log_filter(&cfg.log_level))
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    tracing::info!("Loading Bible data...");
    let t0 = Instant::now();
    let store = match cpdv_bible::BibleStore::load_file(&cfg.data_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("failed to load {}: {e}", cfg.data_path.display());
            std::process::exit(1);
        }
    };
    let s = store.stats();
    tracing::info!(
        "Bible data loaded in {}ms: {} books, {} chapters, {} verses",
        t0.elapsed().as_millis(),
        s.books,
        s.chapters,
        s.verses
    );

    let port = cfg.port;
    let env = cfg.node_env.clone();
    let state = Arc::new(AppState {
        store,
        limiter: RateLimiter::new(cfg.rate_limit_window_ms, cfg.rate_limit_max),
        cfg,
        started: Instant::now(),
    });

    // Express-rate-limit rotates its maps every window; sweep expired ones.
    let sweeper = state.clone();
    tokio::spawn(async move {
        let every = sweeper
            .limiter
            .window()
            .max(std::time::Duration::from_secs(1));
        loop {
            tokio::time::sleep(every).await;
            sweeper.limiter.sweep();
        }
    });

    let app = Router::new()
        .fallback(app::handle)
        .with_state(state)
        .layer(CatchPanicLayer::custom(panic_response))
        .layer(TraceLayer::new_for_http());

    let listener = match tokio::net::TcpListener::bind(("::", port)).await {
        Ok(l) => l,
        Err(_) => tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .expect("bind"),
    };
    tracing::info!("CPDV Bible API (Rust) running on port {port} ({env})");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("signal");
            tokio::select! { _ = ctrl_c => {}, _ = term.recv() => {} }
        }
        #[cfg(not(unix))]
        let _ = ctrl_c.await;
    })
    .await
    .expect("server");
}
