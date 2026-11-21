#!/usr/bin/env python3
"""Exercise a Wasm result through Swarm's managed validation gate on inert roots."""
import argparse
import json
import os
import shutil
from pathlib import Path

from rust_agent_model_route import digest, publish
from rust_native_task_validation import invoke
import rust_managed_auto_worker as managed


# (module (func (export "check") (param i32) (result i32)
#   local.get 0 i32.const 7 i32.eq))
MODULE = bytes.fromhex(
    "0061736d01000000 01060160017f017f 03020100 "
    "07090105636865636b0000 0a0901070020004107460b"
)


def scenario(binary, profile, out, settings):
    """Exercise one module against two genuinely submitted managed candidates."""
    out.mkdir()
    control = dict(id='wasm-decision',
                   command=['python3', '-c',
                            "import os; from pathlib import Path; "
                            "value = '8' if Path('docs/task-2.md').is_file() else '7'; "
                            "os.execvp('swarm-wasm-validation', "
                            "['swarm-wasm-validation', 'rule.wasm', value, '100'])"],
                   criteria=[1], inputs=['rule.wasm'], review_output=True,
                   justification='Verify one deterministic Wasm predicate under the native gate')
    workers = out / 'workers'
    managed.worker.subreaper_scope(
        lambda: managed.run(binary, profile, workers, controls=[control],
                            source_entries={'rule.wasm': MODULE},
                            settings_values=settings, check_fifo=False),
        lambda error: None, managed.parent_death.subreaper,
    )
    birth = json.loads((workers / 'fixture.json').read_bytes())
    root, work_id = Path(birth['root']), birth['work']
    environment = dict(os.environ, CODEX_HOME=str(root / 'codex-home'))

    def call(label, args, data=None):
        return invoke(binary, root, out, label, args, data, environment=environment)

    def show(label):
        code, response = call(label, ['work', 'show', work_id])
        assert code == 0, response
        return response['work']

    def resume(operation, event):
        state = show(event + '-before')
        code, response = call(event, [operation, 'resume', work_id], {
            'schema_version': 1, 'event_id': event, 'expected_revision': state['revision'],
            'reason': 'Owned fixture Wasm validation',
        })
        assert code == 0, response

    code, mission = call('mission', ['mission', 'show', work_id])
    assert code == 0, mission
    if mission['paused']:
        resume('mission', 'wasm-mission-resume')
    if show('planning-state')['planning'].get('paused'):
        resume('planning', 'wasm-planning-resume')

    results = {}
    for case, task_id, expected_exit in [('nominal', 'task', 0), ('refused', 'task-2', 1)]:
        before = show(case + '-before')
        request = dict(schema_version=1, event_id='wasm-control-' + case,
                       expected_revision=before['revision'], task_id=task_id)
        code, result = call(case + '-validate', ['task', 'validate', work_id], request)
        if expected_exit == 0:
            assert code == 0 and result['controls_passed'] and not result['task_accepted'], result
        else:
            assert code != 0 and 'task-validation-controls-failed' in str(result), result
        after = show(case + '-after')
        original, current = (next(t for t in view['tasks'] if t['id'] == task_id)
                             for view in (before, after))
        receipt = current['automatic_validation']
        run = receipt['controls'][0]
        assert run['executed'] and run['exit_code'] == expected_exit, run
        assert run['passed'] == (expected_exit == 0), run
        assert current['status'] == ('submitted' if expected_exit == 0 else 'blocked'), current['status']
        assert current['attempts'] == original['attempts']
        assert [t for t in before['tasks'] if t['id'] != task_id] == [
            t for t in after['tasks'] if t['id'] != task_id
        ]
        results[case] = dict(state=receipt['state'], status=current['status'], control=run)
        print('WASMTIME_NATIVE_' + case.upper() + '_PASS', flush=True)
    return results


def main():
    """Bind the controlled binary and retained qualified Swarm profile to both cases."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'profile', 'control', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(mode=0o700)
    # The fixture authorizes the basename by the normal settings operation.
    # Use an owned local executable directory, never a path on removable media.
    from fixture_area import fixture_root
    tools = fixture_root('wasm-control-tools-')
    program = tools / 'swarm-wasm-validation'
    shutil.copyfile(args.control, program)
    program.chmod(0o700)
    control_sha = digest(args.control.read_bytes())
    assert digest(program.read_bytes()) == control_sha
    original_path = os.environ.get('PATH')
    os.environ['PATH'] = str(tools) + os.pathsep + (original_path or os.defpath)
    try:
        settings = dict(validation_allowed_programs=[
            'cargo', 'git', 'go', 'node', 'npm', 'pytest', 'python', 'python3', 'rustc',
            'swarm-wasm-validation',
        ])
        binary, profile = args.binary.resolve(), args.profile.resolve()
        results = scenario(binary, profile, out / 'managed', settings)
        assert digest(program.read_bytes()) == control_sha
        publish(out / 'result.json', dict(
            status='PASS', **results, accepted_task=False,
            swarm_sha256=digest(binary.read_bytes()), control_sha256=control_sha,
            module_sha256=digest(MODULE),
        ))
    finally:
        if original_path is None:
            os.environ.pop('PATH', None)
        else:
            os.environ['PATH'] = original_path


if __name__ == '__main__':
    main()
