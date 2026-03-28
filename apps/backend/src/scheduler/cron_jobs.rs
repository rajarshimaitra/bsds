use std::collections::HashSet;

use chrono::{NaiveDate, Utc};
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use crate::{
    integrations::whatsapp::WhatsappClient,
    repositories::{members, users},
    services::{membership_service, notification_service},
};

const EXPIRY_REMINDER_DAYS: i64 = 15;
const DAILY_MEMBERSHIP_CRON: &str = "0 5 2 * * *";

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyCronResult {
    pub reminded: u32,
    pub expired: u32,
}

pub async fn register_all(
    sched: &JobScheduler,
    pool: SqlitePool,
) -> Result<(), JobSchedulerError> {
    let pool_for_daily_job = pool.clone();

    let job = Job::new_async(DAILY_MEMBERSHIP_CRON, move |_uuid, _lock| {
        let pool = pool_for_daily_job.clone();

        Box::pin(async move {
            match run_daily_membership_cron(&pool).await {
                Ok(result) => {
                    tracing::info!(
                        reminded = result.reminded,
                        expired = result.expired,
                        "daily membership cron completed"
                    );
                }
                Err(error) => {
                    tracing::error!("daily membership cron failed: {error}");
                }
            }
        })
    })?;

    sched.add(job).await?;
    Ok(())
}

pub async fn run_daily_membership_cron(
    pool: &SqlitePool,
) -> Result<DailyCronResult, membership_service::MembershipServiceError> {
    let reminded = send_expiry_reminders(pool).await?;
    let expired = expire_memberships(pool).await?;

    Ok(DailyCronResult { reminded, expired })
}

pub async fn send_expiry_reminders(
    pool: &SqlitePool,
) -> Result<u32, membership_service::MembershipServiceError> {
    let memberships = membership_service::list_expiring_soon(pool, EXPIRY_REMINDER_DAYS).await?;
    let client = WhatsappClient::from_env();
    let mut reminded_users = HashSet::new();
    let mut reminded = 0u32;

    for membership in memberships {
        let Some((user, days_left)) = resolve_user_for_membership(pool, &membership.member_id, &membership.end_date)
            .await?
        else {
            continue;
        };

        if !reminded_users.insert(user.id.clone()) {
            continue;
        }

        let _ = notification_service::notify_membership_expiry_reminder(
            pool,
            client.as_ref(),
            &user,
            days_left,
        )
        .await;
        reminded += 1;
    }

    Ok(reminded)
}

pub async fn expire_memberships(
    pool: &SqlitePool,
) -> Result<u32, membership_service::MembershipServiceError> {
    let memberships = membership_service::list_expired(pool).await?;
    let client = WhatsappClient::from_env();
    let mut expired_users = HashSet::new();
    let mut users_to_notify = Vec::new();

    for membership in memberships {
        let Some((user, _days_left)) =
            resolve_user_for_membership(pool, &membership.member_id, &membership.end_date).await?
        else {
            continue;
        };

        if expired_users.insert(user.id.clone()) {
            users_to_notify.push(user);
        }
    }

    let _updated = membership_service::auto_expire_memberships(pool).await?;

    for user in &users_to_notify {
        let _ = notification_service::notify_membership_expired(pool, client.as_ref(), user).await;
    }

    Ok(users_to_notify.len() as u32)
}

async fn resolve_user_for_membership(
    pool: &SqlitePool,
    member_record_id: &str,
    end_date: &str,
) -> Result<Option<(crate::db::models::User, i64)>, membership_service::MembershipServiceError> {
    let member = members::find_by_id(pool, member_record_id).await?;
    let Some(member) = member else {
        return Ok(None);
    };

    let Some(user_id) = member.user_id else {
        return Ok(None);
    };

    let user = users::find_by_id(pool, &user_id).await?;
    let Some(user) = user else {
        return Ok(None);
    };

    Ok(Some((user, calculate_days_until(end_date))))
}

fn calculate_days_until(end_date: &str) -> i64 {
    let today = Utc::now().date_naive();

    let parsed = NaiveDate::parse_from_str(&end_date[..10.min(end_date.len())], "%Y-%m-%d");
    match parsed {
        Ok(date) => (date - today).num_days(),
        Err(_) => 0,
    }
}
