//! Integration tests for the database backup mechanism.
//!
//! Verifies that `sqlite3 <db> ".backup <dest>"` — the command used by
//! `make backup` — produces a valid, independently-queryable copy of the
//! production database.

mod common;

use std::process::Command;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run `sqlite3 <src> ".backup <dest>"` and return whether it succeeded.
fn run_sqlite3_backup(src: &std::path::Path, dest: &std::path::Path) -> bool {
    Command::new("sqlite3")
        .arg(src)
        .arg(format!(".backup {}", dest.display()))
        .status()
        .expect(
            "sqlite3 CLI not found — install the sqlite3 package to run backup tests \
             (e.g. `apt install sqlite3` or `brew install sqlite3`)",
        )
        .success()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Backup produces a non-empty file and is queryable with the correct row count.
#[tokio::test]
async fn backup_creates_readable_copy() {
    let (_server, pool, dir) = common::test_app().await;

    // Seed a known set of users so the backup is non-trivial.
    let admin = common::seed_admin_user(&pool).await;
    let _op = common::seed_operator_user(&pool).await;

    let db_path = dir.path().join("test.sqlite3");
    let backup_dir = tempdir().expect("tempdir failed");
    let backup_path = backup_dir.path().join("backup.sqlite3");

    assert!(run_sqlite3_backup(&db_path, &backup_path), "sqlite3 .backup exited non-zero");
    assert!(backup_path.exists(), "backup file was not created");
    assert!(backup_path.metadata().unwrap().len() > 0, "backup file is empty");

    // Open the backup as a read-only pool and verify data.
    let url = format!("sqlite:{}?mode=ro", backup_path.display());
    let bp = sqlx::SqlitePool::connect(&url)
        .await
        .expect("failed to open backup database");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&bp)
        .await
        .expect("COUNT query failed on backup");

    assert_eq!(count, 2, "backup should contain both seeded users");

    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE role = 'ADMIN'")
        .fetch_one(&bp)
        .await
        .expect("admin user not found in backup");

    assert_eq!(email, admin.email, "admin email mismatch in backup");

    bp.close().await;
}

/// Backup contains all expected tables (schema is fully copied).
#[tokio::test]
async fn backup_preserves_schema() {
    let (_server, _pool, dir) = common::test_app().await;

    let db_path = dir.path().join("test.sqlite3");
    let backup_dir = tempdir().expect("tempdir failed");
    let backup_path = backup_dir.path().join("schema-backup.sqlite3");

    assert!(run_sqlite3_backup(&db_path, &backup_path));

    let url = format!("sqlite:{}?mode=ro", backup_path.display());
    let bp = sqlx::SqlitePool::connect(&url)
        .await
        .expect("failed to open backup database");

    let expected_tables = [
        "users",
        "members",
        "memberships",
        "transactions",
        "approvals",
        "sponsors",
        "activity_logs",
    ];

    for table in &expected_tables {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        )
        .bind(table)
        .fetch_one(&bp)
        .await
        .unwrap_or(0);

        assert_eq!(exists, 1, "table `{table}` missing from backup");
    }

    bp.close().await;
}

/// Backup is a snapshot: changes made to the source after the backup are
/// not visible in the backup.
#[tokio::test]
async fn backup_is_independent_snapshot() {
    let (_server, pool, dir) = common::test_app().await;

    // Seed one user, take the backup, then seed another.
    let _admin = common::seed_admin_user(&pool).await;

    let db_path = dir.path().join("test.sqlite3");
    let backup_dir = tempdir().expect("tempdir failed");
    let backup_path = backup_dir.path().join("snapshot.sqlite3");

    assert!(run_sqlite3_backup(&db_path, &backup_path));

    // Add a second user to the source AFTER the backup.
    let _op = common::seed_operator_user(&pool).await;

    // Source now has 2 users; backup must still have only 1.
    let source_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .expect("source COUNT failed");
    assert_eq!(source_count, 2, "source should have 2 users after second seed");

    let url = format!("sqlite:{}?mode=ro", backup_path.display());
    let bp = sqlx::SqlitePool::connect(&url)
        .await
        .expect("failed to open backup database");

    let backup_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&bp)
        .await
        .expect("backup COUNT failed");

    assert_eq!(
        backup_count, 1,
        "backup should reflect only the state at backup time, not later inserts"
    );

    bp.close().await;
}
