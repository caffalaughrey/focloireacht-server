mod config;
mod db;
mod models {
    #[cfg(feature = "lexdb")]
    pub mod lex;
    #[cfg(feature = "termdb")]
    pub mod term;
    pub mod meta;
}
mod routes {
    pub mod health;
    pub mod meta;
    #[cfg(feature = "lexdb")]
    pub mod lex;
    #[cfg(feature = "termdb")]
    pub mod term;
}

use axum::{routing::{get, post}, Router};
use config::Config;
use db::DbPools;
use std::net::SocketAddr;
use tower_http::{trace::TraceLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // init tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env();
    let pools = DbPools::connect(&cfg.lex_db_url(), &cfg.term_db_url()).await?;

    let app_state = pools.clone();

    let mut app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/healthz", get(routes::health::health))
        .route("/meta", get(routes::meta::meta));

    #[cfg(feature = "lexdb")]
    {
        app = app
            .route("/lex/entry", get(routes::lex::get_by_lemma))
            .route("/lex/by-variant", get(routes::lex::get_by_variant))
            .route("/lex/batch", post(routes::lex::post_batch))
            .route("/api/lex/1.0/lookup", post(routes::lex::lookup_legacy));
    }

    #[cfg(feature = "termdb")]
    {
        app = app
            .route("/term/en2ga", get(routes::term::en2ga))
            .route("/term/ga2en", get(routes::term::ga2en))
            .route("/term/domains", get(routes::term::domains))
            .route("/term/domains/search", get(routes::term::domains_search))
            .route("/term/vocab", get(routes::term::vocab))
            .route("/term/validate", get(routes::term::validate))
            .route("/term/batch", post(routes::term::batch));
    }

    let app = app
        .with_state(app_state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = cfg.bind_addr.parse()?;
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
