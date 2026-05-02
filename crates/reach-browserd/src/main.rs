use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, routing::post};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Clone)]
struct AppState {
    actor_tx: mpsc::Sender<ActorMessage>,
}

#[derive(Deserialize, Debug)]
struct CdpRequest {
    method: String,
    #[serde(default)]
    params: Value,
}

enum ActorMessage {
    SendCommand {
        method: String,
        params: Value,
        resp_tx: oneshot::Sender<Value>,
    },
    HealSession {
        resp_tx: oneshot::Sender<()>,
    },
}

#[derive(Deserialize)]
struct VersionResponse {
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

async fn get_debugger_url() -> String {
    let client = reqwest::Client::new();
    loop {
        if let Ok(resp) = client
            .get("http://127.0.0.1:9222/json/version")
            .send()
            .await
        {
            if let Ok(version) = resp.json::<VersionResponse>().await {
                return version.web_socket_debugger_url;
            }
        }
        tracing::warn!("Waiting for Chrome CDP to be ready at 127.0.0.1:9222...");
        sleep(Duration::from_secs(2)).await;
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn get_next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

struct CdpActor {
    req_rx: mpsc::Receiver<ActorMessage>,
}

impl CdpActor {
    async fn run(mut self) {
        loop {
            // Step 1: Connect and attach to a target
            tracing::info!("Connecting to CDP...");
            let (ws_stream, session_id) = match Self::connect_and_attach().await {
                Ok(tuple) => tuple,
                Err(e) => {
                    tracing::error!("Failed to connect and attach: {}. Retrying...", e);
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            tracing::info!("Connected to CDP, session ID: {}", session_id);
            let (mut ws_tx, mut ws_rx) = ws_stream.split();
            let mut pending = HashMap::<u64, oneshot::Sender<Value>>::new();

            // Step 2: Process messages
            'inner: loop {
                tokio::select! {
                    msg = ws_rx.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    // Check if the session is lost
                                    if let Some(error) = json.get("error") {
                                        if let Some(msg_str) = error.get("message").and_then(|m| m.as_str()) {
                                            if msg_str.contains("Session with given id not found") {
                                                tracing::warn!("Session lost detected in CDP response, breaking inner loop...");
                                                break 'inner;
                                            }
                                        }
                                    }

                                    if let Some(id) = json.get("id").and_then(|i| i.as_u64()) {
                                        if let Some(tx) = pending.remove(&id) {
                                            let _ = tx.send(json);
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                tracing::warn!("Websocket error: {}", e);
                                break 'inner;
                            }
                            None => {
                                tracing::warn!("Websocket closed");
                                break 'inner;
                            }
                            _ => {}
                        }
                    }
                    req = self.req_rx.recv() => {
                        match req {
                            Some(ActorMessage::SendCommand { method, params, resp_tx }) => {
                                let id = get_next_id();
                                pending.insert(id, resp_tx);

                                let payload = json!({
                                    "id": id,
                                    "sessionId": session_id,
                                    "method": method,
                                    "params": params,
                                });

                                if ws_tx.send(Message::Text(payload.to_string())).await.is_err() {
                                    tracing::warn!("Failed to send command to WS, breaking...");
                                    break 'inner;
                                }
                            }
                            Some(ActorMessage::HealSession { resp_tx }) => {
                                tracing::info!("Healing session requested explicitly. Forcing reconnect...");
                                let _ = resp_tx.send(());
                                break 'inner;
                            }
                            None => {
                                tracing::info!("Actor channel closed, exiting.");
                                return;
                            }
                        }
                    }
                }
            }
            // Inner loop exited: drop pending map, which cancels all oneshot channels.
            drop(pending);
            tracing::info!("Connection dropped or healing requested. Reconnecting in 2 seconds...");
            sleep(Duration::from_secs(2)).await;
        }
    }

    async fn connect_and_attach() -> Result<(
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        String,
    )> {
        let ws_url = get_debugger_url().await;
        let (mut ws_stream, _) = connect_async(&ws_url)
            .await
            .context("Failed to connect_async")?;

        // 1. Get targets
        let id_targets = get_next_id();
        let msg = json!({
            "id": id_targets,
            "method": "Target.getTargets",
        });
        ws_stream.send(Message::Text(msg.to_string())).await?;

        let mut target_id = String::new();
        while let Some(Ok(Message::Text(txt))) = ws_stream.next().await {
            let v: Value = serde_json::from_str(&txt)?;
            if v.get("id").and_then(|i| i.as_u64()) == Some(id_targets) {
                if let Some(targets) = v.pointer("/result/targetInfos").and_then(|t| t.as_array()) {
                    for t in targets {
                        if let (Some(t_type), Some(t_id)) = (
                            t.get("type").and_then(|s| s.as_str()),
                            t.get("targetId").and_then(|s| s.as_str()),
                        ) {
                            if t_type == "page"
                                && !t
                                    .get("url")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .starts_with("chrome://")
                            {
                                target_id = t_id.to_string();
                                break;
                            }
                        }
                    }
                }
                break;
            }
        }

        if target_id.is_empty() {
            // Create about:blank target
            tracing::info!("No valid page target found. Creating about:blank...");
            let id_create = get_next_id();
            let msg = json!({
                "id": id_create,
                "method": "Target.createTarget",
                "params": {"url": "about:blank"}
            });
            ws_stream.send(Message::Text(msg.to_string())).await?;
            while let Some(Ok(Message::Text(txt))) = ws_stream.next().await {
                let v: Value = serde_json::from_str(&txt)?;
                if v.get("id").and_then(|i| i.as_u64()) == Some(id_create) {
                    target_id = v
                        .pointer("/result/targetId")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    break;
                }
            }
        }

        if target_id.is_empty() {
            anyhow::bail!("Failed to find or create a valid target");
        }

        // 2. Attach to target
        let id_attach = get_next_id();
        let msg = json!({
            "id": id_attach,
            "method": "Target.attachToTarget",
            "params": {"targetId": target_id, "flatten": true}
        });
        ws_stream.send(Message::Text(msg.to_string())).await?;

        let mut session_id = String::new();
        while let Some(Ok(Message::Text(txt))) = ws_stream.next().await {
            let v: Value = serde_json::from_str(&txt)?;
            if v.get("id").and_then(|i| i.as_u64()) == Some(id_attach) {
                session_id = v
                    .pointer("/result/sessionId")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                break;
            }
        }

        if session_id.is_empty() {
            anyhow::bail!("Failed to attach to target");
        }

        Ok((ws_stream, session_id))
    }
}

async fn handle_cdp_request(
    State(state): State<AppState>,
    Json(payload): Json<CdpRequest>,
) -> Json<Value> {
    loop {
        let (resp_tx, resp_rx) = oneshot::channel();
        let req = ActorMessage::SendCommand {
            method: payload.method.clone(),
            params: payload.params.clone(),
            resp_tx,
        };

        if state.actor_tx.send(req).await.is_err() {
            return Json(json!({"error": "Internal error: actor disconnected"}));
        }

        match resp_rx.await {
            Ok(resp) => {
                // The actor handles "Session with given id not found" by breaking the loop,
                // which drops `resp_tx` and results in `Err(_)` here, triggering a retry.
                // Alternatively, we can still catch it here to explicitly request healing if the actor missed it.
                if let Some(error) = resp.get("error") {
                    if let Some(msg) = error.get("message").and_then(|m| m.as_str()) {
                        if msg.contains("Session with given id not found") {
                            tracing::warn!("Session lost, triggering healing explicitly...");
                            let (heal_tx, heal_rx) = oneshot::channel();
                            let _ = state
                                .actor_tx
                                .send(ActorMessage::HealSession { resp_tx: heal_tx })
                                .await;
                            let _ = heal_rx.await;
                            continue; // Retry the request
                        }
                    }
                }
                return Json(resp);
            }
            Err(_) => {
                // Actor dropped the pending request (reconnecting)
                tracing::warn!("Request dropped due to actor reconnect, retrying...");
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let (actor_tx, actor_rx) = mpsc::channel(100);
    let actor = CdpActor { req_rx: actor_rx };
    tokio::spawn(actor.run());

    let state = AppState { actor_tx };

    let app = Router::new()
        .route("/cdp", post(handle_cdp_request))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8401").await?;
    tracing::info!("reach-browserd listening on 0.0.0.0:8401");
    axum::serve(listener, app).await?;

    Ok(())
}
