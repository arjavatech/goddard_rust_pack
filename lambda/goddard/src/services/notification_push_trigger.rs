use aws_sdk_lambda::{primitives::Blob, Client};

/// Best-effort, asynchronous wake-up for the durable notification push outbox.
///
/// The scheduled worker remains the retry/recovery path. This trigger only
/// removes the normal 0-60 second wait after a notification is committed.
pub struct NotificationPushTrigger {
    client: Client,
    worker_function_name: String,
}

impl NotificationPushTrigger {
    pub async fn from_environment() -> Option<Self> {
        let worker_function_name = std::env::var("NOTIFICATION_PUSH_WORKER_FUNCTION_NAME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("AWS_LAMBDA_FUNCTION_NAME")
                    .ok()
                    .filter(|name| name.starts_with("goddard-"))
                    .map(|name| format!("{}-notification-push-worker", name))
            })?;

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        println!(
            "[NotificationPushTrigger] enabled (worker={})",
            worker_function_name
        );
        Some(Self {
            client: Client::new(&config),
            worker_function_name,
        })
    }

    /// Lambda `Event` invocation returns before the worker runs, so this never
    /// adds FCM network latency to the originating API request.
    pub async fn wake(&self) {
        if let Err(error) = self
            .client
            .invoke()
            .function_name(&self.worker_function_name)
            .invocation_type(aws_sdk_lambda::types::InvocationType::Event)
            .payload(Blob::new(b"{}".to_vec()))
            .send()
            .await
        {
            eprintln!(
                "[NotificationPushTrigger] worker wake failed (will retry on schedule): {}",
                error
            );
        }
    }
}
