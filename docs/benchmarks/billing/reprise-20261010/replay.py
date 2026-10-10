#!/usr/bin/env python3
"""Rejeu borné et reprenable des exclusions F4e, sur un binaire explicitement figé."""
import argparse
import hashlib
import json
import os
import sys
from pathlib import Path


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', required=True)
    p.add_argument('--analysis', required=True)
    p.add_argument('--out', required=True)
    a = p.parse_args()
    repo = Path(__file__).resolve().parents[4]
    binary = Path(a.binary).resolve()
    os.environ['BANC_SWARM_BIN'] = str(binary)
    sys.path.insert(0, str(repo / 'benchmarks/billing'))
    from bench import harness, run_s
    from bench.campaign import append
    analysis = json.loads(Path(a.analysis).read_text())
    cases = [c for c in analysis['cases'] if c['fault'] == 'F4e']
    if len(cases) != 31:
        raise SystemExit('Analyse inattendue : 31 exclusions F4e exigées')
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    identity = {'analysis_sha256': sha(a.analysis), 'binary_sha256': sha(binary),
                'bench_sha256': harness.bench_digest(harness.BENCH_DIR),
                'runner_sha256': sha(__file__)}
    previous = harness.read_jsonl(out)
    starts = [r for r in previous if r.get('event') == 'start']
    if starts and starts[0]['identity'] != identity:
        raise SystemExit('Entrées modifiées : reprise dans le même journal refusée')
    done = {(r['key_mode'], r['fault'], r['seed']) for r in previous if r.get('kind') == 'replay'}
    if any(r.get('status') != 'OK' for r in previous if r.get('kind') == 'replay'):
        raise SystemExit('Échec antérieur conservé : diagnostic nécessaire avant une nouvelle campagne')
    append(out, {'kind': 'campaign', 'event': 'start', 'identity': identity, 'planned': len(cases),
                 'not_tested': [{'fault': 'F7', 'key_mode': 'attempt', 'seed': 1010,
                                 'reason': 'preuve F7 exige une lecture SQLite directe ; accès public requis'}],
                 'roles': {'preparer': 'scripted', 'planner': 'scripted', 'reviewer': 'scripted',
                           'settlement': 'deterministic'}, 'real_llm_calls': 0})
    for c in cases:
        case = (c['key_mode'], c['fault'], c['seed'])
        if case in done:
            continue
        if sha(binary) != identity['binary_sha256'] or harness.bench_digest(harness.BENCH_DIR) != identity['bench_sha256']:
            raise SystemExit('Candidat modifié pendant la campagne : arrêt')
        try:
            result = run_s.run(*case)
            root = Path(result['run_dir']) / 'swarm'
            sw = run_s.Swarm(binary, root)
            works = sw.cli(['work', 'list'])
            if len(works) != 1:
                raise ValueError('Une seule mission attendue dans la racine isolée')
            wid = works[0]['id']
            public = {'work': sw.cli(['work', 'show', wid]),
                      'planning': sw.cli(['planning', 'show', wid]),
                      'mission': sw.cli(['mission', 'status', wid])}
            # Les tokens restent privés dans les racines ; seules les empreintes des sorties publiques sont publiées.
            public_hashes = {k: hashlib.sha256(json.dumps(v, sort_keys=True).encode()).hexdigest()
                             for k, v in public.items()}
            result.update(work_id=wid, public_projection_sha256=public_hashes)
        except Exception as e:
            result = {'condition': 'S', 'key_mode': case[0], 'fault': case[1], 'seed': case[2],
                      'status': 'ERREUR', 'error': str(e), 'run_dir': getattr(e, 'run_dir', None)}
        result.update(kind='replay', historical_status=c['status'],
                      historical_attribution=c['attribution'], identity=identity)
        append(out, result)
        print(case, result['status'], result.get('stopped_by'), flush=True)
        if result['status'] != 'OK' or result.get('cleanup', {}).get('errors'):
            append(out, {'kind': 'campaign', 'event': 'stopped', 'reason': 'diagnostic nécessaire', 'case': case})
            return 1
    append(out, {'kind': 'campaign', 'event': 'end', 'identity': identity, 'completed_f4e': len(cases)})
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
