//! Standalone database backup daemon.
//!
//! Runs on a cron schedule, calls `sqlite3 <DB_PATH> ".backup <dest>"`, and
//! prunes old backups. Start it as a separate process on the VPS alongside the
//! main backend.
//!
//! Environment variables (loaded from .env if present):
//!
//!   DB_PATH       — path to the live SQLite database file (required)
//!   BACKUP_DIR    — directory to write backups into          (default: ./backups)
//!   BACKUP_CRON   — cron expression (6-field, seconds first) (default: "0 0 2 * * *" = daily 02:00)
//!   BACKUP_KEEP   — number of most-recent backups to retain  (default: 7)

use std::{env, fs, path::PathBuf};
use tokio::process::Command;

use chrono::Utc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load .env if present — ignore error if missing.
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .init();

    let db_path = env::var("DB_PATH").unwrap_or_else(|_| {
        // Fall back to DATABASE_URL if DB_PATH is not set.
        let url = env::var("DATABASE_URL").expect("DB_PATH or DATABASE_URL must be set");
        // Strip scheme: "sqlite:///abs" → "/abs", "sqlite://rel" → "rel", "sqlite:rel" → "rel"
        let stripped = url.strip_prefix("sqlite:").unwrap_or(&url);
        stripped.strip_prefix("//").unwrap_or(stripped).to_string()
    });

    let backup_dir = env::var("BACKUP_DIR").unwrap_or_else(|_| "./backups".to_string());
    let cron_expr = env::var("BACKUP_CRON").unwrap_or_else(|_| "0 0 2 * * *".to_string());
    let keep: usize = env::var("BACKUP_KEEP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);

    // Ensure backup directory exists.
    fs::create_dir_all(&backup_dir).expect("failed to create BACKUP_DIR");

    info!(db_path, backup_dir, cron = cron_expr, keep, "db-backup daemon starting");

    let mut sched = JobScheduler::new().await.expect("failed to create scheduler");

    let db_path_c = db_path.clone();
    let backup_dir_c = backup_dir.clone();

    let job = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
        let db = db_path_c.clone();
        let dir = backup_dir_c.clone();

        Box::pin(async move {
            run_backup(&db, &dir, keep).await;
        })
    })
    .expect("failed to create backup job");

    sched.add(job).await.expect("failed to register backup job");
    sched.start().await.expect("failed to start scheduler");

    info!("scheduler running — waiting for next backup window");

    // Keep the process alive.
    tokio::signal::ctrl_c().await.ok();
    info!("shutdown signal received, exiting");
    sched.shutdown().await.ok();
}

async fn run_backup(db_path: &str, backup_dir: &str, keep: usize) {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let dest = PathBuf::from(backup_dir).join(format!("backup_{timestamp}.sqlite3"));

    info!(src = db_path, dest = %dest.display(), "starting backup");

    let status = Command::new("sqlite3")
        .arg(db_path)
        .arg(format!(".backup {}", dest.display()))
        .status()
        .await;

    match status {
        Ok(s) if s.success() => {
            let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            info!(dest = %dest.display(), bytes = size, "backup completed");
            prune_old_backups(backup_dir, keep);
        }
        Ok(s) => {
            error!(exit_code = ?s.code(), "sqlite3 backup exited with non-zero status");
        }
        Err(e) => {
            error!(error = %e, "failed to run sqlite3 — is it installed?");
        }
    }
}

fn prune_old_backups(backup_dir: &str, keep: usize) {
    let mut entries: Vec<PathBuf> = match fs::read_dir(backup_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("backup_") && n.ends_with(".sqlite3"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            warn!(error = %e, "could not read backup dir for pruning");
            return;
        }
    };

    // Sort oldest-first by file name (timestamp embedded in name).
    entries.sort();

    if entries.len() > keep {
        let to_delete = entries.len() - keep;
        for path in entries.iter().take(to_delete) {
            match fs::remove_file(path) {
                Ok(()) => info!(path = %path.display(), "pruned old backup"),
                Err(e) => warn!(path = %path.display(), error = %e, "failed to prune backup"),
            }
        }
    }
}
