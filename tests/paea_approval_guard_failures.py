#!/usr/bin/env python3
"""Maintained regression driver for the PAEA approval-guard failure paths.

Purpose
    Runs, in this exact order and in separate fresh child processes it owns:
      1. `valid-control`: the current, untouched guard
         tests/paea-approval-state-fail-closed.cjs with its real environment
         (actual CHROME_BIN default, no seam, no override) in a fresh browser
         output directory; it must exit 0 with exactly 36 checks, 0 unexpected
         diagnostics, its PASS artifact and empty stderr;
      2. `fault-listen`: a synthetic, local-only preload seam (embedded below,
         loaded only in that child via NODE_OPTIONS) makes
         net.Server.prototype.listen fail with a real-shaped in-process
         EACCES; the guard must exit 1, name the listen phase and cause on
         stderr and still preserve its failure artifact;
      3. `fault-launch`: CHROME_BIN points at an intentional nonexistent
         fixture path (never a real user path); the guard must exit 1 with
         the launch-phase cause as the primary diagnostic and close its
         fixture server cleanly;
      4. `fault-write`: the same seam refuses ONLY the failure-artifact write
         on top of the missing-Chrome launch fault; the guard must exit 1
         with BOTH the primary and the secondary cause on stderr, no failure
         artifact and no PASS.

    The exact line `PAEA_APPROVAL_GUARD_FAILURES_PASS 4` is printed on stdout
    ONLY when all four named checks hold and the guard source is byte-stable
    (same SHA-256 before and after every child run).

Inputs
    `--output PATH` (required): a FRESH directory for all retained evidence;
    an existing path must be an empty directory, anything else is refused
    before anything is written. The repository root is derived from this
    file's location; guard, web sources and locales are read, never modified.

Outputs / side effects
    Under <output>/<case>/ it retains node-out (the fresh 0700 directory
    handed to the guard), stdout.raw.txt, stderr.raw.txt, exit.json, env.json
    and, for the two seam cases, preload/seam.cjs; plus summary.json at the
    root. Unix permissions 0700/0600. Nothing is ever deleted: raw stdout,
    raw stderr, real exit codes and failure artifacts of failed cases are
    retained as proof. One child per case belongs to this driver; no user
    process is started, no repository file is modified, no live fixture is
    created, and the evidence root is wherever the caller points --output
    (never a fixed or tmp-only sole copy).

Errors
    Any failed assertion or unexpected exception prints a safe proof path and
    the cause to stderr and exits nonzero; nothing is swallowed and there is
    no destructive cleanup that could replace the primary cause.

Limits
    Python 3.8+ standard library only. No global watchdog, no deadline, no
    retry, no attempt cap, no forced kill: a hung child would hang the driver
    by design (no arbitrary time policy). No OS-level EACCES is performed or
    claimed; both refusals are injected in-process by the synthetic seam,
    which contacts no external service or provider. Launcher vendor wording
    varies, so launch-shaped causes are matched with a documented
    case-insensitive marker list, not an exact vendor string. The --output
    path must not contain whitespace (the seam travels via
    NODE_OPTIONS --require). No OS confinement, provider integration, real
    enrollment or human acceptance is claimed or performed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import traceback
from pathlib import Path
from typing import Any, Dict, List, Optional

REPO_ROOT = Path(__file__).resolve().parent.parent
GUARD_TEST = Path('tests/paea-approval-state-fail-closed.cjs')
MISSING_CHROME_FIXTURE = Path(
    'tests/fixtures/paea-approval-guard-failures/missing-chrome-fixture'
)
PASS_ARTIFACT = 'approval-state-fail-closed.json'
FAILURE_ARTIFACT = 'approval-state-fail-closed-failure.json'
PASS_MARKER = 'PAEA_APPROVAL_GUARD_FAILURES_PASS 4'
EXPECTED_CHECKS = 36
NAMED_CHECK_COUNT = 4
FAULT_ENV_VAR = 'PAEA_FAULT'
NODE_OPTIONS_VAR = 'NODE_OPTIONS'
CHROME_BIN_VAR = 'CHROME_BIN'
LISTEN_FAULT = 'listen'
WRITE_FAULT = 'artifact-write'
SEAM_LISTEN_TOKEN = 'listen EACCES'
SEAM_PROVENANCE_TOKEN = 'verification seam'
SEAM_WRITE_TOKEN = 'injected failure-artifact write refusal (verification seam)'
ARTIFACT_CAUSE_TOKEN = 'failure artifact not written'
ARTIFACT_UNWRITTEN_TOKEN = 'artifact unwritten'
LAUNCH_MARKERS = (
    'launch',
    'browser',
    'chrome',
    'chrome_bin',
    'executable',
    'spawn',
    'enoent',
)

# The seam body below is the exact small preload personally used for the
# one-off verification, re-hosted as a maintained constant of this driver;
# only the provenance header comment changed. It is written solely into this
# driver's fresh per-case evidence directory and loaded by that one child.
PRELOAD_JS = """\
'use strict';
// Maintained synthetic verification seam embedded (as a constant) in
// tests/paea_approval_guard_failures.py; it is written ONLY into this
// driver's fresh per-case evidence directory and loaded by that one child
// via NODE_OPTIONS --require. Local and private by construction: no
// external service, provider or network is contacted, and nothing outside
// the single child process is patched. Faults are selected by PAEA_FAULT
// (comma-separated): 'listen' makes net.Server.prototype.listen fail with a
// real-shaped EACCES instead of binding; 'artifact-write' makes
// fs.writeFileSync refuse only the failure artifact. Nothing else is
// patched; the seam never touches unrelated processes.
const nodeFs=require('node:fs'),nodeNet=require('node:net');
const faults=String(process.env.PAEA_FAULT||'').split(',').filter(Boolean);
if(faults.includes('listen')){
 nodeNet.Server.prototype.listen=function(...args){
  while(args.length>0&&typeof args[args.length-1]==='function')args.pop();
  const self=this;
  process.nextTick(()=>self.emit('error',Object.assign(new Error('listen EACCES 127.0.0.1:0 (injected by verification seam)'),{code:'EACCES',errno:-13,syscall:'listen',address:'127.0.0.1',port:0})));
  return self;
 };
}
if(faults.includes('artifact-write')){
 const write=nodeFs.writeFileSync;
 nodeFs.writeFileSync=function(target,...rest){
  if(String(target).endsWith('approval-state-fail-closed-failure.json'))throw Object.assign(new Error('injected failure-artifact write refusal (verification seam)'),{code:'EACCES',syscall:'open'});
  return write.call(this,target,...rest);
 };
}
"""


class DriverError(Exception):
    """Raised when a named check fails or a precondition is refused.

    The message always names the case and the concrete cause; it never
    carries secrets (only paths, exit codes and driver-authored tokens).
    """


def require(condition: bool, message: str) -> None:
    """Turn every expectation into a hard stop with a precise cause.

    Purpose: assertion primitive for the named checks; a failure must stop
    the driver, never soften into a warning or a filtered continuation.

    Inputs: condition (bool); message (human-readable cause, safe to print).

    Outputs: None.

    Errors: raises DriverError(message) exactly when condition is falsy;
    nothing is logged-and-continued.

    Side effects: none.

    Limits: message quality is the caller's duty; messages are driver
    authored and contain no secrets.
    """
    if not condition:
        raise DriverError(message)


def sha256_file(path: Path) -> str:
    """Return the lowercase SHA-256 hex digest of a file.

    Purpose: prove the guard source is byte-stable before and after every
    child run; the before/after pair is retained in summary.json.

    Inputs: path (existing file).

    Outputs: 64-character hex digest string.

    Errors: OSError propagates (never swallowed).

    Side effects: reads the file in 1 MiB chunks; modifies nothing.

    Limits: streaming digest; memory use is constant regardless of size.
    """
    digest = hashlib.sha256()
    with open(path, 'rb') as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def write_text(path: Path, text: str) -> None:
    """Write UTF-8 text with owner-only permissions.

    Purpose: keep every retained evidence file private (0600) inside the
    0700 evidence root.

    Inputs: path (file to write); text (string content).

    Outputs: None.

    Errors: OSError propagates.

    Side effects: creates or overwrites exactly this one file; never deletes
    anything.

    Limits: the 0600 mode is applied with os.chmod and may degrade on
    non-Unix platforms; the driver targets Unix-style outputs.
    """
    with open(path, 'w', encoding='utf-8', newline='') as handle:
        handle.write(text)
    os.chmod(path, 0o600)


def write_json(path: Path, payload: Dict[str, Any]) -> None:
    """Write a JSON document with a stable, readable layout.

    Purpose: retained machine-readable evidence (exit.json, env.json,
    summary.json).

    Inputs: path; payload (JSON-serialisable dict).

    Outputs: None.

    Errors: TypeError from non-serialisable payloads and OSError from
    write_text both propagate.

    Side effects: writes exactly one file via write_text (0600).

    Limits: keys are sorted; a trailing newline is appended.
    """
    write_text(path, json.dumps(payload, indent=2, sort_keys=True) + '\n')


def read_text(path: Path) -> str:
    """Read retained evidence as text for assertions.

    Purpose: give the named checks an in-memory view of raw retained output.

    Inputs: path (existing file).

    Outputs: file content as text.

    Errors: OSError propagates.

    Side effects: none.

    Limits: undecodable bytes become U+FFFD in this in-memory view only; the
    raw file on disk is never rewritten, so the proof stays intact.
    """
    return path.read_text(encoding='utf-8', errors='replace')


def read_json(path: Path) -> Dict[str, Any]:
    """Parse a retained JSON evidence file.

    Purpose: read failure artifacts written by the guard.

    Inputs: path (existing JSON file).

    Outputs: parsed object.

    Errors: DriverError naming the path and cause when the file is not valid
    JSON (never a silent None); OSError propagates.

    Side effects: none.

    Limits: expects the guard's documented artifact shape (step, message,
    stack, checks, errors, postLog); extra keys are ignored.
    """
    try:
        return json.loads(read_text(path))
    except ValueError as error:
        raise DriverError(
            'retained evidence is not valid JSON: %s (%s)' % (path, error)
        )


def node_command() -> str:
    """Resolve the `node` executable once per case.

    Purpose: fail early with a precise cause instead of a confusing spawn
    error mid-run.

    Inputs: none (uses PATH).

    Outputs: absolute path string of the node executable.

    Errors: DriverError when node is not on PATH.

    Side effects: none.

    Limits: no version pinning; the guard's own recipe governs node version.
    """
    resolved = shutil.which('node')
    require(
        resolved is not None,
        'the `node` executable was not found on PATH; refusing to start any child',
    )
    return str(Path(resolved).resolve())


def build_env(
    fault: Optional[str] = None,
    seam_path: Optional[Path] = None,
    chrome_bin: Optional[str] = None,
) -> Dict[str, str]:
    """Build one child environment by careful inheritance.

    Purpose: give each child exactly what its case needs while preventing
    harness leakage in both directions.

    Inputs: fault (PAEA_FAULT value or None); seam_path (Path of the retained
    seam.cjs or None); chrome_bin (CHROME_BIN override or None to inherit the
    actual default).

    Outputs: environment dict for subprocess.

    Errors: none raised here.

    Side effects: none (pure copy of os.environ).

    Limits: ONLY task-specific variables are touched. PAEA_FAULT (the seam's
    own switch) is always removed first so an outer experiment can never leak
    into the control or the launch fault, then set only when requested.
    NODE_OPTIONS is overridden for the child to exactly `--require=<seam>` so
    no outer --require hook leaks in, and dropped entirely when no seam is
    used. CHROME_BIN is set only for the intentional missing-fixture launch
    cases; the control keeps the actual inherited default. Every other
    variable (PATH, HOME, proxies, locale) is inherited as-is.
    """
    env = dict(os.environ)
    env.pop(FAULT_ENV_VAR, None)
    if seam_path is None:
        env.pop(NODE_OPTIONS_VAR, None)
    else:
        env[NODE_OPTIONS_VAR] = '--require=' + str(seam_path)
    if chrome_bin is not None:
        env[CHROME_BIN_VAR] = chrome_bin
    if fault is not None:
        env[FAULT_ENV_VAR] = fault
    return env


def run_child(command: List[str], env: Dict[str, str], case_dir: Path) -> int:
    """Run one guard child to completion, retaining every raw byte.

    Purpose: execute the guard exactly once per case with no filtering, no
    grep-wait, no timeout and no forced termination.

    Inputs: command (argv list); env (child environment); case_dir (evidence
    directory of this case).

    Outputs: the child's real exit code.

    Errors: OSError from subprocess propagates; a nonzero child exit is
    retained data, never raised here.

    Side effects: writes stdout.raw.txt and stderr.raw.txt (raw bytes,
    streamed straight to disk by the OS) and exit.json {command, cwd,
    exit_code} into case_dir. The child belongs to this driver; nothing is
    killed and no watchdog exists.

    Limits: output size is bounded by disk space alone; the child's console
    is captured to the two files only.
    """
    with open(case_dir / 'stdout.raw.txt', 'wb') as out, open(
        case_dir / 'stderr.raw.txt', 'wb'
    ) as err:
        completed = subprocess.run(
            command, cwd=str(REPO_ROOT), env=env, stdout=out, stderr=err
        )
    write_json(
        case_dir / 'exit.json',
        {
            'command': command,
            'cwd': str(REPO_ROOT),
            'exit_code': completed.returncode,
        },
    )
    return completed.returncode


class CaseEvidence:
    """Retained, never-deleted evidence for one finished child case.

    Attributes: name (case label), dir (case evidence directory), node_dir
    (the fresh private directory handed to the guard), exit_code (the real
    return code, retained even for failed cases).
    """

    def __init__(self, name: str, case_dir: Path, node_dir: Path, exit_code: int):
        self.name = name
        self.dir = case_dir
        self.node_dir = node_dir
        self.exit_code = exit_code

    @property
    def stdout_path(self) -> Path:
        """Path of the retained raw stdout file."""
        return self.dir / 'stdout.raw.txt'

    @property
    def stderr_path(self) -> Path:
        """Path of the retained raw stderr file."""
        return self.dir / 'stderr.raw.txt'

    def stdout(self) -> str:
        """Return the retained raw stdout text (never filtered)."""
        return read_text(self.stdout_path)

    def stderr(self) -> str:
        """Return the retained raw stderr text (never filtered)."""
        return read_text(self.stderr_path)

    def failure_artifact(self) -> Optional[Dict[str, Any]]:
        """Return the parsed failure artifact, or None when absent.

        Errors: DriverError via read_json when present but unparseable.

        Limits: absence is a meaningful result for the write fault.
        """
        path = self.node_dir / FAILURE_ARTIFACT
        if not path.exists():
            return None
        return read_json(path)

    def seam_copy(self) -> List[Path]:
        """Return the retained seam.cjs copies under this case (0 or 1)."""
        return sorted(self.dir.glob('preload-*/seam.cjs'))

    def describe(self) -> Dict[str, Any]:
        """Return the summary.json fragment for this case.

        Outputs: dict with the case directory name and the real exit code.

        Limits: paths are recorded relative to the evidence root to keep the
        summary portable.
        """
        return {'dir': self.dir.name, 'exit_code': self.exit_code}


def run_case(
    name: str,
    output: Path,
    fault: Optional[str] = None,
    with_seam: bool = False,
    chrome_bin: Optional[str] = None,
) -> CaseEvidence:
    """Run one named case end-to-end in its own fresh directories.

    Purpose: create the case's private evidence layout, optionally materialise
    the synthetic seam, run the guard child exactly once and retain its real
    stdout/stderr/exit JSON whatever the outcome; the guard source SHA-256 is
    recorded immediately before and after the child (source-before.json /
    source-after.json) and must be unchanged before return — the whole-run
    before/after assertion remains.

    Inputs: name (filesystem-safe case label); output (evidence root);
    fault (PAEA_FAULT value or None); with_seam (write and preload the
    synthetic seam); chrome_bin (CHROME_BIN override or None).

    Outputs: CaseEvidence with the real exit code.

    Errors: OSError from file or subprocess calls propagates; the child's own
    nonzero exit is data, not an error.

    Side effects: creates two 0700 directories (case, node-out) plus, when
    with_seam, preload/seam.cjs (0600); writes env.json, stdout.raw.txt,
    stderr.raw.txt, exit.json, source-before.json, source-after.json; spawns
    exactly one child owned by this driver.

    Limits: no timeout, retry, attempt cap or signal is ever sent (no global
    watchdog by design); nothing is deleted; the seam path must be
    whitespace-free because it travels via NODE_OPTIONS (validated upstream
    in prepare_output).
    """
    case_dir = Path(tempfile.mkdtemp(prefix=name + '-', dir=str(output)))
    node_dir = Path(tempfile.mkdtemp(prefix='node-out-', dir=str(case_dir)))
    seam_path = None
    if with_seam:
        preload_dir = Path(tempfile.mkdtemp(prefix='preload-', dir=str(case_dir)))
        seam_path = preload_dir / 'seam.cjs'
        write_text(seam_path, PRELOAD_JS)
    env = build_env(fault=fault, seam_path=seam_path, chrome_bin=chrome_bin)
    write_json(
        case_dir / 'env.json',
        {
            'PAEA_FAULT': env.get(FAULT_ENV_VAR),
            'CHROME_BIN': chrome_bin
            if chrome_bin is not None
            else 'inherited actual default (value not recorded)',
            'NODE_OPTIONS': env.get(NODE_OPTIONS_VAR),
        },
    )
    guard_path = REPO_ROOT / GUARD_TEST
    command = [node_command(), str(guard_path), str(node_dir)]
    import hashlib

    def guard_sha() -> str:
        """Return the SHA-256 hex digest of the guard source bytes.

        Reads the actual file bytes from disk at call time (no caching);
        I/O errors propagate to the caller; the file is not mutated.
        """
        return hashlib.sha256(guard_path.read_bytes()).hexdigest()

    sha_before = guard_sha()
    write_json(
        case_dir / 'source-before.json',
        {'schema_version': 1, 'file': str(GUARD_TEST), 'sha256': sha_before},
    )
    exit_code = run_child(command, env, case_dir)
    sha_after = guard_sha()
    write_json(
        case_dir / 'source-after.json',
        {'schema_version': 1, 'file': str(GUARD_TEST), 'sha256': sha_after},
    )
    require(
        sha_before == sha_after,
        name
        + ': the guard source must stay byte-identical during the case '
        '(sha256 %s -> %s; raw stdout/stderr/exit and both source-hash '
        'files retained in %s)' % (sha_before, sha_after, case_dir),
    )
    return CaseEvidence(name, case_dir, node_dir, exit_code)


def require_no_pass(evidence: CaseEvidence, label: str) -> None:
    """Shared no-false-PASS assertion for the three fault cases.

    Purpose: prove a fault case never produced a success signal of any kind.

    Inputs: evidence (CaseEvidence); label (case name for messages).

    Outputs: None.

    Errors: DriverError when the PASS artifact exists or the success JSON
    signature appears on stdout.

    Side effects: none.

    Limits: the signature mirrors the guard's exact success console.log keys.
    """
    require(
        not (evidence.node_dir / PASS_ARTIFACT).exists(),
        label + ': the PASS artifact must not exist',
    )
    require(
        '"unexpected_diagnostics"' not in evidence.stdout(),
        label + ': the success summary JSON must not appear on stdout',
    )


def check_valid_control(evidence: CaseEvidence) -> str:
    """Named check 1 — the valid 36-check control on the current guard.

    Purpose: prove the guard still passes with its real environment (actual
    CHROME_BIN default, no seam, no override) before any fault is injected.

    Inputs: evidence of the control child.

    Outputs: the named-check string recorded in summary.json.

    Errors: DriverError on exit code != 0, missing or mismatched success
    JSON, non-empty stderr, missing PASS artifact or an unexpected failure
    artifact.

    Side effects: none (reads retained evidence).

    Limits: expects exactly the guard's documented success contract — exit 0,
    a final stdout JSON line with checks=36 and unexpected_diagnostics=0, the
    PASS artifact in its fresh output directory and empty stderr — the same
    contract the existing permanent 36-check recipe enforces.
    """
    label = 'valid control'
    require(
        evidence.exit_code == 0,
        label
        + ': the guard must exit 0 (got %r; retained in %s)'
        % (evidence.exit_code, evidence.dir),
    )
    stdout = evidence.stdout()
    require(
        '"checks":36' in stdout,
        label + ': stdout must record 36 checks (raw stdout in %s)' % evidence.stdout_path,
    )
    require(
        '"unexpected_diagnostics":0' in stdout,
        label + ': stdout must record 0 unexpected diagnostics',
    )
    lines = [line for line in stdout.splitlines() if line.strip()]
    require(bool(lines), label + ': stdout must contain the success JSON line')
    try:
        summary = json.loads(lines[-1])
    except ValueError as error:
        raise DriverError(
            label
            + ': the final stdout line is not valid JSON (%s); raw stdout retained at %s'
            % (error, evidence.stdout_path)
        )
    require(
        summary.get('checks') == EXPECTED_CHECKS,
        label + ': the guard must record exactly %d checks' % EXPECTED_CHECKS,
    )
    require(
        summary.get('unexpected_diagnostics') == 0,
        label + ': the guard must record 0 unexpected diagnostics',
    )
    require(
        evidence.stderr() == '',
        label + ': stderr must stay empty (the permanent 36-check recipe requires the same)',
    )
    require(
        (evidence.node_dir / PASS_ARTIFACT).exists(),
        label + ': the PASS artifact must exist in the fresh output directory',
    )
    require(
        not (evidence.node_dir / FAILURE_ARTIFACT).exists(),
        label + ': no failure artifact may exist',
    )
    return (
        'valid control: exit 0, 36 checks, 0 unexpected diagnostics, PASS '
        'artifact, empty stderr (actual CHROME_BIN default, no seam)'
    )


def check_fault_listen(evidence: CaseEvidence) -> str:
    """Named check 2 — synthetic listen fault (real-shaped in-process EACCES).

    Purpose: prove the guard fails closed when its fixture server cannot
    listen: the synthetic, local-only seam (loaded only in this child via
    NODE_OPTIONS) makes net.Server.prototype.listen emit a real-shaped EACCES;
    no OS-level EACCES is performed or claimed.

    Inputs: evidence of the listen-fault child (PAEA_FAULT=listen, seam
    preloaded, actual CHROME_BIN inherited).

    Outputs: the named-check string recorded in summary.json.

    Errors: DriverError unless the exit code is exactly 1, stderr names the
    listen phase and cause (EACCES with seam provenance), exactly one seam
    copy is retained, the failure artifact is preserved with the same cause,
    and no PASS signal exists.

    Side effects: none (reads retained evidence).

    Limits: assertions match the seam's injected message tokens and the
    guard's FAIL lines; the artifact's 'step' must be exactly
    'fixture server' — a wrong-phase artifact is refused even when its
    message mentions listen/chrome — while the artifact stays retained
    verbatim for human review.
    """
    label = 'fault listen'
    require(
        evidence.exit_code == 1,
        label
        + ': the guard must exit 1 (got %r; retained in %s)'
        % (evidence.exit_code, evidence.dir),
    )
    stderr = evidence.stderr()
    require('FAIL' in stderr, label + ': stderr must keep the guard FAIL diagnostics')
    require(
        SEAM_LISTEN_TOKEN in stderr and 'EACCES' in stderr,
        label + ': stderr must name the listen phase and its EACCES cause',
    )
    require(
        SEAM_PROVENANCE_TOKEN in stderr,
        label + ': stderr must attribute the injected cause to the synthetic seam',
    )
    seams = evidence.seam_copy()
    require(
        len(seams) == 1,
        label + ': exactly one retained seam.cjs copy is expected in the case evidence',
    )
    artifact = evidence.failure_artifact()
    require(
        artifact is not None,
        label + ': the failure artifact must be preserved (fixture server error retained)',
    )
    require(
        artifact.get('step') == 'fixture server',
        label
        + ': the failure artifact step must equal fixture server '
        '(got %r; a wrong-phase artifact is refused even when its message '
        'mentions listen/chrome)' % (artifact.get('step'),),
    )
    message = str(artifact.get('message', ''))
    require(
        'listen' in message and 'EACCES' in message,
        label + ': the failure artifact must name the listen cause',
    )
    require_no_pass(evidence, label)
    return (
        'fault listen: exit 1, listen-phase EACCES cause on stderr, failure '
        'artifact preserved, no PASS (synthetic local seam only)'
    )


def check_fault_launch(evidence: CaseEvidence) -> str:
    """Named check 3 — real missing-Chrome launch fault (no seam at all).

    Purpose: prove the guard fails closed when the browser cannot launch:
    CHROME_BIN points at an intentional nonexistent fixture path inside the
    repository (never a real user path); the guard must exit 1 with the
    launch-phase cause as the primary diagnostic and close its fixture server
    cleanly.

    Inputs: evidence of the launch-fault child (no PAEA_FAULT, no seam;
    CHROME_BIN set to the missing fixture path).

    Outputs: the named-check string recorded in summary.json.

    Errors: DriverError unless the exit code is exactly 1, stderr keeps a
    primary FAIL diagnostic whose text is launch-shaped (documented marker
    list), the failure artifact records the same primary cause, no seam copy
    and no cleanup error exist, and no PASS signal exists.

    Side effects: none (reads retained evidence).

    Limits: the launcher's vendor wording varies, so launch-shaped means the
    documented case-insensitive marker list (launch/browser/chrome/
    chrome_bin/executable/spawn/enoent) over stderr plus the artifact's
    message/stack/errors; no OS-level behaviour is claimed; the intentional
    fixture path is recorded in the case env.json.
    """
    label = 'fault launch'
    require(
        evidence.exit_code == 1,
        label
        + ': the guard must exit 1 (got %r; retained in %s)'
        % (evidence.exit_code, evidence.dir),
    )
    stderr = evidence.stderr()
    require('FAIL' in stderr, label + ': stderr must keep the primary FAIL diagnostic')
    require(
        SEAM_PROVENANCE_TOKEN not in stderr,
        label + ': no synthetic seam may leak into this real launch fault',
    )
    require(
        not evidence.seam_copy(),
        label + ': no seam.cjs copy may exist for this seam-free case',
    )
    artifact = evidence.failure_artifact()
    require(
        artifact is not None,
        label + ': the failure artifact must record the launch-phase primary cause',
    )
    require(
        artifact.get('step') == 'browser launch',
        label
        + ': the failure artifact step must equal browser launch '
        '(got %r; a wrong-phase artifact is refused even when its message '
        'mentions chrome/launch)' % (artifact.get('step'),),
    )
    require(
        str(artifact.get('message', '')).strip() != '',
        label + ': the failure artifact must record a non-empty primary cause message',
    )
    blob = (
        stderr
        + '\n'
        + str(artifact.get('message', ''))
        + '\n'
        + str(artifact.get('stack', ''))
        + '\n'
        + json.dumps(artifact.get('errors', []), default=str)
    ).lower()
    require(
        any(marker in blob for marker in LAUNCH_MARKERS),
        label
        + ': the primary cause must be launch-shaped (retained stderr and artifact inspected)',
    )
    cleanup_markers = ('browser.close', 'server.close', 'fatal cleanup')
    require(
        not any(marker in blob for marker in cleanup_markers),
        label
        + ': the fixture server must close cleanly with no browser.close, '
        'server.close or fatal cleanup diagnostic (no silent cleanup failure)',
    )
    require_no_pass(evidence, label)
    return (
        'fault launch: exit 1, launch-phase primary cause, fixture server '
        'closed cleanly, no PASS (intentional nonexistent CHROME_BIN fixture path)'
    )


def check_fault_write(evidence: CaseEvidence) -> str:
    """Named check 4 — synthetic artifact-write refusal on a real launch fault.

    Purpose: prove the guard keeps its no-silent-success contract even when
    the failure artifact itself cannot be written: the seam refuses ONLY the
    failure-artifact write while the missing-Chrome launch fault supplies the
    real primary cause; the guard must exit 1 with BOTH causes on stderr,
    must not write the failure artifact and must not PASS.

    Inputs: evidence of the write-fault child (PAEA_FAULT=artifact-write,
    seam preloaded, CHROME_BIN set to the missing fixture path).

    Outputs: the named-check string recorded in summary.json.

    Errors: DriverError unless the exit code is exactly 1, the secondary
    cause (the guard's unwritable-artifact report carrying the seam's
    injected refusal message) AND an independent primary launch cause are
    both present on stderr, exactly one seam copy is retained, the failure
    artifact is absent, and no PASS signal exists.

    Side effects: none (reads retained evidence).

    Limits: the primary cause is matched with the documented launch marker
    list after excluding the tertiary artifact diagnostics; no OS-level write
    refusal is performed (the refusal is injected in-process by the seam).
    """
    label = 'fault write'
    require(
        evidence.exit_code == 1,
        label
        + ': the guard must exit 1 (got %r; retained in %s)'
        % (evidence.exit_code, evidence.dir),
    )
    stderr = evidence.stderr()
    require(
        SEAM_WRITE_TOKEN in stderr,
        label + ': the secondary artifact-write refusal cause must appear on stderr',
    )
    require(
        ARTIFACT_CAUSE_TOKEN in stderr,
        label + ': the guard must report the unwritable failure artifact on stderr',
    )
    seams = evidence.seam_copy()
    require(
        len(seams) == 1,
        label + ': exactly one retained seam.cjs copy is expected in the case evidence',
    )
    fail_lines = [line for line in stderr.splitlines() if 'FAIL' in line]
    require(bool(fail_lines), label + ': stderr must keep the FAIL diagnostics')
    primary = [
        line
        for line in fail_lines
        if ARTIFACT_CAUSE_TOKEN not in line and ARTIFACT_UNWRITTEN_TOKEN not in line
    ]
    require(
        bool(primary),
        label + ': the primary cause must appear on stderr beside the secondary artifact cause',
    )
    primary_blob = '\n'.join(primary).lower()
    require(
        any(marker in primary_blob for marker in LAUNCH_MARKERS),
        label + ': the primary cause must be the launch failure',
    )
    require(
        evidence.failure_artifact() is None,
        label + ': the failure artifact must be absent (write refused, no silent success)',
    )
    require_no_pass(evidence, label)
    return (
        'fault write: exit 1, primary launch cause and secondary '
        'artifact-write refusal both on stderr, no failure artifact, no PASS'
    )


def run_all(output: Path) -> Dict[str, Any]:
    """Run the four named checks in the mandated order and retain everything.

    Purpose: the valid control runs FIRST on the untouched current guard;
    only after it holds do the three faults run, each in its own fresh child;
    finally the guard source is proven byte-stable across the whole run.

    Inputs: output (fresh evidence root).

    Outputs: the summary dict, also written to <output>/summary.json.

    Errors: DriverError from any failed assertion stops the run immediately
    (later cases are not started and the PASS marker is never printed);
    OSError propagates; a guard source change between the before/after
    SHA-256 digests is a hard failure.

    Side effects: per-case evidence via run_case plus summary.json; writes
    nothing outside --output and never deletes anything.

    Limits: no global watchdog, deadline, retry, attempt cap or forced kill;
    the driver neither claims nor performs OS confinement, provider
    integration, real enrollment or human acceptance.
    """
    guard_path = REPO_ROOT / GUARD_TEST
    require(
        guard_path.is_file(),
        'the current guard test was not found: %s' % guard_path,
    )
    fixture_path = REPO_ROOT / MISSING_CHROME_FIXTURE
    require(
        not fixture_path.exists(),
        'the intentional missing-chrome fixture path must not exist; refusing '
        'to weaken the launch fault: %s' % fixture_path,
    )
    sha_before = sha256_file(guard_path)
    chrome_fixture = str(fixture_path)
    results = []
    cases: Dict[str, Any] = {}

    control = run_case('valid-control', output)
    results.append(check_valid_control(control))
    cases[control.name] = control.describe()

    listen_case = run_case(
        'fault-listen', output, fault=LISTEN_FAULT, with_seam=True
    )
    results.append(check_fault_listen(listen_case))
    cases[listen_case.name] = listen_case.describe()

    launch_case = run_case('fault-launch', output, chrome_bin=chrome_fixture)
    results.append(check_fault_launch(launch_case))
    cases[launch_case.name] = launch_case.describe()

    write_case = run_case(
        'fault-write',
        output,
        fault=WRITE_FAULT,
        with_seam=True,
        chrome_bin=chrome_fixture,
    )
    results.append(check_fault_write(write_case))
    cases[write_case.name] = write_case.describe()

    sha_after = sha256_file(guard_path)
    require(
        sha_after == sha_before,
        'the guard source changed during the run (before %s, after %s)'
        % (sha_before, sha_after),
    )
    require(
        len(results) == NAMED_CHECK_COUNT,
        'internal: exactly %d named checks must be recorded' % NAMED_CHECK_COUNT,
    )
    summary = {
        'schema_version': 1,
        'kind': 'paea-approval-guard-failures-summary-v1',
        'status': 'pass',
        'pass_marker': PASS_MARKER,
        'named_check_count': len(results),
        'named_checks': results,
        'cases': cases,
        'guard': {
            'path': str(GUARD_TEST),
            'sha256_before': sha_before,
            'sha256_after': sha_after,
        },
        'seam': {
            'sha256': hashlib.sha256(PRELOAD_JS.encode('utf-8')).hexdigest(),
            'synthetic': True,
            'local_private_only': True,
        },
        'limits': [
            'no global watchdog, deadline, retry, attempt cap or forced kill',
            'the seam is synthetic and local-only; no external service or '
            'provider is contacted',
            'no OS-level EACCES is performed or claimed; refusals are '
            'injected in-process',
            'every raw output is retained and nothing is ever deleted',
        ],
    }
    write_json(output / 'summary.json', summary)
    return summary


def prepare_output(raw: str) -> Path:
    """Validate --output and create the fresh private evidence root (0700).

    Purpose: honour the FRESH contract; an existing path must be an empty
    directory, anything else is refused BEFORE anything is written; a missing
    path is created private instead of assuming a fixed tmp-only location.

    Inputs: raw --output string.

    Outputs: resolved absolute Path of the evidence root.

    Errors: DriverError for a non-directory, a non-empty directory, or a
    whitespace-containing path (whitespace would break NODE_OPTIONS
    --require for the seam); OSError from mkdir/chmod propagates.

    Side effects: creates the directory tree when absent; chmod 0700 on the
    root; nothing is deleted.

    Limits: 0700 semantics are Unix-style; the driver targets Unix outputs.
    """
    path = Path(raw).expanduser().resolve()
    require(
        not any(char.isspace() for char in str(path)),
        '--output path must not contain whitespace (the seam travels via '
        'NODE_OPTIONS --require); refused: %s' % path,
    )
    if path.exists():
        require(
            path.is_dir(),
            '--output exists and is not a directory; refused: %s' % path,
        )
        require(
            not any(path.iterdir()),
            '--output must be FRESH: a non-empty directory is refused; %s' % path,
        )
    else:
        path.mkdir(parents=True)
    os.chmod(path, 0o700)
    return path


def parse_args(argv: List[str]) -> argparse.Namespace:
    """Parse the driver CLI.

    Purpose: the only argument is --output, the fresh evidence root.

    Inputs: argv list (without the program name).

    Outputs: argparse.Namespace with `output`.

    Errors: SystemExit(2) from argparse when --output is missing (nonzero,
    nothing written).

    Side effects: none.

    Limits: freshness and whitespace rules are enforced later by
    prepare_output so the refusal can name the exact path.
    """
    parser = argparse.ArgumentParser(
        prog='paea_approval_guard_failures.py',
        description=(
            'Maintained PAEA approval-guard failure regression: 1 valid '
            'control + 3 real exit-1 faults, all raw evidence retained.'
        ),
    )
    parser.add_argument(
        '--output',
        required=True,
        help='fresh (nonexistent or empty) directory for all retained evidence',
    )
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point; returns the process exit code.

    Purpose: prepare the fresh evidence root, run all four named checks and
    emit the single PASS marker ONLY when every assertion held and the guard
    source stayed byte-stable.

    Inputs: optional argv list (defaults to sys.argv[1:]).

    Outputs: 0 with the exact stdout line `PAEA_APPROVAL_GUARD_FAILURES_PASS
    4` on full success; nonzero with a safe proof path and the cause on
    stderr on any failure.

    Errors: nothing is swallowed; every exception is printed with its cause
    and traceback, a failed run never prints the PASS marker and never
    removes evidence.

    Side effects: see prepare_output and run_all; the failure summary is
    written best-effort AFTER the cause is printed, so cleanup can never
    replace the primary cause.

    Limits: exit code 2 for CLI/freshness refusals, 1 for any failed check or
    unexpected error.
    """
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        output = prepare_output(args.output)
    except DriverError as error:
        print('FAIL [paea-approval-guard-failures]: %s' % error, file=sys.stderr)
        return 2
    try:
        run_all(output)
    except Exception as error:  # surfaced below; never swallowed
        cause = '%s: %s' % (type(error).__name__, error)
        print('FAIL [paea-approval-guard-failures]: %s' % cause, file=sys.stderr)
        print(
            'proof: %s (raw stdout/stderr/exit JSON of every case is '
            'retained; nothing deleted)' % output,
            file=sys.stderr,
        )
        traceback.print_exc(file=sys.stderr)
        try:
            write_json(
                output / 'summary.json',
                {
                    'schema_version': 1,
                    'kind': 'paea-approval-guard-failures-summary-v1',
                    'status': 'fail',
                    'cause': cause,
                    # completed count is unknown when run_all aborts; never
                    # invent NAMED_CHECK_COUNT completed checks on failure.
                    'expected_named_check_count': NAMED_CHECK_COUNT,
                    'completed_named_check_count': None,
                },
            )
        except OSError as summary_error:
            print(
                'note: the failure summary could not be written: %s'
                % summary_error,
                file=sys.stderr,
            )
        return 1
    print(PASS_MARKER)
    return 0


if __name__ == '__main__':
    sys.exit(main())
