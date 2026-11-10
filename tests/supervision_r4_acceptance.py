"""R4 bounded engine controls, no real paid provider call."""
import json
import subprocess
listed = subprocess.run(['go', 'test', '-list', '^TestSupervisionR4', './internal/engine'], capture_output=True, text=True)
if listed.returncode or 'TestSupervisionR4' not in listed.stdout:
    print(listed.stdout, listed.stderr)
    raise SystemExit('Missing R4 behavioral checks')
command = ['go', 'test', '-v', '-count=1', '-run', 'Test(SupervisionR4|ProviderCooldownRecoveryWaitsAndKeepsExhaustedBudget|ProviderCooldownGuardsDoNotReserveAgentsPlannerOrReviewer|ProviderCooldownUnknownNeedsExplicitAuditedClear|ProviderCooldownClearAllowsExplicitRetryButDoesNotRestoreAttempts)', './internal/engine']
print('COMMAND ' + json.dumps(command), flush=True)
r = subprocess.run(command, capture_output=True, text=True)
print(r.stdout, r.stderr, flush=True)
print('exit_code=' + str(r.returncode), flush=True)
raise SystemExit(r.returncode)
