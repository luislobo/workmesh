use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::auth::is_authorized;
use crate::state::{initial_snapshot_event, AppState};

pub async fn ws_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !is_authorized(&headers, &state.auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| async move {
        handle_socket(state, socket).await;
    })
}

async fn handle_socket(state: AppState, mut socket: WebSocket) {
    let snapshot = state.snapshot().await;
    let initial = initial_snapshot_event(&snapshot);
    if send_event(&mut socket, &initial).await.is_err() {
        return;
    }

    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            maybe_incoming = socket.recv() => {
                match maybe_incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        if send_event(&mut socket, &event).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = state.snapshot().await;
                        let initial = initial_snapshot_event(&snapshot);
                        if send_event(&mut socket, &initial).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: &crate::model::WsEvent) -> anyhow::Result<()> {
    let payload = serde_json::to_string(event)?;
    socket.send(Message::Text(payload)).await?;
    Ok(())
}
