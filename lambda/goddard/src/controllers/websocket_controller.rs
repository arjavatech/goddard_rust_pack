use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, Query, State},
    response::IntoResponse,
    Extension,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthContext,
    models::websocket_message::{ClientMessage, ServerMessage},
    services::connection_registry::ConnectionRegistry,
    services::NotificationService,
};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// WebSocket endpoint: /notifications/ws
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(_query): Query<WsQuery>,
    Extension(auth): Extension<AuthContext>,
    State((registry, notification_service)): State<(Arc<ConnectionRegistry>, Arc<NotificationService>)>,
) -> impl IntoResponse {
    let user_id = auth.user_id;
    
    ws.on_upgrade(move |socket| {
        handle_socket(socket, user_id, registry, notification_service)
    })
}

async fn handle_socket(
    socket: WebSocket,
    user_id: Uuid,
    registry: Arc<ConnectionRegistry>,
    notification_service: Arc<NotificationService>,
) {
    let (sender, mut receiver) = socket.split();
    
    // Create channel for sending messages
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Register this user's connection
    registry.register(user_id, tx).await;
    
    // Get initial unread count
    let unread_count = match notification_service.count_unread_ws(user_id).await {
        Ok(count) => count,
        Err(_) => 0,
    };
    
    // Send welcome message
    let welcome = ServerMessage::connected(unread_count);
    let welcome_json = serde_json::to_string(&welcome).unwrap_or_default();
    
    let mut sender = sender;
    
    // Send initial welcome
    let _ = sender.send(Message::Text(welcome_json)).await;
    
    println!("[WS] WebSocket connection established for user: {}", user_id);
    
    // Spawn task to forward messages from channel to WebSocket
    let mut rx_task = rx;
    tokio::spawn(async move {
        let mut sender_task = sender;
        while let Some(msg) = rx_task.recv().await {
            let _ = sender_task.send(Message::Text(msg)).await;
        }
    });
    
    // Listen for incoming messages from frontend
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg.action.as_str() {
                        "ping" => {
                            let pong = ServerMessage::pong();
                            let pong_json = serde_json::to_string(&pong).unwrap_or_default();
                            let _ = registry.send_to_user(user_id, pong_json).await;
                        }
                        "mark_read" => {
                            if let Some(notif_id) = client_msg.notification_id {
                                println!("[WS] Marking notification {} as read for user {}", notif_id, user_id);
                                let _ = notification_service.mark_read(notif_id, user_id).await;
                            }
                        }
                        "subscribe" => {
                            println!("[WS] User {} subscribed to notifications", user_id);
                        }
                        _ => println!("[WS] Unknown action: {}", client_msg.action),
                    }
                }
            }
            Message::Close(_) => {
                println!("[WS] Close message received from user {}", user_id);
                break;
            }
            _ => {}
        }
    }
    
    // Cleanup on disconnect
    registry.unregister(user_id).await;
    println!("[WS] Connection closed for user {}", user_id);
}
