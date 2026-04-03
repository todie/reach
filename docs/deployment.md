# Deployment

## Docker Image Build

### Local Build

```bash
make image    # docker build -t reach .
```

The Dockerfile is a multi-stage build:

1. **Builder stage** (`rust:1.94-bookworm`): compiles `reach-supervisor` in release mode.
2. **Runtime stage** (`ubuntu:24.04`): installs the X11 stack, Chrome, Node.js, Playwright, Scrapling, noVNC, and copies the compiled binary.

### Build Layers

| Layer | Contents | Approximate Size |
|-------|----------|-----------------|
| 1 | Xvfb, x11vnc, openbox, xdotool, scrot, xclip, fonts, dbus, python3 | ~200 MB |
| 2 | Google Chrome stable | ~300 MB |
| 3 | Node.js 22 + computer-use-mcp | ~100 MB |
| 4 | Playwright + Scrapling + Chromium | ~400 MB |
| 5 | noVNC + websockify | ~10 MB |
| 6 | reach-supervisor binary | ~10 MB |
| 7 | sandbox user + config files | <1 MB |

Total image size is approximately 1 GB.

## Docker Compose

### Basic (sandbox only)

```bash
docker compose up -d
```

This starts a single `reach` container with:

- Port 5900: VNC
- Port 6080: noVNC web viewer
- Port 8400: Supervisor health/metrics API
- 2 GB memory limit, 2 CPUs
- 2 GB shared memory (`/dev/shm`)
- Health check on `http://localhost:8400/health`

### With Observability

```bash
docker compose --profile observability up -d
```

This adds:

- **Prometheus** (`prom/prometheus:v2.51.0`) on `127.0.0.1:9090` -- scrapes reach-supervisor metrics every 15s.
- **Grafana** (`grafana/grafana:10.4.0`) on `127.0.0.1:3000` -- default credentials: admin / `reach` (override with `GRAFANA_PASS` env var).

Both observability services bind to localhost only for security.

### Compose Configuration

```yaml
# Environment variables
GRAFANA_PASS=your-password    # Grafana admin password (default: reach)

# Resource limits (in docker-compose.yml)
memory: 2G
cpus: "2.0"
shm_size: "2gb"
```

## Docker Run (manual)

```bash
docker run --rm \
  -p 5900:5900 \
  -p 6080:6080 \
  -p 8400:8400 \
  --shm-size=2g \
  reach
```

Or interactively:

```bash
docker run --rm -it --shm-size=2g reach /bin/bash
```

## Image Registry

Published images are available at:

```
ghcr.io/todie/reach
```

### Tags

| Tag | Description |
|-----|-------------|
| `x.y.z` | Specific version (e.g. `0.0.1`) |
| `x.y` | Minor version (e.g. `0.0`) |

Tags are created automatically by the release workflow when a `v*` git tag is pushed.

### Pulling

```bash
docker pull ghcr.io/todie/reach:0.0.1
```

## CI/CD

### CI Pipeline (`.github/workflows/ci.yml`)

Triggered on pushes and PRs to `main`. Four jobs:

| Job | Steps |
|-----|-------|
| **check** | cargo fmt --check, cargo clippy -D warnings, cargo deny check |
| **test** | cargo test --workspace |
| **build** | cargo build --workspace --release |
| **docker** | docker build, then smoke test (start container, curl /health, stop) |

The `docker` job depends on `check` and `test` passing first.

### Release Pipeline (`.github/workflows/release.yml`)

Triggered on `v*` tag push. Three jobs:

| Job | Steps |
|-----|-------|
| **build-binaries** | Cross-compile reach-cli for 4 targets (see below) |
| **docker-image** | Build and push to ghcr.io with semver tags |
| **github-release** | Create GitHub release with compiled binaries |

#### Build Targets

| Target | Runner |
|--------|--------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest |
| `aarch64-unknown-linux-gnu` | ubuntu-latest (via cross) |
| `x86_64-apple-darwin` | macos-latest |
| `aarch64-apple-darwin` | macos-latest |

### Security Pipeline (`.github/workflows/security.yml`)

Runs security scanning on the codebase and dependencies.

## Releasing a New Version

1. Update `version` in workspace `Cargo.toml`.
2. Update `CHANGELOG.md` with the new version section.
3. Commit: `git commit -m "chore: release v0.1.0"`
4. Tag: `git tag v0.1.0`
5. Push: `git push origin main --tags`
6. The release workflow handles building, publishing, and creating the GitHub release.
