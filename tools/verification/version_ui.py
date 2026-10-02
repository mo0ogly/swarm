#!/usr/bin/env python3
"""Build the current candidate and run its actual web journey on an isolated root."""
import pathlib, subprocess
root = pathlib.Path(__file__).resolve().parents[2]
subprocess.run(['sh', './build.sh', 'bin/swarm'], cwd=root, check=True)
subprocess.run(['node', 'tests/version_ui.cjs', 'bin/swarm', 'test-results/version-ui-engine'], cwd=root, check=True)
