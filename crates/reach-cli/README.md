# reach-cli

Host-side CLI binary for managing reach sandbox containers and serving MCP tools to AI agents.

## Installation

```bash
# From the workspace root
cargo install --path crates/reach-cli

# Or via make
make install
```

The binary is named `reach`.

## Commands

### reach create

Create a new sandbox container.

```
reach create [--name <name>] [--resolution <WxH>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--name` | `reach` | Container name |
| `--resolution` | `1280x720` | Display resolution (WxH) |

Creates a container from the reach image with the X11 display stack, VNC, and all scraping tools. The container starts immediately.

### reach destroy

Destroy a sandbox container.

```
reach destroy <target>
```

The target is a container name or ID. Stops and removes the container.

### reach list

List running sandbox containers.

```
reach list
```

Shows all containers with the reach label, including name, ID, status, and ports.

### reach connect

Attach an MCP stdio bridge to a sandbox.

```
reach connect <target>
```

Connects stdin/stdout to the MCP protocol handler for a specific sandbox. Used when an AI agent communicates over stdio rather than SSE.

### reach exec

Run a command inside a sandbox.

```
reach exec <target> -- <command...>
```

Executes the given command inside the target container and prints stdout/stderr.

### reach serve

Start the MCP SSE server.

```
reach serve [--port <port>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | `4200` | HTTP port for the SSE server |

Starts an HTTP server that exposes MCP tools (screenshot, click, type, key, browse, scrape, playwright_eval, exec) via Server-Sent Events. AI agents connect to this endpoint.

Configure Claude Code to use it:

```json
{
  "mcpServers": {
    "reach": {
      "url": "http://localhost:4200/sse"
    }
  }
}
```

### reach vnc

Open the noVNC web viewer in your browser.

```
reach vnc <target>
```

Opens `http://localhost:6080` (or the mapped port) in the default browser, giving you a live view of the sandbox desktop.

### reach screenshot

Capture a screenshot from a sandbox.

```
reach screenshot <target> [-o <path>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-o`, `--output` | (stdout) | Output file path for the PNG |

Captures the current display as a PNG image.

## Configuration

reach loads configuration from `~/.config/reach/config.toml`.

```toml
# Default sandbox settings
[sandbox]
image = "ghcr.io/todie/reach:latest"
resolution = "1280x720"
memory = "2g"
cpus = 2.0
shm_size = "2g"

# MCP server settings
[serve]
port = 4200

# Docker connection
[docker]
# Uses local defaults (unix socket or DOCKER_HOST env var)
```

All settings are optional. CLI flags override config file values.

## MCP Server Mode

`reach serve` implements the Model Context Protocol (MCP) over SSE transport. The server:

1. Accepts SSE connections from AI agents.
2. Advertises available tools (screenshot, click, type, key, browse, scrape, playwright_eval, exec).
3. Dispatches tool calls to the target sandbox container via the Docker API (bollard).
4. Returns results (text, images, JSON) through the SSE stream.

Multiple sandboxes can be active simultaneously. The server manages them by name or container ID.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging level (default: `reach=info`) |
| `DOCKER_HOST` | Docker daemon address (default: local socket) |

## Dependencies

- **bollard**: Docker Engine API client
- **clap**: CLI argument parsing (derive macros)
- **axum**: HTTP server for MCP SSE transport
- **tokio**: Async runtime
- **serde / serde_json**: Serialization
- **base64**: Image encoding for screenshots
- **uuid**: Sandbox ID generation
- **open**: Opens URLs in the default browser
