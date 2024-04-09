mod routes;
mod utils;
use std::error::Error;
use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;
use dotenv::dotenv;
use routes::health;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes::{create_link, redirect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "url_shortener".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is a required environment variable");

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await?;

    let (prometheus_layer, metric_handler) = PrometheusMetricLayer::pair();


    let app = Router::new()
        .route("/create", get(create_link))
        .route("/:id", get(redirect))
        .route("/metrics", get(|| async move {
            metric_handler.render()
        }))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .layer(prometheus_layer)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Could not intialize TcpListener");

    tracing::debug!(
        "listener on {}",
        listener
            .local_addr()
            .expect("Could not listener address to local address")
    );

    axum::serve(listener, app)
        .await
        .expect("Could not sucessfully create server");

    Ok(())
}
