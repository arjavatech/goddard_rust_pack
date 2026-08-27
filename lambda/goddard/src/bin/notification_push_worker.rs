use std::sync::Arc;

use goddard_backend::config::database::{get_db_pool, initialize_database};
use goddard_backend::dao::{DeviceTokenDao, NotificationPushOutboxDao};
use goddard_backend::services::{FcmService, PushDeliveryResult};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::Serialize;

const DEFAULT_BATCH_SIZE: i64 = 50;

#[derive(Serialize)]
struct DispatchResult {
    claimed: usize,
    sent: usize,
    retried: usize,
    failed: usize,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    initialize_database()
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    lambda_runtime::run(service_fn(dispatch)).await
}

async fn dispatch(_event: LambdaEvent<serde_json::Value>) -> Result<DispatchResult, Error> {
    let pool = get_db_pool().clone();
    let token_dao = Arc::new(DeviceTokenDao::new(pool.clone()));
    let fcm = match (
        std::env::var("FCM_PROJECT_ID").ok(),
        std::env::var("FCM_CLIENT_EMAIL").ok(),
        std::env::var("FCM_PRIVATE_KEY").ok(),
    ) {
        (Some(project_id), Some(client_email), Some(private_key))
            if !project_id.is_empty() && !client_email.is_empty() && !private_key.is_empty() =>
        {
            FcmService::live(project_id, client_email, private_key, token_dao)
        }
        _ => {
            return Err(
                "FCM worker is missing FCM_PROJECT_ID, FCM_CLIENT_EMAIL, or FCM_PRIVATE_KEY".into(),
            )
        }
    };

    let outbox = NotificationPushOutboxDao::new(pool);
    let jobs = outbox
        .claim_ready(DEFAULT_BATCH_SIZE)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let mut result = DispatchResult {
        claimed: jobs.len(),
        sent: 0,
        retried: 0,
        failed: 0,
    };

    for job in jobs {
        match fcm
            .send_to_token(
                &job.device_token,
                &job.title,
                &job.body,
                job.action_url.as_deref(),
                job.notification_id,
                &job.notification_type,
            )
            .await
        {
            PushDeliveryResult::Delivered => {
                outbox
                    .mark_sent(job.id)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?;
                result.sent += 1;
            }
            PushDeliveryResult::PermanentFailure(error) => {
                outbox
                    .mark_terminal_failure(job.id, &error)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?;
                result.failed += 1;
            }
            PushDeliveryResult::RetryableFailure(error) => {
                outbox
                    .retry_later(job.id, &error)
                    .await
                    .map_err(|e| Error::from(e.to_string()))?;
                result.retried += 1;
            }
        }
    }

    println!(
        "[NotificationPushWorker] claimed={} sent={} retried={} failed={}",
        result.claimed, result.sent, result.retried, result.failed
    );
    Ok(result)
}
