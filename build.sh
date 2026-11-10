#!/bin/sh
# Canonical native Rust candidate build. No Go compiler or fallback.
set -eu
test "$#" -eq 1 || { printf 'usage: %s OUTPUT\n' "$0" >&2; exit 2; }
exec python3 "$(dirname "$0")/tools/rust-migration/build_native.py" --install-candidate "$1"
