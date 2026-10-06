"""Run engine and browser history checks against an isolated temporary root."""
import os
import subprocess
import tempfile

with tempfile.TemporaryDirectory(prefix="swarm-qw6-controls-") as evidence:
    env = dict(os.environ, SWARM_QW6_EVIDENCE=evidence)
    for command in [
        ["go", "test", "-v", "./...", "-run", "TestFirstTaskLaunch|TestManualTaskLimitDoes|TestMissionClosedHistory|TestClosedPlanningKeeps", "-count=1"],
        ["node", "tests/qw6_closed_history_ui.cjs", evidence],
    ]:
        print("Executing:", command, flush=True)
        subprocess.run(command, env=env, check=True)
