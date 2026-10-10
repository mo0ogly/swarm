#!/usr/bin/env python3
"""Audit du rejeu terminé : observations, sorties publiques privées et bilan partageable."""
import argparse
import collections
import hashlib
import json
import os
import subprocess
from pathlib import Path


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--results', required=True)
    p.add_argument('--binary', required=True)
    p.add_argument('--out', required=True)
    p.add_argument('--cli-timeout', type=float, required=True)
    a = p.parse_args()
    rows = [json.loads(s) for s in Path(a.results).read_text().splitlines() if s.strip()]
    runs = [r for r in rows if r.get('kind') == 'replay']
    if not rows or rows[-1].get('event') != 'end' or len(runs) != 31:
        raise SystemExit('Campagne non terminée : aucun bilan complet généré')
    identities = [r['identity'] for r in rows if 'identity' in r]
    if any(i != identities[0] for i in identities) or digest(a.binary) != identities[0]['binary_sha256']:
        raise SystemExit('Identité incohérente : audit refusé')
    cases = {(r['key_mode'], r['fault'], r['seed']) for r in runs}
    if len(cases) != 31:
        raise SystemExit('Doublons dans le rejeu')
    findings = []
    for r in runs:
        expected = (r['status'] == 'OK' and r.get('stopped_by') == 'engine'
                    and r.get('prepare_validation_state') == 'stale'
                    and r.get('order_proof', {}).get('proven')
                    and not r.get('false_success') and not r.get('declared_success')
                    and r['metrics']['payments'] == r['metrics']['doubles'] == r['metrics']['wrong'] == 0
                    and r['metrics']['unpaid'] == r['metrics']['invoices']
                    and r.get('verification', {}).get('dependency_stale_events', 0) >= 1
                    and r.get('verification', {}).get('settle_attempts') == 0
                    and not r.get('cleanup', {}).get('errors'))
        root = Path(r['run_dir']) / 'swarm'
        artifact = Path(r['run_dir']) / 'replay-public-audit.json'
        public = {}
        for key, command in [('work',['work','show']), ('planning',['planning','show']), ('mission',['mission','status'])]:
            raw = subprocess.run([a.binary, '--root', str(root), '--json', *command, r['work_id']],
                                 capture_output=True, text=True, timeout=a.cli_timeout, check=True)
            public[key] = json.loads(raw.stdout)
        data = (json.dumps(public, ensure_ascii=False, indent=2) + '\n').encode()
        fd = os.open(artifact, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, 'wb') as f:
            f.write(data)
        findings.append({'key_mode':r['key_mode'], 'fault':r['fault'], 'seed':r['seed'],
                         'historical_status':r['historical_status'], 'status':r['status'],
                         'safety_observed': bool(expected), 'duration_s':r['duration_s'],
                         'dependency_stale_events':r['verification']['dependency_stale_events'],
                         'work_id':r['work_id'], 'run_dir':r['run_dir'],
                         'public_audit_sha256':digest(artifact),
                         'public_audit_path':str(artifact)})
    report = {'candidate_commit':'3faa8a9f0b6c7e606b3a7bda2c847a96060a9779',
              'result_sha256':digest(a.results), 'identity':identities[0], 'runs':len(runs),
              'states':dict(collections.Counter(r['status'] for r in runs)),
              'expected_safety_observed':sum(r['safety_observed'] for r in findings),
              'historical_timeouts_replayed':sum(r['historical_status']=='DÉLAI' for r in runs),
              'payments':sum(r['metrics']['payments'] for r in runs),
              'wrong':sum(r['metrics']['wrong'] for r in runs),
              'doubles':sum(r['metrics']['doubles'] for r in runs),
              'false_success':sum(bool(r['false_success']) for r in runs),
              'unpaid':sum(r['metrics']['unpaid'] for r in runs),
              'not_tested':rows[0]['not_tested'], 'real_llm_calls':0,
              'independent_review':'not_performed', 'findings':findings}
    Path(a.out).write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ('findings','identity')},ensure_ascii=False))
    return 0 if report['expected_safety_observed'] == 31 else 1


if __name__ == '__main__':
    raise SystemExit(main())
