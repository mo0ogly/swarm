"""Real engine and cockpit on explicitly synthetic, isolated telemetry."""
import os, pathlib, subprocess, tempfile
with tempfile.TemporaryDirectory(prefix='swarm-qw7-controls-') as tmp:
    env=dict(os.environ, SWARM_QW7_EVIDENCE=tmp)
    binary=str(pathlib.Path(tmp)/'swarm')
    for command in [
        ['go','test','./...','-run','TestMissionEffort|TestMissionSpending|TestAttemptLedger|Test.*I18n','-count=1','-v'],
        ['sh','build.sh',binary],
        ['node','tests/qw7_effort_ui.cjs',binary,tmp],
    ]:
        print('Executing',command,flush=True)
        subprocess.run(command,env=env,check=True)
    print('PASS: synthetic isolated telemetry; actual CLI/HTTP, keyboard opening, Escape/focus, FR/EN and both themes; no real provider. Sum 150000ms differs from overlapping elapsed 120000ms. Unknown duration/tools/cost remain unknown. Revision/events unchanged.',flush=True)
