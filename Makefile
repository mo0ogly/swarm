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
