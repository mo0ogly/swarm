"""R2 targeted engine proof; never runs the full suite or paid providers."""
import json
import subprocess
command = ['go', 'test', '-v', '-count=1', '-run', 'Test(ReviewOutput|IndependentReviewPreflightUsesLiteralObservationBeforeReservation|IndependentReviewStepGatesDossierBeforeProviderCall|ControlsPrecedeReviewAndAreNotRepeated|ReviewEvidenceRejectsImportedStaleAndWrongAttempt|ChangedPendingReportRechecksOnceWithoutNewWorker|ResumeValidationUsesAttemptWorkspace)', '.']
print('COMMAND ' + json.dumps(command), flush=True)
result = subprocess.run(command, capture_output=True, text=True)
print(result.stdout, flush=True)
if result.stderr:
    print(result.stderr, flush=True)
print('exit_code=' + str(result.returncode), flush=True)
raise SystemExit(result.returncode)
