# reach

AI-drivable containerized desktop sandbox.

reach gives AI agents a full Linux desktop inside Docker -- with a browser, screen capture, mouse/keyboard control, and web scraping -- all exposed through MCP tools.

**Three components:**

1. **Docker image** -- Ubuntu 24.04 with Xvfb, openbox, x11vnc, noVNC, Chrome, Playwright, Scrapling
2. **Rust CLI** (`reach`) -- manages sandbox containers from the host
3. **MCP server** (`reach serve`) -- exposes sandbox tools to AI agents via SSE

## Status

**Phase 1 -- container desktop: complete.** Entering Phase 2.

| Area | Status |
|------|--------|
| Type system | Done -- 5 layers, 1197 lines |
| CLI commands | All 8 implemented and working |
| MCP tools | All 8 implemented |
| Docker image | Builds and runs (e2e tested) |
| Tests | 88 passing (52 unit + 36 e2e) |
| Scrapling | Compatible with 0.4.3 API (Fetcher/StealthyFetcher with `.get()`) |

**Coming next (Phase 2):**
- MCP SSE server integration testing
- Agent-driven workflow demos
- Multi-sandbox orchestration

## Architecture

```
  Claude Code / AI Agent
        |
        | MCP (SSE, port 4200)
        v
  +-------------+
  | reach serve  |  <-- host process (reach-cli)
  +-------------+
        |
        | docker exec / Docker API (bollard)
        v
  +------------------------------------------+
  | reach sandbox container                  |
  |                                          |
  |  reach-supervisor (PID 1)                |
  |    |                                     |
  |    +-- Xvfb :99         (virtual display)|
  |    +-- openbox           (window manager)|
  |    +-- x11vnc :5900          (VNC server)|
  |    +-- noVNC :6080          (web viewer) |
  |    +-- health API :8400                  |
  |                                          |
  |  Chrome / Playwright / Scrapling         |
  +------------------------------------------+
```

## Quickstart

```bash
# Build the Docker image
make image

# Start a sandbox with docker compose
docker compose up -d

# Or use the CLI directly
reach create --name my-sandbox --resolution 1280x720
reach list
reach vnc my-sandbox          # open desktop in browser
reach screenshot my-sandbox -o shot.png
reach exec my-sandbox -- ls -la
reach destroy my-sandbox

# Start the MCP server for AI agents
reach serve --port 4200

# Run tests
make test                     # 52 unit tests
cargo test --workspace -- --ignored   # 36 e2e tests (requires Docker)
```

## CLI Commands (all 8 implemented)

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `reach create` | Create a new sandbox container | `--name`, `--resolution` |
| `reach destroy` | Destroy a sandbox container | `<target>` |
| `reach list` | List running sandbox containers | -- |
| `reach connect` | Attach MCP stdio bridge to a sandbox | `<target>` |
| `reach exec` | Run a command inside a sandbox | `<target> -- <command>` |
| `reach serve` | Start MCP SSE server | `--port` (default: 4200) |
| `reach vnc` | Open noVNC in browser | `<target>` |
| `reach screenshot` | Capture a screenshot | `<target>`, `-o/--output` |

## MCP Tools (all 8 implemented)

These tools are exposed by `reach serve` to AI agents:

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `screenshot` | Capture the sandbox display as a PNG | -- |
| `click` | Click at screen coordinates | `x`, `y`, `button` |
| `type` | Type text via keyboard | `text` |
| `key` | Send a key combination | `keys` (e.g. "ctrl+c") |
| `browse` | Navigate Chrome to a URL | `url` |
| `scrape` | Extract structured content from a page (Scrapling Fetcher/StealthyFetcher `.get()`) | `url`, `selector` |
| `playwright_eval` | Execute Playwright script | `script` |
| `exec` | Run a shell command in the sandbox | `command` |

## Ports

| Port | Service |
|------|---------|
| 4200 | MCP SSE server (host) |
| 5900 | VNC (container) |
| 6080 | noVNC web viewer (container) |
| 8400 | Supervisor health/metrics API (container) |

## Observability

Start with the observability profile to get Prometheus and Grafana:

```bash
docker compose --profile observability up -d
```

- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin / `reach`)

## Install

```bash
# From source
cargo install --path crates/reach-cli

# Or build everything
make build
make image
```

## Documentation

- [Architecture](docs/architecture.md)
- [MCP Tools Reference](docs/mcp-tools.md)
- [Deployment](docs/deployment.md)
- [Web Scraping](docs/scraping.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## License

MIT
