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

# Run and reread the offline P02 simulation. Each run needs a new directory.
.PHONY: check-p02-pilot verify-p02-pilot
P02_PILOT_EVIDENCE ?= .cache/ci-p02-pilot
export P02_PILOT_EVIDENCE
check-p02-pilot:
	mkdir -p "$$(dirname -- "$$P02_PILOT_EVIDENCE")"
	python3 tools/verification/recipe.py run --recipe tools/verification/recipes/p02-pilot-simulated.json --output "$$P02_PILOT_EVIDENCE"
	$(MAKE) verify-p02-pilot

# Verify transport evidence integrity before recalculating the closure.
verify-p02-pilot:
	python3 tools/verification/recipe.py verify --evidence "$$P02_PILOT_EVIDENCE"
	python3 -B tests/p02_pilot/run.py --verify-output "$$P02_PILOT_EVIDENCE/step-0001.stdout"
