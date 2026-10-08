#!/usr/bin/env bash
# Local cockpit lifecycle; all data stays in the selected project.
set -euo pipefail
SWARM_SOURCE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${SWARM_SOURCE_DIR}/tools/swarm_local.py" "$@"
