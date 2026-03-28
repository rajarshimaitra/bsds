/// Production bootstrap — creates initial staff accounts from env vars or staff.toml.
///
/// Reads from .env (via dotenvy) first:
///   BOOTSTRAP_ADMIN_EMAIL, BOOTSTRAP_ADMIN_NAME, BOOTSTRAP_ADMIN_PHONE, BOOTSTRAP_ADMIN_PASSWORD
///   BOOTSTRAP_OPERATOR_EMAIL, BOOTSTRAP_OPERATOR_NAME, BOOTSTRAP_OPERATOR_PHONE, BOOTSTRAP_OPERATOR_PASSWORD
///   BOOTSTRAP_ORGANISER_EMAIL, BOOTSTRAP_ORGANISER_NAME, BOOTSTRAP_ORGANISER_PHONE, BOOTSTRAP_ORGANISER_PASSWORD
///
/// Falls back to --config staff.toml if none of the above are set.
/// Accounts that already exist (by email) are skipped — safe to re-run.
/// All accounts are created with is_temp_password=true (forced change on first login).

use std::env;
use std::fs;
use serde::Deserialize;
use uuid::Uuid;

struct StaffEntry {
    name: String,
    email: String,
    phone: String,
    role: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct TomlStaffEntry {
    name: String,
    email: String,
    phone: String,
    role: String,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StaffConfig {
    staff: Vec<TomlStaffEntry>,
}

fn staff_from_env() -> Vec<StaffEntry> {
    let roles = [
        ("ADMIN",     "BOOTSTRAP_ADMIN_EMAIL",     "BOOTSTRAP_ADMIN_NAME",     "BOOTSTRAP_ADMIN_PHONE",     "BOOTSTRAP_ADMIN_PASSWORD"),
        ("OPERATOR",  "BOOTSTRAP_OPERATOR_EMAIL",  "BOOTSTRAP_OPERATOR_NAME",  "BOOTSTRAP_OPERATOR_PHONE",  "BOOTSTRAP_OPERATOR_PASSWORD"),
        ("ORGANISER", "BOOTSTRAP_ORGANISER_EMAIL",  "BOOTSTRAP_ORGANISER_NAME", "BOOTSTRAP_ORGANISER_PHONE", "BOOTSTRAP_ORGANISER_PASSWORD"),
    ];

    roles.iter().filter_map(|(role, email_key, name_key, phone_key, pw_key)| {
        let email = env::var(email_key).ok()?;
        Some(StaffEntry {
            role: role.to_string(),
            email,
            name: env::var(name_key).unwrap_or_else(|_| role.to_string()),
            phone: env::var(phone_key).unwrap_or_default(),
            password: env::var(pw_key).unwrap_or_else(|_| "ChangeMe@123".to_string()),
        })
    }).collect()
}

fn staff_from_toml(config_path: &str) -> Vec<StaffEntry> {
    let content = fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("Failed to read {config_path}: {e}"));
    let config: StaffConfig = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {config_path}: {e}"));
    config.staff.into_iter().map(|e| StaffEntry {
        role: e.role,
        email: e.email,
        name: e.name,
        phone: e.phone,
        password: e.password.unwrap_or_else(|| "ChangeMe@123".to_string()),
    }).collect()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let staff = {
        let from_env = staff_from_env();
        if !from_env.is_empty() {
            from_env
        } else {
            let args: Vec<String> = env::args().collect();
            let config_path = args.windows(2)
                .find(|w| w[0] == "--config")
                .map(|w| w[1].clone())
                .unwrap_or_else(|| "staff.toml".to_string());
            staff_from_toml(&config_path)
        }
    };

    let db_path = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:../../sqlite/bsds-dashboard.sqlite3".to_string());

    let pool = bsds_backend::db::connect(&db_path).await;

    println!("\n=== BSDS Production Bootstrap ===\n");
    println!("{:<15} {:<35} {:<12}", "ROLE", "EMAIL", "STATUS");
    println!("{}", "-".repeat(65));

    for entry in &staff {
        let role = entry.role.to_uppercase();
        if !["ADMIN", "OPERATOR", "ORGANISER", "MEMBER"].contains(&role.as_str()) {
            eprintln!("SKIP  {} — invalid role '{}' (must be ADMIN/OPERATOR/ORGANISER/MEMBER)", entry.email, entry.role);
            continue;
        }

        let exists: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
            .bind(entry.email.trim().to_lowercase())
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

        if exists.is_some() {
            println!("{:<15} {:<35} {}", role, entry.email, "SKIPPED (already exists)");
            continue;
        }

        let user_id = Uuid::new_v4().to_string();
        let placeholder_member_id = format!("PENDING-{}", &user_id[..8].to_uppercase());
        let hashed = bcrypt::hash(&entry.password, 12).expect("bcrypt hash failed");

        let result = sqlx::query(
            "INSERT INTO users
               (id, member_id, name, email, phone, address, password, is_temp_password, role,
                membership_status, created_at, updated_at)
             VALUES
               (?, ?, ?, ?, ?, '', ?, 1, ?,
                'PENDING_PAYMENT', datetime('now'), datetime('now'))",
        )
        .bind(&user_id)
        .bind(&placeholder_member_id)
        .bind(entry.name.trim())
        .bind(entry.email.trim().to_lowercase())
        .bind(entry.phone.trim())
        .bind(&hashed)
        .bind(&role)
        .execute(&pool)
        .await;

        match result {
            Ok(_) => println!("{:<15} {:<35} {}", role, entry.email, "CREATED"),
            Err(e) => eprintln!("ERROR {}: {e}", entry.email),
        }
    }

    println!("\nBootstrap complete. All new accounts require a password change on first login.\n");
}
