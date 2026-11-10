from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit
p='tests/rust_mission_worker_recovery.py'
edit(p,"        limited=request(server,'GET','/api/v1/mission?'+urlencode(dict(work=f.work,action='status')),expected=500)\n        assert limited['failure']['code']=='mission-status-planning-unported'", "        status=request(server,'GET','/api/v1/mission?'+urlencode(dict(work=f.work,action='status')))\n        actual=next(t for t in status['tasks'] if t['task']['id']=='task')['result']\n        assert actual['state']=='completed_unproven' and actual['validation_state']=='not_started'")
edit(p,"http_historical_status_limitation='mission-status-planning-unported'", "http_historical_completed_blocked_readable=True")
