# Contributing to reach

## Dev Setup

```bash
# Clone the repo
git clone https://github.com/todie/reach.git
cd reach

# Build the workspace
cargo build --workspace

# Install pre-commit hooks
make hooks

# Build the Docker image
make image
```

### Prerequisites

- Rust stable (1.94+, edition 2024)
- Docker
- cargo-deny (`cargo install cargo-deny`)
- pre-commit (`pip install pre-commit`)
- cargo-watch (optional, for `make dev`)

## PR Process

1. Branch from `main`. Use a descriptive branch name (e.g. `feat/screenshot-tool`, `fix/vnc-connection`).
2. Make your changes. Keep commits focused.
3. Run the checks locally before pushing:
   ```bash
   make ci-check    # fmt + lint + deny + test
   ```
4. Open a PR against `main` with a clear description of what changed and why.
5. CI must pass (format, clippy, deny, test, Docker build + smoke test).
6. One approval required for merge.

## Commit Conventions

Use [conventional commits](https://www.conventionalcommits.org/):

```
feat: add screenshot MCP tool
fix: handle VNC connection timeout
docs: update architecture diagram
chore: bump tokio to 1.38
test: add integration test for reach create
refactor: extract Docker client into module
```

The type prefix is required. Scope is optional. Keep the subject line under 72 characters.

## Code Style

- Run `make fmt` (cargo fmt) before every commit. Max line width is 100.
- Run `make lint` (cargo clippy -D warnings) and fix all warnings.
- The pre-commit hooks will enforce formatting automatically.

## Where to Put Code

| Code runs on... | Crate | Binary |
|-----------------|-------|--------|
| Host machine | `crates/reach-cli/` | `reach` |
| Inside container | `crates/reach-supervisor/` | `reach-supervisor` |

- New CLI commands: add a variant to `Command` in `commands/mod.rs`, create a new module in `commands/`.
- New MCP tools: implement in `mcp.rs`.
- New supervised processes: add to `processes.rs`.
- Docker interaction: goes through `docker.rs`.

## Testing

```bash
# Unit tests
make test              # cargo test --workspace

# Integration tests (require Docker)
make test-integration  # cargo test --workspace -- --ignored

# Full CI check
make ci-check          # fmt + lint + deny + test
```

Integration tests are marked with `#[ignore]` and require a running Docker daemon. They create and destroy real containers.

## Adding Dependencies

- Add workspace-level dependencies to the root `Cargo.toml` under `[workspace.dependencies]`.
- Reference them in crate `Cargo.toml` files with `.workspace = true`.
- Run `cargo deny check` to verify license compatibility.

## Docker Image

The Dockerfile is a multi-stage build:

1. **Builder stage:** Compiles `reach-supervisor` using `rust:1.94-bookworm`.
2. **Runtime stage:** Ubuntu 24.04 with X11 stack, Chrome, Node.js, Playwright, Scrapling, noVNC.

After changing the Dockerfile, verify with:

```bash
make image
make run
# In another terminal:
curl http://localhost:8400/health
```
