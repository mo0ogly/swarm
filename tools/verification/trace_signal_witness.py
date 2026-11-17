"""Observe one declared inert primary SIGTERM, with control and recovery.

Uses the already reviewed transport's publisher/hex decoder and native outcome
helper. Caller qualification/holds/umask077/pins/20GiB guards remain required.
No compiler, arbitrary process-tree or tracer-failure qualification is provided.
"""
import ast
import json
import os
from pathlib import Path
import signal
import sys

import trace_process_witness as base


WORKER = """import json, os, signal, sys
print(json.dumps({'pid':os.getpid(),'argv':sys.argv,'env':dict(os.environ)}), flush=True)
if sys.argv[1] == 'signal':
    os.kill(os.getpid(), signal.SIGTERM)
"""


def observe(out, admission, strace, python, mode):
    """Observe a fixed control or self-SIGTERM worker and both terminal results.

    The existing native receipt records the tracer, independently of the exact
    primary PID/exec/env/terminal trace. Installed strace documentation promises
    the same terminating signal; this fixture verifies that actual mapping.
    Expected signal accepts only the sole numeric helper issue; additional
    causes, unknown primary trace or inconsistent channels refuse with chaining.
    All raw files remain private and retained, including upon refusal. No claim
    about bytes read, exe integrity at exec time or other signals is made.
    """
    base.require(mode in ('control', 'signal'), 'signal-fixture-mode')
    out.mkdir(mode=0o700)
    base.publish(out / 'worker.py', WORKER)
    argv = [str(python), str(out / 'worker.py'), mode, 'INERT_SIGTERM_TRANSPORT']
    env = {'LC_ALL': 'C', 'PYTHONUTF8': '1', 'PYTHONCOERCECLOCALE': '0'}
    size = max(len(os.fsencode(x)) for x in argv + [f'{k}={v}' for k, v in env.items()])
    command = [str(strace), '-ff', '-v', '-xx', '-s', str(size), '-e',
               'trace=execve,exit,exit_group', '-o', str(out / 'trace'), '--', *argv]
    expected_signal = int(signal.SIGTERM) if mode == 'signal' else None
    expected_tracer = -expected_signal if expected_signal is not None else 0
    failure = None
    native = None
    try:
        admission.check()
        process = base.outcomes.execute(command, out=out, name='tracer', admission=admission,
                                       publish=base.publish, digest=base.sha,
                                       failure_code='signal-fixture-tracer-nonzero', cwd=out, env=env)
        tracer_rc = process.returncode
    except Exception as error:
        failure = error
        native = getattr(error, 'native_outcome', {})
        tracer_rc = native.get('subprocess', {}).get('returncode')
        base.publish(out / 'tracer-error.json', json.dumps(base.outcomes.failure_document(error), indent=2) + '\n')
    try:
        base.require(tracer_rc == expected_tracer, 'signal-fixture-tracer-numeric')
        base.require((failure is None) == (mode == 'control'), 'signal-fixture-cause-presence')
        if failure is not None:
            original = native.get('subprocess', {})
            base.require(type(failure) is ValueError and str(failure) == 'signal-fixture-tracer-nonzero'
                         and failure.__cause__ is None and failure.__context__ is None
                         and original.get('execution_attempted') is True
                         and original.get('signal') == expected_signal
                         and original.get('issues') == [
                             {'type': 'ValueError', 'message': 'signal-fixture-tracer-nonzero'}]
                         and native.get('receipt_publication') == {'ok': True}
                         and native.get('admission_after', {}).get('checked') is True
                         and native.get('admission_after', {}).get('ok') is True
                         and 'diagnostic_publication' not in native
                         and original.get('stdout_sha256') == base.sha(out / 'tracer.stdout')
                         and original.get('stderr_sha256') == base.sha(out / 'tracer.stderr'),
                         'signal-fixture-additional-native-issues')
        marker = json.loads((out / 'tracer.stdout').read_bytes())
        base.require(type(marker.get('pid')) is int and marker.get('argv') == argv[1:]
                     and marker.get('env') == env, 'signal-fixture-primary-marker')
        traces = sorted(out.glob('trace.*'))
        base.require(traces == [out / ('trace.' + str(marker['pid']))], 'signal-fixture-one-primary')
        lines = traces[0].read_text().splitlines()
        executions = [base.EXEC.fullmatch(line) for line in lines if line.startswith('execve(')]
        base.require(len(executions) == 1 and executions[0] is not None, 'signal-fixture-exec-syntax')
        executable, arguments, environment = executions[0].groups()
        base.require(ast.literal_eval('b' + executable) == os.fsencode(python)
                     and base.hex_array(arguments) == [os.fsencode(x) for x in argv]
                     and sorted(base.hex_array(environment)) == sorted(
                         os.fsencode(f'{k}={v}') for k, v in env.items()), 'signal-fixture-exact-exec')
        terminal = '+++ killed by SIGTERM +++' if mode == 'signal' else '+++ exited with 0 +++'
        base.require(lines[-1] == terminal, 'signal-fixture-primary-terminal')
        base.require(all(p.stat().st_mode & 0o777 == 0o600 for p in out.iterdir() if p.is_file()),
                     'signal-fixture-private-files')
        admission.check()
    except Exception as error:
        if failure is not None:
            raise error from failure
        raise
    return {'mode': mode, 'tracer_returncode': tracer_rc, 'primary_pid': marker['pid'],
            'primary_exit_code': 0 if mode == 'control' else None, 'primary_signal': expected_signal,
            'primary_terminal': terminal, 'trace_sha256': base.sha(traces[0]),
            'native_receipt_sha256': base.sha(out / 'tracer-result.json')}


def run_signal_witness(out, admission, strace, python):
    """Observe control→self-SIGTERM→fresh recovery on caller-held storage.

    Requires Linux, an empty private0700 leaf, absolute pinned tools, umask077,
    stable source/helper bytes, qualified FD holds and caller reserve checks.
    Only these three inert workers may be launched; no other process is killed.
    Retains signal receipts unchanged through recovery. Refusal preserves causes
    and partial evidence; result has no compiler or behavioral admission effect.
    """
    out, strace, python = Path(out), Path(strace), Path(python)
    base.require(sys.platform == 'linux', 'signal-fixture-linux-only')
    base.require(out.is_absolute() and out.is_dir() and not list(out.iterdir())
                 and out.stat().st_mode & 0o777 == 0o700, 'signal-fixture-private-empty-leaf')
    base.require(strace.is_absolute() and python.is_absolute(), 'signal-fixture-explicit-tools')
    admission.check()
    cases = [observe(out / 'healthy', admission, strace, python, 'control'),
             observe(out / 'sigterm', admission, strace, python, 'signal')]
    fault = out / 'sigterm'
    before = {p.name: base.sha(p) for p in fault.iterdir() if p.is_file()}
    cases.append(observe(out / 'recovery', admission, strace, python, 'control'))
    base.require(before == {p.name: base.sha(p) for p in fault.iterdir() if p.is_file()},
                 'signal-fixture-original-signal-receipts-changed')
    result = {'scope': 'inert-declared-primary-sigterm-only', 'cases': cases,
              'signal_receipts_unchanged_after_recovery': True,
              'complete_build_qualification': False, 'behavioral_admission': False,
              'unqualified': ['other-signals', 'tracer-failure', 'corrupt-capture', 'forks',
                              'arbitrary-environments', 'file-input-closure', 'compiler-qualification']}
    admission.check()
    base.publish(out / 'result.json', json.dumps(result, indent=2) + '\n')
    return result
