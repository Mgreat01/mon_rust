use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::Response,
};
use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;

use crate::{app_state::AppState, error::ApiResult, services::auth_service::decode_token};

#[derive(Deserialize)]
pub struct WebSocketQuery {
    token: String,
}

pub async fn connect(
    State(state): State<AppState>,
    Query(query): Query<WebSocketQuery>,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    let claims = decode_token(&query.token)?;
    Ok(upgrade.on_upgrade(move |socket| stream(socket, state, claims.tenant_id)))
}

async fn stream(mut socket: WebSocket, state: AppState, tenant_id: uuid::Uuid) {
    let mut events = state.events.subscribe();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) if event.tenant_id() == tenant_id => {
                        let Ok(json) = serde_json::to_string(&event) else { continue; };
                        if socket.send(Message::Text(json.into())).await.is_err() { break; }
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}
