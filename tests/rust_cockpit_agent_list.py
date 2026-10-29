#!/usr/bin/env python3
"""Verify the public cockpit agent-list JSON contract: legacy Go vs native Rust.

Purpose: on one fresh private fixture Root, an actual legacy-Go reference list
and the native Rust list of the same ordinary empty work must publish one
identical public JSON document of exactly 14 keys — again after a real Rust
mission pause creating manual controls without consent, and again after a real
unpause preserving the manual mode — with pilotage.snapshot_as_of and explicit per-edge evaluated_at clocks as the
only dynamic observations, each bounded by its actual call window. Every snapshot must be RFC3339 and fall inside the
real UTC start/end window of its own call (no duration cap); project_view_key
must hash the exact canonical Root bytes; saved settings and the public work
must survive every reader; the Rust list must also publish JSON without --json,
refuse unknown work and invalid extra CLI arguments, and, once a task exists,
in --initial-tasks mode compare TODO actions, pause, priority and budgets; then
refuse a dependent task with cockpit-task-projection-unported instead of
manufacturing empty task_actions/reviews.

Preconditions: two frozen local executables (--go-binary legacy reference,
--rust-binary native) and one fresh --output directory; no live Root, no
network, no provider account.

Errors: every call retains argv, exit code and both exact streams under the
evidence directory; refusals retain their native failure wire (code or causes);
any failure publishes the original traceback with fixture ownership before
re-raising; a zero exit with empty or unparseable JSON is itself a failure.

Side effects: one private 0700 fixture Root plus evidence files; public CLI
commands only; the legacy Go binary stays an offline reference that is never
installed into the Root and never a production fallback; no direct SQLite edit, no
providers/watch call, no timeout, watchdog or retry, and no launch or consent
is ever inferred.

Limits: initial-task mode covers TODO tasks without dependencies/history or
planning/agents; default mode retains the original empty-work baseline; malformed
controls/priority corruption fixtures are not part of this harness; malformed
public control requests are checked. Leap-second60 is not representable by the
Python datetime parser; ordinary timestamps are bounded only by actual calls.
This does not qualify PAEA/Biba or full nonempty-work cockpit behavior.
"""
import argparse
import json
import os
import re
import subprocess
import tempfile
import time
import traceback
from datetime import datetime, timedelta, timezone
from pathlib import Path

import rust_agent_model_route as retained

# Exact public Go cockpit key set; nested values are compared recursively.
EXPECTED_KEYS = frozenset({'agents','autonomy','autonomy_label','cost','cost_text',
    'independent_reviews','paused','pilotage','post_delivery','priority','slots',
    'task_actions','validation','work'})
# Public cockpit agent-list CLI under test; single definition on purpose.
LIST_COMMAND = ('agent', 'list')
DYNAMIC_PATHS = frozenset({('pilotage', 'snapshot_as_of')})
RFC3339 = re.compile(r'(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(\.\d+)?([Zz]|[+-]\d{2}:\d{2})')
CREDENTIAL_MARKERS = ('TOKEN', 'SECRET', 'PASSWD', 'PASSWORD', 'KEY', 'AWS_',
                      'OPENAI', 'ANTHROPIC', 'GEMINI', 'GOOGLE_APPLICATION',
                      'AZURE', 'GITHUB', 'GITLAB', 'SSH_', 'GPG_', 'PGP_',
                      'CREDENTIAL', 'SERVICE_ACCOUNT', 'COOKIE')


def canonical_root_bytes(root):
    """Hash the actual canonical Root pathname bytes, matching Go Store.root.

    Work JSON is not a project identity. Resolve this owned fixture's path and
    retain Unix filesystem bytes without a lossy UTF-8 conversion.
    """
    return os.fsencode(Path(root).resolve())


def rfc3339_utc(text, label):
    """Parse one strict RFC3339 timestamp into an aware UTC datetime.

    Accepts Z/z or a numeric offset and any fraction length (truncated past
    microseconds); rejects missing offsets, impossible dates and leap-second
    60. No duration bound is applied; callers compare against real windows.
    """
    assert isinstance(text, str), (label, 'snapshot-not-a-string', text)
    match = RFC3339.fullmatch(text)
    assert match, (label, 'not-rfc3339', text)
    fraction = match.group(7)
    microsecond = int((fraction[1:] + '000000')[:6]) if fraction else 0
    offset = match.group(8)
    try:
        if offset in ('Z', 'z'):
            zone = timezone.utc
        else:
            sign = 1 if offset[0] == '+' else -1
            assert int(offset[1:3]) <= 23 and int(offset[4:6]) <= 59, (label, 'invalid-rfc3339-offset')
            zone = timezone(sign * timedelta(hours=int(offset[1:3]), minutes=int(offset[4:6])))
        return datetime(int(match.group(1)), int(match.group(2)), int(match.group(3)),
                        int(match.group(4)), int(match.group(5)), int(match.group(6)),
                        microsecond, zone).astimezone(timezone.utc)
    except ValueError as error:
        raise AssertionError((label, 'impossible-snapshot', text)) from error


def failure_codes(wire):
    """Native refusal codes: failure.code plus every string in failure.causes."""
    failure = wire.get('failure') if isinstance(wire, dict) else None
    if not isinstance(failure, dict):
        return []
    codes = [failure['code']] if isinstance(failure.get('code'), str) else []
    causes = failure.get('causes')
    if isinstance(causes, list):
        codes.extend(cause for cause in causes if isinstance(cause, str))
    return codes


def sanitized_environment():
    """Parent environment without credential-like names; only names are recorded."""
    environment, stripped = {}, []
    for name, value in os.environ.items():
        if any(marker in name.upper() for marker in CREDENTIAL_MARKERS):
            stripped.append(name)
        else:
            environment[name] = value
    return environment, sorted(stripped)


def require_document_shape(document, label):
    """Exactly fourteen top-level keys including every named contract section."""
    assert isinstance(document, dict), (label, 'document-not-an-object')
    assert set(document) == EXPECTED_KEYS, (label, 'expected-exact-14-keys', sorted(document))


def compare_lists(left, right, left_label, right_label, left_window, right_window, pair, root):
    """Recursively require one identical public agent-list document.

    Both sides must be objects with exactly the same 14 top-level keys and the
    named contract sections; every value except the single dynamic
    pilotage.snapshot_as_of and pilotage.edges[index].evaluated_at observations must be exactly equal (dict key sets,
    list lengths, scalars; booleans never match numbers, numbers compare by
    value). Each side's snapshot_as_of and every dependency edge clock must be RFC3339 and fall inside its own
    call's real UTC start/end window, the left call may not observe after the
    right call, and no duration cap is applied. Each side's project_view_key
    must be lowercase sha256 over the fixture's canonical Root pathname bytes, which also forces cross-implementation equality.
    Every edge must carry its own valid clock; all graph facts remain strict.
    Returns one evidence entry for the retained snapshot ledger.
    """
    require_document_shape(left, left_label)
    require_document_shape(right, right_label)
    dynamic = []

    def walk(a, b, path):
        if path in DYNAMIC_PATHS or (len(path) == 4 and path[:2] == ('pilotage', 'edges')
                                    and type(path[2]) is int and path[3] == 'evaluated_at'):
            dynamic.append((a, b))
            return
        if isinstance(a, dict) or isinstance(b, dict):
            assert isinstance(a, dict) and isinstance(b, dict), (pair, path, 'object-shape')
            assert set(a) == set(b), (pair, path, 'key-set', sorted(a), sorted(b))
            for key in sorted(a):
                walk(a[key], b[key], path + (key,))
        elif isinstance(a, list) or isinstance(b, list):
            assert isinstance(a, list) and isinstance(b, list) and len(a) == len(b), (pair, path, 'list-shape')
            for index, (item_a, item_b) in enumerate(zip(a, b)):
                walk(item_a, item_b, path + (index,))
        else:
            assert a == b and isinstance(a, bool) == isinstance(b, bool), (pair, path, 'value-differs', a, b)

    walk(left, right, ())
    edges = left['pilotage'].get('edges', [])
    assert isinstance(edges, list), (pair, 'edges-not-array')
    assert len(dynamic) == 1 + len(edges), (pair, 'dynamic-observation-count', len(dynamic))
    for index, (left_edge, right_edge) in enumerate(dynamic[:-1]):
        # Sorted map traversal visits edges before snapshot_as_of; every edge
        # clock is validated, never removed as an arbitrary volatile field.
        for raw, label, window in ((left_edge, left_label, left_window),
                                    (right_edge, right_label, right_window)):
            moment = rfc3339_utc(raw, label + '-edge-' + str(index))
            assert window[0] <= moment.timestamp() <= window[1], (pair, 'edge-clock-outside-call', index)
        assert rfc3339_utc(left_edge, left_label) <= rfc3339_utc(right_edge, right_label)
    left_raw, right_raw = dynamic[-1]
    left_moment = rfc3339_utc(left_raw, left_label)
    right_moment = rfc3339_utc(right_raw, right_label)
    for moment, label, window in ((left_moment, left_label, left_window),
                                  (right_moment, right_label, right_window)):
        start = datetime.fromtimestamp(window[0], tz=timezone.utc)
        end = datetime.fromtimestamp(window[1], tz=timezone.utc)
        assert start <= moment <= end, (label, 'snapshot-outside-real-call-window',
                                        moment.isoformat(), start.isoformat(), end.isoformat())
    assert left_moment <= right_moment, (pair, 'snapshot-order', left_moment, right_moment)
    for side, label in ((left, left_label), (right, right_label)):
        key = side['pilotage']['project_view_key']
        assert isinstance(key, str) and re.fullmatch(r'[0-9a-f]{64}', key), (label, 'project-view-key-shape', key)
        assert key == retained.digest(canonical_root_bytes(root)), \
            (label, 'project-view-key-not-canonical-root-hash')
    return {'pair': pair, 'left_label': left_label, 'left_raw': left_raw,
            'left_utc': left_moment.isoformat(), 'right_label': right_label,
            'right_raw': right_raw, 'right_utc': right_moment.isoformat()}


class Fixture:
    """One owned closed Root plus the fresh evidence directory and call ledger.

    The Root is created before any binary runs and is never removed, so partial
    failures keep their original state and ownership for the parent; only the
    public CLI mutates it. Every call retains argv, exit code, exact stdout and
    stderr bytes and its real wall-clock window; the inherited environment is
    sanitized once and fixture inputs are published before use.
    """

    def __init__(self, go, rust, output, owned=None):
        """Create a private Root, or reuse the supplied owned inert fixture only.

        `owned` is a retained model-route fixture created by this harness; its
        public producers and private provider environment remain authoritative.
        No arbitrary Root path or live configuration is accepted by the CLI.
        """
        self.go = go
        self.rust = rust
        self.output = output
        self.environment, self.stripped = sanitized_environment()
        self.root = (owned.root if owned is not None else
                     Path(tempfile.mkdtemp(prefix='swarm-cockpit-agent-list-'))).resolve()
        if owned is not None:
            self.environment = {k: v for k, v in owned.environment.items()
                                if not any(m in k.upper() for m in CREDENTIAL_MARKERS)}
        self.settings = self.root / '.swarm' / 'engine-settings.json'
        self.work = None
        self.sequence = 0
        self.calls = []
        self.checks = []
        self.snapshots = []
        self.refusals = []

    def check(self, name, condition, detail=None):
        """Retain one named assertion only after it passed; false fails loudly."""
        assert condition, (name, detail)
        self.checks.append(name)
        retained.publish(self.output / 'checks.json', self.checks)
        print('PASS ' + name, flush=True)

    def call(self, binary, args, value=None, json_flag=True):
        """Run one public CLI call with no shell, timeout, retry or stdin use.

        Any input document is published as a fresh owned file first; stdout and
        stderr are always captured and published even on success, so refusal
        and failure evidence never lose their originals. Returns the completed
        result and the exact (start, end) wall-clock window of the call.
        """
        self.sequence += 1
        name = 'call-%03d' % self.sequence
        argv = [str(binary), *args, '--root', str(self.root)]
        if json_flag:
            argv.append('--json')
        if value is not None:
            path = self.output / (name + '.input.json')
            retained.publish(path, value)
            argv += ['--input', str(path)]
        started = time.time()
        result = subprocess.run(argv, env=self.environment, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        ended = time.time()
        retained.publish(self.output / (name + '.stdout'), result.stdout)
        retained.publish(self.output / (name + '.stderr'), result.stderr)
        self.calls.append({'name': name, 'argv': argv, 'exit': result.returncode,
                           'start': started, 'end': ended,
                           'stdout_sha256': retained.digest(result.stdout),
                           'stderr_sha256': retained.digest(result.stderr)})
        retained.publish(self.output / 'calls.json', self.calls)
        return result, (started, ended)

    def ledger(self, result):
        """Pointer to the retained evidence of the last call; nothing is lost."""
        record = self.calls[-1]
        return {'call': record['name'], 'argv': record['argv'], 'exit': record['exit'],
                'stdout': str(self.output / (record['name'] + '.stdout')),
                'stderr': str(self.output / (record['name'] + '.stderr'))}

    def done(self, binary, args, value=None):
        """Require exit zero with retained streams; any success body is accepted."""
        result, _ = self.call(binary, args, value)
        assert result.returncode == 0, ('command-failed', args, result.returncode, self.ledger(result))
        return result

    def ok(self, binary, args, value=None, json_flag=True):
        """Require a real nonempty JSON-object success; empty JSON never passes."""
        result, window = self.call(binary, args, value, json_flag)
        assert result.returncode == 0, ('success-required', args, result.returncode, self.ledger(result))
        try:
            wire = json.loads(result.stdout)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise AssertionError(('success-without-json', args, self.ledger(result))) from error
        assert isinstance(wire, dict) and wire, ('empty-json-success', args, self.ledger(result))
        return wire, window

    def refused(self, binary, args, value=None, expected=None, name='refusal'):
        """Require a real refusal carrying its native failure wire; retain it all.

        The failure JSON is read from stderr with stdout as retained fallback; a
        zero exit or a wire without a failure object fails. expected, when
        given, must appear as failure.code or inside failure.causes. Returns
        the exact wire and the retained call record.
        """
        result, _ = self.call(binary, args, value)
        assert result.returncode != 0, ('refusal-expected', args, self.ledger(result))
        wire = None
        for stream in (result.stderr, result.stdout):
            try:
                candidate = json.loads(stream)
            except (UnicodeDecodeError, json.JSONDecodeError):
                continue
            if isinstance(candidate, dict) and isinstance(candidate.get('failure'), dict):
                wire = candidate
                break
        assert wire is not None, ('refusal-without-failure-json', args, self.ledger(result))
        codes = failure_codes(wire)
        if expected is not None:
            assert expected in codes, ('refusal-code-mismatch', expected, codes, wire, self.ledger(result))
        record = dict(self.calls[-1])
        self.refusals.append({'name': name, 'call': record['name'], 'argv': record['argv'],
                              'exit': record['exit'], 'codes': codes, 'failure': wire})
        retained.publish(self.output / 'refusals.json', self.refusals)
        return wire, record

    def witness(self):
        """Read saved settings, Work and native controls once native init exists."""
        settings = self.settings.read_bytes() if self.settings.exists() else None
        shown, _ = self.ok(self.go, ['work', 'show', self.work])
        controls = self.ok(self.rust, ['mission', 'show', self.work])[0] if settings is not None else None
        return settings, shown['work'], controls

    def reader_immutability(self, baseline, suffix):
        """Saved settings, Work and controls must survive every reader call."""
        current = self.witness()
        self.check('reader-preserves-saved-settings-' + suffix, current[0] == baseline[0])
        self.check('reader-preserves-public-work-' + suffix, current[1] == baseline[1])
        self.check('reader-preserves-mission-controls-' + suffix, current[2] == baseline[2])

    def list_document(self, binary, json_flag=True):
        """One actual public agent-list call; returns its document and window."""
        return self.ok(binary, [*LIST_COMMAND, self.work], json_flag=json_flag)


def initial_task_journey(fixture):
    """Compare initial TODO actions, pause, saved priorities and budget refusals.

    All writes use the public native CLI on the owned fixture; every read must
    preserve settings, Work and controls. A dependent task is added only after
    this valid control, to retain an explicit unsupported-shape refusal. No
    provider, launch or consent is requested; budget values are test inputs.
    """
    def pair(label, ready, reason=None):
        """Compare the whole fourteen-key document and assert actual readiness."""
        before = fixture.witness()
        left, lw = fixture.list_document(fixture.go)
        right, rw = fixture.list_document(fixture.rust)
        fixture.snapshots.append(compare_lists(left, right, 'go', 'rust', lw, rw, label, fixture.root))
        retained.publish(fixture.output / 'snapshots.json', fixture.snapshots)
        fixture.check(label + '-documents-match', True)
        task = right['pilotage']['tasks']['fixture-task']
        start = next(a for a in right['task_actions']['fixture-task'] if a['kind'] == 'start')
        fixture.check(label + '-ready-and-start', task['ready'] is ready and start['disponible'] is ready)
        if reason is not None:
            fixture.check(label + '-reason', start.get('raison') == reason, start)
        fixture.reader_immutability(before, label)
        return right

    pair('initial-todo', True)
    def control(op, event, **extra):
        """Apply a revision-bound public control without enabling consent."""
        current, _ = fixture.ok(fixture.rust, ['work', 'show', fixture.work])
        return fixture.ok(fixture.rust, ['mission', op, fixture.work], dict(
            schema_version=1, event_id=event, expected_revision=current['work']['revision'],
            reason='Owned initial-task compatibility witness', **extra))
    control('pause', 'initial-task-pause')
    pair('initial-todo-paused', False,
         "Départs suspendus : réautoriez les départs depuis l'en-tête du cockpit.")
    control('unpause', 'initial-task-unpause')
    control('priority', 'initial-task-priority', task_id='fixture-task', priority='7')
    prioritized = pair('initial-todo-priority', True)
    fixture.check('priority-contains-only-persisted-row', prioritized['priority'] == {'fixture-task': 7})
    for label, limit, reserve, ready in [('affordable', 10, 1, True), ('insufficient', 1, 2, False),
                                          ('restored', 10, 1, True)]:
        current, _ = fixture.ok(fixture.rust, ['work', 'show', fixture.work])
        fixture.ok(fixture.rust, ['budget', 'apply', fixture.work], dict(
            schema_version=1, event_id='initial-task-budget-' + label,
            expected_revision=current['work']['revision'], budget=dict(
                limit_usd=limit, reserve_per_launch_usd=reserve,
                estimate_source='Owned deterministic fixture', reference_date='2026-10-28')))
        pair('initial-todo-budget-' + label, ready,
             None if ready else 'Budget estimatif insuffisant pour un nouveau départ.')


def global_workspace_journey(go, rust, output):
    """Prove global overlap reason precedence using a valid queued native intent.

    Two distinct ordinary Works share one newly owned private Root. The target
    Work has an insufficient saved budget and no agent. Another Work's preview
    and preparation create a real queued intent without running its provider.
    Reconciliation cancels that unregistered intent through the public CLI.
    The complete target document must match Go before/during/after; budget,
    target Work and settings are preserved. No SQLite mutation or launch occurs.
    """
    owned = retained.Fixture(rust, output / 'producer', 'global-overlap')
    output.mkdir(exist_ok=True)
    observed = Fixture(go, rust, output, owned)
    created, _ = observed.ok(rust, ['work', 'create'], dict(
        schema_version=1, event_id='global-target-create', expected_revision=0,
        title='Global overlap target', objective='Observe a refusal reason',
        scope='Owned inert Root', criteria=['Preserve budget and intent history']))
    observed.work = created['work']['id']
    observed.ok(rust, ['task', 'add', observed.work], dict(
        schema_version=1, event_id='global-target-task', expected_revision=1,
        id='target-task', title='Initial task', deliverable='Read-only observation',
        criteria=['Exact Go/Rust reason']))
    observed.ok(rust, ['budget', 'apply', observed.work], dict(
        schema_version=1, event_id='global-target-budget', expected_revision=2,
        budget=dict(limit_usd=1, reserve_per_launch_usd=2,
                    estimate_source='Owned fixture', reference_date='2026-10-28')))
    def pair(label, reason):
        """Require exact documents and refusal text without target mutations."""
        baseline = observed.witness()
        left, lw = observed.list_document(go)
        right, rw = observed.list_document(rust)
        observed.snapshots.append(compare_lists(left, right, 'go', 'rust', lw, rw, label, observed.root))
        retained.publish(output / 'snapshots.json', observed.snapshots)
        action = next(a for a in right['task_actions']['target-task'] if a['kind'] == 'start')
        observed.check(label, action['disponible'] is False and action.get('raison') == reason, action)
        observed.reader_immutability(baseline, label)
    budget_reason = 'Budget estimatif insuffisant pour un nouveau départ.'
    pair('before-global-overlap-budget-reason', budget_reason)
    preview = owned.prepare(preview=True)
    observed.check('queued-intent-preview-no-process', preview['process_started'] is False)
    prepared = owned.prepare()
    observed.check('valid-queued-intent-no-process',
                   prepared['agent']['status'] == 'queued' and prepared['process_started'] is False)
    pair('global-overlap-precedes-budget', 'Un agent occupe déjà cet espace de travail.')
    owned.ok(['agent', 'reconcile', prepared['agent']['id']])
    pair('reconciled-global-overlap-budget-recovery', budget_reason)
    observed.check('inert-provider-never-ran', not (owned.root / 'model-received.json').exists())
    retained.publish(output / 'result.json', dict(status='PASS', checks=observed.checks,
                     root=str(owned.root), provider_started=False, direct_sqlite_edits=False))
    print('COCKPIT_GLOBAL_OVERLAP_VERIFIED ' + str(len(observed.checks)), flush=True)


def run(go, rust, output, initial_tasks=False, todo_dependencies=False):
    """Drive the whole closed-fixture contract and publish the final result.

    Publishes the fixture birth and inputs records, then executes the legacy
    reference list, native init and list comparisons, JSON-without-flag checks,
    pause/unpause comparisons, the refusal set, the malformed controls and the
    immutability/installation witnesses. Any failure publishes the original
    traceback with the Root path and named checks before re-raising; the Root
    is kept for inspection.
    """
    fixture = Fixture(go, rust, output)
    try:
        go_sha256 = retained.digest(go.read_bytes())
        rust_sha256 = retained.digest(rust.read_bytes())
        retained.publish(output / 'birth.json', {
            'kind': 'owned_inert_cockpit_fixture_birth', 'root': str(fixture.root),
            'go_sha256': go_sha256, 'rust_sha256': rust_sha256,
            'legacy_go_reference_only': True, 'installed': False,
            'production_fallback': False, 'live_database': False,
            'provider_started': False,
            'environment_credential_names_stripped': fixture.stripped})
        retained.publish(output / 'inputs.json', {
            'go_binary': str(go), 'rust_binary': str(rust),
            'go_sha256': go_sha256, 'rust_sha256': rust_sha256,
            'output': str(output), 'live_database': False,
            'global_watchdog': None, 'command_timeout': None, 'retries': 0,
            'provider_started': False, 'direct_sqlite_edits': False,
            'scope': 'closed cockpit agent-list contract; legacy Go is an offline reference only'})
        check = fixture.check
        check('fixture-root-private', fixture.root.stat().st_mode & 0o777 == 0o700)
        fixture.done(go, ['init'])
        check('go-init-succeeds', True)
        check('go-init-leaves-saved-settings-absent', not fixture.settings.exists())
        created, _ = fixture.ok(go, ['work', 'create'], dict(
            schema_version=1, event_id='cockpit-agent-list-create', expected_revision=0,
            title='Cockpit agent list fixture', objective='One ordinary empty work',
            scope='Owned closed fixture', criteria=['Exact public list contract']))
        fixture.work = created['work']['id']
        check('go-work-create-ordinary-empty', bool(fixture.work))
        reference_work, _ = fixture.ok(go, ['work', 'show', fixture.work])
        check('work-has-no-tasks', not reference_work['work'].get('tasks'))
        baseline = fixture.witness()
        go_list, go_window = fixture.list_document(go)
        check('go-agent-list-json', True)
        fixture.reader_immutability(baseline, 'go-reference')
        fixture.done(rust, ['init'])
        check('rust-init-succeeds', True)
        check('rust-init-adds-saved-settings', fixture.settings.is_file())
        native_work, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        check('native-work-show-matches-reference', native_work['work'] == reference_work['work'])
        go_again, go_again_window = fixture.list_document(go)
        fixture.snapshots.append(compare_lists(
            go_list, go_again, 'go-list', 'go-list-after-native-init',
            go_window, go_again_window, 'go-reference-stability', fixture.root))
        retained.publish(output / 'snapshots.json', fixture.snapshots)
        check('go-reference-list-stable-across-native-init', True)
        rust_list, rust_window = fixture.list_document(rust)
        fixture.snapshots.append(compare_lists(
            go_list, rust_list, 'go-list', 'rust-list', go_window, rust_window,
            'agent-list-contract', fixture.root))
        retained.publish(output / 'snapshots.json', fixture.snapshots)
        check('agent-list-matches-except-snapshot-as-of', True)
        baseline = fixture.witness()
        go_bare, go_bare_window = fixture.list_document(go, json_flag=False)
        fixture.snapshots.append(compare_lists(
            go_list, go_bare, 'go-list', 'go-list-without-json-flag',
            go_window, go_bare_window, 'go-json-without-flag', fixture.root))
        rust_bare, rust_bare_window = fixture.list_document(rust, json_flag=False)
        fixture.snapshots.append(compare_lists(
            rust_list, rust_bare, 'rust-list', 'rust-list-without-json-flag',
            rust_window, rust_bare_window, 'rust-json-without-flag', fixture.root))
        retained.publish(output / 'snapshots.json', fixture.snapshots)
        check('go-list-json-without-flag', True)
        check('rust-list-json-without-flag', True)
        fixture.reader_immutability(baseline, 'initial-and-no-json-lists')
        paused_state, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        fixture.ok(rust, ['mission', 'pause', fixture.work], dict(
            schema_version=1, event_id='cockpit-agent-list-pause',
            expected_revision=paused_state['work']['revision'],
            reason='Create explicit safe manual controls'))
        check('rust-mission-pause-succeeds', True)
        controls, _ = fixture.ok(rust, ['mission', 'show', fixture.work])
        check('pause-creates-manual-controls', controls['paused'] is True, controls)
        check('pause-creates-no-consent', controls['consent_enabled'] is None, controls)
        paused_autonomy = controls['autonomy']
        baseline = fixture.witness()
        go_paused, go_paused_window = fixture.list_document(go)
        rust_paused, rust_paused_window = fixture.list_document(rust)
        fixture.snapshots.append(compare_lists(
            go_paused, rust_paused, 'go-list', 'rust-list',
            go_paused_window, rust_paused_window, 'post-pause-agent-list', fixture.root))
        retained.publish(output / 'snapshots.json', fixture.snapshots)
        check('post-pause-lists-match-except-snapshot-as-of', True)
        fixture.reader_immutability(baseline, 'post-pause')
        resumed_state, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        fixture.ok(rust, ['mission', 'unpause', fixture.work], dict(
            schema_version=1, event_id='cockpit-agent-list-unpause',
            expected_revision=resumed_state['work']['revision'],
            reason='Resume while preserving the manual controls'))
        check('rust-mission-unpause-succeeds', True)
        resumed, _ = fixture.ok(rust, ['mission', 'show', fixture.work])
        check('unpause-clears-paused-state', resumed['paused'] is False, resumed)
        check('unpause-preserves-manual-mode',
              resumed['autonomy'] == paused_autonomy and resumed['consent_enabled'] is None, resumed)
        baseline = fixture.witness()
        go_resumed, go_resumed_window = fixture.list_document(go)
        rust_resumed, rust_resumed_window = fixture.list_document(rust)
        fixture.snapshots.append(compare_lists(
            go_resumed, rust_resumed, 'go-list', 'rust-list',
            go_resumed_window, rust_resumed_window, 'post-unpause-agent-list', fixture.root))
        retained.publish(output / 'snapshots.json', fixture.snapshots)
        check('post-unpause-lists-match-except-snapshot-as-of', True)
        fixture.reader_immutability(baseline, 'post-unpause')
        # Every negative below follows the positive list/pause witnesses above.
        fixture.refused(rust, [*LIST_COMMAND, 'cockpit-agent-list-missing-work'], name='unknown-work')
        check('unknown-work-refused', True)
        fixture.refused(rust, [*LIST_COMMAND, fixture.work, 'unexpected-extra-argument'], name='extra-arguments')
        check('invalid-extra-cli-argument-refused', True)
        fixture.refused(rust, [*LIST_COMMAND, fixture.work, '--definitely-not-a-fixture-flag'], name='unknown-flag')
        check('unknown-cli-flag-refused', True)
        fixture.reader_immutability(baseline, 'list-refusals')
        current, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        fixture.ok(rust, ['task', 'add', fixture.work], dict(
            schema_version=1, event_id='cockpit-agent-list-task',
            expected_revision=current['work']['revision'], id='fixture-task',
            title='Task projection witness', deliverable='Refusal evidence',
            criteria=['Refuse instead of manufacturing empty sections']))
        check('native-task-add-succeeds', True)
        tasked, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        check('work-has-one-task',
              isinstance(tasked['work'].get('tasks'), list) and len(tasked['work']['tasks']) == 1,
              tasked['work'].get('tasks'))
        baseline = fixture.witness()
        go_tasked, _ = fixture.list_document(go)
        check('go-reference-list-succeeds-with-task', isinstance(go_tasked, dict))
        if initial_tasks:
            initial_task_journey(fixture)
            current, _ = fixture.ok(rust, ['work', 'show', fixture.work])
            fixture.ok(rust, ['task', 'add', fixture.work], dict(
                schema_version=1, event_id='cockpit-agent-list-dependent-task',
                expected_revision=current['work']['revision'], id='dependent-task',
                title='Unsupported dependency witness', deliverable='Explicit refusal',
                criteria=['No manufactured projection'], depends=['fixture-task']))
            if todo_dependencies:
                before = fixture.witness()
                left, lw = fixture.list_document(go)
                right, rw = fixture.list_document(rust)
                fixture.snapshots.append(compare_lists(left, right, 'go', 'rust', lw, rw,
                                         'todo-dependency-graph', fixture.root))
                retained.publish(output / 'snapshots.json', fixture.snapshots)
                check('todo-dependency-documents-match', True)
                check('todo-dependency-waiting-and-not-ready',
                      right['pilotage']['tasks']['dependent-task']['waiting_on'] == ['fixture-task']
                      and right['pilotage']['tasks']['dependent-task']['ready'] is False)
                edge = right['pilotage']['edges']
                check('todo-dependency-edge-is-unvalidated', len(edge) == 1
                      and edge[0]['satisfied_now'] is False
                      and edge[0]['reason_code'] == 'dependency_waiting')
                actions = {a['kind']: a for a in right['task_actions']['dependent-task']}
                check('todo-dependency-start-reason', actions['start'].get('raison') ==
                      'Dépendance non validée ou périmée : fixture-task.')
                check('todo-dependency-override-reason', actions['override'].get('raison') ==
                      "Acceptez d'abord la dépendance fixture-task (revue ou dérogation).")
                check('todo-dependency-priority-does-not-invent-default-row',
                      right['priority'] == {'fixture-task': 7})
                fixture.reader_immutability(before, 'todo-dependency')
                for task_id, dependencies in [('extra-task', []),
                        ('dependent-multi', ['extra-task', 'fixture-task']),
                        ('dependent-chain', ['dependent-task', 'dependent-multi'])]:
                    current, _ = fixture.ok(rust, ['work', 'show', fixture.work])
                    fixture.ok(rust, ['task', 'add', fixture.work], dict(
                        schema_version=1, event_id='cockpit-add-' + task_id,
                        expected_revision=current['work']['revision'], id=task_id,
                        title='Dependency order witness', deliverable='Read-only graph',
                        criteria=['Preserve saved dependency ordering'], depends=dependencies))
                before = fixture.witness()
                left, lw = fixture.list_document(go)
                right, rw = fixture.list_document(rust)
                fixture.snapshots.append(compare_lists(left, right, 'go', 'rust', lw, rw,
                                         'todo-multiple-dependency-graph', fixture.root))
                retained.publish(output / 'snapshots.json', fixture.snapshots)
                check('todo-multiple-dependency-documents-match', True)
                check('todo-multiple-dependency-preserves-waiting-order',
                      right['pilotage']['tasks']['dependent-multi']['waiting_on'] ==
                      ['extra-task', 'fixture-task'])
                check('todo-multiple-dependency-preserves-edge-order',
                      [(e['from_task_id'], e['to_task_id']) for e in right['pilotage']['edges']] ==
                      [('fixture-task', 'dependent-task'), ('extra-task', 'dependent-multi'),
                       ('fixture-task', 'dependent-multi'), ('dependent-task', 'dependent-chain'),
                       ('dependent-multi', 'dependent-chain')])
                multi = {a['kind']: a for a in right['task_actions']['dependent-multi']}
                check('todo-multiple-dependency-first-reason', multi['start'].get('raison') ==
                      'Dépendance non validée ou périmée : extra-task.')
                fixture.reader_immutability(before, 'todo-multiple-dependency')
                current, _ = fixture.ok(rust, ['work', 'show', fixture.work])
                fixture.ok(rust, ['task', 'update', fixture.work], dict(
                    schema_version=1, event_id='cockpit-agent-list-blocked-task',
                    expected_revision=current['work']['revision'], id='dependent-task',
                    status='blocked', blocker='Owned unsupported-state witness'))
            baseline = fixture.witness()
        _, record = fixture.refused(rust, [*LIST_COMMAND, fixture.work],
                                    expected='cockpit-task-projection-unported', name='task-projection')
        check('rust-list-refuses-task-projection-unported', True)
        manufactured = (output / (record['name'] + '.stdout')).read_bytes()
        check('no-manufactured-task-actions-or-reviews', manufactured == b'')
        latest, _ = fixture.ok(rust, ['work', 'show', fixture.work])
        revision = latest['work']['revision']
        fixture.refused(rust, ['mission', 'pause', fixture.work],
                        dict(schema_version=1, event_id='malformed-pause-without-reason',
                             expected_revision=revision),
                        name='malformed-pause-without-reason')
        check('malformed-pause-without-reason-refused', True)
        fixture.refused(rust, ['mission', 'pause', fixture.work],
                        dict(schema_version=999, event_id='malformed-pause-unknown-schema',
                             expected_revision=revision, reason='Unsupported schema version'),
                        name='malformed-pause-unknown-schema')
        check('malformed-pause-unknown-schema-refused', True)
        mission_after, _ = fixture.ok(rust, ['mission', 'show', fixture.work])
        check('malformed-controls-leave-mission-unpaused-without-consent',
              mission_after['paused'] is False and mission_after['consent_enabled'] is None, mission_after)
        fixture.reader_immutability(baseline, 'malformed-controls')
        # Public argv ledger establishes that this harness never invokes an
        # installer. It does not trace subprocesses or prove OS confinement.
        check('binaries-kept-outside-fixture-root',
              fixture.root not in go.parents and fixture.root not in rust.parents)
        check('binaries-unchanged',
              retained.digest(go.read_bytes()) == go_sha256
              and retained.digest(rust.read_bytes()) == rust_sha256)
        if initial_tasks:
            global_workspace_journey(go, rust, output / 'global-workspace')
        assert len(fixture.checks) >= 30, 'named-journey-assertions-incomplete'
        retained.publish(output / 'result.json', {
            'status': 'PASS', 'checks': fixture.checks, 'work': fixture.work,
            'root': str(fixture.root), 'go_sha256': go_sha256, 'rust_sha256': rust_sha256,
            'binaries_unchanged': True, 'calls': len(fixture.calls),
            'snapshots': fixture.snapshots, 'refusals': fixture.refusals,
            'legacy_go_reference_only': True, 'installed': False,
            'production_fallback': False, 'provider_started': False,
            'providers_or_watch_invoked': False, 'live_database': False,
            'direct_sqlite_edits': False, 'agents_launched': False,
            'consent_granted': False, 'global_watchdog': None,
            'command_timeout': None, 'retries': 0,
            'priority_corruption_fixture': 'left_to_parent_no_actual_schema_evidence',
            'slice': ('todo-dependencies-no-history-agents-planning' if todo_dependencies else
                      'initial-todo-no-dependencies-history-agents-planning' if initial_tasks
                      else 'no-tasks-planning-agents; rust refuses cockpit-task-projection-unported')})
        print('COCKPIT_AGENT_LIST_VERIFIED ' + str(len(fixture.checks)), flush=True)
    except BaseException:
        retained.publish(output / 'failure.json', dict(
            status='FAIL', root=str(fixture.root), work=fixture.work,
            checks=list(fixture.checks), cause=traceback.format_exc()))
        raise


def main():
    """Require a fresh evidence directory and existing binaries before any Root."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--go-binary', type=Path, required=True)
    parser.add_argument('--rust-binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--initial-tasks', action='store_true', help='Require initial TODO task projection')
    parser.add_argument('--todo-dependencies', action='store_true', help='Require TODO dependency graph projection')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False, mode=0o700)
    retained.publish(output / 'driver.py', Path(__file__).read_bytes())
    run(args.go_binary.resolve(strict=True), args.rust_binary.resolve(strict=True), output,
        args.initial_tasks or args.todo_dependencies, args.todo_dependencies)


if __name__ == '__main__':
    main()
