#!/usr/bin/env python3
"""Verify export correspondence through fixtures and public CLIs, without builds.

Explicit existing binary/hash and fresh transcript use the same prerequisites as
paea_public_contract_test.py. Original document/export/CLI bytes are retained.
No live Root/provider/server is used. Tests include intended non-authority limits.
No global watchdog, implicit retries or E3 claim.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/paea'))
import verify_canonical as verifier


def encoded(value):
    """Serialize test data only; expected canonical bytes come from frozen vectors."""
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()


class CanonicalVerifier(unittest.TestCase):
    """Correspondence and refusal tests; successful matching never authenticates a pair."""

    @classmethod
    def setUpClass(cls):
        """Load versioned schema and complete frozen fixtures; main binds binary bytes."""
        cls.schema_raw = (ROOT / 'schemas/paea/contract-v1.schema.json').read_bytes()
        cls.vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())['vectors']
        if len(cls.vectors) != 14:
            raise ValueError('frozen-vector-set-changed')

    def setUp(self):
        """Allocate one isolated root per method and preserve no live state."""
        self.temporary = tempfile.TemporaryDirectory(prefix='paea-verifier-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def pair(self, index):
        """Build document and complete envelope from retained independent expected values."""
        vector = self.vectors[index]
        document = copy.deepcopy({key: vector[key] for key in ('proposal', 'context')})
        result = {key: vector[key] for key in ('canonical', 'action_digest', 'intent_id')}
        result.update(schema_version=1, canonicalization='paea-json-v1',
                      authority='not-established', scope='offline-data-only')
        return document, result

    def checked(self, document_raw, result_raw):
        """Compare pure verifier result including consumed-byte identities and scope."""
        report = verifier.verify(document_raw, result_raw, self.schema_raw)
        self.assertEqual(report['status'], 'canonical-correspondence-verified')
        self.assertEqual(report['validation'], 'schema-and-canonical-correspondence-only')
        self.assertEqual(report['authority'], 'not-established')
        self.assertEqual(report['effect'], 'not-established')
        self.assertEqual(report['schema_id'], 'urn:swarm:paea:contract:v1')
        self.assertEqual(report['schema_sha256'], hashlib.sha256(self.schema_raw).hexdigest())
        self.assertEqual(report['document_sha256'], hashlib.sha256(document_raw).hexdigest())
        self.assertEqual(report['result_sha256'], hashlib.sha256(result_raw).hexdigest())
        return report

    def test_existing_binary_exports_match_frozen_oracle_and_verifier(self):
        """All fourteen Rust CLI exports equal frozen output before Python verification."""
        for index in range(14):
            with self.subTest(vector=self.vectors[index]['name']):
                document, expected = self.pair(index)
                raw = encoded(document)
                argv = [str(self.binary), 'protection', 'canonicalize', '--input', '-', '--json']
                output = subprocess.run(argv, input=raw, cwd=self.root, capture_output=True)
                self.transcript.append({'kind': 'rust-export', 'argv': argv, 'input_hex': raw.hex(),
                                        'exit': output.returncode, 'stdout_hex': output.stdout.hex(),
                                        'stderr_hex': output.stderr.hex()})
                self.assertEqual(output.returncode, 0, output.stderr)
                self.assertEqual(output.stderr, b'')
                self.assertEqual(json.loads(output.stdout), expected)
                report = self.checked(raw, output.stdout)
                self.assertEqual(report['action_digest'], expected['action_digest'])
                self.assertEqual(list(self.root.iterdir()), [])

    def test_tampered_exports_and_documents_refuse_then_recover(self):
        """Mutations cannot survive exact correspondence; each has a valid control/recovery."""
        for index in (0, 8):
            document, result = self.pair(index)
            raw, exported = encoded(document), encoded(result)
            bad_results = []
            for key, value in (('canonical', result['canonical'] + ' '),
                               ('action_digest', 'f'*64), ('intent_id', 'f'*64),
                               ('authority', 'approved'), ('schema_version', True),
                               ('PRIVATE-CANARY-NOT-A-PERMISSION', 'secret')):
                bad = copy.deepcopy(result)
                bad[key] = value
                bad_results.append(encoded(bad))
            bad = copy.deepcopy(result)
            del bad['scope']
            bad_results.append(encoded(bad))
            for bad in bad_results:
                self.checked(raw, exported)
                with self.assertRaisesRegex(verifier.Refusal, '^canonical-correspondence-mismatch$'):
                    verifier.verify(raw, bad, self.schema_raw)
                self.checked(raw, exported)
            changed = copy.deepcopy(document)
            changed['context']['principal_id'] = 'substituted-principal'
            self.checked(raw, exported)
            with self.assertRaises(verifier.Refusal):
                verifier.verify(encoded(changed), exported, self.schema_raw)
            self.checked(raw, exported)

    def test_representation_equivalence_and_non_authority_limit(self):
        """New request IDs/label order preserve content; a matching forged pair remains data."""
        for index in (0, 8):
            document, result = self.pair(index)
            original = self.checked(encoded(document), encoded(result))
            document['proposal']['request_id'] = 'new-request'
            document['context']['sources'][0]['confidentiality_labels'].reverse()
            report = self.checked(encoded(document), encoded(result))
            self.assertEqual(report['action_digest'], original['action_digest'])
            self.assertNotEqual(report['document_sha256'], original['document_sha256'])
        # Explicitly demonstrate why schema+correspondence cannot imply admission:
        # this amount passes shape but exceeds Rust's semantic i64 representation.
        document, result = self.pair(0)
        document['proposal']['action']['amount_minor'] = '9223372036854775808'
        expected = verifier.vector('forged-pair', document['proposal']['action'],
                                   document['proposal']['request_id'], **document['context'])
        result.update({k: expected[k] for k in ('canonical', 'action_digest', 'intent_id')})
        self.checked(encoded(document), encoded(result))

    def test_raw_ambiguities_refuse_before_correspondence(self):
        """Duplicate decoded names, floats, invalid UTF-8/surrogates and trailing data refuse."""
        for index in (0, 8):
            document, result = self.pair(index)
            raw, exported = encoded(document), encoded(result)
            cases = [raw.replace(b'"schema_version":1', b'"schema_version":1e0', 1),
                     raw + b' {}', raw + b'\xff',
                     raw.replace(b'"request-1"', b'"\\ud800"', 1),
                     raw.replace(b'"proposal":', b'"proposal":{},"propo\\u0073al":', 1)]
            for bad in cases:
                self.assertNotEqual(bad, raw)
                self.checked(raw, exported)
                with self.assertRaises((ValueError, UnicodeError, RecursionError)):
                    verifier.verify(bad, exported, self.schema_raw)
                self.checked(raw, exported)
            self.checked(raw, exported)
            with self.assertRaises(verifier.Refusal):
                verifier.verify(raw, exported[:-1] + b',"scope":"offline-data-only"}', self.schema_raw)
            self.checked(raw, exported)

    def call(self, document_raw, result_raw, document_path=None, stdout=None):
        """Invoke the actual verifier CLI in a temp root, retaining all input/output bytes.

        document_path can select only an isolated missing/directory/FIFO fixture.
        /dev/full is an explicit output-failure fixture; no original is overwritten.
        """
        document_file = self.root / 'document.json'
        result_file = self.root / 'export.json'
        document_file.write_bytes(document_raw)
        result_file.write_bytes(result_raw)
        argv = [sys.executable, str(ROOT / 'tools/paea/verify_canonical.py'),
                '--document', str(document_file if document_path is None else document_path),
                '--result', str(result_file)]
        output = subprocess.run(argv, cwd=self.root,
                                stdout=subprocess.PIPE if stdout is None else stdout,
                                stderr=subprocess.PIPE)
        self.assertEqual(document_file.read_bytes(), document_raw)
        self.assertEqual(result_file.read_bytes(), result_raw)
        self.transcript.append({'kind': 'python-verifier', 'argv': argv,
                                'document_hex': document_raw.hex(), 'result_hex': result_raw.hex(),
                                'exit': output.returncode,
                                'stdout_hex': None if output.stdout is None else output.stdout.hex(),
                                'stderr_hex': output.stderr.hex(),
                                'writer': 'capture' if stdout is None else '/dev/full'})
        return output

    def test_public_cli_safe_refusal_io_and_recovery(self):
        """Actual CLI refuses private malformed values and IO fixtures, then recovers."""
        for index in (0, 8):
            document, result = self.pair(index)
            raw, exported = encoded(document), encoded(result)
            fifo = self.root / 'fifo'
            os.mkfifo(fifo)
            for name, doc, path, code in (
                ('malformed', b'{"PRIVATE-CANARY-NOT-A-PERMISSION":', None, 'canonical-json-invalid'),
                ('missing', raw, self.root / 'missing', 'canonical-io-unavailable'),
                ('directory', raw, self.root, 'canonical-regular-file-required'),
                ('fifo', raw, fifo, 'canonical-regular-file-required'),
            ):
                with self.subTest(domain=index, case=name):
                    control = self.call(raw, exported)
                    self.assertEqual(control.returncode, 0, control.stderr)
                    self.assertEqual(json.loads(control.stdout), self.checked(raw, exported))
                    refused = self.call(doc, exported, document_path=path)
                    self.assertEqual(refused.returncode, 2)
                    self.assertEqual(refused.stdout, b'')
                    self.assertEqual(json.loads(refused.stderr), {'status': 'refused', 'code': code})
                    self.assertNotIn(b'PRIVATE-CANARY-NOT-A-PERMISSION', refused.stderr)
                    recovery = self.call(raw, exported)
                    self.assertEqual(recovery.returncode, 0, recovery.stderr)
                    self.assertEqual(json.loads(recovery.stdout), self.checked(raw, exported))
            fifo.unlink()

    def test_local_schema_corruption_refuses_without_default_context(self):
        """Broken/wrong-profile schemas get schema diagnostics; weakening cannot fill defaults.

        These are local byte fixtures, never writes to the repository schema. A
        schema digest records identity, without authenticating the candidate.
        """
        document, result = self.pair(0)
        raw, exported = encoded(document), encoded(result)
        for schema in (b'true', b'{}', b'{', b'{"$id":1,"$id":2}',
                       b'{"$id":"urn:swarm:paea:contract:v1","type":7}',
                       encoded({'$id': verifier.SCHEMA_ID, '$ref': 'https://unused.invalid/schema'}),
                       encoded({'$id': verifier.SCHEMA_ID, '$defs': {'x': {'$id': 'https://unused.invalid'}}})):
            self.checked(raw, exported)
            with self.assertRaisesRegex(verifier.Refusal, '^canonical-verifier-schema-invalid$'):
                verifier.verify(raw, exported, schema)
            self.checked(raw, exported)
        weak = encoded({'$id': verifier.SCHEMA_ID, 'type': 'object'})
        for bad in ({'proposal': document['proposal'], 'context': {}}, {}):
            self.checked(raw, exported)
            with self.assertRaisesRegex(verifier.Refusal, '^canonical-document-shape-invalid$'):
                verifier.verify(encoded(bad), exported, weak)
            self.checked(raw, exported)
        original = verifier.verify(raw, exported, self.schema_raw)
        equivalent_schema = json.dumps(json.loads(self.schema_raw), indent=3).encode()
        other = verifier.verify(raw, exported, equivalent_schema)
        self.assertEqual(original['action_digest'], other['action_digest'])
        self.assertNotEqual(original['schema_sha256'], other['schema_sha256'])

    def test_generator_preserves_frozen_vectors_without_cache_writes(self):
        """Run exact generator/publisher in a minimal fixture and assert outputs/no pycache.

        Only these two source files are copied, not a checkout or mission state.
        PYTHONDONTWRITEBYTECODE is removed to exercise the generator's own contract.
        """
        fixture = self.root / 'generator'
        for source in ('tools/paea/make_contract_vectors.py', 'tools/verification/recipe.py'):
            path = fixture / source
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((ROOT / source).read_bytes())
        environment = dict(os.environ)
        environment.pop('PYTHONDONTWRITEBYTECODE', None)
        destination = fixture / 'vectors.json'
        argv = [sys.executable, str(fixture / 'tools/paea/make_contract_vectors.py'), str(destination)]
        output = subprocess.run(argv, cwd=fixture, env=environment, capture_output=True)
        self.transcript.append({'kind': 'python-generator', 'argv': argv,
                                'exit': output.returncode, 'stdout_hex': output.stdout.hex(),
                                'stderr_hex': output.stderr.hex()})
        self.assertEqual(output.returncode, 0, output.stderr)
        self.assertEqual(json.loads(destination.read_bytes())['vectors'], self.vectors)
        self.assertEqual(list(fixture.rglob('*.pyc')), [])

    def test_partial_output_no_progress_and_failure_preserve_errors(self):
        """Partial unbuffered writes consume each byte once; zero/OS failure raises.

        Local writer doubles isolate OS short-write behavior. They supplement the
        real /dev/full public test and do not prove kernel fault handling globally.
        """
        payload = b'complete-report'
        consumed = bytearray()

        def partial(_descriptor, pending):
            """Consume at most three fixture bytes, retaining order without duplication."""
            chunk = bytes(pending[:3])
            consumed.extend(chunk)
            return len(chunk)

        with patch.object(verifier.os, 'write', side_effect=partial):
            verifier.write_stdout(payload)
        self.assertEqual(bytes(consumed), payload)
        consumed.clear()
        with patch.object(verifier.sys, 'stdout', None), patch.object(verifier.os, 'write', side_effect=partial):
            verifier.write_stdout(payload)
        self.assertEqual(bytes(consumed), payload)
        for failure in (0, OSError('PRIVATE-CANARY-NOT-A-PERMISSION')):
            consumed.clear()
            with patch.object(verifier.os, 'write', side_effect=partial):
                verifier.write_stdout(payload)
            self.assertEqual(bytes(consumed), payload)
            with patch.object(verifier.os, 'write', side_effect=[3, failure]):
                with self.assertRaises(OSError):
                    verifier.write_stdout(payload)
            consumed.clear()
            with patch.object(verifier.os, 'write', side_effect=partial):
                verifier.write_stdout(payload)
            self.assertEqual(bytes(consumed), payload)

    def test_public_output_failure_is_nonzero_and_recovers(self):
        """Exercise unwriteable stdout after a valid control, then compare recovery bytes."""
        document, result = self.pair(8)
        raw, exported = encoded(document), encoded(result)
        control = self.call(raw, exported)
        self.assertEqual(control.returncode, 0)
        with Path('/dev/full').open('wb', buffering=0) as full:
            refused = self.call(raw, exported, stdout=full)
        self.assertEqual(refused.returncode, 2)
        self.assertEqual(json.loads(refused.stderr), {'status': 'refused', 'code': 'canonical-io-unavailable'})
        recovery = self.call(raw, exported)
        self.assertEqual(recovery.returncode, 0, recovery.stderr)
        self.assertEqual(recovery.stdout, control.stdout)


def main():
    """Bind explicit existing binary and fresh transcript, execute nine named checks.

    Missing prerequisites refuse before collection. Retain failed observations;
    exclusive atomic publication must not overwrite older evidence. Changed binary
    invalidates success. No compilation, live state or implicit retry is allowed.
    """
    keys = ('SWARM_PAEA_TEST_BINARY', 'SWARM_PAEA_TEST_BINARY_SHA256', 'SWARM_PAEA_TEST_TRANSCRIPT')
    if any(not os.environ.get(key) for key in keys):
        print('PRECONDITION_REFUSED: explicit binary/hash/fresh transcript required')
        return 2
    try:
        binary = Path(os.environ[keys[0]]).resolve(strict=True)
        binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    except OSError as error:
        print(f'PRECONDITION_REFUSED: binary IO errno={error.errno}')
        return 2
    expected = os.environ[keys[1]]
    destination = Path(os.environ[keys[2]])
    if (len(expected) != 64 or any(c not in '0123456789abcdef' for c in expected)
            or binary_digest != expected
            or not os.access(binary, os.X_OK) or destination.exists()
            or not destination.parent.is_dir()):
        print('PRECONDITION_REFUSED: artifact identity or destination')
        return 2
    CanonicalVerifier.binary, CanonicalVerifier.transcript = binary, []
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(CanonicalVerifier))
    stable = hashlib.sha256(binary.read_bytes()).hexdigest() == expected
    spec = importlib.util.spec_from_file_location('paea_verifier_publisher', ROOT / 'tools/verification/recipe.py')
    publisher = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(publisher)
    publisher.publish(destination, {'tests_run': result.testsRun, 'successful': result.wasSuccessful(),
                                   'binary_sha256': expected, 'binary_stable': stable,
                                   'observations': CanonicalVerifier.transcript})
    print('PAEA_VERIFIER_BINARY_STABLE' if stable else 'PAEA_VERIFIER_BINARY_CHANGED')
    return 0 if stable and result.wasSuccessful() and result.testsRun == 9 else 2


if __name__ == '__main__':
    raise SystemExit(main())
