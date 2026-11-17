"""Observe strace's own refusal only; the traced primary stays UNKNOWN.

Sibling of the reviewed trace_process_witness nucleus (caller pins that source;
reviewed basis SHA256 8f8be5fff1762bb542b325585ababcf1b21fc2730e35e26ba9c983b4666d7e71).
No parser, controller, storage layer, capture plan, retry, timeout or overwrite
is added: one control, one refusal fixture, one recovery, all through the
existing executor and positive observer. The caller owns source/tool pins, a
fresh private 0700 leaf under umask 077, qualified storage with held
descriptors/admission and the external 20-GiB host/output guard.
"""
import json
import os
from pathlib import Path

import trace_process_witness as base


def run_tracer_refusal(out, admission, strace, python):
    """Observe one strace self-refusal while the primary outcome stays UNKNOWN.

    Preconditions: ``out`` is an absolute, currently empty, mode-0700 leaf on
    qualified storage under umask 077; ``strace`` and ``python`` are absolute
    pinned executables; ``admission`` is the caller's held admission carrying
    the qualified-volume descriptors and the external 20-GiB guard; no
    overwrite, retry, timeout or capability change exists on this path.

    Sequence: a fresh code-0 control via base.observe_case (which also
    publishes the declared worker), then a fresh fault leaf where base.publish
    creates the regular file ``refusal/blocker`` and strace is run with
    ``-o refusal/blocker/trace`` — an output parent that is a regular file —
    over that worker with fully known argv/env (-s derived from the exact
    strings, -xx), through base.outcomes.execute exactly, with base.publish,
    base.sha and the caller admission; then a fresh code-0 recovery and a
    byte-identity check of every fault receipt.

    Outputs: a result dict (also published as result.json) recording the
    control and recovery cases verbatim and a refusal entry whose
    primary_outcome is always 'UNKNOWN' and capture_complete always False,
    with the observed positive-nonzero tracer returncode retained verbatim,
    recorded stdout-empty/stderr-nonempty facts and channel hashes (content is
    never parsed for error words), blocker/trace observations, and
    complete_build_qualification/behavioral_admission both False.

    Errors: only the single expected ValueError('fixture-tracer-nonzero') is
    accepted, requiring execution_attempted, no signal, no additional issues,
    successful receipt publication and post-execution admission, and channel
    hashes matching the private files; the full original failure_document is
    published first, and any extra cause is blocking and retained. Later validation/recovery/publication
    failures group the original refusal with a consumer-visible native carrier;
    primary status stays UNKNOWN even when a channel hash fails or a
    BaseException interruption occurs; interruptions remain in BaseExceptionGroup. A tracer
    exit that is not positive-nonzero, changed blocker bytes, a stray trace
    file, non-0600 files, or fault receipts changed by recovery each refuse
    with ValueError.

    Effects/limits: writes only exclusive 0600 files inside ``out``; admission
    is re-checked at entry, before the fault and before publication. A refusal
    or error never proves the absence of side effects and never states whether
    or how the traced worker executed; signals, forks, other fault modes,
    trace semantics and OS/full-graph isolation remain unqualified.
    """
    out = Path(out)
    base.require(out.is_absolute() and out.is_dir() and not list(out.iterdir()),
                 'trace-refusal-new-empty-leaf')
    base.require(out.stat().st_mode & 0o777 == 0o700, 'trace-refusal-private-leaf')
    strace, python = Path(strace), Path(python)
    base.require(strace.is_absolute() and python.is_absolute(),
                 'trace-refusal-explicit-executables')
    admission.check()
    nonce = 'TRACER_OWN_REFUSAL_WITNESS'
    control = base.observe_case(out / 'control', admission, strace, python, 0, nonce)

    fault = out / 'refusal'
    fault.mkdir(mode=0o700)
    base.require(fault.stat().st_mode & 0o777 == 0o700, 'trace-refusal-private-leaf')
    blocker = fault / 'blocker'
    base.publish(blocker, 'refusal-blocker-parent-is-regular-file\n')
    blocker_before = base.sha(blocker)
    argv = [str(python), str(out / 'control' / 'worker.py'), nonce, '0']
    env = {'LC_ALL': 'C', 'PYTHONUTF8': '1', 'PYTHONCOERCECLOCALE': '0'}
    # Same exact-strings sizing rule as the reviewed control fixture.
    string_size = max(len(os.fsencode(s)) for s in argv + [f'{k}={v}' for k, v in env.items()])
    command = [str(strace), '-ff', '-v', '-xx', '-s', str(string_size),
               '-e', 'trace=execve,exit,exit_group', '-o', str(blocker / 'trace'),
               '--', *argv]
    failure = None
    child = None

    def with_custody(later, phase):
        """Group a later failure or interruption with the known tracer refusal.

        The consumer carrier retains both native receipts, not only error
        wording. This grants no primary status: it remains UNKNOWN/incomplete.
        """
        combined = BaseExceptionGroup('Tracer refusal and later observation failed', [failure, later])
        combined.native_outcome = {
            'earlier_expected_tracer_refusal': getattr(failure, 'native_outcome', None),
            'following_failure': base.outcomes.failure_document(later),
            'phase': phase, 'primary_outcome': 'UNKNOWN', 'capture_complete': False}
        return combined
    try:
        admission.check()
        child = base.outcomes.execute(command, out=fault, name='tracer', admission=admission,
                                      publish=base.publish, digest=base.sha,
                                      failure_code='fixture-tracer-nonzero', cwd=fault, env=env)
    except BaseException as error:
        failure = error
        try:
            base.publish(fault / 'tracer-error.json',
                         json.dumps(base.outcomes.failure_document(error), indent=2) + '\n')
        except BaseException as publication:
            raise with_custody(publication, 'native-refusal-document-publication') from None
    try:
        details, receipt = {}, {}
        if failure is not None:
            details = getattr(failure, 'native_outcome', {})
            receipt = details.get('subprocess', {})
            tracer_rc = receipt.get('returncode')
        else:
            tracer_rc = child.returncode
        if failure is not None:
            guard = (type(failure) is ValueError
                     and str(failure) == 'fixture-tracer-nonzero'
                     and failure.__cause__ is None and failure.__context__ is None
                     and receipt.get('execution_attempted') is True
                     and receipt.get('signal') is None
                     and receipt.get('issues') == [
                         {'type': 'ValueError', 'message': 'fixture-tracer-nonzero'}]
                     and details.get('receipt_publication') == {'ok': True}
                     and details.get('admission_after', {}).get('checked') is True
                     and details.get('admission_after', {}).get('ok') is True
                     and 'diagnostic_publication' not in details
                     and receipt.get('stdout_sha256') == base.sha(fault / 'tracer.stdout')
                     and receipt.get('stderr_sha256') == base.sha(fault / 'tracer.stderr'))
            if not guard:
                raise ValueError('trace-refusal-unexpected-native-issues') from failure
        # The tracer's own numeric refusal: retain the actual code; the primary's
        # status is never invented from it, and stdout/stderr content is never parsed.
        base.require(type(tracer_rc) is int and tracer_rc > 0, 'trace-refusal-tracer-outcome')
        stdout_empty = (fault / 'tracer.stdout').stat().st_size == 0
        stderr_nonempty = (fault / 'tracer.stderr').stat().st_size > 0
        base.require(blocker.is_file() and base.sha(blocker) == blocker_before,
                     'trace-refusal-blocker-changed')
        base.require(all(p.name != 'trace' and not p.name.startswith('trace.')
                         for p in fault.iterdir()), 'trace-refusal-stray-trace')
        base.require(all(p.stat().st_mode & 0o777 == 0o600 for p in fault.iterdir() if p.is_file()),
                     'trace-refusal-private-files')
        retained = {str(p.relative_to(fault)): base.sha(p) for p in fault.iterdir() if p.is_file()}

        recovery = base.observe_case(out / 'recovery', admission, strace, python, 0, nonce)
        base.require(retained == {str(p.relative_to(fault)): base.sha(p)
                                  for p in fault.iterdir() if p.is_file()},
                     'trace-refusal-original-fault-changed')
        admission.check()
        result = {
            'scope': 'tracer-own-refusal-primary-unknown',
            'control': control,
            'refusal': {
                'case': fault.name,
                'fixture': 'strace-output-parent-is-regular-file',
                'worker': 'declared-worker-reused-from-control-leaf',
                'expected_tracer_failure_code': 'fixture-tracer-nonzero',
                'assertions': ['sole-expected-valueerror', 'tracer-nonzero-positive-numeric',
                               'no-signal-no-additional-issues', 'receipt-and-admission-publication',
                               'channel-hash-match', 'blocker-bytes-unchanged', 'no-stray-trace',
                               'private-files-0600', 'fault-receipts-immutable-across-recovery'],
                'execution_attempted': receipt.get('execution_attempted'),
                'tracer_returncode': tracer_rc,
                'primary_outcome': 'UNKNOWN',
                'capture_complete': False,
                'observed_stdout_empty': stdout_empty,
                'observed_stderr_nonempty': stderr_nonempty,
                'stdout_sha256': receipt.get('stdout_sha256'),
                'stderr_sha256': receipt.get('stderr_sha256'),
                'blocker_sha256': blocker_before,
                'usable_trace_files': [],
                'native_receipts': ['tracer-result.json', 'tracer-admission-after.json'],
                'failure_document': 'tracer-error.json',
                'failure_document_sha256': base.sha(fault / 'tracer-error.json'),
                'expected_refusal_retained': True,
            },
            'recovery': recovery,
            'fault_receipts_unchanged_after_fresh_recovery': True,
            'complete_build_qualification': False,
            'behavioral_admission': False,
            'unqualified': ['primary-execution-and-exit-status',
                            'trace-capture-completeness-and-semantics', 'stderr-message-content',
                            'signals', 'forks-and-arbitrary-producers', 'other-fault-injection-modes',
                            'compiler-file-input-closure', 'full-process-graph-or-os-isolation'],
        }
        base.publish(out / 'result.json', json.dumps(result, indent=2) + '\n')
        return result
    except BaseException as later:
        if failure is not None:
            raise with_custody(later, 'post-refusal-validation-recovery-or-publication') from None
        raise
