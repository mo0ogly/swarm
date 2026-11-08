#!/usr/bin/env python3
"""Maintained method/baseline/fault counterexamples for the consent driver's
failure and flush contract (tests/paea_consent_ui_recipe.py).

Method: deterministic doubles replace subprocess/threading INSIDE the imported
driver module — labelled fake executable, labelled raw channel bytes — so the
faults are reproducible without a real Rust build, without any timeout and
without touching any foreign process. The doubles are explicitly labelled
control fixtures: they prove the DRIVER's bookkeeping contract, never
identity, never enrollment, never a native product qualification (the real
qualification stays the recipe's browser step).

Discrimination history: the supervisor's isolated control probe
(glm-consent-proof-preservation-20261101T040336Z/supervisor-personal/,
sha256-pinned in that dossier's status.json) ran equivalent doubles against
the pre-fix driver and measured the RED: `rust-failed-harness-live` blocked
on a live harness.wait before cleanup, `child-output-not-drained` returned
PASS with zero joins and no retained rust/harness stdout, baseline had 0
joins. This suite encodes those measured defects as refusals and must stay
green on the fixed driver: baseline PASS with 4 tracked streamers joined and
both labelled channels retained; a Rust failure while the harness runs
raises IMMEDIATELY (any wait on a live child raises the WouldBlock
sentinel); the INVERSE fault — a harness failure while Rust still waits on
the marker — fails typed with the harness primary cause, killing live Rust
BEFORE any wait and never blocking before cleanup/channel drain
(harness-failed-rust-live, harness-error-after-rust-exit); a Rust death
before publication fails without starting the harness; an undrained
streamer refuses the PASS (EOF never reached); a changed binary refuses
the PASS; a resolution failure propagates typed.

Errors: any unexpected exception is a suite failure. Outputs: a control
report JSON with one result per variant (verdict, streamer/join counts,
retained-channel booleans) written into the directory given as argv[1] —
fresh per run, never overwritten.
"""
import contextlib,datetime,importlib.util,io,json,os,subprocess,sys,threading,types
from pathlib import Path

DRIVER = Path(__file__).resolve().parent / 'paea_consent_ui_recipe.py'

class WouldBlock(Exception):
    """Sentinel: the driver called wait() on a live child on a failure path."""

def load_driver():
    spec = importlib.util.spec_from_file_location('consentdriver', DRIVER)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

def play_variant(name: str, out: Path, fixture: Path) -> dict:
    """Run one labelled variant against freshly imported driver doubles."""
    mod = load_driver()
    ctx = {'harness_started': False, 'threads_started': 0, 'joins': 0,
           'wait_before_cleanup': False, 'popen_calls': 0}
    all_live = name == 'child-output-not-drained'

    class Thread:
        """Double streamer: runs synchronously unless the variant starves it."""
        def __init__(self, target, args, daemon=True, name=None):
            self.target, self.args, self.name = target, args, name
        def start(self):
            ctx['threads_started'] += 1
            if not all_live:
                self.target(*self.args)
        def join(self, timeout=None):
            ctx['joins'] += 1
        def is_alive(self):
            return all_live

    class Proc:
        """Double child: labelled pipes, variant-owned terminal states."""
        def __init__(self, kind):
            self.kind, self.killed = kind, False
            self.stdout = io.BytesIO(b'labelled raw stdout')
            self.stderr = io.BytesIO(b'labelled raw stderr')
            self.returncode = None
        def poll(self):
            if self.killed:
                self.returncode = -9
            elif self.kind == 'rust':
                if name == 'rust-died-before-publication':
                    self.returncode = 1
                elif ctx['harness_started']:
                    if name == 'rust-failed-harness-live':
                        self.returncode = 1
                    elif name == 'harness-failed-rust-live':
                        # The live-Rust sentinel case: Rust stays live holding
                        # the marker; any wait() on it while live must hit the
                        # WouldBlock sentinel before cleanup.
                        self.returncode = None
                    else:
                        self.returncode = 0
                else:
                    self.returncode = None
            else:
                if name in ('harness-failed-rust-live', 'harness-error-after-rust-exit'):
                    # Harness fails (exit 3) in both paired directions while
                    # the sibling side of the contract is exercised: Rust
                    # still waits on the marker (live-Rust sentinel) or has
                    # already exited.
                    self.returncode = None if not ctx['harness_started'] else 3
                else:
                    self.returncode = None if name == 'rust-failed-harness-live' else 0
            return self.returncode
        def wait(self):
            # A real Popen.wait() can only block on a LIVE child (poll() is
            # None); the WouldBlock sentinels below therefore fire only for a
            # live child, so a sibling that already reached a terminal state
            # (e.g. Rust exit 0 before a later harness error) is collected
            # without any blocking, exactly like the real contract expects.
            if (self.kind == 'harness' and not self.killed
                    and self.poll() is None
                    and name == 'rust-failed-harness-live'):
                ctx['wait_before_cleanup'] = True
                raise WouldBlock('driver waits on live harness before error/cleanup')
            if (self.kind == 'rust' and not self.killed
                    and self.poll() is None
                    and name in ('harness-failed-rust-live',
                                 'harness-error-after-rust-exit')):
                ctx['wait_before_cleanup'] = True
                raise WouldBlock('driver waits on live rust after harness '
                                 'failure, before cleanup and channel drain')
            return self.poll()
        def kill(self):
            self.killed, self.returncode = True, -9
        def terminate(self):
            self.kill()
        def communicate(self, timeout=None):
            # Compatible with the pre-fix communicate architecture: returns
            # the labelled bytes once and marks the child terminal via poll().
            self.poll()
            return self.stdout.read(), self.stderr.read()

    def popen(argv, **kwargs):
        ctx['popen_calls'] += 1
        if argv[0] == 'node':
            ctx['harness_started'] = True
            if name == 'changed-binary':
                fixture.write_bytes(b'labelled control fixture, changed bytes')
            (out / 'consent-bound-ui.json').write_text(json.dumps(
                {'checks': ['fixture-check-1', 'fixture-check-2'],
                 'unexpected_diagnostics': []}))
            if name == 'harness-error-after-rust-exit':
                # Double harness: the first fails (exit 3) and terminates;
                # the double returns it so the driver's loop sees the real
                # terminal state, then the sibling logic applies.
                first = Proc('harness')
                first.returncode = 3
                return first
            return Proc('harness')
        if name != 'rust-died-before-publication':
            (Path(kwargs['env']['CONSENT_BROWSER_DIR']) / 'server.json').write_text('{}')
        return Proc('rust')

    def resolve(spec):
        if name == 'resolve-fail':
            raise SystemExit('resolve-refused')
        return fixture
    mod.resolve_binary = resolve
    mod.subprocess = types.SimpleNamespace(Popen=popen, PIPE=subprocess.PIPE,
                                           run=subprocess.run)
    mod.threading = types.SimpleNamespace(Thread=Thread, Event=threading.Event)
    os.environ['SWARM_PAEA_SOURCE_FIXTURE'] = str(out.parent)
    captured = io.StringIO()
    verdict = 'driver_returned_PASS'
    try:
        with contextlib.redirect_stdout(captured):
            mod.run(fixture, out)
    except WouldBlock as error:
        verdict = f'WOULD_BLOCK: {error}'
    except SystemExit as error:
        verdict = f'SYSTEM_EXIT: {error}'
    except BaseException as error:
        verdict = f'{type(error).__name__}: {error}'
    return {
        'variant': name,
        'verdict': verdict,
        'pass_printed': 'PAEA_CONSENT_BROWSER_VERIFIED' in captured.getvalue(),
        'harness_started': ctx['harness_started'],
        'threads_started': ctx['threads_started'],
        'joins': ctx['joins'],
        'wait_before_cleanup': ctx['wait_before_cleanup'],
        'popen_calls': ctx['popen_calls'],
        'retained_rust_stdout': (out / 'rust.stdout').exists(),
        'retained_harness_stdout': (out / 'harness.stdout').exists(),
        'labelled_bytes_intact': ((out / 'rust.stdout').read_bytes() == b'labelled raw stdout')
                                 if (out / 'rust.stdout').exists() else False,
    }

def main() -> int:
    out_root = Path(sys.argv[1]).resolve()
    # Evidence is never overwritten: a leftover directory from an earlier run
    # (kept for diagnosis) is archived aside with a timestamp and the fresh
    # run starts empty at the declared path — same convention as the drivers
    # themselves, so a recipe step stays replayable after any input changes.
    if out_root.exists():
        archive = out_root.with_name(
            f"{out_root.name}-superseded-{datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}"
        )
        out_root.rename(archive)
        print(f"prior evidence archived aside: {archive}", flush=True)
    out_root.mkdir(parents=True, exist_ok=False)
    fixture = out_root / 'labelled-fake-executable'
    fixture.write_bytes(b'labelled control fixture, unchanged bytes')
    fixture.chmod(0o700)
    results = [play_variant(name, out_root / name, fixture)
               for name in ('valid-baseline', 'rust-failed-harness-live',
                            'harness-failed-rust-live',
                            'harness-error-after-rust-exit',
                            'rust-died-before-publication',
                            'child-output-not-drained', 'changed-binary',
                            'resolve-fail')]
    # The per-variant report is written BEFORE the contract assertions: a RED
    # run (frozen pre-fix driver) keeps the measured counts, not only the
    # first failed assertion.
    report = {
        'schema_version': 1,
        'kind': 'paea-consent-driver-control-v2',
        'scope': 'deterministic labelled doubles on the driver module; driver '
                 'bookkeeping contract only — no identity, no enrollment, no '
                 'native qualification',
        'results': results,
        'at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }
    (out_root / 'driver-control.json').write_text(json.dumps(report, indent=2) + '\n')
    # Contract assertions: each encodes a measured pre-fix defect (RED) or the
    # baseline method, and must hold on the fixed driver (GREEN).
    by = {r['variant']: r for r in results}
    assert by['valid-baseline']['pass_printed'], 'baseline must still PASS'
    assert by['valid-baseline']['threads_started'] == 4 and by['valid-baseline']['joins'] == 4, \
        'baseline: every streamer of BOTH children tracked and joined'
    assert by['valid-baseline']['labelled_bytes_intact'], 'baseline: labelled channel bytes retained'
    live = by['rust-failed-harness-live']
    assert live['verdict'].startswith('SYSTEM_EXIT') and 'while the harness ran' in live['verdict'], \
        f'rust failure while harness live must fail immediately, got: {live["verdict"]}'
    assert not live['wait_before_cleanup'], 'no wait on a live harness before cleanup'
    assert live['retained_rust_stdout'] and live['labelled_bytes_intact'], \
        'rust failure: raw channels retained before verdict'
    paired = by['harness-failed-rust-live']
    assert paired['verdict'].startswith('SYSTEM_EXIT') and not paired['pass_printed'], \
        f'harness failure while Rust lives must fail typed, got: {paired["verdict"]}'
    assert 'browser harness failed' in paired['verdict'] or 'exit 3' in paired['verdict'], \
        f'failure must carry the harness primary cause, got: {paired["verdict"]}'
    assert not paired['wait_before_cleanup'], \
        'no wait on live Rust after harness failure, before cleanup/drain'
    late = by['harness-error-after-rust-exit']
    assert late['verdict'].startswith('SYSTEM_EXIT') and not late['pass_printed'], \
        f'harness failure with Rust live must fail typed with the harness cause, got: {late["verdict"]}'
    assert not late['wait_before_cleanup'], \
        'no wait on live Rust on the harness-failure path (kill owns terminality)'
    early = by['rust-died-before-publication']
    assert early['verdict'].startswith('SYSTEM_EXIT') and 'before publishing server.json' in early['verdict'], \
        f'rust death before publication must fail typed, got: {early["verdict"]}'
    assert not early['harness_started'], 'harness must not start on a refused publication'
    undrained = by['child-output-not-drained']
    assert undrained['verdict'].startswith('SYSTEM_EXIT') and not undrained['pass_printed'], \
        f'undrained streamers must refuse the PASS, got: {undrained["verdict"]}'
    assert ('EOF never reached' in undrained['verdict']
            or 'not fully drained' in undrained['verdict']), \
        f'refusal must name the undrained EOF, got: {undrained["verdict"]}'
    changed = by['changed-binary']
    assert not changed['pass_printed'] and 'changed during the run' in changed['verdict'], \
        f'changed binary must refuse the PASS, got: {changed["verdict"]}'
    assert by['resolve-fail']['verdict'] == 'SYSTEM_EXIT: resolve-refused', \
        f'resolution failure must propagate typed, got: {by["resolve-fail"]["verdict"]}'
    print(json.dumps({'checks': 16, 'variants': len(results),
                      'unexpected_diagnostics': 0}, separators=(',', ':')))
    return 0

if __name__ == '__main__':
    sys.exit(main())
