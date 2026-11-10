#!/usr/bin/env python3
"""Prove command success survives failed CLI delivery without another control.

The owned Root uses a real inert worker and actual control. /dev/full is an
explicit output fault, not a full user volume. No policy, SQL or history bypass,
model account, global watchdog or implicit task acceptance is used.
"""
import argparse, json, subprocess, traceback
from pathlib import Path
import rust_native_task_validation as native


def scenario(binary, out):
    """Refuse an authority-bearing request without mutation, then lose delivery
    after real execution and durable publication. Exact replay must recover the
    original result and witness, including the known Work failure diagnostic.
    """
    setup = native.public.flow(binary, out, control_script=native.CONTROL)
    root = Path(setup['root']); work = setup['work_id']
    r = dict(schema_version=1, event_id='validation-output', expected_revision=setup['revision'], task_id='task')
    code, preview = native.invoke(binary, root, out, 'preview', ['task', 'validation-preview', work], r)
    assert code == 0 and not preview['execution_authorized']
    code, before = native.invoke(binary, root, out, 'before', ['work', 'show', work]); assert code == 0
    code, refused = native.invoke(binary, root, out, 'injected-command', ['task', 'validate', work],
                                  dict(r, command=['python3', '-c', 'raise SystemExit(0)']))
    assert code != 0 and 'task-validation-request-invalid' in refused['failure']['causes'], refused
    code, after = native.invoke(binary, root, out, 'unchanged', ['work', 'show', work])
    assert code == 0 and after == before and not (root / 'validation-executed.json').exists()
    inp = out / 'output-fault.input.json'; native.fixture.publish(inp, r)
    argv = [str(binary), 'task', 'validate', work, '--root', str(root), '--input', str(inp), '--json']
    native.fixture.publish(out / 'output-fault.command.json', dict(argv=argv, stdout='/dev/full', global_watchdog=None))
    with open('/dev/full', 'wb', buffering=0) as full:
        result = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=full, stderr=subprocess.PIPE)
    native.fixture.publish(out / 'output-fault.stderr', result.stderr)
    failure = json.loads(result.stderr)
    assert result.returncode != 0 and failure['failure']['code'] == 'storage_unavailable', failure
    assert work in failure['failure']['message'] and 'révision' in failure['failure']['message'], failure
    marker = (root / 'validation-executed.json').read_bytes()
    code, committed = native.invoke(binary, root, out, 'readback', ['work', 'show', work]); assert code == 0
    task = committed['work']['tasks'][0]
    assert task['status'] == 'submitted' and task['automatic_validation']['state'] == 'pending_review'
    code, replay = native.invoke(binary, root, out, 'replay', ['task', 'validate', work], r)
    assert code == 0 and replay['replayed'] and replay['controls_passed'] and not replay['task_accepted'] and not replay['retired'], replay
    assert replay['work'] == committed['work'] and (root / 'validation-executed.json').read_bytes() == marker
    code, retired = native.invoke(binary, root, out, 'cannot-retire-published', ['task', 'validation-retire', work], r)
    assert code != 0 and 'task-validation-already-published' in retired['failure']['causes'], retired
    assert (root / 'validation-executed.json').read_bytes() == marker
    return dict(status='PASS', source_command_refused=True, failed_output_preserves_commit=True,
                replay_without_execution=True, published_result_not_retired=True,
                accepted=False, real_provider=False, installed=False)


def main():
    """Retain exact binary and imported local test inputs before/after the flow."""
    import sys
    p = argparse.ArgumentParser(description=__doc__); p.add_argument('--binary', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path); a = p.parse_args()
    binary = a.binary.resolve(); out = a.output.resolve(); out.mkdir(exist_ok=False)
    sources = {str(Path(m.__file__)): native.fixture.digest(Path(m.__file__).read_bytes())
               for m in list(sys.modules.values()) if getattr(m, '__file__', None)
               and str(Path(m.__file__).resolve()).startswith(str(Path('tests').resolve()))}
    sources[str(Path(__file__))] = native.fixture.digest(Path(__file__).read_bytes())
    inputs = dict(binary=str(binary), binary_sha256=native.fixture.digest(binary.read_bytes()), sources=sources)
    native.fixture.publish(out / 'inputs.json', inputs)
    try:
        result = scenario(binary, out)
        assert native.fixture.digest(binary.read_bytes()) == inputs['binary_sha256']
        assert all(native.fixture.digest(Path(n).read_bytes()) == h for n, h in sources.items())
        native.fixture.publish(out / 'result.json', result); print(json.dumps(result))
    except BaseException:
        native.fixture.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc())); raise


if __name__ == '__main__': main()
