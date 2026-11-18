"""Pure selector contract tests, separate from producer and lifecycle qualification.

The real module function is exercised. Imports read source files; the selector
itself performs no IO, process call or argument mutation. Local valid controls
precede injected errors; recovery does not depend on method ordering. Observed
values are ordinary synthetic data, never proof of an owned live image.
"""
import copy
import inspect
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from swarm_version import select_version


H_A = 'a' * 64
H_B = 'b' * 64
H_C = 'c' * 64


def _obs(version, sha):
    """Construct a dedicated synthetic observation without persistence or authority."""
    return {'version': version, 'binary_sha256': sha}


class SelectVersionArticleV6P7Test(unittest.TestCase):

    def control(self, *, observed=True):
        """Check valid absent/prior inputs using the same scalar fixture context."""
        self.assertEqual(select_version('v0.1.0', H_A, None), 'v0.1.1-dev.1')
        if observed:
            observation = _obs('v0.1.1-dev.381', H_B)
            self.assertEqual(select_version('v0.1.0', H_A, None,
                                            installed=observation), 'v0.1.1-dev.382')
            previous = {'version': 'v0.1.1-dev.381', 'source_digest': H_A,
                        'binary_sha256': H_B}
            self.assertEqual(select_version('v0.1.0', H_A, previous,
                                            installed=observation), 'v0.1.1-dev.381')

    def tearDown(self):
        """Verify recovery; pre-feature baselines can recover only through old API."""
        self.control(observed='installed' in inspect.signature(select_version).parameters)


    def test_005_interface_is_keyword_only_with_none_default(self):
        param = inspect.signature(select_version).parameters['installed']
        self.assertEqual(param.kind, inspect.Parameter.KEYWORD_ONLY)
        self.assertIsNone(param.default)
        self.assertEqual(
            select_version('v0.1.0', H_A, {}, installed=None), 'v0.1.1-dev.1'
        )
        self.control(observed=False)
        with self.assertRaises(TypeError):
            select_version('v0.1.0', H_A, {}, 'v0.1.1-dev.1')

    # -- valid controls: legacy three-argument path (no observation) --

    def test_010_legacy_start_and_digest_rules(self):
        self.assertEqual(select_version('v0.1.0', H_A, None), 'v0.1.1-dev.1')
        record = {'version': 'v0.1.1-dev.7', 'source_digest': H_A}
        self.assertEqual(select_version('v0.1.0', H_A, record), 'v0.1.1-dev.7')
        self.assertEqual(select_version('v0.1.0', H_C, record), 'v0.1.1-dev.8')

    def test_015_legacy_falsy_or_unmatched_previous_falls_to_dev1(self):
        self.assertEqual(select_version('v0.1.0', H_A, ''), 'v0.1.1-dev.1')
        self.assertEqual(
            select_version('v0.1.0', H_A, {'unrelated': 1}), 'v0.1.1-dev.1'
        )
        self.assertEqual(
            select_version('v0.1.0', H_A, {'version': 'v0.1.0-dev.9', 'source_digest': H_A}),
            'v0.1.1-dev.1',
        )
        self.assertEqual(
            select_version('v0.1.0', H_A, {'version': 'v0.2.0-dev.1', 'source_digest': H_A}),
            'v0.1.1-dev.1',
        )

    def test_020_legacy_pins_verbatim_head_behavior(self):
        # Non-canonical counters are retained and advanced exactly as at HEAD.
        record = {'version': 'v0.1.1-dev.007', 'source_digest': H_A}
        self.assertEqual(select_version('v0.1.0', H_A, record), 'v0.1.1-dev.007')
        self.assertEqual(select_version('v0.1.0', H_C, record), 'v0.1.1-dev.8')
        # A non-string prior version still crashes as at HEAD; tightening the
        # legacy branch is out of scope here (producer-side record validation
        # belongs to the wiring packet).
        self.control(observed=False)
        with self.assertRaises(AttributeError):
            select_version('v0.1.0', H_A, {'version': 123, 'source_digest': H_A})
        # A None published label still fails as at HEAD (TypeError from re).
        self.control(observed=False)
        with self.assertRaises(TypeError):
            select_version(None, H_A, None)

    def test_025_legacy_published_suffix_and_invalid_base(self):
        self.assertEqual(select_version('v0.1.0-rc.1', H_A, None), 'v0.1.1-dev.1')
        self.assertEqual(select_version('v0.1.0+build.5', H_A, None), 'v0.1.1-dev.1')
        for bad in ('0.1.0', 'v01.1.0', 'v0.1', 'not-a-version', ''):
            self.control(observed=False)
            with self.assertRaises(ValueError):
                select_version(bad, H_A, None)

    # -- valid controls: observed-image branch --

    def test_030_observed_no_prior_allocates_above_observed_counter(self):
        # Packet example: a clean source has no local-build.json while the
        # actually running owned image is dev.381, so the next label is 382.
        observation = _obs('v0.1.1-dev.381', H_B)
        self.assertEqual(
            select_version('v0.1.0', H_A, None, installed=observation),
            'v0.1.1-dev.382',
        )

    def test_035_observed_prior_below_floor_cannot_stay_cached(self):
        # Packet example: the on-disk record says dev.380 while dev.381 runs;
        # the stale number is never handed out again, digest unchanged or not.
        record = {'version': 'v0.1.1-dev.380', 'source_digest': H_A, 'binary_sha256': H_C}
        observation = _obs('v0.1.1-dev.381', H_B)
        self.assertEqual(
            select_version('v0.1.0', H_A, record, installed=observation),
            'v0.1.1-dev.382',
        )
        self.assertEqual(
            select_version('v0.1.0', H_C, record, installed=observation),
            'v0.1.1-dev.382',
        )

    def test_040_observed_prior_above_observed_same_digest_is_retained(self):
        record = {'version': 'v0.1.1-dev.382', 'source_digest': H_A}
        self.assertEqual(
            select_version('v0.1.0', H_A, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.382',
        )
        # Above the observed counter no image hash is needed for retention.

    def test_045_observed_prior_above_observed_changed_digest_advances(self):
        record = {'version': 'v0.1.1-dev.382', 'source_digest': H_A}
        self.assertEqual(
            select_version('v0.1.0', H_C, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.383',
        )

    def test_050_observed_equal_label_same_hash_same_digest_retained(self):
        record = {
            'version': 'v0.1.1-dev.381',
            'source_digest': H_A,
            'binary_sha256': H_B,
        }
        self.assertEqual(
            select_version('v0.1.0', H_A, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.381',
        )

    def test_055_observed_equal_label_different_hash_allocates_above_floor(self):
        record = {
            'version': 'v0.1.1-dev.381',
            'source_digest': H_A,
            'binary_sha256': H_C,
        }
        self.assertEqual(
            select_version('v0.1.0', H_A, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.382',
        )

    def test_056_observed_equal_label_changed_digest_advances_even_with_equal_hash(self):
        # Contract precedence: at the observed label a matching hash is
        # necessary for retention, while a changed source digest always
        # advances the allocation as before, subject to the floor.
        record = {
            'version': 'v0.1.1-dev.381',
            'source_digest': H_A,
            'binary_sha256': H_B,
        }
        self.assertEqual(
            select_version('v0.1.0', H_C, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.382',
        )

    def test_060_observed_lower_installed_family_needs_no_counter_floor(self):
        observation = _obs('v0.1.0-dev.9', H_B)
        self.assertEqual(
            select_version('v0.1.0', H_A, None, installed=observation),
            'v0.1.1-dev.1',
        )
        target_record = {'version': 'v0.1.1-dev.3', 'source_digest': H_A}
        self.assertEqual(
            select_version('v0.1.0', H_A, target_record, installed=observation),
            'v0.1.1-dev.3',
        )
        self.assertEqual(
            select_version('v0.1.0', H_C, target_record, installed=observation),
            'v0.1.1-dev.4',
        )
        # An unchanged prior in a higher family is already ordered, even at zero.
        self.assertEqual(
            select_version('v0.1.0', H_A,
                           {'version': 'v0.1.1-dev.0', 'source_digest': H_A},
                           installed=observation), 'v0.1.1-dev.0')
        lower_record = {'version': 'v0.1.0-dev.7', 'source_digest': H_A}
        self.assertEqual(
            select_version('v0.1.0', H_A, lower_record, installed=observation),
            'v0.1.1-dev.1',
        )

    def test_065_observed_lower_prior_family_leaves_numbering_to_floor(self):
        record = {'version': 'v0.1.0-dev.7', 'source_digest': H_A, 'binary_sha256': H_C}
        self.assertEqual(
            select_version('v0.1.0', H_A, record, installed=_obs('v0.1.1-dev.381', H_B)),
            'v0.1.1-dev.382',
        )

    # -- fault cases --

    def test_070_fault_installed_family_above_target_refused(self):
        for version in ('v0.2.0-dev.1', 'v0.1.2-dev.1', 'v1.0.0-dev.1'):
            self.control(observed=True)
            with self.assertRaises(ValueError):
                select_version('v0.1.0', H_A, None, installed=_obs(version, H_B))

    def test_075_fault_prior_family_above_target_refused(self):
        observation = _obs('v0.1.1-dev.381', H_B)
        for version in ('v0.2.0-dev.1', 'v0.1.2-dev.1'):
            record = {'version': version, 'source_digest': H_A}
            self.control(observed=True)
            with self.assertRaises(ValueError):
                select_version('v0.1.0', H_A, record, installed=observation)

    def test_080_fault_malformed_installed_never_treated_as_absent(self):
        bad_observations = [
            False,
            0,
            'v0.1.1-dev.381',
            {},
            {'version': 'v0.1.1-dev.381'},
            {'version': 'v0.1.1-dev.381', 'binary_sha256': H_B, 'extra': 1},
            _obs('v0.1.1-dev.0381', H_B),
            _obs('v0.1.1', H_B),
            _obs('v0.1.1-dev.381+x', H_B),
            _obs('v0.1.1-dev.381 ', H_B),
            _obs(381, H_B),
            _obs('v0.1.1-dev.381', 'A' * 64),
            _obs('v0.1.1-dev.381', 'a' * 63),
            _obs('v0.1.1-dev.381', 'g' * 64),
            _obs('v0.1.1-dev.381', 123),
            _obs('v0.1.1-dev.381', None),
        ]
        for bad in bad_observations:
            self.control(observed=True)
            with self.assertRaises(ValueError):
                select_version('v0.1.0', H_A, None, installed=bad)

    def test_085_fault_malformed_prior_refused_only_in_observed_branch(self):
        observation = _obs('v0.1.1-dev.381', H_B)
        bad_priors = [
            {},
            {'source_digest': H_A},
            False,
            0,
            {'version': 'v0.1.1', 'source_digest': H_A},
            {'version': 'v0.1.1-dev.007', 'source_digest': H_A},
            {'version': 'v0.1.1-dev.5+x', 'source_digest': H_A},
            {'version': 42, 'source_digest': H_A},
            {'version': None, 'source_digest': H_A},
            ['v0.1.1-dev.5'],
            '',
        ]
        for bad in bad_priors:
            self.control(observed=True)
            with self.assertRaises(ValueError):
                select_version('v0.1.0', H_A, bad, installed=observation)
        # A record claiming the installed version without a comparable string
        # hash is incoherent and refused on the same-digest path.
        for sha in (None, 123, '', 'b' * 63, 'B' * 64, 'g' * 64):
            record = {
                'version': 'v0.1.1-dev.381',
                'source_digest': H_A,
                'binary_sha256': sha,
            }
            self.control(observed=True)
            with self.assertRaises(ValueError):
                select_version('v0.1.0', H_A, record, installed=observation)
        # The identical non-canonical prior is still retained on the legacy
        # path (see test_020): the refusal is observed-branch only.

    def test_090_inputs_are_never_mutated(self):
        record = {
            'version': 'v0.1.1-dev.381',
            'source_digest': H_A,
            'binary_sha256': H_B,
            'effective_inputs': {'profile': {'rustc': '1.0'}},
        }
        observation = _obs('v0.1.1-dev.381', H_B)
        record_snapshot = copy.deepcopy(record)
        observation_snapshot = copy.deepcopy(observation)
        select_version('v0.1.0', H_A, record, installed=observation)
        select_version('v0.1.0', H_C, record, installed=observation)
        self.control(observed=True)
        with self.assertRaises(ValueError):
            select_version('v0.1.0', H_A, record, installed=_obs('v0.2.0-dev.1', H_B))
        self.control(observed=True)
        with self.assertRaises(ValueError):
            select_version('v0.1.0', H_A, record, installed=_obs('v0.1.1-dev.0381', H_B))
        select_version('v0.1.0', H_A, record)
        self.assertEqual(record, record_snapshot)
        self.assertEqual(observation, observation_snapshot)

    def test_095_recovery_after_faults_is_stateless(self):
        self.control(observed=True)
        with self.assertRaises(ValueError):
            select_version('v0.1.0', H_A, None, installed=_obs('v0.1.1-dev.0381', H_B))
        observation = _obs('v0.1.1-dev.381', H_B)
        for _ in range(3):
            self.assertEqual(
                select_version('v0.1.0', H_A, None, installed=observation),
                'v0.1.1-dev.382',
            )
        self.assertEqual(select_version('v0.1.0', H_A, None), 'v0.1.1-dev.1')
        self.control(observed=True)
        with self.assertRaises(ValueError):
            select_version(
                'v0.1.0',
                H_A,
                {'version': 'v0.1.1-dev.007', 'source_digest': H_A},
                installed=observation,
            )
        legacy_record = {'version': 'v0.1.1-dev.007', 'source_digest': H_A}
        self.assertEqual(select_version('v0.1.0', H_A, legacy_record), 'v0.1.1-dev.007')


if __name__ == '__main__':
    unittest.main(verbosity=2)