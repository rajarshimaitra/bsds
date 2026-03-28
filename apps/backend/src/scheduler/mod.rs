pub mod cron_jobs;

use std::future::pending;

use sqlx::SqlitePool;
use tokio_cron_scheduler::JobScheduler;

pub async fn start(pool: SqlitePool) {
    let sched = match JobScheduler::new().await {
        Ok(sched) => sched,
        Err(error) => {
            tracing::error!("failed to create scheduler: {error}");
            return;
        }
    };

    if let Err(error) = cron_jobs::register_all(&sched, pool).await {
        tracing::error!("failed to register scheduler jobs: {error}");
        return;
    }

    if let Err(error) = sched.start().await {
        tracing::error!("failed to start scheduler: {error}");
        return;
    }

    tracing::info!("scheduler started");

    pending::<()>().await;
}
