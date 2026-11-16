#!/usr/bin/env python3
"""Source-only proposed minimal managed preparation; NOT execution evidence.

Future scope: actual checkout/profile admission, one owned Root with its declared
bootstrap Work and one managed target Work, two inert planner steps, one owned
mission-watch worker/report, explicit managed control and read-only review inputs.
Stop before reviewer W, SDK or F. No acceptance, reset, retry or global watchdog.
All fixture budgets are explicit test data, not operator defaults or real prices.

This private candidate remains incomplete until the separate watch-ownership
correction, whole-source review and independent review are received. Source/API
inspection and AST parsing prove neither imports nor runtime effects. A future
outer allocation must restore the exact normal CLI locally, revalidate sources,
volume identity, space/reservations and rights. Never execute its external proof
alias. Old ordinary Work/failure and historical evidence remain untouched.
"""

import argparse
import importlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys

NEEDLE = "proposal=json.load(open('planning-proposal.json'))"
TRACKED_BYTES = b'Committed planning input' + bytes([10])
REPORT_MARKER = 'MANAGED-I0-REPORT'
EVENT_STARTED = 'mission-worker-started'
EVENT_EXITED = 'mission-worker-exited'
EVENT_SUBMITTED = 'mission-report-submitted'
EVENT_WAITING = 'mission-report-waiting'
REQUIRED_TRANSPORT = ('Failure', 'OwnedRun', 'report_terminal')
PRODUCER_MODULES = ('rust_agent_model_route', 'fixture_area', 'rust_planning_bootstrap',
                    'rust_managed_repository_bootstrap', 'rust_gieth_effective_dependency',
                    'rust_managed_planning')

CONTROL_CODE = (
    "from pathlib import Path; "
    "tracked = Path('tracked.txt').read_bytes(); "
    "report = Path('docs/task.md').read_text(encoding='utf-8'); "
    "assert tracked == " + repr(TRACKED_BYTES) + ", tracked; "
    "assert " + repr(REPORT_MARKER) + " in report, report[:200]; "
    "print('managed-i0-control-read-report-and-source')"
)

WORKER_PROGRAM = r'''#!/usr/bin/python3
import sys, os, json, hashlib
raw = sys.stdin.buffer.read()
args = sys.argv[1:]
observed = dict(argv=args, pid=os.getpid(), ppid=os.getppid(),
                prompt_sha256=hashlib.sha256(raw).hexdigest(), prompt_bytes=len(raw))
os.makedirs('docs', exist_ok=True)
body = chr(10).join(['# MANAGED-I0-REPORT', '',
                     'prompt_sha256=' + observed['prompt_sha256'],
                     'prompt_bytes=' + str(observed['prompt_bytes']),
                     'worker_pid=' + str(observed['pid']),
                     'workspace=' + os.getcwd(), ''])
with open('docs/task.md', 'x', encoding='utf-8') as stream:
    stream.write(body)
    stream.flush()
    os.fsync(stream.fileno())
print(json.dumps({'type': 'system', 'subtype': 'init', 'model': 'fixture-managed-i0'}), flush=True)
print(json.dumps({'type': 'assistant', 'message': {'model': 'fixture-managed-i0',
      'content': [{'type': 'text', 'text': body}]}}), flush=True)
print(json.dumps({'type': 'result', 'usage': {'input_tokens': 7, 'output_tokens': 8}}), flush=True)
'''


class SourceMissing(Exception):
    '''A required actual supplied source could not be resolved for preparation.

    Raised instead of guessing any API, module name, proposal format or
    ownership implementation. code is a stable static reason; facts carry
    only observations (names, counts, paths exactly as supplied). This is a
    source-stage refusal, never runtime evidence about the product.
    '''

    def __init__(self, code, facts=None):
        '''Retain the static reason and the observation facts; performs no IO.'''
        super().__init__(code)
        self.code = code
        self.facts = dict(facts or {})


def _retain(errors, thunk):
    '''Run one cleanup or observation step, retaining its failure cause.

    Mirrors the supplied suite discipline: the thunk exception is appended to
    errors and never raised here, so cleanup can never mask a primary failure
    and cleanup failures can never be lost. No retry is attempted.
    '''
    try:
        thunk()
    except BaseException as error:
        errors.append(error)


def _origin_entry(module, fixture):
    '''Record importlib real origin path and sha256 for one imported module.

    Read-only recording of the actual supplied file bytes; a missing or
    non-file origin (for example a namespace package) is retained as None
    rather than guessed. Never rewrites, copies or re-resolves anything.
    '''
    spec = getattr(module, '__spec__', None)
    origin = getattr(spec, 'origin', None) or getattr(module, '__file__', None)
    entry = {'origin': str(origin) if origin else None}
    if origin and Path(origin).is_file():
        entry['sha256'] = fixture.digest(Path(origin).read_bytes())
    return entry


def load_modules(source_root, parent_transport_name=None, feedback_module_name=None):
    '''Import the actual supplied checkout producers through the explicit source root.

    Purpose: resolve every dependency from the caller-provided tests directory
    so a candidate module living outside the checkout still uses the real
    supplied implementations, and retain real module-origin hashes plus the
    root argument for review.

    Precondition: source_root is the existing tests directory of the supplied
    checkout (a caller argument, never derived from __file__). The parent
    transport module name must be supplied: the authoring evidence quoted its
    Failure/OwnedRun/report_terminal source without a filesystem path, so the
    preparer refuses (SourceMissing) instead of guessing a name or inventing
    ownership.

    Outputs: (modules dict, transport module, feedback PROGRAM template
    admitted with exactly one literal needle, origins document). Side
    effects: inserts source_root at the front of sys.path and imports the
    named modules (their own top level runs; nothing else executes).
    Errors: SourceMissing with the exact missing source for absent
    directories, failed imports, incomplete transport attributes, or a
    PROGRAM template whose needle count is not exactly one. Limits: import
    side effects are those of the supplied modules themselves; no spoofing,
    patching or copying is performed.
    '''
    root = Path(source_root)
    if not root.is_dir():
        raise SourceMissing('source-root-missing', {'source_root': str(root)})
    sys.path.insert(0, str(root))
    modules = {}
    for name in PRODUCER_MODULES:
        try:
            modules[name] = importlib.import_module(name)
        except BaseException as error:
            raise SourceMissing('producer-module-import-refused', {'name': name}) from error
    if not parent_transport_name:
        raise SourceMissing('parent-transport-unnamed', {
            'required': 'module exposing Failure, OwnedRun, report_terminal',
            'note': 'the supplied Gi W/F parent transport source reached the preparer author '
                    'without its path; supply its actual module name'})
    try:
        transport = importlib.import_module(parent_transport_name)
    except BaseException as error:
        raise SourceMissing('parent-transport-import-refused', {'name': parent_transport_name}) from error
    missing = [attribute for attribute in REQUIRED_TRANSPORT if not hasattr(transport, attribute)]
    if missing:
        raise SourceMissing('parent-transport-incomplete', {'name': parent_transport_name, 'missing': missing})
    fixture = modules['rust_agent_model_route']
    entries = dict((name, _origin_entry(module, fixture)) for name, module in modules.items())
    entries[parent_transport_name] = _origin_entry(transport, fixture)
    origins = {'schema_version': 1, 'source_root_argument': str(root), 'modules': entries}
    template = _feedback_template(modules['rust_managed_planning'], fixture,
                                  feedback_module_name, origins)
    # Existing imported/cached modules must resolve inside this actual tests
    # directory. Hash observations alone would not establish that condition.
    for name, entry in origins['modules'].items():
        origin = entry.get('origin')
        if origin is None:
            raise SourceMissing('producer-origin-missing', {'module': name})
        try:
            Path(origin).resolve(strict=True).relative_to(root.resolve(strict=True))
        except (OSError, ValueError) as error:
            raise SourceMissing('producer-origin-outside-source-root', {'module': name}) from error
    # Inventory actual repository-local transitive Python imports as well;
    # this remains an input inventory, not interpreter or OS authentication.
    transitive = {}
    for name, module in list(sys.modules.items()):
        origin = getattr(getattr(module, '__spec__', None), 'origin', None)
        if not origin or not Path(origin).is_file():
            continue
        actual = Path(origin).resolve(strict=True)
        if actual.is_relative_to(root.resolve(strict=True).parent):
            transitive[name] = _origin_entry(module, fixture)
    origins['repository_local_transitive_modules'] = transitive
    return modules, transport, template, origins


def _feedback_template(managed, fixture, feedback_module_name, origins):
    '''Resolve the canonical feedback PROGRAM containing exactly one literal needle.

    Resolution order: an explicit --feedback-module name first, then the real
    `feedback` attribute already used by rust_managed_planning (its own code
    renders feedback.PROGRAM and replaces the needle). The chosen template is
    admitted only when PROGRAM is a string containing exactly one needle; the
    module origin is recorded. SourceMissing names every attempted source and
    the observed needle counts; no substitute program is ever invented.
    '''
    candidates = []
    if feedback_module_name:
        try:
            candidates.append(importlib.import_module(feedback_module_name))
        except BaseException as error:
            raise SourceMissing('feedback-module-import-refused', {'name': feedback_module_name}) from error
    attribute = getattr(managed, 'feedback', None)
    if attribute is not None and hasattr(attribute, 'PROGRAM'):
        candidates.append(attribute)
    counts = {}
    for candidate in candidates:
        program = getattr(candidate, 'PROGRAM', None)
        name = getattr(candidate, '__name__', 'candidate')
        counts[name] = program.count(NEEDLE) if isinstance(program, str) else None
        if isinstance(program, str) and program.count(NEEDLE) == 1:
            origins['modules'][name] = _origin_entry(candidate, fixture)
            return program
    raise SourceMissing('feedback-program-needle', {
        'needle': NEEDLE, 'observed_counts': counts,
        'attempted': ['--feedback-module argument', 'rust_managed_planning.feedback attribute']})


def resolve_proposal_helper(managed):
    '''Return the actual proposal publisher that rust_managed_planning itself uses.

    That module submits root and child proposals through its module-level
    name `proposal` (observed call sites: proposal(f, work, scope, steps));
    the preparer reuses exactly that callable and never re-implements or
    guesses the planning-proposal.json format. Absence or a non-callable
    raises SourceMissing naming the exact unresolved source.
    '''
    helper = getattr(managed, 'proposal', None)
    if not callable(helper):
        raise SourceMissing('managed-planning-proposal-helper', {
            'module': 'rust_managed_planning', 'attribute': 'proposal',
            'note': 'definition not present in the preparer author evidence; '
                    'refusing to invent the proposal format'})
    return helper


def frozen_program(template, proposal):
    '''Render the planning provider program with the current proposal frozen inside.

    The canonical feedback PROGRAM reads the proposal through exactly one
    literal needle `proposal=json.load(open('planning-proposal.json'))`;
    planning I0 cannot read the Root copy, so the needle is replaced by
    `proposal=json.loads(<repr>)` of the canonical JSON of the actual published proposal document,
    frozen before each planning claim. Returns (program_text,
    proposal_literal_bytes). The needle count is re-verified; any drift
    raises SourceMissing rather than rendering a partial or double-spliced
    program. No file is written here.
    '''
    if template.count(NEEDLE) != 1:
        raise SourceMissing('planning-program-needle-drift', {'count': template.count(NEEDLE)})
    literal = json.dumps(proposal, sort_keys=True, ensure_ascii=False)
    program = template.replace(NEEDLE, 'proposal=json.loads(' + repr(literal) + ')')
    if NEEDLE in program:
        raise SourceMissing('planning-program-needle-residue', {})
    return program, literal.encode('utf-8')


def managed_source_repository(f, repository, fixture, checkout, out):
    '''Create the tiny committed managed source inside the owned fixture Root.

    Purpose: give planning and the managed lane a real committed repository
    without ever writing to the user checkout. Preconditions: f.root exists;
    checkout is the supplied repository whose git config supplies user.name
    and user.email (read-only discovery through
    rust_managed_repository_bootstrap.git, whose contract allows read-only
    configuration discovery only). Behaviour: creates f.root/source (mode
    0700), runs git init --quiet, writes one tracked file, git add, and one
    quiet commit passing the discovered identity by -c together with
    core.hooksPath=/dev/null and commit.gpgsign=false; publishes the base
    revision and a full file inventory. Returns the base revision string.
    Errors: git failures propagate with their retained bytes inside
    CalledProcessError; a missing identity raises SourceMissing instead of
    inventing one. Limits: exactly one commit; no hooks, no signing, no dirty
    private content, and no later source replacement while children run.
    '''
    source = f.root / 'source'
    source.mkdir(mode=0o700)
    repository.git(source, 'init', '--quiet')
    name = repository.git(checkout, 'config', 'user.name').decode('utf-8').strip()
    email = repository.git(checkout, 'config', 'user.email').decode('utf-8').strip()
    if not name or not email:
        raise SourceMissing('git-identity-missing', {'checkout': str(checkout),
            'note': 'the owned fixture commit refuses to invent an identity'})
    fixture.publish(source / 'tracked.txt', TRACKED_BYTES)
    repository.git(source, 'add', '--', 'tracked.txt')
    repository.git(source, '-c', 'user.name=' + name, '-c', 'user.email=' + email,
                   '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false',
                   'commit', '--quiet', '-m', 'Owned managed I0 fixture source')
    base = repository.git(source, 'rev-parse', 'HEAD').decode('utf-8').strip()
    fixture.publish(out / 'managed-source.json', dict(schema_version=1, base=base,
        inventory=repository.inventory(source),
        identity='discovered read-only from the checkout config and applied by -c inside the owned fixture only'))
    return base


def work_revision(f, work_id):
    '''Return the actual current work document through the public show command.

    Every revision-bearing request is built from this fresh observation, so
    expected_revision is never replayed or guessed. CLI failures propagate
    with their retained original bytes.
    '''
    return f.ok(['work', 'show', work_id])['work']


def mutate(f, work_id, tail, event, reason=None, task_id='task'):
    '''Issue one revisioned public CLI mutation against the live revision.

    Reads work show first, builds schema_version 1 with event_id and the
    actual expected_revision, includes task_id unless task_id is None, and
    attaches reason when given. Returns the parsed success reply; refusals
    raise through f.ok with the original stderr JSON retained by
    Fixture.call. No retry, no revision guessing, no hidden fallback.
    '''
    request = dict(schema_version=1, event_id=event,
                   expected_revision=work_revision(f, work_id)['revision'])
    if task_id is not None:
        request['task_id'] = task_id
    if reason is not None:
        request['reason'] = reason
    return f.ok(list(tail) + [work_id], request)


def resume_if_paused(f, kind, work_id, event, reason):
    '''Resume exactly this paused mission or planning through the public CLI.

    Mirrors the supplied validation admission, which resumes a paused mission
    and paused planning before candidate preparation and review-input
    inspection. The paused flag is read live (mission show, or the work
    planning document); no request is issued when not paused. Returns the
    reply or None. Resume requests carry no task_id.
    '''
    if kind == 'mission':
        paused = f.ok(['mission', 'show', work_id])['paused']
    else:
        paused = bool(work_revision(f, work_id)['planning'].get('paused', False))
    if not paused:
        return None
    return mutate(f, work_id, [kind, 'resume'], event, reason=reason, task_id=None)


def planning_claim(f, helper, fixture, out, work_id, index, scope, steps, template):
    """Publish actual pending-event proposal, freeze its whole document, then step.

    Reads the current public Work and passes that document to the established
    proposal producer; retains its exact output before rendering the canonical
    inert PROGRAM. No task/claim runs until the literal document is saved and
    installed. Source-stage proposal only; CLI/shape errors propagate unchanged.
    """
    current = work_revision(f, work_id)
    helper(f, current, scope, steps)
    proposal_raw = (f.root / 'planning-proposal.json').read_bytes()
    proposal = json.loads(proposal_raw)
    assert isinstance(proposal, dict) and proposal['operations'] == steps
    expected = [e['id'] for e in current['planning']['inbox']
                if e['scope'] == scope and not e.get('decision')]
    assert proposal['input_events'] == expected
    program, literal = frozen_program(template, proposal)
    fixture.publish(out / ('planning-claim-%02d.proposal.json' % index), proposal_raw)
    fixture.publish(out / ('planning-claim-%02d.literal.json' % index), literal)
    fixture.publish(out / ('planning-claim-%02d.program.py' % index), program.encode('utf-8'))
    fixture.publish(f.root / 'codex', program.encode('utf-8'))
    os.chmod(f.root / 'codex', 0o700)
    return f.ok(['planning', 'step', work_id])


# Bounded replacement packet for mission_watch_handoff only: one local owner
# subclass plus this function. The supplied transport module
# (rust_unstarted_review_process.py) is used exactly as-is; no API is invented.
def mission_watch_handoff(transport, fixture, f, work_id, out):
    '''Run ONE foreground engine-owned mission watch until one immutable handoff.

    Purpose: obtain the single managed worker and its report handoff through the
    watch own dispatch and relay authority; a manual agent.prepare worker is
    deliberately never used. Event grammar: exactly the four events emitted by
    rust-cli/src/mission_watch.rs (mission-worker-started, mission-worker-exited,
    mission-report-submitted, mission-report-waiting); every other emitted event
    is retained in records untouched and only checked for mission_complete.

    Preconditions: the mission is started and not paused for work_id; planning is
    paused so no extra planner can be dispatched; the fixture provider f exposes
    binary/root/environment with the inert worker program; no other conductor
    runs for this Root.

    Behaviour: spawns exactly (binary, --root, root, --json, mission, watch, work)
    under one local PipedWatchRun owner. Both regular rawfiles are opened and
    appended to run.streams BEFORE spawning, in the base OwnedRun capture-name
    order (stdout.raw then stderr.raw; the child stderr is run.streams[1]).
    Immediately after Popen the stdout PIPE is appended to run.streams and marked
    as a pipe BEFORE launch.json publication, pidfd_open and any mirroring, so
    pending(), summary(), finish_capture(), settle() and report_terminal() always
    see it while it is held. Every stdout chunk is mirrored byte-for-byte into
    stdout.raw and parsed per line; an unparsable line retains the original typed
    parser exception plus a sha256; a parsed frame without a string event field is
    an invalid frame. Success requires exactly one started, exactly one exited
    with exit_code 0 and exit_signal None, exactly one submitted, all for this
    work and the same agent_id in started-exited-submitted order, no waiting or
    refusal, no unparsable or invalid line, no record claiming mission_complete,
    a delivered SIGINT, the pipe observed closed, and the whole numeric watch
    exit zero with the owner reaped, wait-validated, finalized and free of
    pending effects or errors. After the first submission, or immediately on any
    incomplete condition, the parent sends exactly one supported SIGINT to this
    owned child only (signal.pidfd_send_signal on the held pidfd, else os.kill on
    the owned unreaped PID; never a synthetic PID and never Popen.send_signal);
    a failed send retains its typed original cause and can never become success,
    including after the following drain. The parent then drains stdout to EOF,
    classifies the tail by splitting on the actual LF byte (trailing empty piece
    dropped, never fsync on a pipe), records the pipe close in its own
    close-state (a failed close keeps the actual stream in run.streams and its
    typed cause for the terminal fail-stop path), publishes events.json, then
    consumes the actual whole numeric exit through OwnedRun.settle (wait_kernel
    discipline: waitid WNOWAIT then consume; a pid mismatch is an error; ECHILD
    is never read as success; Popen.poll/wait/communicate are never called).
    OwnedRun.settle asserts neither errors nor finalization, so this function
    inspects reaped, wait_validated, finalized, pending() and both error lists
    explicitly before any success.

    Outputs: dict(events, agent_id, revision, settlement, stdout_pipe_close) only
    on the genuine single healthy handoff. Errors: transport.Failure(
    managed-watch-settlement-unknown, primary+cleanup+signal+settle causes,
    owner=run) when the settlement step itself fails, otherwise transport.Failure(
    managed-watch-incomplete, primary+cleanup+signal+owner causes, owner=run,
    facts=problems); the caller terminal path (report_terminal) then settles or
    explicitly holds the owner with every original cause, never masking the
    primary. Side effects: evidence bytes under out; one SIGINT to the owned
    watch; fixture state only through prior public calls. Limits: no timeout, no
    sleep, no retry, no polling; a watch that ignores SIGINT blocks the parent by
    the declared no-timeout policy until an external operator intervenes.
    '''
    EVENT_STARTED = 'mission-worker-started'
    EVENT_EXITED = 'mission-worker-exited'
    EVENT_SUBMITTED = 'mission-report-submitted'
    EVENT_WAITING = 'mission-report-waiting'
    WATCH_EVENTS = (EVENT_STARTED, EVENT_EXITED, EVENT_SUBMITTED, EVENT_WAITING)

    class PipedWatchRun(transport.OwnedRun):
        """Own one pipe alongside raw files, preserving a one-attempt receipt.

        The inherited pending/summary/terminal protocol sees every stream. The
        pipe is never flushed/fsynced or closed twice, and observed closure
        never fabricates a successful close return. Source, not runtime proof.
        """

        def __init__(self, output, argv, environment, observe=False):
            '''Allocate the base owner plus an empty pipe marker list.'''
            super().__init__(output, argv, environment, observe)
            self.pipes = []
            self.pipe_close = dict(close_attempted=False, close_returned=False,
                                   closed_observed=None, state='not-created')

        def mark_pipe(self, stream):
            '''Register one stream as an owned pipe: flushed, never fsynced.'''
            self.pipes.append(stream)

        def _is_pipe(self, stream):
            '''Identity test against the marked pipe streams.'''
            return any(stream is marked for marked in self.pipes)

        def close_pipe_once(self):
            """Attempt the owned pipe close once and keep actual typed outcomes.

            A failed/interrupted close is never retried here. The owner retains
            the stream and its receipt for pending()/terminal fail-stop checks.
            Observed closure never fabricates a successful close return.
            """
            state = self.pipe_close
            if state['close_attempted'] or not self.pipes:
                return
            stream = self.pipes[0]
            state.update(close_attempted=True, state='unknown')
            try:
                stream.close()
                state['close_returned'] = True
            except BaseException as error:
                self.errors.append(error)
            try:
                state['closed_observed'] = bool(stream.closed)
                if state['closed_observed']:
                    state['state'] = 'closed'
            except BaseException as error:
                self.errors.append(error)

        def finish_capture(self):
            """Capture once after known reap, reusing the pipe close receipt.

            Raw files are flushed/fsynced/closed with original causes retained.
            The pipe is never flushed/fsynced or closed twice. Unknown closure
            keeps finalized false and the actual owner pending at the terminal.
            """
            transport.check(self.process is None or self.reaped, 'parent-close-before-settlement')
            if self.capture_attempted:
                return
            self.stream_closures = [dict(close_attempted=False, close_returned=False,
                                         closed_observed=None, state='open')
                                    for _ in self.streams]
            self.capture_attempted = True
            for stream, state in zip(self.streams, self.stream_closures):
                if self._is_pipe(stream):
                    self.close_pipe_once()
                    state.update(self.pipe_close)
                    continue
                for action in (stream.flush, lambda stream=stream: os.fsync(stream.fileno())):
                    try:
                        action()
                    except BaseException as error:
                        self.errors.append(error)
                state['close_attempted'] = True
                state['state'] = 'unknown'
                try:
                    stream.close()
                    state['close_returned'] = True
                except BaseException as error:
                    self.errors.append(error)
                try:
                    state['closed_observed'] = bool(stream.closed)
                    if state['closed_observed']:
                        state['state'] = 'closed'
                except BaseException as error:
                    self.errors.append(error)
            if self.pidfd is not None:
                self.pidfd_close_state = 'unknown'
                try:
                    os.close(self.pidfd)
                    self.pidfd = None
                    self.pidfd_close_state = 'closed'
                except BaseException as error:
                    self.errors.append(error)
            self.finalized = (all(state['state'] == 'closed' for state in self.stream_closures)
                              and self.pidfd_close_state != 'unknown')
            for name in ('stdout.raw', 'stderr.raw'):
                try:
                    self.captured[name] = (self.output / name).read_bytes()
                except BaseException as error:
                    self.errors.append(error)

    run = PipedWatchRun(out, [str(f.binary), '--root', str(f.root), '--json',
                              'mission', 'watch', work_id], f.environment)
    primary = None
    errors = []
    signal_errors = []
    records = []
    started, exited, submitted, waiting = [], [], [], []
    unparsed, parser_errors, invalid = [], [], []
    stdout_raw = None
    pipe = None
    pipe_state = run.pipe_close

    def _retain(error_list, action):
        '''Run one cleanup or publication action, retaining its original typed exception.'''
        try:
            action()
        except BaseException as error:
            error_list.append(error)

    def mirror(raw):
        '''Append one stdout chunk byte-for-byte to the owned stdout.raw evidence file.'''
        stdout_raw.write(raw)
        stdout_raw.flush()

    def classify(line):
        '''Parse one complete stdout line and update the event ledgers.

        Returns the parsed record, None when the line did not parse (the
        original typed parser exception is retained next to the line digest),
        or False for a parsed frame without a string event field. Nothing is
        dropped: unparsable and invalid lines are retained by sha256, every
        parsed record is kept untouched, and the four supported watch events
        update their ledgers.
        '''
        try:
            value = json.loads(line)
        except BaseException as error:
            unparsed.append(fixture.digest(line))
            parser_errors.append(error)
            return None
        records.append(value)
        if not isinstance(value, dict) or not isinstance(value.get('event'), str):
            invalid.append(fixture.digest(line))
            return False
        event = value['event']
        if event == EVENT_STARTED:
            started.append(value)
        elif event == EVENT_EXITED:
            exited.append(value)
        elif event == EVENT_SUBMITTED:
            submitted.append(value)
        elif event == EVENT_WAITING:
            waiting.append(value)
        return value

    def _position(entry):
        '''Return the record index of one retained ledger entry, or -1 when absent.'''
        for index, record in enumerate(records):
            if record is entry:
                return index
        return -1

    try:
        run.output.mkdir(mode=0o700, exist_ok=False)
        run.publish('command.json', dict(argv=run.argv, global_watchdog=None,
                    scope='one managed mission watch; one worker; one handoff; SIGINT after'))
        for name in ('stdout.raw', 'stderr.raw'):
            stream = (run.output / name).open('xb')
            run.streams.append(stream)
            if name == 'stdout.raw':
                stdout_raw = stream
        run.process = subprocess.Popen(run.argv, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE,
                                       stderr=run.streams[1], env=run.environment)
        # Own the stdout pipe before any further fallible publication, pidfd
        # open or mirroring, so every owner consumer sees it while it is held.
        run.streams.append(run.process.stdout)
        run.mark_pipe(run.process.stdout)
        pipe = run.process.stdout
        run.pipe_close['state'] = 'open'
        pipe_state = run.pipe_close
        run.publish('launch.json', dict(pid=run.process.pid))
        run.pidfd = os.pidfd_open(run.process.pid)
    except BaseException as error:
        primary = error

    try:
        if primary is None:
            try:
                while True:
                    raw = run.process.stdout.readline()
                    if not raw:
                        primary = transport.Failure('managed-watch-eof-before-handoff',
                                                    facts=dict(events=len(records),
                                                               note='watch stdout ended before one immutable submission'))
                        break
                    mirror(raw)
                    value = classify(raw)
                    if value is None:
                        primary = transport.Failure('managed-watch-stdout-unparsable',
                                                    list(parser_errors),
                                                    facts=dict(unparsed_sha256=list(unparsed)))
                        break
                    if value is False:
                        primary = transport.Failure('managed-watch-stdout-invalid-frame',
                                                    facts=dict(invalid_sha256=list(invalid)))
                        break
                    if value.get('mission_complete'):
                        primary = transport.Failure('managed-watch-mission-complete-claimed',
                                                    facts=dict(event=value.get('event')))
                        break
                    if value['event'] in WATCH_EVENTS and value.get('work') != work_id:
                        primary = transport.Failure('managed-watch-event-work-mismatch',
                                                    facts=dict(event=value['event'],
                                                               work=value.get('work')))
                        break
                    if value['event'] == 'mission-watch-blocked':
                        primary = transport.Failure('managed-watch-blocked-incomplete', facts=dict(event=value['event']))
                        break
                    if waiting:
                        primary = transport.Failure('managed-watch-report-waiting-incomplete',
                                                    facts=dict(waiting=[e.get('reason') for e in waiting]))
                        break
                    if len(started) > 1:
                        primary = transport.Failure('managed-watch-second-worker',
                                                    facts=dict(started=[e.get('agent_id') for e in started]))
                        break
                    bad = [e for e in exited if e.get('exit_code') != 0 or e.get('exit_signal') is not None]
                    if bad:
                        primary = transport.Failure('managed-watch-worker-exited-unsuccessfully',
                                                    facts=dict(exited=[(e.get('exit_code'), e.get('exit_signal')) for e in bad]))
                        break
                    if submitted:
                        break
            except BaseException as error:
                primary = error

        if run.process is not None:
            def _sigint():
                '''Send supported SIGINT to exactly this owned watch: pidfd first, owned PID fallback.'''
                if run.pidfd is not None:
                    signal.pidfd_send_signal(run.pidfd, signal.SIGINT)
                else:
                    os.kill(run.process.pid, signal.SIGINT)
            try:
                _sigint()
            except BaseException as error:
                # A failed signal followed by any drain can never be silently
                # successful: the typed cause is retained and the gate refuses.
                signal_errors.append(error)
            buffer = b''
            try:
                while True:
                    rest = run.process.stdout.read(65536)
                    if not rest:
                        break
                    mirror(rest)
                    buffer += rest
            except BaseException as error:
                if primary is None:
                    primary = error
                else:
                    errors.append(error)
            pieces = buffer.split(b'\n')
            if pieces and pieces[-1] == b'':
                pieces.pop()
            for line in pieces:
                value = classify(line)
                if primary is None and value is None:
                    primary = transport.Failure('managed-watch-stdout-unparsable',
                                                list(parser_errors),
                                                facts=dict(unparsed_sha256=list(unparsed)))
                elif primary is None and value is False:
                    primary = transport.Failure('managed-watch-stdout-invalid-frame',
                                                facts=dict(invalid_sha256=list(invalid)))
            run.close_pipe_once()

        _retain(errors, lambda: run.publish('events.json', dict(
            schema_version=1,
            records=records,
            unparsed_sha256=unparsed,
            invalid_frame_sha256=invalid,
            watch_signal=dict(attempted=run.process is not None, send_errors=len(signal_errors)),
            stdout_pipe_close=pipe_state,
            worker_started=[e.get('agent_id') for e in started],
            worker_exited=[dict(agent_id=e.get('agent_id'), exit_code=e.get('exit_code'),
                                exit_signal=e.get('exit_signal')) for e in exited],
            report_submitted=[dict(agent_id=e.get('agent_id'), revision=e.get('revision'))
                              for e in submitted],
            report_waiting=[dict(agent_id=e.get('agent_id'), reason=e.get('reason'))
                            for e in waiting])))

        if run.process is None:
            _retain(errors, run.finish_capture)
        else:
            try:
                run.settle()
            except BaseException as error:
                raise transport.Failure('managed-watch-settlement-unknown',
                                        ([primary] if primary is not None else []) + errors + signal_errors + list(run.errors) + [error],
                                        owner=run) from error

        problems = []
        if len(started) != 1:
            problems.append('worker-started-count-' + str(len(started)))
        if len(exited) != 1:
            problems.append('worker-exited-count-' + str(len(exited)))
        if len(submitted) != 1:
            problems.append('report-submitted-count-' + str(len(submitted)))
        if waiting:
            problems.append('report-waiting-occurrences-' + str(len(waiting)))
        if unparsed:
            problems.append('unparsable-stdout-lines-' + str(len(unparsed)))
        if invalid:
            problems.append('invalid-stdout-frames-' + str(len(invalid)))
        if signal_errors:
            problems.append('watch-signal-undelivered-' + str(len(signal_errors)))
        if started and exited and exited[0].get('agent_id') != started[0].get('agent_id'):
            problems.append('exit-agent-mismatch')
        if started and submitted and submitted[0].get('agent_id') != started[0].get('agent_id'):
            problems.append('submission-agent-mismatch')
        if exited and (exited[0].get('exit_code') != 0 or exited[0].get('exit_signal') is not None):
            problems.append('worker-exit-unsuccessful')
        for ledger, label in ((started, 'started'), (exited, 'exited'),
                              (submitted, 'submitted'), (waiting, 'waiting')):
            if any(e.get('work') != work_id for e in ledger):
                problems.append('event-work-mismatch-' + label)
        if len(started) == 1 and len(exited) == 1 and len(submitted) == 1:
            if not _position(started[0]) < _position(exited[0]) < _position(submitted[0]):
                problems.append('event-order-invalid')
        agent = started[0].get('agent_id') if started else None
        if len(started) == 1 and (not isinstance(agent, str) or not agent):
            problems.append('worker-agent-not-identified')
        if len(submitted) == 1:
            revision = submitted[0].get('revision')
            if not isinstance(revision, int) or isinstance(revision, bool):
                problems.append('submission-revision-not-numeric')
        if any(isinstance(e, dict) and e.get('event') == 'mission-watch-blocked' for e in records):
            problems.append('watch-blocked')
        stopped = [e for e in records if isinstance(e, dict) and e.get('event') == 'mission-watch-stopped']
        if len(stopped) != 1 or not all(e.get('work') == work_id and e.get('reason') == 'sigint'
                and e.get('children_still_running') == 0 and e.get('lease_released') is True
                and e.get('root_closed') is True for e in stopped):
            problems.append('watch-stop-not-owned-clean-sigint')
        if run.status != 0:
            problems.append('watch-exit-' + str(run.status))
        if not run.reaped:
            problems.append('owner-reap-unknown')
        if not run.wait_validated:
            problems.append('owner-wait-unvalidated')
        if not run.finalized:
            problems.append('owner-capture-unclosed')
        if run.errors:
            problems.append('owner-errors-' + str(len(run.errors)))
        problems.extend('owner-pending-' + reason for reason in run.pending())
        if pipe_state is not None and pipe_state['state'] != 'closed':
            problems.append('stdout-pipe-close-' + str(pipe_state['state']))
        if any(isinstance(e, dict) and e.get('mission_complete') for e in records):
            problems.append('event-claimed-mission-complete')
        if primary is not None or problems or errors or signal_errors:
            raise transport.Failure('managed-watch-incomplete',
                                    ([primary] if primary is not None else []) + errors + signal_errors + list(run.errors),
                                    owner=run,
                                    facts=dict(problems=problems, events=len(records),
                                               worker_agent=agent))
        return dict(events=records, agent_id=submitted[0].get('agent_id'),
                    revision=submitted[0].get('revision'),
                    settlement=run.safe_summary(), stdout_pipe_close=dict(pipe_state))
    except BaseException as unexpected:
        raise transport.Failure('managed-watch-owned-failure',
            [unexpected, *([] if primary is None else [primary]),
             *errors, *signal_errors, *parser_errors, *run.errors], owner=run) from unexpected



def prepare(args):
    '''Execute the whole bounded preparation and return the READY payload.

    Preconditions: args.normal is the qualified normal CLI binary;
    args.normal_profile its qualification profile with the matching sibling
    after.json inventory; args.source_root the supplied tests directory;
    args.output a fresh evidence directory path; args.parent_transport the
    actual module name of the supplied Gi W/F parent transport;
    args.feedback_module optionally names the feedback PROGRAM module.

    Outputs: the READY_MANAGED_CONTROLS_NOT_REVIEWED result document (also
    published as managed-i0.json). Side effects: exactly one owned fixture
    Root in the actual saved fixture area, one fresh managed Work, one tiny
    committed repository, fixture provider program replacements between
    phases only (never while a child runs), and every request, reply,
    program, proposal and event retained as bytes under the output
    directory.

    Errors: SourceMissing for unresolved actual sources; every product
    refusal, assertion failure or watch incompleteness propagates with
    original causes for the caller terminal handling. Limits: no reviewer W,
    SDK or F; no acceptance, automatic review, retry, timeout, watchdog,
    cancellation/corruption/recovery campaign, reset/TTL/authority change;
    an explicit inert-fixture USD estimate budget (10/1) is saved before mission start,
    following the established managed-auto producer; no price/cost claim is made.
    '''
    # Bind relative profile keys to the actual producer checkout without
    # rewriting profile/inventory bytes or pretending __file__ points elsewhere.
    out = Path(args.output).absolute()
    args.normal = str(Path(args.normal).absolute())
    args.normal_profile = str(Path(args.normal_profile).absolute())
    source_tests = Path(args.source_root).resolve(strict=True)
    assert source_tests.is_dir() and source_tests.name == 'tests'
    args.source_root = str(source_tests)
    os.chdir(source_tests.parent)
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    modules, transport, template, origins = load_modules(
        args.source_root, args.parent_transport, args.feedback_module)
    fixture = modules['rust_agent_model_route']
    bootstrap = modules['rust_planning_bootstrap']
    repository = modules['rust_managed_repository_bootstrap']
    gieth = modules['rust_gieth_effective_dependency']
    managed = modules['rust_managed_planning']
    fixture.publish(out / 'module-origins.json', origins)
    fixture.publish(out / 'source-root.json', {
        'source_root': str(Path(args.source_root).resolve()),
        'scope': 'explicit caller argument; imports resolve here and nowhere else'})
    binary = Path(args.normal)
    profile = Path(args.normal_profile)
    assert not binary.is_symlink(), 'proof-only alias must be restored locally first'
    assert binary.stat().st_dev == source_tests.parent.stat().st_dev, 'normal CLI must remain local'
    gieth.qualify(binary, profile, out)
    fixture.publish(out / 'budgets.json', dict(
        schema_version=1,
        planning_enable=dict(max_activations=3, max_tasks=2, max_decisions=3, max_review_calls=2),
        delegate_step=dict(max_tasks=1, max_activations=2),
        mission=dict(slots=1, provider='fixture', role='worker', level='standard'),
        usd_budget_on_fresh_work=dict(limit_usd=10, reserve_per_launch_usd=1,
            estimate_source='Owned inert managed fixture; explicit declared input', reference_date='2026-11-16'),
        bootstrap_fixture_budget='declared by the Fixture constructor and left untouched',
        retries=None, timeouts_seconds=None, hidden_product_ceilings='none',
        automatic_review=False, task_acceptance=False))
    f = fixture.Fixture(binary, out / 'setup', 'managed-i0')
    bootstrap_work = f.work
    work_id = bootstrap.fresh_work(f, 'managed-i0-create')
    f.work = work_id
    fixture.publish(out / 'works.json', dict(schema_version=1, root=str(f.root),
        bootstrap_work=bootstrap_work, managed_work=work_id,
        note='one fresh managed target Work; the declared bootstrap Work stays intact'))
    base = managed_source_repository(f, repository, fixture,
                                     Path(args.source_root).resolve().parent, out)
    helper = resolve_proposal_helper(managed)
    controls = {'req-1': [dict(id='report-and-source', command=['python3', '-c', CONTROL_CODE],
                               criteria=[1], inputs=['docs/task.md', 'tracked.txt'],
                               justification='Read the worker report and the committed tracked source '
                                              'inside the candidate workspace')]}
    f.ok(['planning', 'enable', work_id], bootstrap.request(
        event='managed-i0-enable', max_activations=3, max_tasks=2, max_decisions=3,
        max_review_calls=2, repository=dict(path='source', committed_only=True), checks=controls))
    delegate_steps = [dict(kind='delegate', id='child', title='Owned child scope',
                           next='Plan the single owned requirement', requirements=['req-1'],
                           max_tasks=1, max_activations=2)]
    first = planning_claim(f, helper, fixture, out, work_id, 1, 'root', delegate_steps, template)
    assert first['run']['decision_applied']
    state = first['run']['work']
    assert not state.get('tasks')
    assert any(s['id'] == 'child' and s['parent'] == 'root' for s in state['planning']['scopes'])
    task_steps = [dict(kind='task', id='task', title='Owned managed report',
                       next='Write the owned report from the attributed workspace',
                       deliverable='docs/task.md', criteria=['Owned managed report'],
                       requirements=['req-1'])]
    second = planning_claim(f, helper, fixture, out, work_id, 2, 'child', task_steps, template)
    assert second['run']['decision_applied']
    state = second['run']['work']
    tasks = state.get('tasks') or []
    assert len(tasks) == 1 and tasks[0]['id'] == 'task'
    policy = tasks[0].get('validation_policy') or {}
    assert len(policy.get('controls') or []) == 1
    storage = Path(work_revision(f, work_id)['planning']['repository']['storage'])
    fixture.publish(out / 'managed-repository.json', dict(storage=str(storage), base=base))
    current = work_revision(f, work_id)
    f.ok(['budget', 'apply', work_id], dict(schema_version=1,
         event_id='managed-i0-explicit-fixture-budget', expected_revision=current['revision'],
         budget=dict(limit_usd=10, reserve_per_launch_usd=1,
                     estimate_source='Owned inert managed fixture; explicit declared input',
                     reference_date='2026-11-16')))
    start_reply = f.ok(['mission', 'start', work_id], dict(
        schema_version=1, event_id='managed-i0-start',
        expected_revision=work_revision(f, work_id)['revision'],
        reason='Prepare managed I0 controls with one owned worker', slots=1,
        profile=dict(provider='fixture', role='worker', workspace='source',
                     level='standard', capture_output=True)))
    fixture.publish(out / 'mission-start.json', start_reply)
    status = f.ok(['mission', 'status', work_id])
    fixture.publish(out / 'mission-status-pre-watch.json', status)
    assert status['tasks'] and all(t['state'] == 'waiting' for t in status['tasks'])
    mutate(f, work_id, ['planning', 'pause'], 'managed-i0-plan-pause',
           reason='Avoid extra planners during the single owned mission watch', task_id=None)
    fixture.publish(out / 'worker-program.py', WORKER_PROGRAM.encode('utf-8'))
    fixture.publish(f.root / 'codex', WORKER_PROGRAM.encode('utf-8'))
    os.chmod(f.root / 'codex', 0o700)
    resume_if_paused(f, 'mission', work_id, 'managed-i0-resume-pre-watch',
                     'Single owned engine-dispatched mission watch')
    watch = mission_watch_handoff(transport, fixture, f, work_id, out / 'watch')
    fixture.publish(out / 'watch-summary.json', dict(schema_version=1,
        agent_id=watch['agent_id'], handoff_revision=watch['revision'],
        stdout_pipe_close=watch['stdout_pipe_close'], settlement=watch['settlement']))
    mutate(f, work_id, ['mission', 'pause'], 'managed-i0-mission-pause',
           reason='Preserve the single immutable handoff evidence', task_id=None)
    resume_if_paused(f, 'mission', work_id, 'managed-i0-resume-mission-controls',
                     'Explicit isolated managed control preparation')
    resume_if_paused(f, 'planning', work_id, 'managed-i0-resume-planning-controls',
                     'Explicit isolated managed control preparation')
    before = work_revision(f, work_id)
    candidate_reply = mutate(f, work_id, ['task', 'validation-candidate'], 'managed-i0-candidate')
    assert candidate_reply['cohort_task_count'] == 1
    assert candidate_reply['control_count'] == 1
    assert not candidate_reply['job_created']
    assert work_revision(f, work_id) == before
    entries = [p for p in (storage / 'candidates').glob('*/candidate.json')
               if json.loads(p.read_bytes()).get('task') == 'task']
    assert len(entries) == 1
    candidate = json.loads(entries[0].read_bytes())
    assert Path(candidate['workspace']).name == 'copy'
    fixture.publish(out / 'candidate.json', candidate)
    validate_reply = mutate(f, work_id, ['task', 'validate'], 'managed-i0-validate')
    assert validate_reply['controls_passed'] and not validate_reply['task_accepted']
    fixture.publish(out / 'validate.json', validate_reply)
    fixture.publish(out / 'work-after-validate.json', work_revision(f, work_id))
    state = work_revision(f, work_id)
    markers = dict((p.name, fixture.digest(p.read_bytes())) for p in f.root.glob('model-received*'))
    review_reply = mutate(f, work_id, ['task', 'review-inputs'], 'managed-i0-review-inputs')
    preview = review_reply['preview']
    assert preview['invocation_authorized'] is False
    assert preview['provider_started'] is False
    assert preview['task_accepted'] is False
    assert preview['native_control_evidence_verified'] is True
    assert work_revision(f, work_id) == state
    assert dict((p.name, fixture.digest(p.read_bytes())) for p in f.root.glob('model-received*')) == markers
    fixture.publish(out / 'review-inputs.json', review_reply)
    result = dict(schema_version=1, status='READY_MANAGED_CONTROLS_NOT_REVIEWED',
                  root=str(f.root), bootstrap_work=bootstrap_work, work=work_id,
                  task='task', worker_agent=watch['agent_id'],
                  handoff_revision=watch['revision'],
                  candidate_workspace=str(candidate['workspace']),
                  controls={'requirement': 'req-1', 'count': 1, 'ids': ['report-and-source']},
                  review_inputs={'invocation_authorized': False, 'provider_started': False,
                                 'task_accepted': False, 'native_control_evidence_verified': True},
                  module_origins=origins,
                  stop_boundary='reviewer W, SDK and F are separate later allocations; '
                                'nothing accepted and nothing reviewed here')
    fixture.publish(out / 'managed-i0.json', result)
    return result


def _emit_failure(output, payload):
    """Atomically publish a safe diagnostic, retaining write/fsync/close causes.

    Every allocated descriptor gets one close attempt. Publication failure may
    leave visible bytes, never inferred rollback; incomplete temporaries remain.
    This function owns no child and grants no process release.
    """
    import tempfile
    errors = []
    raw = (json.dumps(payload, sort_keys=True) + '\n').encode('utf-8')
    file_fd = directory_fd = None
    temporary = None
    try:
        file_fd, name = tempfile.mkstemp(prefix='.failure.', dir=Path(output))
        temporary = Path(name)
        remaining = memoryview(raw)
        while remaining:
            size = os.write(file_fd, remaining)
            if size <= 0:
                raise OSError('zero diagnostic write')
            remaining = remaining[size:]
        os.fsync(file_fd)
        assert temporary.read_bytes() == raw
        os.replace(temporary, Path(output) / 'failure.json')
        temporary = None
        directory_fd = os.open(Path(output), os.O_RDONLY | os.O_DIRECTORY)
        os.fsync(directory_fd)
    except BaseException as error:
        errors.append(error)
    finally:
        for descriptor in (directory_fd, file_fd):
            if descriptor is not None:
                try:
                    os.close(descriptor)
                except BaseException as error:
                    errors.append(error)
    if temporary is not None:
        payload['incomplete_temporary'] = str(temporary)
    try:
        sys.stderr.write(json.dumps(payload, sort_keys=True) + '\n')
        sys.stderr.flush()
    except BaseException as error:
        errors.append(error)
    if errors:
        raise BaseExceptionGroup('failure diagnostic publication errors', errors)


def main(argv=None):
    '''Parse arguments, run prepare(), and own the terminal failure protocol.

    Returns 0 only when prepare returned the READY payload. On SourceMissing
    it publishes failure.json with the static code and facts (source-stage
    refusal; no owners exist yet). On any other failure it delegates to the
    supplied transport report_terminal when the parent transport module can
    be resolved, so owned processes and handles are settled or explicitly
    held with original causes; otherwise a contained structured document is
    emitted. No retry, no kill, no inferred success, no masking.
    '''
    parser = argparse.ArgumentParser(
        description='Bounded source-only managed I0 fixture preparer (see module docstring).')
    parser.add_argument('--normal', required=True,
                        help='qualified normal CLI binary (hashed against the profile; never built or copied)')
    parser.add_argument('--normal-profile', required=True,
                        help='qualification profile JSON; its sibling after.json inventory must match byte-for-byte')
    parser.add_argument('--source-root', required=True,
                        help='supplied tests directory holding the actual checkout producer modules')
    parser.add_argument('--output', required=True,
                        help='fresh evidence directory (never /tmp; fixture roots live in the repository fixture area)')
    parser.add_argument('--parent-transport', default='rust_unstarted_review_process',
                        help='module name of the supplied Gi W/F parent transport (Failure/OwnedRun/report_terminal)')
    parser.add_argument('--feedback-module', default=None,
                        help='optional explicit module name exposing PROGRAM with the planning needle')
    args = parser.parse_args(argv)
    out = Path(args.output).absolute()
    try:
        result = prepare(args)
    except SourceMissing as missing:
        try:
            _emit_failure(out, dict(schema_version=1, status='SOURCE_MISSING',
                                   code=missing.code, facts=missing.facts,
                                   no_rollback_inference=True))
        except BaseException as secondary:
            raise BaseExceptionGroup('source failure and diagnostic failure', [missing, secondary])
        return 1
    except BaseException as error:
        transport = None
        if args.parent_transport:
            try:
                transport = importlib.import_module(args.parent_transport)
            except BaseException:
                transport = None
        if transport is not None and hasattr(transport, 'report_terminal'):
            return transport.report_terminal(error, out)
        try:
            _emit_failure(out, dict(schema_version=1, status='PREPARATION_FAILED',
                                   causes=[type(error).__name__],
                                   no_rollback_inference=True))
        except BaseException as secondary:
            raise BaseExceptionGroup('preparation and diagnostic failure', [error, secondary])
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
