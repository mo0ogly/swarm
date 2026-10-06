"""Four quick wins; real public operations and UI, isolated labeled fixtures."""
import os,pathlib,subprocess,tempfile,json
with tempfile.TemporaryDirectory(prefix='swarm-four-qw-') as tmp:
    binary=str(pathlib.Path(tmp)/'swarm')
    commands=[['sh','build.sh',binary],
      ['node','tests/qw5_resolve_blocking_limit_ui.cjs',binary,tmp+'/budget'],
      ['node','tests/qw5_quota_entry_ui.cjs',binary,tmp+'/quotas'],
      ['python3','tests/qw6_acceptance.py'],
      ['go','test','-v','./...','-run','TestIndependentReviewStepGatesDossierBeforeProviderCall|TestControlsPrecedeReview|TestReviewEvidence','-count=1'],
      ['python3','tests/qw7_acceptance.py']]
    for command in commands:
        print('Executing',command,flush=True)
        run=subprocess.run(command,capture_output=True,text=True)
        if run.returncode:
            print(run.stdout,run.stderr,flush=True)
            raise SystemExit(run.returncode)
        if command[0]=='node':
            print(json.dumps(json.loads(run.stdout),ensure_ascii=False,separators=(',',':')),flush=True)
        else:
            lines=run.stdout.splitlines()
            for i,line in enumerate(lines):
                if 'CLI isolated mission status language=' in line:
                    print('\n'.join(lines[i:i+5]),flush=True)
                elif line.startswith('{') or 'provider_started=' in line or 'isolated engine:' in line or line.startswith('PASS four'):
                    print(line,flush=True)
        print('Recorded command exit_code=0',flush=True)
    print('PASS four QW isolated recipes: actual public CLI/UI, FR/EN and both themes, normal/error/recovery. Synthetic provider/telemetry explicitly identified; no claim of autonomous live mission completion.',flush=True)
