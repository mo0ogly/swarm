"""Observe only three declared inert Python processes under strace.

This is a transport nucleus, not compiler/file-input qualification or a generic
trace parser. The caller owns source pins, a fresh private output directory,
qualified storage/held descriptors and the unchanged 20-GiB host/output guard.
Signals, forks, tracer failures and publication faults are not qualified here.
"""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import sys

import native_build_outcome as outcomes


WORKER = """import json, os, sys
print(json.dumps({'pid':os.getpid(),'argv':sys.argv,'env':dict(os.environ)}), flush=True)
sys.exit(int(sys.argv[2]))
"""
STRING = r'"(?:\\x[0-9a-fA-F]{2})*"'
EXEC = re.compile(r'^execve\((' + STRING + r'), (\[.*\]), (\[.*\])\) = 0$')


def sha(path):
    """Hash current bytes; propagate IO failures, establishing no provenance."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(condition, reason):
    """Refuse a fixture mismatch even under python -O, without dumping raw data."""
    if not condition:
        raise ValueError(reason)


def publish(path, text):
    """Exclusively write/sync a private UTF-8 receipt, retaining cleanup causes.

    Existing paths refuse. IO failure may leave a visible partial or complete
    file; no rollback, overwrite or retry is attempted. Directory-sync failures
    also propagate. Callers must keep their storage hold through this operation.
    """
    stream = None
    errors = []
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        try:
            stream = os.fdopen(descriptor, 'w', encoding='utf-8')
        except BaseException as primary:
            try:
                os.close(descriptor)
            except BaseException as cleanup:
                raise BaseExceptionGroup('Receipt stream and close failed', [primary, cleanup])
            raise
        stream.write(text)
        stream.flush()
        os.fsync(stream.fileno())
    except BaseException as error:
        errors.append(error)
    finally:
        if stream is not None:
            try:
                stream.close()
            except BaseException as error:
                errors.append(error)
    if errors:
        raise errors[0] if len(errors) == 1 else BaseExceptionGroup('Receipt issues', errors)
    fd = os.open(Path(path).parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    except BaseException as primary:
        try:
            os.close(fd)
        except BaseException as cleanup:
            raise BaseExceptionGroup('Receipt directory sync and close failed', [primary, cleanup])
        raise
    os.close(fd)


def hex_array(value):
    """Decode only complete strace -xx arrays of byte strings.

    Unexpected tokens, abbreviations and malformed arrays refuse; this accepts
    no general strace grammar or descendant-generated data contract.
    """
    values = re.findall(STRING, value)
    require(value == '[' + ', '.join(values) + ']', 'trace-fixture-array-syntax')
    return [ast.literal_eval('b' + token) for token in values]


def observe_case(out, admission, strace, python, code, nonce):
    """Run one declared no-fork worker and compare its trace and primary marker.

    Requires a new private leaf and held admission. Native outcome receipts bind
    the directly spawned tracer; only an exact traced primary PID/exec/terminal
    plus matching stdout establishes this fixture's primary exit. Nonzero tracer
    outcomes retain their original error receipt. Any mismatch propagates with
    the tracer result in the exception cause; missing primary evidence is never
    interpreted as no execution. No input-read/window-stability claim is made.
    """
    out.mkdir(mode=0o700)
    publish(out / 'worker.py', WORKER)
    argv = [str(python), str(out / 'worker.py'), nonce, str(code)]
    env = {'LC_ALL': 'C', 'PYTHONUTF8': '1', 'PYTHONCOERCECLOCALE': '0'}
    # The fixture has no descendants and all its argv/env strings are known.
    # This is exact fixture-data allocation, not a bound on arbitrary builds.
    string_size = max(len(os.fsencode(s)) for s in argv + [f'{k}={v}' for k, v in env.items()])
    command = [str(strace), '-ff', '-v', '-xx', '-s', str(string_size),
               '-e', 'trace=execve,exit,exit_group', '-o', str(out / 'trace'), '--', *argv]
    failure = None
    try:
        admission.check()
        child = outcomes.execute(command, out=out, name='tracer', admission=admission,
                                 publish=publish, digest=sha, failure_code='fixture-tracer-nonzero',
                                 cwd=out, env=env)
        tracer_rc = child.returncode
    except Exception as error:
        failure = error
        primary_receipt = getattr(error, 'native_outcome', {}).get('subprocess', {})
        tracer_rc = primary_receipt.get('returncode')
        publish(out / 'tracer-error.json', json.dumps(outcomes.failure_document(error), indent=2) + '\n')
    try:
        require(tracer_rc == code, 'trace-fixture-tracer-outcome')
        require((failure is None) == (code == 0), 'trace-fixture-nonzero-cause')
        if failure is not None:
            details = failure.native_outcome
            require(type(failure) is ValueError and str(failure) == 'fixture-tracer-nonzero'
                    and failure.__cause__ is None and failure.__context__ is None
                    and primary_receipt.get('execution_attempted') is True
                    and primary_receipt.get('signal') is None
                    and primary_receipt.get('issues') == [
                        {'type': 'ValueError', 'message': 'fixture-tracer-nonzero'}]
                    and details.get('receipt_publication') == {'ok': True}
                    and details.get('admission_after', {}).get('checked') is True
                    and details.get('admission_after', {}).get('ok') is True
                    and 'diagnostic_publication' not in details
                    and primary_receipt.get('stdout_sha256') == sha(out / 'tracer.stdout')
                    and primary_receipt.get('stderr_sha256') == sha(out / 'tracer.stderr'),
                    'trace-fixture-unexpected-native-issues')
        marker = json.loads((out / 'tracer.stdout').read_bytes())
        require(type(marker.get('pid')) is int, 'trace-fixture-primary-pid')
        require(marker.get('argv') == argv[1:] and marker.get('env') == env,
                'trace-fixture-marker-mismatch')
        traces = sorted(out.glob('trace.*'))
        require(traces == [out / ('trace.' + str(marker['pid']))], 'trace-fixture-unexpected-tree')
        lines = traces[0].read_text().splitlines()
        executions = [EXEC.fullmatch(line) for line in lines if line.startswith('execve(')]
        require(len(executions) == 1 and executions[0] is not None, 'trace-fixture-exec-syntax')
        executable, arguments, environment = executions[0].groups()
        require(ast.literal_eval('b' + executable) == os.fsencode(python), 'trace-fixture-executable')
        require(hex_array(arguments) == [os.fsencode(s) for s in argv], 'trace-fixture-argv')
        require(sorted(hex_array(environment)) == sorted(os.fsencode(f'{k}={v}') for k, v in env.items()),
                'trace-fixture-environment')
        require(lines[-1] == f'+++ exited with {code} +++', 'trace-fixture-primary-terminal')
        require(all(p.stat().st_mode & 0o777 == 0o600 for p in out.iterdir() if p.is_file()),
                'trace-fixture-private-files')
        admission.check()
    except Exception as error:
        if failure is not None:
            raise error from failure
        raise
    return {'case': out.name, 'assertions': ['tracer-numeric', 'primary-marker', 'one-primary-pid',
            'exact-exec-argv-environment', 'primary-terminal'], 'expected_fixture_exit': code,
            'tracer_returncode': tracer_rc, 'observed_primary_exit': code,
            'trace_sha256': sha(traces[0]), 'native_receipt_sha256': sha(out / 'tracer-result.json'),
            'expected_nonzero_error_retained': failure is not None}


def run_witness(out, admission, strace, python):
    """Observe healthy→explicit exit7→fresh recovery with immutable fault receipts.

    Caller supplies a private process with umask077, an empty mode0700 leaf on
    held qualified storage, absolute
    strace/Python executables, exact source pins and external guard checks. This
    invokes three inert Python processes through the existing native executor;
    it never invokes Cargo, installed Swarm, providers or service operations.
    Refusals propagate with original native causes/partial evidence retained.
    Result qualifies only this declared no-fork fixture transport; signals,
    missing/corrupt capture, tracer/IO faults and arbitrary trees remain open.
    """
    out = Path(out)
    require(out.is_absolute() and out.is_dir() and not list(out.iterdir()), 'trace-fixture-new-empty-leaf')
    require(out.stat().st_mode & 0o777 == 0o700, 'trace-fixture-private-leaf')
    strace, python = Path(strace), Path(python)
    require(strace.is_absolute() and python.is_absolute(), 'trace-fixture-explicit-executables')
    admission.check()
    nonce = 'INERT_PROCESS_TRACE_WITNESS'
    cases = [observe_case(out / 'healthy', admission, strace, python, 0, nonce),
             observe_case(out / 'exit-seven', admission, strace, python, 7, nonce)]
    fault = out / 'exit-seven'
    before = {str(p.relative_to(fault)): sha(p) for p in fault.iterdir() if p.is_file()}
    cases.append(observe_case(out / 'recovery', admission, strace, python, 0, nonce))
    require(before == {str(p.relative_to(fault)): sha(p) for p in fault.iterdir() if p.is_file()},
            'trace-fixture-original-fault-changed')
    result = {'scope': 'inert-process-trace-transport-nucleus', 'cases': cases,
              'fault_receipts_unchanged_after_fresh_recovery': True,
              'complete_build_qualification': False, 'behavioral_admission': False,
              'unqualified': ['signals', 'tracer-fault', 'capture-corruption', 'publication-cleanup-fault',
                              'forks-and-arbitrary-producers', 'compiler-file-input-closure']}
    admission.check()
    publish(out / 'result.json', json.dumps(result, indent=2) + '\n')
    return result
