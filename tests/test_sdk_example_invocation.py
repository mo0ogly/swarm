"""Refusal guards for SDK build records; no Cargo, Root or provider execution."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools/verification'))
import verify_sdk_example as example


class InvocationTests(unittest.TestCase):
    """A valid invocation reaches source validation; faults stop before it."""

    def test_invocation_refusal_and_same_input_recovery(self):
        """Assert args, wrapper and profile faults against a valid-shape witness.

        The witness deliberately lacks a source snapshot and must reach that
        exact boundary, not PASS. This checks record validation only; actual
        SDK feature/source qualification remains a separate retained build.
        """
        with tempfile.TemporaryDirectory(prefix='sdk-invocation-') as directory:
            root = Path(directory)
            keys = ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                    'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET', 'RUSTUP_TOOLCHAIN', 'CARGO_HOME']
            valid = dict(argv=['cargo', 'build', '--manifest-path', str(root / 'rust/Cargo.toml'),
                              '--offline', '--locked', '-p', 'swarm-cli', '--example', 'worker',
                              '--message-format=json'], cwd=str(root),
                         ambient_override_presence={key: False for key in keys},
                         profile_environment=dict(CARGO_PROFILE_DEV_DEBUG='0',
                                                  CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0'))
            record = root / 'invocation.json'

            def attempt(data, name, expected=None):
                """Use fresh output; preserve each refused qualification locally."""
                record.write_text(json.dumps(data))
                if expected:
                    with self.assertRaisesRegex(ValueError, expected):
                        example.qualify(root, root / 'cargo', root, 'worker', root / name, record)
                else:
                    with self.assertRaises(FileNotFoundError) as failure:
                        example.qualify(root, root / 'cargo', root, 'worker', root / name, record)
                    self.assertEqual(Path(failure.exception.filename), root / 'sdk-source-snapshot.json')

            attempt(valid, 'witness')
            invalid = copy.deepcopy(valid)
            invalid['argv'].append('--all-features')
            attempt(invalid, 'arguments', 'example-invocation-arguments')
            invalid = copy.deepcopy(valid)
            invalid['ambient_override_presence']['RUSTC_WRAPPER'] = True
            attempt(invalid, 'wrapper', 'example-ambient-compiler-override')
            invalid = copy.deepcopy(valid)
            invalid['profile_environment']['CARGO_INCREMENTAL'] = '1'
            attempt(invalid, 'profile', 'example-profile-environment')
            attempt(valid, 'recovery')


if __name__ == '__main__':
    unittest.main()
