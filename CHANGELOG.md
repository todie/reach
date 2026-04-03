# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-04-02

### Added

- Cargo workspace with two crates: reach-cli and reach-supervisor.
- reach-cli with command scaffolds: create, destroy, list, connect, exec, serve, vnc, screenshot.
- reach-supervisor with PID 1 process supervision, health API, and Prometheus metrics endpoint.
- Multi-stage Dockerfile: Rust builder + Ubuntu 24.04 runtime with Xvfb, openbox, x11vnc, noVNC, Chrome, Playwright, Scrapling.
- Docker Compose with observability profile (Prometheus, Grafana).
- GitHub Actions CI: format, clippy, deny, test, Docker build + smoke test.
- GitHub Actions release workflow: cross-compiled binaries (linux x86_64/aarch64, macOS x86_64/aarch64), Docker image push to ghcr.io, GitHub release.
- Security scanning workflow.
- Makefile with build, dev, lint, test, and container targets.
- Pre-commit hook configuration.
- cargo-deny configuration for license and advisory checks.
- Chrome managed policies (disable updates and first-run).
- Openbox window manager configuration.

[Unreleased]: https://github.com/todie/reach/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/todie/reach/releases/tag/v0.0.1
