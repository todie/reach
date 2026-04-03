.PHONY: build release image compose down dev fmt lint deny test test-integration run shell screenshot ci-check ci-build install hooks clean

# Primary targets
build:
	cargo build --workspace

release:
	cargo build --workspace --release

image:
	docker build -t reach .

compose:
	docker compose up -d

down:
	docker compose down

# Development
dev:
	cargo watch -x 'check --workspace'

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

deny:
	cargo deny check

test:
	cargo test --workspace

test-integration:
	cargo test --workspace -- --ignored

# Container
run:
	docker run --rm -p 5900:5900 -p 6080:6080 -p 8400:8400 --shm-size=2g reach

shell:
	docker run --rm -it --shm-size=2g reach /bin/bash

screenshot:
	docker exec $$(docker ps -q -f ancestor=reach | head -1) scrot -z /tmp/shot.png && \
	docker cp $$(docker ps -q -f ancestor=reach | head -1):/tmp/shot.png ./screenshot.png

# CI
ci-check: fmt lint deny test

ci-build: release image

# Install
install:
	cargo install --path crates/reach-cli

hooks:
	pre-commit install

# Clean
clean:
	cargo clean
	docker rmi reach 2>/dev/null || true
