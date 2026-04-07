use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Args;
use reach_cli::docker::DockerClient;
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
            let f = if stealth {
                "StealthyFetcher"
            } else {
                "Fetcher"
            };
            py(
                state,
                target,
                &format!(
                    "from scrapling import {f}; r = {f}().get('{url}'); \
                     elems = r.css('{sel}'); \
                     import json; print(json.dumps([{{'content': e.text, 'tag': e.tag}} for e in elems]))"
                ),
            )
            .await
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
