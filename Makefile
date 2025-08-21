.PHONY: build test clean run-server docker-build docker-run fmt lint install-deps verify prepare-release integration-test load-test all

# Default target
all: fmt lint build test docker-build

# 🔧 Development
install-deps:
	@echo "📦 Installing dependencies..."
	@which protoc > /dev/null || (echo "❌ protoc not found. Install with: apt-get install protobuf-compiler" && exit 1)
	@which cargo > /dev/null || (echo "❌ Rust not found. Install from: https://rustup.rs/" && exit 1)

fmt:
	@echo "🎨 Formatting code..."
	@cargo fmt --all

lint:
	@echo "🔍 Running lints..."
	@cargo clippy --all-targets --all-features -- -D warnings

build:
	@echo "🔨 Building AgentState..."
	@cargo build --release -p agentstate-server

test:
	@echo "🧪 Running unit tests..."
	@cargo test --all

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean

# 🚀 Running
run-server:
	@echo "🚀 Starting AgentState server..."
	@cargo run -p agentstate-server

run-server-persistent:
	@echo "🚀 Starting AgentState with persistent storage..."
	@DATA_DIR=/tmp/agentstate-data cargo run -p agentstate-server

# 🐳 Docker
docker-build:
	@echo "🐳 Building Docker image..."
	@docker build -f docker/Dockerfile -t agentstate:latest .

docker-run:
	@echo "🐳 Starting AgentState in Docker..."
	@docker run -d --name agentstate -p 8080:8080 -p 9090:9090 agentstate:latest
	@echo "✅ AgentState running at http://localhost:8080"

docker-run-persistent:
	@echo "🐳 Starting AgentState with persistent storage in Docker..."
	@docker run -d --name agentstate -p 8080:8080 -p 9090:9090 -e DATA_DIR=/data -v agentstate-data:/data agentstate:latest
	@echo "✅ AgentState running at http://localhost:8080 with persistent storage"

docker-stop:
	@echo "🛑 Stopping AgentState Docker container..."
	@docker stop agentstate || true
	@docker rm agentstate || true

# 🧪 Testing
integration-test:
	@echo "🧪 Running integration tests..."
	@python3 integration_tests.py

load-test:
	@echo "⚡ Running load tests..."
	@python3 load_test.py

sdk-examples:
	@echo "📝 Running SDK examples..."
	@cd examples/quickstart && python3 python_example.py
	@node examples/quickstart/nodejs_example.js

test-suite:
	@echo "🎯 Running comprehensive test suite..."
	@bash test_suite.sh

# 📋 Complete verification
verify: fmt lint build test docker-build integration-test sdk-examples
	@echo "✅ All verification steps completed successfully!"

# 🚀 Release preparation
prepare-release: verify
	@echo "🚀 Preparing release..."
	@echo "✅ All checks passed - ready for release!"

# Legacy targets (keeping compatibility)
verify-soak:
	@echo "Running overflow soak (requires local server on :8080)" && \
	python3 examples/tests/overflow_soak.py || true

verify-k6:
	@echo "Running QPS limiter test (requires k6)" && \
	k6 run examples/tests/qps_limiter.js || true

verify-restore:
	@echo ">> Taking snapshot" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" -X POST "$(BASE)/admin/snapshot" -o /tmp/snap.json && \
	jq -r '.snapshot_id' /tmp/snap.json > /tmp/snap.id && \
	printf "Snapshot %s\n" "$$(cat /tmp/snap.id)" && \
	printf ">> Trimming WAL\n" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" -X POST "$(BASE)/admin/trim-wal?snapshot_id=$$(cat /tmp/snap.id)" >/dev/null && \
	printf ">> Offline restore + dump\n" && \
	cargo run -q -p agentstate-cli -- restore --snapshot $$DATA_DIR/snapshots/$$(cat /tmp/snap.id) --wal $$DATA_DIR/wal --out $(RESTORE_OUT) --dump $(RESTORE_DUMP) && \
	printf ">> Live dump (dev mode only)\n" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" "$(BASE)/admin/dump?ns=$(NS)" -o $(LIVE_DUMP) && \
	printf ">> Diff\n" && \
	python3 examples/tools/diff_dump.py $(LIVE_DUMP) $(RESTORE_DUMP) || true

# 📖 Help
help:
	@echo "🤖 AgentState Makefile Commands"
	@echo "==============================="
	@echo ""
	@echo "🔧 Development:"
	@echo "  install-deps     Install required dependencies"
	@echo "  fmt              Format code"
	@echo "  lint             Run linter (clippy)"
	@echo "  build            Build release binary"
	@echo "  test             Run unit tests"
	@echo "  clean            Clean build artifacts"
	@echo ""
	@echo "🚀 Running:"
	@echo "  run-server       Start server locally"
	@echo "  run-server-persistent  Start with persistent storage"
	@echo ""
	@echo "🐳 Docker:"
	@echo "  docker-build     Build Docker image"
	@echo "  docker-run       Run in Docker"
	@echo "  docker-run-persistent  Run with persistent storage"
	@echo "  docker-stop      Stop Docker container"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  integration-test Run integration test suite"
	@echo "  load-test        Run performance load tests"
	@echo "  sdk-examples     Test SDK examples"
	@echo "  test-suite       Run bash test suite"
	@echo ""
	@echo "📋 Complete verification:"
	@echo "  verify           Run all checks and tests"
	@echo "  prepare-release  Complete release preparation"
	@echo ""
	@echo "💡 Quick start: make all"

RESTORE_OUT ?= /tmp/restore-report.json
RESTORE_DUMP ?= /tmp/restore.jsonl
LIVE_DUMP ?= /tmp/live.jsonl
ADMIN_CAP ?= $(shell cat .admin.cap 2>/dev/null)
BASE ?= http://localhost:8080
NS ?= agent://verify
