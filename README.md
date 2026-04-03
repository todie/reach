# reach

AI-drivable containerized desktop sandbox.

reach gives AI agents a full Linux desktop inside Docker -- with a browser, screen capture, mouse/keyboard control, and web scraping -- all exposed through MCP tools.

**Three components:**

1. **Docker image** -- Ubuntu 24.04 with Xvfb, openbox, x11vnc, noVNC, Chrome, Playwright, Scrapling
2. **Rust CLI** (`reach`) -- manages sandbox containers from the host
3. **MCP server** (`reach serve`) -- exposes sandbox tools to AI agents via SSE

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
# Start a sandbox with docker compose
docker compose up -d

# Or use the CLI
reach create --name my-sandbox --resolution 1280x720

# Start the MCP server for AI agents
reach serve --port 4200

# Open the desktop in your browser
reach vnc my-sandbox

# Capture a screenshot
reach screenshot my-sandbox -o shot.png

# Run a command inside the sandbox
reach exec my-sandbox -- ls -la
```

## CLI Reference

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

## MCP Tool Reference

These tools are exposed by `reach serve` to AI agents:

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `screenshot` | Capture the sandbox display as a PNG | -- |
| `click` | Click at screen coordinates | `x`, `y`, `button` |
| `type` | Type text via keyboard | `text` |
| `key` | Send a key combination | `keys` (e.g. "ctrl+c") |
| `browse` | Navigate Chrome to a URL | `url` |
| `scrape` | Extract structured content from a page | `url`, `selector` |
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
