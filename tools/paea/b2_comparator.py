#!/usr/bin/env python3
"""B2 hardened-workflow comparator: the laboratory's own protections, scripted.

G6 main comparator (E3-PROTOCOL: « Comparateur principal : workflow
déterministe B2 disposant des mêmes protections métier connues. Ne pas
affaiblir B2… »). It implements NO business protection of its own: every
admission path is the existing pilot laboratory's, imported unchanged from
tests/paea_pilot_lab_test.py — pinned-CLI canonicalization with before/after
hash check, epoch-approval authority, intent-stable sinks, separate-process
oracle, declared reconciliation, witness before every fault. B2 is the
scripted deterministic workflow around those primitives: fixed scenario
order, one fresh sink per trial, per-trial comparable records (decision,
exit, duration) and a closed report carrying the protocol's honest labels
(simulated authority, same-UID isolation, independence not established).
Any deviation stops publication and is never scored; no model, campaign,
retry or network is involved.
"""
import hashlib
import importlib.util
import json
import math
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / 'tests'))

from tools.paea import pilot_lab as lab  # noqa: E402

REPORT = 'paea-b2-report.json'
SCENARIO_FAMILIES = ('T04-substitution', 'T06-T21-crash-recovery', 'T05-T17-revocation-order')


def digest(raw):
    """SHA256 of exact bytes; content identity only, never an author."""
    return hashlib.sha256(raw).hexdigest()


def percentile(values, fraction):
    """Nearest-rank percentile of measured seconds; empty input is a caller bug."""
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, math.ceil(fraction * len(ordered)) - 1))
    return ordered[index]


def load_pilot_driver():
    """Import the maintained pilot test module; its primitives are the B2 protections.

    The module imports repository tools through its own sys.path setup, so a
    plain import by file location keeps every helper byte-identical to the
    qualified recipe. Any change to that file changes this comparator too —
    intended: B2 and the candidate share one admission contract.
    """
    spec = importlib.util.spec_from_file_location(
        'paea_pilot_lab_test', ROOT / 'tests/paea_pilot_lab_test.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class B2Runner:
    """Deterministic B2 workflow runner on the shared pilot driver.

    Each trial builds a fresh PilotLab instance bound to its own fresh
    evidence subdirectory (the driver's setUpClass creates the directory
    itself and re-checks the pinned binary at tearDownClass), records
    comparable per-trial rows, and reports deviations instead of scoring
    them.
    """

    def __init__(self, driver, evidence, binary_hash):
        self.driver = driver
        self.evidence = Path(evidence).resolve()
        self.binary_hash = binary_hash
        self.trials = []
        self.deviations = []

    def suite(self, slug, method_name):
        """Fresh driver instance on a fresh evidence directory for one trial."""
        trial_dir = self.evidence / slug
        if trial_dir.exists():
            raise FileExistsError('trial evidence directory must not exist: %s' % trial_dir)
        os.environ['SWARM_PAEA_PILOT_EVIDENCE'] = str(trial_dir)
        instance = self.driver.PilotLab(method_name)
        instance.setUpClass()
        return instance

    def record(self, name, domain, decision, start, exit_code=None, notes=None):
        """Append one comparable trial row with its wall-clock duration."""
        row = {'scenario': name, 'domain': domain, 'decision': decision,
               'duration_s': round(time.monotonic() - start, 6)}
        if exit_code is not None:
            row['exit'] = exit_code
        if notes:
            row['notes'] = notes
        self.trials.append(row)
        return row

    def deviation(self, name, domain, cause):
        """Record a deviation; publication is refused later, evidence is kept."""
        self.deviations.append({'scenario': name, 'domain': domain, 'cause': cause})
        self.record(name, domain, 'DEVIATION', time.monotonic(), notes={'cause': cause})

    def t04(self, domain):
        """T04: substituted content refused pre-effect, sink unchanged, equivalent JSON accepted.

        Suite setup sits inside the guarded region on purpose: an environment
        failure at setUpClass (binary replaced between trials) is a recorded
        deviation, not a crash with an unwritten trial directory.
        """
        name, start = 'T04-substitution', time.monotonic()
        suite = None
        try:
            suite = self.suite('T04-%s' % domain, 'test_t04_substitution_then_equivalent_json_recovery')
            suite.witness(domain)
            root, doc, report, oids = suite.prepare(domain)
            before = suite.oracle(root, domain)
            changed = json.loads(json.dumps(doc))
            if domain == 'payment':
                changed['proposal']['action']['amount_minor'] = '420001'
            else:
                changed['proposal']['action']['candidate_oid'] = oids['other']
            refused = suite.commit(root, changed)
            denied = refused.returncode == 2 and b'laboratory-approval-missing' in refused.stderr
            unchanged = suite.oracle(root, domain) == before
            equivalent = json.dumps(doc, indent=2, sort_keys=True).encode()
            accepted = suite.commit(root, doc, raw=equivalent)
            suite.assert_effect(suite.oracle(root, domain), domain, report)
            if denied and unchanged and accepted.returncode == 0:
                self.record(name, domain, 'refused-then-accepted', start,
                            exit_code=accepted.returncode, notes={'refused_exit': refused.returncode})
            else:
                self.deviation(name, domain, 'refusal=%s unchanged=%s accepted=%s' % (denied, unchanged, accepted.returncode))
        except (AssertionError, OSError, KeyError, ValueError) as error:
            self.deviation(name, domain, '%s: %s' % (type(error).__name__, error))
        finally:
            if suite is not None:
                suite.tearDownClass()

    def t06_t21(self, domain):
        """T06/T21: post-effect crash leaves one effect, reconciliation is exact, no duplicate.

        Mirrors the qualified driver scenario: UNKNOWN before observation, one
        effect/marker, RECONCILED_COMMITTED, replay under a new transport key
        with the same report accepted idempotently, then a new context with
        another digest refused as laboratory-intent-conflict.
        """
        name, start = 'T06-T21-crash-recovery', time.monotonic()
        suite = None
        try:
            suite = self.suite('T06-T21-%s' % domain, 'test_t06_t21_crash_new_attempt_exact_marker_and_reconciliation')
            suite.witness(domain)
            root, doc, report, _ = suite.prepare(domain)
            crashed = suite.commit(root, doc, fault=True)
            if crashed.returncode != 86 or crashed.stdout != b'':
                self.deviation(name, domain, 'crash exit=%s stdout=%r' % (crashed.returncode, crashed.stdout))
                return
            after = suite.oracle(root, domain)
            if domain == 'payment':
                effects = len(after['effects'])
            else:
                effects = len([row for row in after['refs'] if row['ref'].startswith('refs/paea-lab/intents/')])
            if domain == 'git':
                db = lab.connect(root)
                try:
                    effects_table = db.execute('SELECT count(*) FROM effects').fetchone()[0]
                finally:
                    db.close()
            else:
                effects_table = effects
            outcome = lab.reconcile(root, report)
            replay = json.loads(json.dumps(doc))
            replay['proposal']['request_id'] = 'b2-replay-transport-key'
            replay_report = suite.report(replay)
            replayed = suite.commit(root, replay)
            conflict_doc = json.loads(json.dumps(doc))
            conflict_doc['context']['instance_id'] = 'b2-other-instance'
            other = suite.report(conflict_doc)
            lab.authority(root, 'approve', other)
            conflict = suite.commit(root, conflict_doc)
            conflict_reconcile = lab.reconcile(root, other)
            unchanged = suite.oracle(root, domain) == after
            if (effects == 1 and effects_table == (0 if domain == 'git' else 1)
                    and replay_report == report and replayed.returncode == 0 and unchanged
                    and other['intent_id'] == report['intent_id']
                    and other['action_digest'] != report['action_digest']
                    and conflict.returncode == 2 and b'laboratory-intent-conflict' in conflict.stderr
                    and conflict_reconcile == {'state': 'UNKNOWN', 'cause': 'laboratory-intent-conflict'}):
                self.record(name, domain, 'unknown-recovered-once', start,
                            exit_code=replayed.returncode, notes={'reconcile': outcome['state']})
            else:
                self.deviation(name, domain, 'effects=%s reconcile=%s replay=%s conflict=%s unchanged=%s' % (
                    effects, outcome['state'], replayed.returncode, conflict.returncode, unchanged))
        except (AssertionError, OSError, KeyError, ValueError) as error:
            self.deviation(name, domain, '%s: %s' % (type(error).__name__, error))
        finally:
            if suite is not None:
                suite.tearDownClass()

    def t05_t17(self, domain):
        """T05/T17: revocation before first use refuses; current grant commits; reverse order holds.

        Mirrors the qualified driver scenario minus its contention extension:
        ordered case (approve → revoke → refuse → re-approve → commit) with the
        durable ordering journal, then the reverse case (commit before revoke).
        """
        name, start = 'T05-T17-revocation-order', time.monotonic()
        suite = None
        try:
            suite = self.suite('T05-T17-%s' % domain, 'test_t05_t17_ordered_revocation_first_use_and_opposite_order')
            suite.witness(domain)
            root, doc, report, _ = suite.prepare(domain)
            before = suite.oracle(root, domain)
            lab.authority(root, 'revoke')
            denied = suite.commit(root, doc)
            refused = denied.returncode == 2 and b'laboratory-epoch-revoked' in denied.stderr
            unchanged = suite.oracle(root, domain) == before
            not_committed = lab.reconcile(root, report)['state'] == 'RECONCILED_NOT_COMMITTED'
            lab.authority(root, 'approve', report)
            accepted = suite.commit(root, doc)
            suite.assert_effect(suite.oracle(root, domain), domain, report)
            db = lab.connect(root)
            try:
                ordering = db.execute('SELECT event,epoch FROM ordering ORDER BY seq').fetchall()
            finally:
                db.close()
            reverse, reverse_doc, reverse_report, _ = suite.prepare(domain)
            reverse_first = suite.commit(reverse, reverse_doc)
            reverse_observed = suite.oracle(reverse, domain)
            lab.authority(reverse, 'revoke')
            reverse_denied = suite.commit(reverse, reverse_doc)
            reverse_unchanged = suite.oracle(reverse, domain) == reverse_observed
            reverse_reconciled = lab.reconcile(reverse, reverse_report)['state'] == 'RECONCILED_COMMITTED'
            if (refused and unchanged and not_committed and accepted.returncode == 0
                    and ordering == [('approve', 1), ('revoke', 2), ('approve', 2), ('commit', 2)]
                    and reverse_first.returncode == 0 and reverse_denied.returncode == 2
                    and reverse_unchanged and reverse_reconciled):
                self.record(name, domain, 'revoked-refused-then-current-grant-commits', start,
                            exit_code=accepted.returncode)
            else:
                self.deviation(name, domain, 'refused=%s unchanged=%s not_committed=%s accepted=%s ordering=%s reverse=%s/%s/%s/%s' % (
                    refused, unchanged, not_committed, accepted.returncode, ordering,
                    reverse_first.returncode, reverse_denied.returncode, reverse_unchanged, reverse_reconciled))
        except (AssertionError, OSError, KeyError, ValueError) as error:
            self.deviation(name, domain, '%s: %s' % (type(error).__name__, error))
        finally:
            if suite is not None:
                suite.tearDownClass()

    def run_all(self):
        """Run the three scenario families on both domains in fixed order."""
        for domain in ('payment', 'git'):
            self.t04(domain)
            self.t06_t21(domain)
            self.t05_t17(domain)

    def report_value(self):
        """Closed comparable report: per-trial rows, counts, latencies, honest labels."""
        durations = [row['duration_s'] for row in self.trials]
        return {
            'schema_version': 1,
            'comparator': 'b2-hardened-workflow',
            'comparator_basis': 'pilot-laboratory-primitives-reused-unchanged',
            'scope': 'isolated-laboratory',
            'binary_sha256': self.binary_hash,
            'scenario_families': list(SCENARIO_FAMILIES),
            'trials': self.trials,
            'deviations': self.deviations,
            'counts': {
                'trials': len(self.trials),
                'refused_pre_effect': sum(1 for row in self.trials if row['decision'].startswith(('refused', 'revoked'))),
                'recovered_unknown': sum(1 for row in self.trials if row['decision'] == 'unknown-recovered-once'),
                'deviations': len(self.deviations),
            },
            'latency_s': {
                'p50': percentile(durations, 0.50) if durations else None,
                'p95': percentile(durations, 0.95) if durations else None,
            },
            'authority': 'simulated',
            'isolation': 'same-uid',
            'independent_evaluation': 'not-established',
            'protected_invariants': 'not-established',
        }


def main():
    """Run six B2 trials, refuse publication on any deviation, publish the closed report.

    Requires the pilot recipe's environment: a pinned existing binary plus its
    SHA256 and a NEW parent evidence directory (created here; each trial then
    gets its own fresh subdirectory via the driver's setUpClass). The binary
    must stay byte-identical for the whole run — every trial's tearDownClass
    re-checks it. No retry, no partial success report.
    """
    binary = os.environ.get('SWARM_PAEA_TEST_BINARY')
    binary_hash = os.environ.get('SWARM_PAEA_TEST_BINARY_SHA256', '')
    evidence = os.environ.get('SWARM_PAEA_PILOT_EVIDENCE')
    if not binary or len(binary_hash) != 64 or not evidence or Path(evidence).exists():
        raise SystemExit('usage: SWARM_PAEA_TEST_BINARY=<pinned binary> '
                         'SWARM_PAEA_TEST_BINARY_SHA256=<its sha256> '
                         'SWARM_PAEA_PILOT_EVIDENCE=<new dir> b2_comparator.py')
    try:
        actual_hash = digest(Path(binary).resolve(strict=True).read_bytes())
    except OSError as error:
        raise SystemExit('binary-unreadable: %s' % error) from error
    if actual_hash != binary_hash:
        raise SystemExit('binary-hash-mismatch')
    driver = load_pilot_driver()
    runner = B2Runner(driver, evidence, binary_hash)
    runner.evidence.parent.mkdir(parents=True, exist_ok=True)
    runner.evidence.mkdir()
    runner.run_all()
    value = runner.report_value()
    if value['counts']['deviations']:
        print(json.dumps({'state': 'DEVIATION', 'deviations': runner.deviations}),
              file=sys.stderr, flush=True)
        return 1
    spec = importlib.util.spec_from_file_location('paea_evidence_publisher', ROOT / 'tools/verification/recipe.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    destination = runner.evidence / REPORT
    module.publish(destination, (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode())
    print('PAEA_B2_REPORT_PUBLISHED trials=%d p50=%.3fs p95=%.3fs sha256=%s' % (
        value['counts']['trials'], value['latency_s']['p50'], value['latency_s']['p95'],
        digest(destination.read_bytes())), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
