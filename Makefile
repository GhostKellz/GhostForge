.PHONY: help build test clean install dev docker fmt clippy audit bench gui cli

# Default target
help:
	@echo "GhostForge - Next-Generation Gaming Platform Manager"
	@echo ""
	@echo "Available targets:"
	@echo "  build     - Build the project in release mode"
	@echo "  dev       - Build in development mode"
	@echo "  test      - Run all tests"
	@echo "  clean     - Clean build artifacts"
	@echo "  install   - Install to system"
	@echo "  fmt       - Format code"
	@echo "  clippy    - Run clippy lints"
	@echo "  audit     - Run security audit"
	@echo "  gui       - Run GUI mode"
	@echo "  cli       - Run CLI mode"
	@echo "  docker    - Build Docker image"
	@echo "  docker-dev - Run development container"
	@echo "  bench     - Run benchmarks"

# Build targets
build:
	cargo build --release --all-features

dev:
	cargo build --all-features

# Test targets
test:
	cargo test --verbose --all-features

test-gui:
	cargo test --features gui

test-integration:
	./target/release/forge --version
	./target/release/forge info --full

# Development targets
fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

audit:
	cargo audit

watch:
	cargo watch -x "run -- --help"

# Application targets
gui: build
	./target/release/forge gui

cli: build
	./target/release/forge

demo: build
	@echo "🎮 Running GhostForge Demo..."
	./target/release/forge info --full
	./target/release/forge wine list
	./target/release/forge graphics list --available
	./target/release/forge game list

# Installation
install: build
	sudo cp target/release/forge /usr/local/bin/
	@echo "✅ GhostForge installed to /usr/local/bin/forge"

uninstall:
	sudo rm -f /usr/local/bin/forge
	@echo "🗑️ GhostForge uninstalled"

# Docker targets
docker:
	docker build -t ghostforge:latest .

docker-dev:
	docker-compose up ghostforge-dev

docker-gui:
	@echo "Starting GhostForge GUI in Docker..."
	@echo "Make sure X11 forwarding is enabled: xhost +local:"
	docker-compose up ghostforge-gui

# Maintenance
clean:
	cargo clean
	docker-compose down --rmi all --volumes --remove-orphans

# Zen 3D optimization setup
setup-zen3d:
	@echo "🚀 Setting up Zen 3D V-Cache optimizations..."
	@if grep -q "AMD" /proc/cpuinfo && grep -qE "(5800X3D|5900X3D|5950X3D)" /proc/cpuinfo; then \
		echo "✅ AMD Zen 3D V-Cache CPU detected"; \
		sudo cpupower frequency-set -g performance || echo "⚠️ cpupower not available"; \
		echo "🔧 CPU governor set to performance for gaming"; \
	else \
		echo "ℹ️ No Zen 3D V-Cache CPU detected"; \
	fi

# Gaming setup
setup-gaming:
	@echo "🎮 Setting up gaming environment..."
	@which wine > /dev/null || (echo "❌ Wine not found. Install with: sudo apt install wine"; exit 1)
	@which winetricks > /dev/null || (echo "⚠️ Winetricks not found. Install with: sudo apt install winetricks")
	@which nvidia-smi > /dev/null && echo "✅ NVIDIA GPU detected" || echo "ℹ️ No NVIDIA GPU found"
	@lsmod | grep -q amdgpu && echo "✅ AMD GPU detected" || echo "ℹ️ No AMD GPU found"
	@echo "✅ Gaming environment check complete"

# Battle.net setup
setup-battlenet: build
	@echo "⚡ Setting up Battle.net for WoW/Diablo 4..."
	./target/release/forge launcher setup battlenet

# Development environment
dev-env:
	@echo "🔧 Setting up development environment..."
	rustup update
	rustup component add rustfmt clippy
	cargo install cargo-watch cargo-audit
	@echo "✅ Development environment ready"

# Release preparation
prepare-release:
	@echo "📦 Preparing release..."
	$(MAKE) fmt
	$(MAKE) clippy
	$(MAKE) test
	$(MAKE) audit
	$(MAKE) build
	@echo "✅ Release preparation complete"

# Quick development cycle
quick: fmt clippy test dev

# Performance testing
perf-test: build
	@echo "📊 Running performance tests..."
	time ./target/release/forge info --full
	@echo "🎮 Testing container performance..."
	./target/release/forge container list