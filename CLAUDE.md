# CLAUDE.md -- Agent Instructions for reach

## Project Overview

reach is an AI-drivable containerized desktop sandbox. It provides three things:

1. A Docker image with a full X11 desktop (Xvfb + openbox + x11vnc + noVNC), Chrome, Playwright, and Scrapling
2. A Rust CLI (`reach`) that manages sandbox containers from the host
3. An MCP server (`reach serve`) that exposes sandbox tools to AI agents via SSE

Current phase: **Phase 1 -- container desktop (complete), entering Phase 2**. All 8 CLI commands are implemented, the type system is designed (5 layers, 1197 lines), 10 MCP tools are implemented (including `page_text` and `auth_handoff` for JS-heavy SPAs and login handoffs), and e2e tests pass (Phase 1: 88 tests, +3 e2e tests for the new tools).

## Tech Stack

- **Language:** Rust (2024 edition)
- **Workspace:** Cargo workspace with two crates
- **Container base:** Ubuntu 24.04 (runtime), rust:1.94-bookworm (builder)
- **Docker client:** bollard
- **Web framework:** axum 0.8
- **CLI framework:** clap 4 (derive)
- **Process supervision:** nix (signals), tokio (async)
- **Metrics:** prometheus crate (text format)
- **License:** MIT

## Crate Structure

```
crates/
  reach-cli/          # Host-side binary. Manages containers, runs MCP server.
                      # Binary name: `reach`
  reach-supervisor/   # Runs inside the container as PID 1.
                      # Binary name: `reach-supervisor`
```

- `reach-cli` depends on bollard, clap, axum, base64, uuid, open
- `reach-supervisor` depends on nix, prometheus, axum

## Build Commands

```
make build              # cargo build --workspace
make release            # cargo build --workspace --release
make image              # docker build -t reach .
make compose            # docker compose up -d
make down               # docker compose down
make fmt                # cargo fmt --all
make lint               # cargo clippy --workspace -- -D warnings
make deny               # cargo deny check
make test               # cargo test --workspace
make test-integration   # cargo test --workspace -- --ignored
make ci-check           # fmt + lint + deny + test
make ci-build           # release + image
make install            # cargo install --path crates/reach-cli
make hooks              # pre-commit install
make clean              # cargo clean + docker rmi
make run                # docker run with ports 5900/6080/8400
make shell              # interactive bash inside container
```

## Code Conventions

- Rust 2024 edition. Use `cargo fmt --all` before committing.
- `cargo clippy --workspace -- -D warnings` must pass with zero warnings.
- `rustfmt.toml`: max_width = 100
- `clippy.toml`: too-many-arguments-threshold = 8
- Commit messages: conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `refactor:`)
- Pre-commit hooks are configured via `.pre-commit-config.yaml`. Run `make hooks` to install.
- `cargo deny check` enforces license and advisory policies.

## Container Ports

| Port | Service     |
|------|-------------|
| 5900 | VNC (x11vnc) |
| 6080 | noVNC (web)  |
| 8400 | Supervisor health/metrics API |

## Key Files

| File | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage build: rust builder + ubuntu runtime |
| `docker-compose.yml` | Main service + observability profile (prometheus, grafana) |
| `Makefile` | All build/dev/CI targets |
| `deny.toml` | cargo-deny license and advisory config |
| `config/chrome-policies.json` | Chrome managed policies (disable updates, first-run) |
| `config/openbox-rc.xml` | Window manager configuration |
| `scripts/setup-scrapling.sh` | Scrapling browser dependency installer |
| `prometheus.yml` | Prometheus scrape config for reach-supervisor |

## CI/CD

GitHub Actions workflows in `.github/workflows/`:

- `ci.yml` -- format, clippy, deny, test, docker build + smoke test
- `release.yml` -- triggered on `v*` tags; builds binaries (4 targets), pushes Docker image to ghcr.io, creates GitHub release
- `security.yml` -- security scanning

## Working on This Repo

1. Always run `make fmt` and `make lint` before committing.
2. Do not modify generated files in `target/`.
3. Host-side code goes in `reach-cli`. Container-side code goes in `reach-supervisor`.
4. The MCP protocol implementation lives in `crates/reach-cli/src/mcp.rs`.
5. Docker interaction is abstracted in `crates/reach-cli/src/docker.rs`.
6. Configuration loading is in `crates/reach-cli/src/config.rs`.
7. When adding a new CLI command, add the variant to `commands/mod.rs` and create the corresponding module.
8. When adding a new supervised process, add it to `processes.rs` in reach-supervisor.
9. Python helpers that run inside the container should be embedded as `pub const` strings in `docker.rs` (see `PAGE_TEXT_SCRIPT` / `AUTH_HANDOFF_SCRIPT`) so the binary stays self-contained.

## Persistent Chrome Profiles

`reach create --persist-profile <name>` mounts `~/.local/share/reach/profiles/<name>` (host) into the container at `/home/sandbox/.config/google-chrome-profiles/<name>`. The host root is overridable via `sandbox.profile_dir` in `~/.config/reach/config.toml`. Pass the same profile name to `page_text` / `auth_handoff` via `use_profile` so a one-time login carries across sandbox restarts.
