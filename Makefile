.PHONY: build test frontend smoke test-install

SWARM_OUTPUT ?= bin/swarm

build:
	sh ./build.sh "$(SWARM_OUTPUT)"
test:
	cargo test --manifest-path rust/Cargo.toml --locked --workspace --lib
	npm test
frontend:
	cd frontend && npm ci --ignore-scripts --no-audit --no-fund && npm run build
smoke: build
	python3 tests/rust_native_http_cli.py --binary "$(abspath $(SWARM_OUTPUT))" --output .cache/native-smoke-$$(date +%s)-$$$$

# Requires a local Docker Engine and a checkout without deploy/install.env.
test-install:
	python3 tests/rust_native_install.py --output .cache/install-native-$$(date +%s)-$$$$
	python3 tests/rust_docker_install.py --output .cache/install-docker-$$(date +%s)-$$$$

# Deterministic agent processes: no paid model or API calls.
.PHONY: test-process
test-process: build
	SWARM_STATUS_CONTRACT_BINARY="$(abspath $(SWARM_OUTPUT))" python3 tools/verification/recipe.py run --recipe tools/verification/recipes/agent-logs-status-rust.json --output .cache/native-process-$$(date +%s)-$$$$
