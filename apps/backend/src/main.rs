#![allow(dead_code)]

mod auth;
mod db;
mod integrations;
mod repositories;
mod routes;
mod scheduler;
mod seed;
mod services;
mod support;

use axum::{Router, http::{HeaderValue, Method, header}};
use sqlx::SqlitePool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router(pool: SqlitePool) -> Router {
    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let cors = CorsLayer::new()
        .allow_origin(frontend_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        .allow_credentials(true);

    Router::new()
        .nest("/api/auth",         routes::auth::router())
        .nest("/api/onboarding",   routes::onboarding::router())
        .nest("/api/members",      routes::members::router())
        .nest("/api/memberships",  routes::memberships::router())
        .nest("/api/my-membership",routes::my_membership::router())
        .nest("/api/transactions", routes::transactions::router())
        .nest("/api/approvals",    routes::approvals::router())
        .nest("/api/audit-log",    routes::audit_log::router())
        .nest("/api/activity-log", routes::activity_log::router())
        .nest("/api/dashboard",    routes::dashboard::router())
        .nest("/api/sponsors",     routes::sponsors::router())
        .nest("/api/sponsor-links",routes::sponsor_links::router())
        .nest("/api/receipts",     routes::receipts::router())
        .nest("/api/payments",     routes::payments::router())
        .nest("/api/webhooks",       routes::webhooks::router())
        .nest("/api/cron",           routes::cron::router())
        .nest("/api/notifications",  routes::notifications::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(pool)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:../../sqlite/bsds-dashboard.sqlite3".to_string());
    let pool = db::connect(&database_url).await;

    tokio::spawn(scheduler::start(pool.clone()));

    let app = build_router(pool);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await
        .expect("failed to bind backend listener");

    tracing::info!("bsds-backend listening on http://0.0.0.0:5000");
    axum::serve(listener, app).await.expect("backend server failed");
}
