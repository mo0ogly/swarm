from pathlib import Path
import sys,json
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit,atomic
p='tests/rust_mission_worker_recovery.py'
edit(p,"        server=native.Server(binary,root,out,'reopened',environment=f.environment)","        assert f.ok(['mission','show',f.work])['consent_enabled'] is True\n        public.mission_control(f,'stop','status-read-quiesce')\n        server=native.Server(binary,root,out,'reopened',environment=f.environment)")
edit(p,"        assert view['consent_enabled'] is True","        assert view['consent_enabled'] is False")
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260')
atomic(b/'replan1270.json',json.dumps(dict(original='recipe1269/step-0001.stderr',cause='historical aggregate detects advancing owned supervision heartbeat during status read; exact retained cause mission-status-supervision-changed',observed='absence/fresh/ambiguity/restoration already reached; failure at saved zero margin read',decision='Separate status selection test from live writer: public mission stop after recovery and before fresh web server, preserved consent revocation; no retry or snapshot guard weakening',owner='Codex G1/G3',scope_limit='Does not establish all reads during a concurrent heartbeat; existing fail-closed coherence contract preserved'),indent=2)+'\n')
