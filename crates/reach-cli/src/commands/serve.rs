use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Args;
use reach_cli::docker::{
    AuthHandoffOptions, DockerClient, PageTextOptions, ProfileMount, novnc_url,
};
use reach_cli::mcp::{
    JsonRpcRequest, JsonRpcResponse, McpInitializeResult, ToolResponse, tool_definitions,
};
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Args)]
pub struct ServeArgs {
    /// Port for the MCP SSE server
    #[arg(long, default_value = "4200")]
    pub port: u16,

    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Target sandbox (default: first running)
    #[arg(long)]
    pub sandbox: Option<String>,
}

struct AppState {
    docker: DockerClient,
    default_sandbox: Option<String>,
}

pub async fn run(args: ServeArgs) -> anyhow::Result<()> {
    let port = args.port;
    let host = args.host.clone();

    let state = Arc::new(AppState {
        docker: DockerClient::new()?,
        default_sandbox: args.sandbox,
    });

    let app = Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/mcp", get(sse_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(state);

    let addr = format!("{host}:{port}");
    println!("reach MCP server listening on {addr}");
    println!("Connect: claude mcp add reach --url http://{addr}/mcp");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn resolve_sandbox(state: &AppState, requested: Option<&str>) -> anyhow::Result<String> {
    if let Some(name) = requested.or(state.default_sandbox.as_deref()) {
        return Ok(name.to_string());
    }
    let sandboxes = state.docker.list().await?;
    sandboxes
        .into_iter()
        .find(|s| matches!(s.status, reach_cli::docker::SandboxStatus::Running))
        .map(|s| s.name)
        .ok_or_else(|| anyhow::anyhow!("no running sandbox found"))
}

async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(handle_mcp(&state, &req).await)
}

async fn sse_handler() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = tokio_stream::once(Ok(Event::default().event("endpoint").data("/mcp")));
    Sse::new(stream)
}

async fn handle_mcp(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => {
            let init = McpInitializeResult::default();
            JsonRpcResponse::success(req.id.clone(), serde_json::to_value(init).unwrap())
        }
        "tools/list" => {
            let tools = tool_definitions();
            JsonRpcResponse::success(req.id.clone(), serde_json::json!({ "tools": tools }))
        }
        "tools/call" => {
            let tool = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req.params.get("arguments").cloned().unwrap_or_default();
            let sandbox_arg = args.get("sandbox").and_then(|v| v.as_str());

            let target = match resolve_sandbox(state, sandbox_arg).await {
                Ok(t) => t,
                Err(e) => {
                    return JsonRpcResponse::success(
                        req.id.clone(),
                        serde_json::to_value(ToolResponse::error(e.to_string())).unwrap(),
                    );
                }
            };

            let result = dispatch(state, tool, &args, &target).await;
            JsonRpcResponse::success(req.id.clone(), serde_json::to_value(result).unwrap())
        }
        "notifications/initialized" | "ping" => {
            JsonRpcResponse::success(req.id.clone(), serde_json::json!({}))
        }
        _ => JsonRpcResponse::error(
            req.id.clone(),
            -32601,
            format!("unknown method: {}", req.method),
        ),
    }
}

async fn dispatch(
    state: &AppState,
    tool: &str,
    args: &serde_json::Value,
    target: &str,
) -> ToolResponse {
    match tool {
        "screenshot" => match state.docker.screenshot(target).await {
            Ok(bytes) => {
                use base64::Engine;
                ToolResponse::image(
                    base64::engine::general_purpose::STANDARD.encode(&bytes),
                    "image/png",
                )
            }
            Err(e) => ToolResponse::error(e.to_string()),
        },
        "click" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
            let btn = match args.get("button").and_then(|v| v.as_str()) {
                Some("right") => "3",
                Some("middle") => "2",
                _ => "1",
            };
            sh(
                state,
                target,
                &format!("xdotool mousemove {x} {y} click {btn}"),
            )
            .await
        }
        "type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            sh(
                state,
                target,
                &format!("xdotool type -- '{}'", text.replace('\'', "'\\''")),
            )
            .await
        }
        "key" => {
            let combo = args
                .get("combo")
                .and_then(|v| v.as_str())
                .unwrap_or("Return");
            sh(state, target, &format!("xdotool key {combo}")).await
        }
        "browse" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("about:blank");
            sh(
                state,
                target,
                &format!(
                    "google-chrome --no-sandbox --disable-gpu '{}' &",
                    url.replace('\'', "%27")
                ),
            )
            .await
        }
        "scrape" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let sel = args
                .get("selector")
                .and_then(|v| v.as_str())
                .unwrap_or("body");
            let stealth = args
                .get("stealth")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let script = if stealth {
                format!(
                    "from scrapling.fetchers import StealthyFetcher; r = StealthyFetcher.fetch('{url}', headless=True); \
                     elems = r.css('{sel}'); \
                     import json; print(json.dumps([{{'content': e.text, 'tag': e.tag}} for e in elems]))"
                )
            } else {
                format!(
                    "from scrapling.fetchers import Fetcher; r = Fetcher.get('{url}'); \
                     elems = r.css('{sel}'); \
                     import json; print(json.dumps([{{'content': e.text, 'tag': e.tag}} for e in elems]))"
                )
            };
            py(state, target, &script).await
        }
        "playwright_eval" => {
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            py(state, target, script).await
        }
        "exec" => {
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("echo");
            sh(state, target, cmd).await
        }
        "page_text" => {
            let url = match args.get("url").and_then(|v| v.as_str()) {
                Some(u) if !u.is_empty() => u.to_string(),
                _ => return ToolResponse::error("page_text: missing required `url`"),
            };
            let opts = PageTextOptions {
                url,
                wait_for: args
                    .get("wait_for")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                selector: args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                timeout_ms: args
                    .get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30_000),
                user_data_dir: args
                    .get("use_profile")
                    .and_then(|v| v.as_str())
                    .map(ProfileMount::container_path_for),
            };
            match state.docker.page_text(target, &opts).await {
                Ok(out) => match serde_json::to_string_pretty(&out) {
                    Ok(s) => ToolResponse::text(s),
                    Err(e) => ToolResponse::error(e.to_string()),
                },
                Err(e) => ToolResponse::error(e.to_string()),
            }
        }
        "auth_handoff" => {
            let url = match args.get("url").and_then(|v| v.as_str()) {
                Some(u) if !u.is_empty() => u.to_string(),
                _ => return ToolResponse::error("auth_handoff: missing required `url`"),
            };
            let opts = AuthHandoffOptions {
                url,
                wait_for_selector: args
                    .get("wait_for_selector")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                wait_for_url_contains: args
                    .get("wait_for_url_contains")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                timeout_seconds: args
                    .get("timeout_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(300),
                user_data_dir: args
                    .get("use_profile")
                    .and_then(|v| v.as_str())
                    .map(ProfileMount::container_path_for),
            };

            // Resolve the noVNC URL up-front so we can include it in the
            // response no matter which branch the helper takes.
            let vnc = match state.docker.find(target).await {
                Ok(sandbox) => sandbox
                    .ports
                    .novnc
                    .map(|p| novnc_url("localhost", p))
                    .unwrap_or_else(|| novnc_url("localhost", 6080)),
                Err(_) => novnc_url("localhost", 6080),
            };

            match state.docker.auth_handoff(target, &opts).await {
                Ok(out) => {
                    let body = serde_json::json!({
                        "status": out.status,
                        "vnc_url": vnc,
                        "url": out.url,
                        "message": out.message,
                        "instructions": "Open the vnc_url in your browser to log in. Re-call \
                                          `auth_handoff` (with wait_for_*) or `page_text` once done.",
                    });
                    match serde_json::to_string_pretty(&body) {
                        Ok(s) => ToolResponse::text(s),
                        Err(e) => ToolResponse::error(e.to_string()),
                    }
                }
                Err(e) => {
                    let body = serde_json::json!({
                        "status": "error",
                        "vnc_url": vnc,
                        "message": e.to_string(),
                    });
                    ToolResponse::error(
                        serde_json::to_string_pretty(&body).unwrap_or_else(|_| e.to_string()),
                    )
                }
            }
        }
        "browser_cdp" => {
            let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("");
            let params = args.get("params").cloned().unwrap_or(serde_json::json!({}));
            cdp(state, target, method, params).await
        }
        "browser_js" => {
            let expression = args
                .get("expression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            cdp(
                state,
                target,
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true
                }),
            )
            .await
        }
        "browser_click" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
            cdp(
                state,
                target,
                "Input.dispatchMouseEvent",
                serde_json::json!({
                    "type": "mousePressed",
                    "x": x,
                    "y": y,
                    "button": "left",
                    "clickCount": 1
                }),
            )
            .await;
            cdp(
                state,
                target,
                "Input.dispatchMouseEvent",
                serde_json::json!({
                    "type": "mouseReleased",
                    "x": x,
                    "y": y,
                    "button": "left",
                    "clickCount": 1
                }),
            )
            .await
        }
        "browser_type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            for ch in text.chars() {
                cdp(
                    state,
                    target,
                    "Input.dispatchKeyEvent",
                    serde_json::json!({
                        "type": "char",
                        "text": ch.to_string()
                    }),
                )
                .await;
            }
            ToolResponse::text("ok")
        }
        "browser_key" => {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            cdp(
                state,
                target,
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyDown",
                    "key": key
                }),
            )
            .await;
            cdp(
                state,
                target,
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyUp",
                    "key": key
                }),
            )
            .await
        }
        _ => ToolResponse::error(format!("unknown tool: {tool}")),
    }
}

async fn sh(state: &AppState, target: &str, cmd: &str) -> ToolResponse {
    match state
        .docker
        .exec(target, &["bash".into(), "-c".into(), cmd.into()])
        .await
    {
        Ok(out) if out.exit_code == 0 => ToolResponse::text(if out.stdout.is_empty() {
            "ok".into()
        } else {
            out.stdout
        }),
        Ok(out) => ToolResponse::error(format!("exit {}: {}", out.exit_code, out.stderr)),
        Err(e) => ToolResponse::error(e.to_string()),
    }
}

async fn py(state: &AppState, target: &str, script: &str) -> ToolResponse {
    match state
        .docker
        .exec(target, &["python3".into(), "-c".into(), script.into()])
        .await
    {
        Ok(out) if out.exit_code == 0 => ToolResponse::text(out.stdout),
        Ok(out) => ToolResponse::error(format!("exit {}: {}", out.exit_code, out.stderr)),
        Err(e) => ToolResponse::error(e.to_string()),
    }
}

async fn cdp(
    state: &AppState,
    target: &str,
    method: &str,
    params: serde_json::Value,
) -> ToolResponse {
    let port = match state.docker.find(target).await {
        Ok(sandbox) => match sandbox.ports.browserd {
            Some(p) => p,
            None => return ToolResponse::error("browserd port not exposed"),
        },
        Err(e) => return ToolResponse::error(e.to_string()),
    };

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/cdp");
    let payload = serde_json::json!({
        "method": method,
        "params": params
    });

    match client.post(&url).json(&payload).send().await {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => {
                if let Some(err) = json.get("error") {
                    ToolResponse::error(err.to_string())
                } else {
                    ToolResponse::text(
                        serde_json::to_string_pretty(&json).unwrap_or_else(|_| "success".into()),
                    )
                }
            }
            Err(e) => ToolResponse::error(e.to_string()),
        },
        Err(e) => ToolResponse::error(e.to_string()),
    }
}
