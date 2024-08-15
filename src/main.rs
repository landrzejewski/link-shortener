mod auth;
mod dao;
mod model;
mod routes;
mod utils;

use auth::auth;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, patch, post};
use axum::{serve, Router};
use dao::delete_expired;
use dotenvy::dotenv;
use routes::{create_link, get_link_statistics, health, redirect, update_link};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::net::TcpListener;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use utils::get_env;

const DEFAULT_TRACING_LEVEL: &str = "link_shortener=debug";
const DATABASE_MAX_CONNECTIONS: u32 = 20;
const CLEANING_JOB_CRON_EXPRESSION: &str = "1/60 * * * * *";

#[tokio::main]
async fn main() {
    _ = dotenv();
    let database_url = get_env("DATABASE_URL");
    let server_address = get_env("SERVER_ADDRESS");
    configure_tracing();
    let db_connection_pool = create_db_connection_pool(&database_url).await;
    configure_scheduler(db_connection_pool.clone()).await;
    let listener = create_listener(&server_address).await;
    let router = create_router(db_connection_pool.clone());
    serve(listener, router)
        .await
        .expect("Server failed to start");
}

fn configure_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or(DEFAULT_TRACING_LEVEL.into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn create_db_connection_pool(database_url: &str) -> Pool<Postgres> {
    PgPoolOptions::new()
        .max_connections(DATABASE_MAX_CONNECTIONS)
        .connect(database_url)
        .await
        .expect("Creating database connection pool failed")
}

async fn configure_scheduler(db_connection_pool: Pool<Postgres>) {
    let scheduler = JobScheduler::new()
        .await
        .expect("Creating scheduler failed");
    scheduler
        .add(create_cleaning_job(
            CLEANING_JOB_CRON_EXPRESSION,
            db_connection_pool,
        ))
        .await
        .expect("Adding cleaning job to scheduler failed");
    scheduler.start().await.expect("Starting scheduler failed");
}

fn create_cleaning_job(cron_expression: &str, db_connection_pool: Pool<Postgres>) -> Job {
    Job::new_async(cron_expression, move |_, _| {
        let connection_pool = db_connection_pool.clone();
        Box::pin(async move { delete_expired(connection_pool).await })
    })
    .expect("Creating cleaning job failed")
}

async fn create_listener(server_address: &str) -> TcpListener {
    let listener = TcpListener::bind(&server_address)
        .await
        .expect("Creating tcp listener failed");
    tracing::info!("Listening on address: {}", server_address);
    listener
}

fn create_router(db_connection_pool: Pool<Postgres>) -> Router {
    // let (prometheus_layer, prometheus_handle) = PrometheusMetricLayer::pair();
    Router::new()
        .route("/links", post(create_link))
        .route(
            "/:id/statistics",
            get(get_link_statistics)
                .route_layer(from_fn_with_state(db_connection_pool.clone(), auth)),
        )
        .route(
            "/:id",
            patch(update_link)
                .route_layer(from_fn_with_state(db_connection_pool.clone(), auth))
                .get(redirect),
        )
        .route("/health", get(health))
        //.route("/metrics", get(|| async move { prometheus_handle.render() }))
        .layer(TraceLayer::new_for_http())
        //.layer(prometheus_layer)
        .with_state(db_connection_pool.clone())
}
