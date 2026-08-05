use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct ConnectionRegistry {
    connections: Arc<RwLock<HashMap<Uuid, tokio::sync::mpsc::UnboundedSender<String>>>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new user connection
    pub async fn register(&self, user_id: Uuid, sender: tokio::sync::mpsc::UnboundedSender<String>) {
        let mut conns = self.connections.write().await;
        conns.insert(user_id, sender);
        println!("[WS] Registered connection for user: {}", user_id);
    }

    /// Unregister a user connection
    pub async fn unregister(&self, user_id: Uuid) {
        let mut conns = self.connections.write().await;
        conns.remove(&user_id);
        println!("[WS] Unregistered connection for user: {}", user_id);
    }

    /// Send message to a specific user
    pub async fn send_to_user(&self, user_id: Uuid, message: String) -> bool {
        let conns = self.connections.read().await;
        if let Some(sender) = conns.get(&user_id) {
            sender.send(message).is_ok()
        } else {
            false
        }
    }

    /// Get count of active connections (for debugging)
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

impl Clone for ConnectionRegistry {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
        }
    }
}
