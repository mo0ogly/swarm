"""Proposed first-import guard, identical for immutable source16 and corrected17.

One allocated isolated interpreter per entry, no sys.modules deletion/stub/module
replacement, candidate function call, provider, child, CLI or Work mutation.
Import effects are REAL: caller must admit complete source/config/loader first.
Import failure remains its original exception; receipt failure is also incomplete.
"""
import hashlib
import importlib
import json
import os
from pathlib import Path
import sys

ENTRIES = (
    'rust_unstarted_review_process',
    'rust_unstarted_review_retention',
    'rust_unstarted_review_parent',
    'rust_unstarted_claim_prepare',
    'rust_planning_worker_public_flow',
    'rust_mission_manual_review',
    'rust_inert_source_originals',
)


def main():
    """Observe a true first import and canonical symbols, retaining loaded origins.

    Args: admitted assembly, exact entry from ENTRIES, new receipt path. Requires
    -I -S -B/nonoptimized interpreter. Return0 means import/symbol guard only;
    original failure propagates after receipt attempt, never means native readiness.
    No candidate implementation is replaced or invoked to repair initialization.
    """
    assembly = Path(sys.argv[1]).resolve(strict=True)
    entry = sys.argv[2]
    receipt = Path(sys.argv[3])
    if entry not in ENTRIES or not (assembly/'tests'/(entry+'.py')).is_file():
        raise ValueError('entry-or-source-not-admitted-not-behavioral-RED')
    if not sys.flags.isolated or not sys.flags.no_site or not sys.dont_write_bytecode or sys.flags.optimize:
        raise ValueError('interpreter-not-admitted-not-behavioral-RED')
    if any(name in sys.modules for name in ENTRIES):
        raise ValueError('candidate-not-first-import-not-behavioral-RED')
    original_path = list(sys.path)
    sys.path.insert(0, str(assembly/'tests'))
    succeeded = False
    try:
        importlib.import_module(entry)
        process = importlib.import_module('rust_unstarted_review_process')
        retention = importlib.import_module('rust_unstarted_review_retention')
        source = importlib.import_module('rust_inert_source_originals')
        flow = importlib.import_module('rust_planning_worker_public_flow')
        if process.Failure.__module__ != 'rust_unstarted_review_process':
            raise AssertionError('canonical-Failure-type-changed')
        if not all(callable(value) for value in (process.check, retention.read_at, source.planner_step,
                                                 source.worker_report, source.publish_fresh, flow.flow)):
            raise AssertionError('consumer-symbol-contract-incomplete')
        succeeded = True
    finally:
        origins = {}
        for name, module in sorted(sys.modules.items()):
            file = getattr(module, '__file__', None)
            if file and Path(file).is_file():
                path = Path(file).resolve()
                origins[name] = dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        resolver = sys.modules.get('fixture_area')
        data = dict(scope='IMPORT_CONTRACT_ONLY', first_entry=entry, succeeded=succeeded,
                    executable=sys.executable, version=sys.version, original_path=original_path, final_path=sys.path,
                    fixture_repo=str(getattr(resolver, '_REPO', None)), fixture_area_cached=str(getattr(resolver, '_AREA', None)),
                    origins=origins, candidate_functions_invoked=False)
        with receipt.open('x') as stream:
            json.dump(data, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
    print('IMPORT_CONTRACT_OK '+entry, flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
