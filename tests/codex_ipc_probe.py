#!/usr/bin/env python3
"""Run the owned Tokio signal probe with a healthy sibling before confinement.

Baseline expects the retained UnixStream panic. Patched mode requires actual
signals, child reaping, reactor recreation and unchanged Unix/filesystem denials.
No model, watchdog, retry, live root or inferred product acceptance.
"""
import argparse
from pathlib import Path
import subprocess
from fixture_area import fixture_root
from rust_agent_model_route import publish, digest


def main():
    """Freeze a supplied probe and retain exact channels, roots and observables.

    Requires fresh evidence output. IO/process/assertion failures preserve all
    preceding evidence; only explicitly named baseline panic is expected.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--mode', choices=['baseline', 'patched'], required=True)
    args = parser.parse_args()
    out = args.output.resolve();out.mkdir(mode=0o700)
    binary = args.binary.resolve()
    publish(out / 'binary.json', {'path': str(binary), 'sha256': digest(binary.read_bytes()), 'mode': args.mode})
    for mode in ['open', 'confined']:
        root = fixture_root('codex-ipc-probe-');workspace = root / 'out'
        workspace.mkdir();(root / '.swarm').mkdir()
        publish(root / 'outside-effect', b'BASELINE')
        argv = [str(binary), str(root), str(workspace), mode]
        publish(out / (mode + '.argv.json'), argv)
        run = subprocess.run(argv, input=b'', capture_output=True)
        publish(out / (mode + '.stdout'), run.stdout);publish(out / (mode + '.stderr'), run.stderr)
        publish(out / (mode + '.exit.json'), {'exit': run.returncode})
        if mode == 'confined' and args.mode == 'baseline':
            if run.returncode == 0 or b'failed to create UnixStream' not in run.stderr:
                raise ValueError('original-signal-panic-not-reproduced')
            if (root / 'outside-effect').read_bytes() != b'BASELINE':
                raise ValueError('original-panic-mutated-target')
            print('IPC_BASELINE native UnixStream panic REPRODUCED', flush=True)
        else:
            required = [b'IPC_PROBE first-reactor signal-and-child PASS',
                        b'IPC_PROBE recreated-reactor signal-and-child PASS', b'IPC_PROBE PASS']
            if mode == 'confined':
                required.append(b'IPC_PROBE unix-and-outside-refusals PASS')
            if run.returncode != 0 or run.stderr or not all(x in run.stdout for x in required):
                raise ValueError('signal-or-child-probe-failed-' + mode)
            if (workspace / 'ipc-result').read_bytes() != b'SIGNALS_AND_CHILDREN_OK\n':
                raise ValueError('probe-workspace-content-missing')
            expected = b'BASELINE' if mode == 'confined' else b'UNTRUSTED_EFFECT'
            if (root / 'outside-effect').read_bytes() != expected:
                raise ValueError('outside-consumer-content-wrong')
            print('IPC_PROBE_' + mode + ' signals/reaping/recreation/content PASS', flush=True)
    publish(out / 'result.json', {'status': 'PASS', 'mode': args.mode,
        'scope': 'Tokio signal/child component under unchanged Swarm profile, no real model'})


if __name__ == '__main__':
    main()
