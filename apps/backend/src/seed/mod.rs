pub mod fixtures;

use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) {
    println!("Seeding users...");
    fixtures::seed_users(pool).await;
    println!("Seeding sub-members...");
    fixtures::seed_sub_members(pool).await;
    println!("Seeding member records...");
    fixtures::seed_member_records(pool).await;
    println!("Seeding memberships...");
    fixtures::seed_memberships(pool).await;
    println!("Seeding sponsors...");
    fixtures::seed_sponsors(pool).await;
    println!("Seeding sponsor links...");
    fixtures::seed_sponsor_links(pool).await;
    println!("Seeding transactions...");
    fixtures::seed_transactions(pool).await;
    println!("Seeding receipts...");
    fixtures::seed_receipts(pool).await;
    println!("Seeding approvals...");
    fixtures::seed_approvals(pool).await;
    println!("Seeding audit logs...");
    fixtures::seed_audit_logs(pool).await;
    println!("Seeding activity logs...");
    fixtures::seed_activity_logs(pool).await;
    println!("Seed complete.");
}
