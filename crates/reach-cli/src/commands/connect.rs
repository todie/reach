use reach_cli::docker::DockerClient;
use reach_cli::mcp::{
    JsonRpcRequest, JsonRpcResponse, McpInitializeResult, RequestId, ToolResponse,
    tool_definitions,
};
use clap::Args;
use std::io::{BufRead, Write};

#[derive(Args)]
pub struct ConnectArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(args: ConnectArgs) -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    let _sandbox = docker.find(&args.target).await?;

    tracing::info!(target = args.target, "MCP stdio bridge started");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    RequestId::Number(0),
                    -32700,
                    format!("parse error: {e}"),
                );
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                continue;
            }
        };

        let response = handle_request(&docker, &args.target, &request).await;
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_request(
    docker: &DockerClient,
    target: &str,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
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
            let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or_default();
            dispatch_tool(docker, target, req, tool_name, &arguments).await
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

async fn dispatch_tool(
    docker: &DockerClient,
    target: &str,
    req: &JsonRpcRequest,
    tool: &str,
    args: &serde_json::Value,
) -> JsonRpcResponse {
    let result = match tool {
        "screenshot" => match docker.screenshot(target).await {
            Ok(bytes) => {
                use base64::Engine;
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                ToolResponse::image(data, "image/png")
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
            exec_cmd(docker, target, &format!("xdotool mousemove {x} {y} click {btn}")).await
        }
        "type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            exec_cmd(
                docker,
                target,
                &format!("xdotool type -- '{}'", text.replace('\'', "'\\''")),
            )
            .await
        }
        "key" => {
            let combo = args.get("combo").and_then(|v| v.as_str()).unwrap_or("Return");
            exec_cmd(docker, target, &format!("xdotool key {combo}")).await
        }
        "browse" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
            exec_cmd(
                docker,
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
            let selector = args.get("selector").and_then(|v| v.as_str()).unwrap_or("body");
            let stealth = args.get("stealth").and_then(|v| v.as_bool()).unwrap_or(true);
            let fetcher = if stealth { "StealthyFetcher" } else { "Fetcher" };
            let script = format!(
                "from scrapling import {fetcher}; r = {fetcher}().get('{url}'); \
                 elems = r.css('{selector}'); \
                 import json; print(json.dumps([{{'content': e.text, 'tag': e.tag}} for e in elems]))"
            );
            exec_python(docker, target, &script).await
        }
        "playwright_eval" => {
            let script = args.get("script").and_then(|v| v.as_str()).unwrap_or("");
            exec_python(docker, target, script).await
        }
        "exec" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("echo");
            exec_cmd(docker, target, cmd).await
        }
        _ => ToolResponse::error(format!("unknown tool: {tool}")),
    };

    JsonRpcResponse::success(req.id.clone(), serde_json::to_value(result).unwrap())
}

async fn exec_cmd(docker: &DockerClient, target: &str, cmd: &str) -> ToolResponse {
    match docker
        .exec(target, &["bash".into(), "-c".into(), cmd.into()])
        .await
    {
        Ok(out) if out.exit_code == 0 => {
            ToolResponse::text(if out.stdout.is_empty() { "ok".into() } else { out.stdout })
        }
        Ok(out) => ToolResponse::error(format!("exit {}: {}", out.exit_code, out.stderr)),
        Err(e) => ToolResponse::error(e.to_string()),
    }
}

async fn exec_python(docker: &DockerClient, target: &str, script: &str) -> ToolResponse {
    match docker
        .exec(target, &["python3".into(), "-c".into(), script.into()])
        .await
    {
        Ok(out) if out.exit_code == 0 => ToolResponse::text(out.stdout),
        Ok(out) => ToolResponse::error(format!("exit {}: {}", out.exit_code, out.stderr)),
        Err(e) => ToolResponse::error(e.to_string()),
    }
}
