use bsds_backend::{db, seed};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = db::connect("sqlite:../../sqlite/bsds-dashboard.sqlite3").await;

    seed::run(&pool).await;
}
