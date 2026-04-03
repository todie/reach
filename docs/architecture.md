# Architecture

## Overview

reach is a three-layer system:

1. **AI agent** (Claude Code or any MCP client) communicates via MCP over SSE.
2. **reach CLI** runs on the host, manages Docker containers, and serves MCP tools.
3. **reach sandbox** is a Docker container running a full X11 desktop with browser automation tools.

```
  AI Agent (Claude Code, etc.)
        |
        | MCP protocol (SSE, port 4200)
        v
  +------------------+
  | reach serve       |  Host process (reach-cli binary)
  | - SSE transport   |
  | - Tool dispatch   |
  | - bollard client  |
  +------------------+
        |
        | Docker Engine API / docker exec
        v
  +--------------------------------------------------+
  | Sandbox Container (Ubuntu 24.04)                 |
  |                                                  |
  |  reach-supervisor (PID 1, port 8400)             |
  |    |                                             |
  |    +-- Xvfb :99            Virtual framebuffer   |
  |    +-- openbox             Window manager        |
  |    +-- x11vnc :5900        VNC server            |
  |    +-- noVNC :6080         Web-based VNC client  |
  |                                                  |
  |  Application layer:                              |
  |    +-- Google Chrome       (headed, on :99)      |
  |    +-- Playwright          (Chromium, headless)  |
  |    +-- Scrapling           (adaptive scraping)   |
  |    +-- xdotool / scrot     (X11 automation)      |
  +--------------------------------------------------+
```

## Container Internals

### Display Stack

The container runs a virtual X11 display using Xvfb at `:99`. This allows graphical applications (Chrome, etc.) to render without a physical monitor.

```
Xvfb :99 (1280x720x24)
  |
  +-- openbox (window manager, manages window placement)
  |     |
  |     +-- Chrome, other GUI apps
  |
  +-- x11vnc (exports :99 as VNC on port 5900)
        |
        +-- noVNC (websockify on port 6080, proxies VNC to WebSocket)
```

### Process Supervision

`reach-supervisor` is the container entrypoint and runs as PID 1. It starts and monitors all child processes.

| Process | Command | Port | Restart |
|---------|---------|------|---------|
| Xvfb | `Xvfb :99 -screen 0 1280x720x24 -ac` | -- | always |
| openbox | `openbox --sm-disable` | -- | always |
| x11vnc | `x11vnc -display :99 -forever -shared -nopw` | 5900 | always |
| noVNC | `/opt/noVNC/utils/novnc_proxy --vnc localhost:5900 --listen 6080` | 6080 | always |

The supervisor:

1. Cleans stale X11 lock files (`/tmp/.X99-lock`).
2. Starts all processes in dependency order (Xvfb first, then openbox, then x11vnc, then noVNC).
3. Monitors child processes and restarts any that exit unexpectedly.
4. Listens for SIGTERM/SIGINT and performs graceful shutdown (stop children in reverse order).

### Health API

The supervisor exposes an HTTP server on port 8400:

- `GET /health` -- returns JSON `{"status": "ok", "service": "reach-supervisor"}`.
- `GET /metrics` -- returns Prometheus text format metrics.

Docker Compose uses `/health` for container health checks.

### Signal Handling

- **SIGTERM** -- graceful shutdown. Stops all children, then exits.
- **SIGINT** -- same as SIGTERM.
- Child processes are reaped by the supervisor (PID 1 responsibility).

## MCP Protocol Flow

```
1. AI agent connects to reach serve (SSE on port 4200)
2. Agent sends tool call (e.g. screenshot, click, type)
3. reach serve dispatches via Docker API:
   - screenshot: docker exec scrot -> base64 encode -> return
   - click/type/key: docker exec xdotool
   - browse: docker exec chrome or playwright
   - scrape: docker exec python3 scrapling script
   - exec: docker exec arbitrary command
4. Result returned to agent via SSE
```

### Tool Dispatch

`reach serve` translates MCP tool calls into `docker exec` commands inside the target sandbox container. The bollard Docker client handles communication with the Docker Engine API.

For screenshot capture, the flow is:

1. `docker exec <container> scrot -z /tmp/shot.png`
2. Read the file from the container
3. Base64 encode and return as MCP image content

For browser automation:

1. Playwright commands are executed via `docker exec python3 -c "<script>"`
2. Scrapling commands follow a similar pattern with the scrapling Python API

## Host-Side Architecture (reach-cli)

```
reach-cli/
  src/
    main.rs        -- CLI entrypoint, clap parser, tracing setup
    commands/
      mod.rs       -- Command enum and dispatch
      create.rs    -- Create sandbox container
      destroy.rs   -- Destroy sandbox container
      list.rs      -- List running sandboxes
      connect.rs   -- MCP stdio bridge
      exec.rs      -- Run command in sandbox
      serve.rs     -- MCP SSE server
      vnc.rs       -- Open noVNC in browser
      screenshot.rs -- Capture screenshot
    config.rs      -- Load ~/.config/reach/config.toml
    docker.rs      -- bollard Docker client wrapper
    mcp.rs         -- MCP protocol (tool definitions, SSE, dispatch)
```

## Container-Side Architecture (reach-supervisor)

```
reach-supervisor/
  src/
    main.rs        -- Entrypoint, starts supervisor + health server
    processes.rs   -- Process definitions, start/stop/restart logic
    health.rs      -- axum HTTP server (/health, /metrics)
    signals.rs     -- SIGTERM/SIGINT handler
```

## Networking

| Port | Listener | Protocol | Purpose |
|------|----------|----------|---------|
| 4200 | reach serve (host) | HTTP/SSE | MCP server for AI agents |
| 5900 | x11vnc (container) | VNC | Direct VNC access |
| 6080 | noVNC (container) | HTTP/WS | Browser-based VNC viewer |
| 8400 | reach-supervisor (container) | HTTP | Health checks and Prometheus metrics |

## Security Considerations

- The sandbox user (`sandbox`) has passwordless sudo. This is intentional for agent flexibility but means the container should be treated as untrusted.
- VNC has no password by default (`-nopw`). Do not expose port 5900 to the internet.
- Chrome runs with managed policies that disable auto-updates and first-run dialogs.
- The container uses `--shm-size=2g` to prevent Chrome crashes from insufficient shared memory.
