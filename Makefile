.PHONY: build test frontend smoke test-install
build:
	mkdir -p bin
	CGO_ENABLED=0 go build -trimpath -o bin/swarm .
test:
	go test ./...
	npm test
frontend:
	cd frontend && npm ci --ignore-scripts --no-audit --no-fund && npm run build
smoke: build
	SWARM_BINARY="$(CURDIR)/bin/swarm" python3 tests/smoke.py

# Requires a local Docker Engine and a checkout without deploy/install.env.
test-install:
	python3 tests/install_smoke.py

# Deterministic agent processes: no paid model or API calls.
.PHONY: test-process
test-process: build
	python3 tests/automatic_validation_process.py bin/swarm test-results/process-validation
	python3 tests/organized_coordination_process.py bin/swarm test-results/process-team nominal
	python3 tests/organized_coordination_process.py bin/swarm test-results/process-shared shared
	python3 tests/organized_coordination_process.py bin/swarm test-results/process-restart restart
