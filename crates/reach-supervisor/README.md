# reach-supervisor

PID 1 process supervisor for reach sandbox containers. Runs inside the Docker container as the entrypoint.

## Overview

reach-supervisor starts and monitors all processes that make up the sandbox desktop environment. It runs as PID 1, which means it is responsible for reaping zombie processes and handling shutdown signals.

## Process Table

The supervisor manages these processes in dependency order:

| Process | Command | Port | Depends On | Restart Policy |
|---------|---------|------|------------|----------------|
| Xvfb | `Xvfb :99 -screen 0 1280x720x24 -ac` | -- | -- | always |
| openbox | `openbox --sm-disable` | -- | Xvfb | always |
| x11vnc | `x11vnc -display :99 -forever -shared -nopw` | 5900 | Xvfb | always |
| noVNC | `/opt/noVNC/utils/novnc_proxy --vnc localhost:5900 --listen 6080` | 6080 | x11vnc | always |

### Startup Sequence

1. Clean stale X11 lock files (`/tmp/.X99-lock`).
2. Start Xvfb (virtual framebuffer on display `:99`).
3. Wait for Xvfb to be ready.
4. Start openbox (window manager).
5. Start x11vnc (VNC server exporting display `:99`).
6. Start noVNC (web-based VNC client proxying to x11vnc).
7. Start the health API server on port 8400.

### Restart Policy

If a managed process exits unexpectedly, the supervisor restarts it. This handles transient failures (e.g. x11vnc crashing under load).

## Health API

HTTP server on port 8400 using axum.

### GET /health

Returns the supervisor health status as JSON.

**Response:**

```json
{
  "status": "ok",
  "service": "reach-supervisor"
}
```

Used by Docker health checks:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8400/health"]
  interval: 10s
  timeout: 5s
  retries: 3
```

### GET /metrics

Returns Prometheus text format metrics.

**Response:**

```
# HELP process_cpu_seconds_total Total user and system CPU time spent in seconds.
# TYPE process_cpu_seconds_total counter
process_cpu_seconds_total 0.42
...
```

Scraped by Prometheus every 15s (configured in `prometheus.yml`).

## Signal Handling

| Signal | Behavior |
|--------|----------|
| SIGTERM | Graceful shutdown: stop all children in reverse order, then exit |
| SIGINT | Same as SIGTERM |

The shutdown sequence:

1. Signal received.
2. Stop noVNC.
3. Stop x11vnc.
4. Stop openbox.
5. Stop Xvfb.
6. Abort the health API server.
7. Exit with code 0.

## PID 1 Responsibilities

As PID 1 in the container, reach-supervisor must:

- **Reap zombie processes**: any orphaned child processes are adopted by PID 1, which must `wait()` on them.
- **Forward signals**: SIGTERM from Docker stop is delivered to PID 1, which coordinates shutdown of all children.
- **Never exit unexpectedly**: if the supervisor crashes, the container dies.

## Module Structure

```
src/
  main.rs        -- Entrypoint. Initializes tracing, starts supervisor
                    and health server, waits for shutdown signal.
  processes.rs   -- Supervisor struct. Defines managed processes, handles
                    start_all() and stop_all(), cleans X11 locks.
  health.rs      -- axum HTTP server with /health and /metrics routes.
  signals.rs     -- Async signal handler for SIGTERM and SIGINT using
                    tokio::signal.
```

## Configuration

The supervisor is configured via environment variables set in the Dockerfile:

| Variable | Default | Description |
|----------|---------|-------------|
| `DISPLAY` | `:99` | X11 display number |
| `HOME` | `/home/sandbox` | Home directory for the sandbox user |
| `RUST_LOG` | `reach_supervisor=info` | Logging level |

## Dependencies

- **nix**: POSIX signal and process management
- **prometheus**: Metrics collection and text format encoding
- **axum**: HTTP server for health and metrics endpoints
- **tokio**: Async runtime, signal handling, process spawning
- **tracing / tracing-subscriber**: Structured logging

## Building

The supervisor is built inside the Docker multi-stage build:

```dockerfile
FROM rust:1.94-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY crates/ crates/
RUN cargo build --release -p reach-supervisor
```

The compiled binary is copied to `/usr/local/bin/reach-supervisor` in the runtime stage.

To build locally (for testing, not for container use):

```bash
cargo build -p reach-supervisor
```
