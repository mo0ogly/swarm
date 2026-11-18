"""Inert proposal tests for packet ARTICLE-V6-P7-PRODUCER-OBSERVED-FLOOR.

They exercise the actual tools/swarm_version.prepare observed-branch admission
on isolated inert fixture files created under fixture_area(), never executed. Explicit
unit doubles stand in for native_builder, source_identity, _read_prepared and
verify_candidate, and the fake builder records every requested build label so
predicted and corrected numbering can be observed without any real build or
qualification. select_version, _parse_canonical_dev and prepare are never
replaced, so every admission and numbering outcome comes from the real code.
No provider, process, native toolchain, HTTP or live root is used and no
historical campaign or suite is replayed. Named retained runs establish results; the doubles establish producer dispatch and admission only, not the
real record validator, archive or default profile.
"""
from unittest.mock import patch
import hashlib
import importlib.util
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
_HERE = Path(__file__).resolve().parent
_ROOT = _HERE.parent
for _entry in (str(_ROOT / 'tools'), str(_HERE)):
    if _entry not in sys.path:
        sys.path.insert(0, _entry)
from fixture_area import fixture_area
_SPEC = importlib.util.spec_from_file_location('swarm_version', _ROOT / 'tools' / 'swarm_version.py')
version = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(version)
PUBLISHED = 'v0.3.8'
PREFIX = 'v0.3.9-dev.'
OBSERVED_VERSION = 'v0.3.9-dev.381'
OBSERVED_IMAGE = b'currently-running-observed-image'
OBSERVED_HASH = hashlib.sha256(OBSERVED_IMAGE).hexdigest()
CACHED_IMAGE = b'qualified-cached-image-bytes'
CACHED_HASH = hashlib.sha256(CACHED_IMAGE).hexdigest()
FROZEN_IMAGE = b'frozen-observed-image'
BROAD_DIGEST = 'broad-source-identity-constant'
DELIVERY_DIGEST = 'delivery-identity-constant'

def effective_digest(effective):
    """Pure digest rule shared by the source_identity double and the fixtures."""
    payload = json.dumps(effective, sort_keys=True, separators=(',', ':')).encode('utf-8')
    return hashlib.sha256(b'observed-producer-packet:' + payload).hexdigest()

def observation(label=OBSERVED_VERSION, image_hash=OBSERVED_HASH):
    """Return a synthetic version/hash DTO; this performs no actual image observation."""
    return {'version': label, 'binary_sha256': image_hash}

def source_identity_double(source, effective=None):
    """Return fixed broad/delivery or computed effective fixture facts; never probe source/toolchain."""
    if effective is None:
        return (BROAD_DIGEST, {})
    if effective == {}:
        return (DELIVERY_DIGEST, {})
    return (effective_digest(effective), dict(effective))

class FakeProfile:

    def __init__(self):
        """Initialize only the synthetic receipt/call logs supplied to this unit double."""
        self.published = []

    def digest(self, artifact):
        """Hash the complete owned fixture file; filesystem errors propagate."""
        return hashlib.sha256(Path(artifact).read_bytes()).hexdigest()

    def publish(self, path, data):
        """Write bytes/text to an owned fixture path and record it; this is not durable native publication."""
        target = Path(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data if isinstance(data, bytes) else data.encode('utf-8'))
        self.published.append(target)

class FakeBuilder:
    """Inert builder double: records labels, writes after.json and a frozen image."""

    def __init__(self, after_inputs, frozen_bytes):
        """Initialize only the synthetic receipt/call logs supplied to this unit double."""
        self.profile = FakeProfile()
        self.after_inputs = after_inputs
        self.frozen_bytes = frozen_bytes
        self.release_calls = []
        self.builds = []
        self.verified = []

    def release(self, label):
        """Record the label and deliberately accept syntax; real native release parsing is not exercised here."""
        self.release_calls.append(label)
        return True

    def build(self, source, destination, target, label):
        """Record a requested label and write inert image/after.json fixture bytes; run no compiler."""
        self.builds.append(label)
        profile_dir = Path(destination) / 'profile'
        profile_dir.mkdir(parents=True, exist_ok=True)
        (profile_dir / 'after.json').write_text(json.dumps(self.after_inputs))
        frozen = Path(destination) / 'frozen-swarm'
        frozen.write_bytes(self.frozen_bytes)
        return {'binary': str(frozen), 'metadata': {'version': label}}

def read_prepared_ok():
    """Return a unit reader that loads a fixture record without validating real retained proofs."""

    def read_prepared(source, expected_sha):
        """Load or fail the configured synthetic record; no real archive/native qualification is performed."""
        state = json.loads((Path(source) / 'deploy' / 'local-build.json').read_bytes())
        return (state, None, b'raw-image', {}, {}, {})
    return read_prepared

def read_prepared_drifted():
    """Return a unit reader that supplies deliberately changed fixture history."""

    def read_prepared(source, expected_sha):
        """Load or fail the configured synthetic record; no real archive/native qualification is performed."""
        state = json.loads((Path(source) / 'deploy' / 'local-build.json').read_bytes())
        drifted = dict(state)
        drifted['version'] = 'v0.3.9-dev.999'
        return (drifted, None, b'raw-image', {}, {}, {})
    return read_prepared

def read_prepared_raising(exc):
    """Return a unit reader that raises the exact supplied original exception."""

    def read_prepared(source, expected_sha):
        """Load or fail the configured synthetic record; no real archive/native qualification is performed."""
        raise exc
    return read_prepared

def read_prepared_unreachable():
    """Return a reader whose invocation is a test failure, not a successful empty proof."""

    def read_prepared(source, expected_sha):
        """Load or fail the configured synthetic record; no real archive/native qualification is performed."""
        raise AssertionError('_read_prepared must not run before observed admission completes')
    return read_prepared

def write_manifest(root):
    """Write valid FR/EN notes and the fixture published version into this owned root."""
    (root / 'version_history.json').write_text(json.dumps({'releases': [{'version': PUBLISHED}], 'local_changes': {'summary': {'fr': 'Livraison locale', 'en': 'Local delivery'}, 'items': [{'fr': 'Admission observee', 'en': 'Observed admission'}]}}))

def write_binary(root, data=OBSERVED_IMAGE):
    """Write nonexecuted fixture image bytes and return the owned path."""
    binary = root / 'swarm'
    binary.write_bytes(data)
    return binary

def write_record(root, record):
    """Serialize exactly the supplied fixture record, including malformed/null variants."""
    (root / 'deploy').mkdir(parents=True, exist_ok=True)
    (root / 'deploy' / 'local-build.json').write_text(json.dumps(record))

def write_cached_artifact(root, record):
    """Write inert cached bytes and update only the test setup record with their path/hash."""
    artifact = root / '.cache' / 'rust-migration-binaries' / 'cached-swarm'
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_bytes(CACHED_IMAGE)
    record['artifact'] = str(artifact)
    record['binary_sha256'] = CACHED_HASH
    return artifact

def touch_effective(root, effective):
    """Create named owned fixture files so effective-input selection can be exercised without a compiler."""
    for name in effective:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('effective source\n')

def base_record(**overrides):
    """Return a synthetic source record with caller overrides; no actual qualification or IO."""
    record = {'schema_version': 2, 'engine': 'rust', 'version': PREFIX + '10', 'source_digest': effective_digest({'src/lib.rs': 'h1'}), 'candidate_digest': BROAD_DIGEST, 'binary_sha256': CACHED_HASH, 'effective_inputs': {'src/lib.rs': 'h1'}, 'artifact': 'unused-by-this-double', 'binary': {'version': PREFIX + '10'}, 'evidence': 'unused-by-this-double'}
    record.update(overrides)
    return record

class ObservedProducerAdmission(unittest.TestCase):

    def setUp(self):
        """Isolate the two fixture environment overrides and forbid native/process/network boundaries."""
        for name in ('SWARM_VERSION', 'SWARM_RUST_TARGET_DIR'):
            previous = os.environ.get(name)

            def restore(previous=previous, name=name):
                """Restore exactly the captured environment value after the owning test case."""
                if previous is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = previous
            self.addCleanup(restore)
            os.environ.pop(name, None)
        self.forbidden_boundaries = []
        for owner, name in [(version.subprocess, 'run'), (version.subprocess, 'Popen'), (version.subprocess, 'check_output'), (version.os, 'kill'), (version.urllib.request, 'urlopen')]:
            guard = patch.object(owner, name, side_effect=AssertionError('forbidden actual boundary ' + name))
            self.forbidden_boundaries.append(guard.start())
            self.addCleanup(guard.stop)

    def make_root(self):
        """Allocate an owned durable fixture and require its eventual cleanup, preserving cleanup errors."""
        root = Path(tempfile.mkdtemp(prefix='observed-producer-', dir=fixture_area()))
        self.addCleanup(shutil.rmtree, root)
        return root

    def swap(self, name, replacement):
        """Replace one unit boundary and register restoration; never replace prepare or its selectors."""
        original = getattr(version, name)
        self.addCleanup(setattr, version, name, original)
        setattr(version, name, replacement)

    def install_doubles(self, after_inputs=None, frozen_bytes=FROZEN_IMAGE, read_prepared=None):
        """Install explicit inert producer/qualification boundaries and return their call log."""
        box = FakeBuilder({} if after_inputs is None else after_inputs, frozen_bytes)
        self.swap('native_builder', lambda source: box)
        self.swap('source_identity', source_identity_double)
        self.swap('verify_candidate', lambda candidate, label, notes: box.verified.append((str(candidate), label)))
        self.swap('_read_prepared', read_prepared_unreachable() if read_prepared is None else read_prepared)
        return box

    def state_bytes(self, root):
        """Read owned source-record bytes if present for before/after assertions."""
        path = root / 'deploy' / 'local-build.json'
        return path.read_bytes() if path.exists() else None

    def assert_no_publication(self, root, binary, box, image):
        """Require no build/probe/evidence/candidate publication and unchanged owned installed bytes."""
        self.assertEqual(box.builds, [])
        self.assertEqual(box.verified, [])
        self.assertFalse((root / '.cache' / 'local-delivery').exists())
        self.assertEqual(list(binary.parent.glob(binary.name + '.candidate-*')), [])
        self.assertEqual(binary.read_bytes(), image)

    def test_control_no_prior_observed_allocates_above_observation(self):
        """Exercise control no prior observed allocates above observation with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        observed = observation()
        box = self.install_doubles(after_inputs={'src/main.rs': 'after-no-prior'})
        candidate, state = version.prepare(root, binary, installed=observed)
        self.assertEqual(box.builds, [PREFIX + '382'])
        self.assertEqual([label for _, label in box.verified], [PREFIX + '382'])
        self.assertEqual(state['version'], PREFIX + '382')
        self.assertEqual(candidate.read_bytes(), FROZEN_IMAGE)
        self.assertEqual(state['binary_sha256'], hashlib.sha256(FROZEN_IMAGE).hexdigest())
        self.assertEqual(state['source_digest'], effective_digest({'src/main.rs': 'after-no-prior'}))
        self.assertEqual(observed, observation())
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)

    def test_control_above_floor_cached_version_is_retained(self):
        """Exercise control above floor cached version is retained with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = base_record(version=PREFIX + '500')
        artifact = write_cached_artifact(root, record)
        write_record(root, record)
        before = self.state_bytes(root)
        observed = observation()
        box = self.install_doubles(read_prepared=read_prepared_ok())
        result_artifact, result_state = version.prepare(root, binary, installed=observed)
        self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
        self.assertEqual(result_artifact, artifact)
        self.assertEqual(result_state, record)
        self.assertEqual(self.state_bytes(root), before)
        self.assertEqual(observed, observation())

    def test_control_equal_override_reuses_only_qualified_same_image_cache(self):
        """Exercise control equal override reuses only qualified same image cache with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root, CACHED_IMAGE)
        record = base_record(version=OBSERVED_VERSION)
        write_cached_artifact(root, record)
        write_record(root, record)
        before = self.state_bytes(root)
        observed = observation(image_hash=CACHED_HASH)
        os.environ['SWARM_VERSION'] = OBSERVED_VERSION
        box = self.install_doubles(read_prepared=read_prepared_ok())
        result_artifact, result_state = version.prepare(root, binary, installed=observed)
        self.assert_no_publication(root, binary, box, CACHED_IMAGE)
        self.assertEqual(box.release_calls, [OBSERVED_VERSION])
        self.assertEqual(result_artifact, binary)
        self.assertEqual(result_state, record)
        self.assertEqual(self.state_bytes(root), before)
        self.assertEqual(observed, observation(image_hash=CACHED_HASH))

    def test_predicted_and_actual_selector_calls_both_honour_observed_floor(self):
        """Exercise predicted and actual selector calls both honour observed floor with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        after_inputs = {'src/main.rs': 'after-both-calls'}
        record = base_record(version=PREFIX + '10', source_digest=effective_digest(after_inputs))
        write_record(root, record)
        touch_effective(root, record['effective_inputs'])
        observed = observation()
        box = self.install_doubles(after_inputs=after_inputs, read_prepared=read_prepared_ok())
        candidate, state = version.prepare(root, binary, installed=observed)
        self.assertEqual(box.builds, [PREFIX + '382'])
        self.assertEqual(state['version'], PREFIX + '382')
        self.assertEqual(state['source_digest'], effective_digest(after_inputs))
        self.assertEqual(observed, observation())
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)

    def test_below_floor_cached_version_is_skipped_for_fresh_build(self):
        """Exercise below floor cached version is skipped for fresh build with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = base_record(version=PREFIX + '10')
        artifact = write_cached_artifact(root, record)
        write_record(root, record)
        observed = observation()
        box = self.install_doubles(after_inputs={'src/main.rs': 'after-below'}, read_prepared=read_prepared_ok())
        candidate, state = version.prepare(root, binary, installed=observed)
        self.assertEqual(box.builds, [PREFIX + '382'])
        self.assertEqual(state['version'], PREFIX + '382')
        self.assertTrue(artifact.is_file())
        self.assertEqual(artifact.read_bytes(), CACHED_IMAGE)
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)
        self.assertEqual(observed, observation())

    def test_equal_counter_different_image_cache_is_skipped_for_fresh_build(self):
        """Exercise equal counter different image cache is skipped for fresh build with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = base_record(version=OBSERVED_VERSION)
        artifact = write_cached_artifact(root, record)
        write_record(root, record)
        observed = observation()
        box = self.install_doubles(after_inputs={'src/main.rs': 'after-equal'}, read_prepared=read_prepared_ok())
        candidate, state = version.prepare(root, binary, installed=observed)
        self.assertEqual(box.builds, [PREFIX + '382'])
        self.assertEqual(state['version'], PREFIX + '382')
        self.assertTrue(artifact.is_file())
        self.assertEqual(artifact.read_bytes(), CACHED_IMAGE)
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)

    def test_higher_explicit_override_builds_chosen_label(self):
        """Exercise higher explicit override builds chosen label with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = base_record(version=PREFIX + '10')
        write_record(root, record)
        override = PREFIX + '500'
        os.environ['SWARM_VERSION'] = override
        observed = observation()
        box = self.install_doubles(after_inputs={'src/main.rs': 'after-override'}, read_prepared=read_prepared_ok())
        candidate, state = version.prepare(root, binary, installed=observed)
        self.assertEqual(box.release_calls, [override])
        self.assertEqual(box.builds, [override])
        self.assertEqual(state['version'], override)
        self.assertEqual(observed, observation())
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)

    def test_override_lower_than_observed_refused(self):
        """Exercise override lower than observed refused with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = base_record()
            write_record(root, record)
            before = self.state_bytes(root)
            os.environ['SWARM_VERSION'] = PREFIX + '380'
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_unreachable())
            with self.assertRaisesRegex(ValueError, 'older than the observed installed version'):
                version.prepare(root, binary, installed=observed)
            self.assertEqual(box.release_calls, [PREFIX + '380'])
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_override_unparseable_label_refused(self):
        """Exercise override unparseable label refused with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = base_record()
            write_record(root, record)
            before = self.state_bytes(root)
            os.environ['SWARM_VERSION'] = 'v0.3.9-dev-latest'
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_unreachable())
            with self.assertRaisesRegex(ValueError, 'canonical comparable dev label'):
                version.prepare(root, binary, installed=observed)
            self.assertEqual(box.release_calls, ['v0.3.9-dev-latest'])
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_override_equal_label_without_same_image_cache_refused(self):
        """Exercise override equal label without same image cache refused with actual prepare and owned inert fixtures; no native qualification."""
        for scenario in ('absent-record', 'different-image'):
            with self.subTest(scenario=scenario):
                self.valid_control()
                try:
                    root = self.make_root()
                    write_manifest(root)
                    binary = write_binary(root)
                    record = None
                    if scenario == 'different-image':
                        record = base_record(version=OBSERVED_VERSION)
                        write_cached_artifact(root, record)
                        write_record(root, record)
                    before = self.state_bytes(root)
                    os.environ['SWARM_VERSION'] = OBSERVED_VERSION
                    observed = observation()
                    box = self.install_doubles(read_prepared=None if record is None else read_prepared_ok())
                    with self.assertRaisesRegex(ValueError, 'only servable'):
                        version.prepare(root, binary, installed=observed)
                    self.assertEqual(box.release_calls, [OBSERVED_VERSION])
                    self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
                    self.assertEqual(self.state_bytes(root), before)
                    self.assertEqual(observed, observation())
                    if record is not None:
                        self.assertTrue(Path(record['artifact']).is_file())
                        self.assertEqual(Path(record['artifact']).read_bytes(), CACHED_IMAGE)
                finally:
                    self.valid_control()

    def test_malformed_observation_refused_before_writes_or_build(self):
        """Exercise malformed observation refused before writes or build with actual prepare and owned inert fixtures; no native qualification."""
        for scenario, bad in (('leading-zero', {'version': 'v0.3.9-dev.0381', 'binary_sha256': OBSERVED_HASH}), ('not-a-dict', 'v0.3.9-dev.381')):
            with self.subTest(scenario=scenario):
                self.valid_control()
                try:
                    root = self.make_root()
                    write_manifest(root)
                    binary = write_binary(root)
                    observed = bad
                    box = self.install_doubles()
                    regex = 'canonical local dev label' if scenario == 'leading-zero' else 'must be a dict'
                    with self.assertRaisesRegex(ValueError, regex):
                        version.prepare(root, binary, installed=observed)
                    self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
                    self.assertIsNone(self.state_bytes(root))
                    if isinstance(observed, dict):
                        self.assertEqual(observed, {'version': 'v0.3.9-dev.0381', 'binary_sha256': OBSERVED_HASH})
                finally:
                    self.valid_control()

    def test_present_legacy_record_refused_when_observed(self):
        """Exercise present legacy record refused when observed with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = {'version': PREFIX + '5', 'source_digest': 'legacy-digest'}
            write_record(root, record)
            before = self.state_bytes(root)
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_unreachable())
            with self.assertRaisesRegex(ValueError, 'legacy record'):
                version.prepare(root, binary, installed=observed)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_unqualified_proof_refused_with_original_cause(self):
        """Exercise unqualified proof refused with original cause with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = base_record(version=PREFIX + '500')
            write_record(root, record)
            before = self.state_bytes(root)
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_raising(ValueError('prepared-build-unqualified')))
            with self.assertRaisesRegex(ValueError, 'prepared-build-unqualified'):
                version.prepare(root, binary, installed=observed)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_missing_proof_refused_with_original_cause(self):
        """Exercise missing proof refused with original cause with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = base_record(version=PREFIX + '500')
            write_record(root, record)
            before = self.state_bytes(root)
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_raising(FileNotFoundError('prepared-build-missing-proof')))
            with self.assertRaisesRegex(FileNotFoundError, 'prepared-build-missing-proof'):
                version.prepare(root, binary, installed=observed)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_changed_history_refused(self):
        """Exercise changed history refused with actual prepare and owned inert fixtures; no native qualification."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            record = base_record(version=PREFIX + '500')
            write_record(root, record)
            before = self.state_bytes(root)
            observed = observation()
            box = self.install_doubles(read_prepared=read_prepared_drifted())
            with self.assertRaisesRegex(ValueError, 'changed history'):
                version.prepare(root, binary, installed=observed)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
            self.assertEqual(observed, observation())
        finally:
            self.valid_control()

    def test_legacy_two_argument_cache_reuse_unchanged(self):
        """Exercise legacy two argument cache reuse unchanged with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = base_record(version=PREFIX + '5')
        artifact = write_cached_artifact(root, record)
        write_record(root, record)
        box = self.install_doubles(read_prepared=read_prepared_ok())
        result_artifact, result_state = version.prepare(root, binary)
        self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
        self.assertEqual(result_artifact, artifact)
        self.assertEqual(result_state, record)

    def test_legacy_two_argument_numbering_unchanged(self):
        """Exercise legacy two argument numbering unchanged with actual prepare and owned inert fixtures; no native qualification."""
        root = self.make_root()
        write_manifest(root)
        binary = write_binary(root)
        record = {'version': PREFIX + '5', 'source_digest': 'legacy-digest'}
        write_record(root, record)
        box = self.install_doubles(after_inputs={'src/main.rs': 'after-legacy'})
        candidate, state = version.prepare(root, binary)
        self.assertEqual(box.builds, [PREFIX + '6'])
        self.assertEqual(state['version'], PREFIX + '6')
        self.assertEqual(candidate.read_bytes(), FROZEN_IMAGE)
        self.assertEqual(binary.read_bytes(), OBSERVED_IMAGE)

    def valid_control(self, *, legacy=False):
        """Run an isolated real-prepare witness and require its cleanup to succeed.

    The independent nested case restores ambient doubles/environment even when
    the parent has already configured a fault. This adds no native execution.
    """
        method = 'test_legacy_two_argument_cache_reuse_unchanged' if legacy else 'test_control_no_prior_observed_allocates_above_observation'
        result = unittest.TestResult()
        ObservedProducerAdmission(method).run(result)
        self.assertTrue(result.wasSuccessful(), repr(result.errors + result.failures))

    def tearDown(self):
        """Require every native/process/network boundary to remain unused."""
        for boundary in self.forbidden_boundaries:
            boundary.assert_not_called()

    def test_legacy_error_precedence_stays_before_override_validation(self):
        """A malformed legacy record keeps its original error precedence."""
        self.valid_control(legacy=True)
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            write_record(root, [1])
            before = self.state_bytes(root)
            box = self.install_doubles()
            box.release = lambda label: False
            os.environ['SWARM_VERSION'] = 'invalid-by-builder'
            with self.assertRaises(AttributeError):
                version.prepare(root, binary)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
        finally:
            self.valid_control(legacy=True)

    def test_present_null_or_invalid_source_digest_is_not_absent_history(self):
        """Present null/invalid record fields refuse, unlike a genuinely absent file."""
        variants = [None, False, 0, {}, [], [1]]
        for digest in [None, '', 3]:
            value = base_record(version=PREFIX + '500')
            value['source_digest'] = digest
            variants.append(value)
        value = base_record(version=PREFIX + '500')
        del value['source_digest']
        variants.append(value)
        for value in variants:
            with self.subTest(value_type=type(value).__name__):
                self.valid_control()
                try:
                    root = self.make_root()
                    write_manifest(root)
                    binary = write_binary(root)
                    if isinstance(value, dict) and value.get('schema_version') == 2:
                        write_cached_artifact(root, value)
                    write_record(root, value)
                    before = self.state_bytes(root)
                    box = self.install_doubles(read_prepared=read_prepared_ok())
                    with self.assertRaises(ValueError):
                        version.prepare(root, binary, installed=observation())
                    self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
                    self.assertEqual(self.state_bytes(root), before)
                finally:
                    self.valid_control()

    def test_observed_history_read_error_is_not_hidden_by_exists(self):
        """A direct record read denial propagates, even if a metadata probe says false."""
        self.valid_control()
        try:
            root = self.make_root()
            write_manifest(root)
            binary = write_binary(root)
            write_record(root, base_record())
            before = self.state_bytes(root)
            state_path = root / 'deploy/local-build.json'
            box = self.install_doubles(read_prepared=read_prepared_unreachable())
            primary = PermissionError(13, 'inert record read denied')
            exists = Path.exists
            read_text = Path.read_text

            def observed_exists(path):
                """Double only this owned metadata lookup; retain all other lookups."""
                return False if path == state_path else exists(path)

            def observed_read(path, *args, **kwargs):
                """Inject this exact record error; forward all other owned reads."""
                if path == state_path:
                    raise primary
                return read_text(path, *args, **kwargs)
            with patch.object(Path, 'exists', observed_exists), patch.object(Path, 'read_text', observed_read):
                with self.assertRaises(PermissionError) as caught:
                    version.prepare(root, binary, installed=observation())
            self.assertIs(caught.exception, primary)
            self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
            self.assertEqual(self.state_bytes(root), before)
        finally:
            self.valid_control()

    def test_late_cache_proof_failure_or_record_drift_refuses(self):
        """A second proof read cannot become a fallback rebuild or unrelated state."""
        for drift in [False, True]:
            with self.subTest(drift=drift):
                self.valid_control()
                try:
                    root = self.make_root()
                    write_manifest(root)
                    binary = write_binary(root)
                    record = base_record(version=PREFIX + '500')
                    artifact = write_cached_artifact(root, record)
                    write_record(root, record)
                    before = self.state_bytes(root)
                    calls = []
                    primary = FileNotFoundError('inert late proof disappeared')

                    def read_twice(source, expected_sha):
                        """Return a valid first fixture receipt, then inject the late fault."""
                        calls.append(expected_sha)
                        state = json.loads((source / 'deploy/local-build.json').read_bytes())
                        if len(calls) > 1:
                            if not drift:
                                raise primary
                            state['candidate_digest'] = 'inert changed history'
                        return (state, None, b'raw-image', {}, {}, {})
                    box = self.install_doubles(read_prepared=read_twice)
                    with self.assertRaises(ValueError if drift else FileNotFoundError) as caught:
                        version.prepare(root, binary, installed=observation())
                    if not drift:
                        self.assertIs(caught.exception, primary)
                    self.assertEqual(len(calls), 2)
                    self.assert_no_publication(root, binary, box, OBSERVED_IMAGE)
                    self.assertEqual(self.state_bytes(root), before)
                    self.assertEqual(artifact.read_bytes(), CACHED_IMAGE)
                finally:
                    self.valid_control()
if __name__ == '__main__':
    unittest.main()
