// OpenCode OS — HUD WebSocket handler (RFC 24 §4).
// Each client gets its own socket. The server:
//  1. subscribes to the kernel-bus broadcast channel,
//  2. fans-out events as JSON to the connected client.
// HUD side channel `mission` for status, `audit` for sensible actions, `chat`
// for steer. Phase 0 delivers them all on a single channel; Phase 7 will split.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast::error::RecvError;

use crate::core::bus::BusEvent;
use crate::core::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_loop(socket, state))
}

async fn client_loop(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut bus_rx = state.bus().subscribe();

    // Stream kernel events to client.
    let send_task = tokio::spawn(async move { fan_out(&mut sender, &mut bus_rx).await });

    // Receive ping/pong or steer from client (Phase 0: ignore; Phase 4: steer).
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Text(t) => {
                    tracing::debug!(msg = %t, "HUD recv text (ignored in Phase 0)");
                }
                Message::Binary(b) => {
                    tracing::debug!(n = b.len(), "HUD recv binary (ignored in Phase 0)");
                }
            }
        }
    });

    let _ = futures_util::future::join_all([send_task, recv_task]).await;
}

async fn fan_out(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    bus_rx: &mut tokio::sync::broadcast::Receiver<BusEvent>,
) {
    loop {
        match bus_rx.recv().await {
            Ok(event) => {
                let json = match serde_json::to_string(&event) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!(error = %e, "HUD: failed to serialize event");
                        continue;
                    }
                };
                if sender.send(Message::Text(json)).await.is_err() {
                    return;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "HUD: kernel bus lagged; dropping events");
            }
            Err(RecvError::Closed) => return,
        }
    }
}
