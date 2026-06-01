use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};

use crate::state::AppState;

/// GET /api/v1/ws/:wallet_id — WebSocket endpoint for real-time payment notifications
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    axum::extract::Path(wallet_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, wallet_id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, wallet_id: String) {
    tracing::info!("WebSocket connected for wallet {}", wallet_id);

    let mut rx = state.hub.subscribe(&wallet_id).await;

    loop {
        tokio::select! {
            // Forward payment events to the WebSocket client
            Ok(event) = rx.recv() => {
                let json = serde_json::to_string(&event).unwrap_or_default();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            // Handle incoming messages (ping/pong, close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket disconnected for wallet {}", wallet_id);
}
