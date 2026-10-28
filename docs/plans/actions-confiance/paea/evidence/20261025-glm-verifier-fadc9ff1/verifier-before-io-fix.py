#!/usr/bin/env python3
"""Check an offline canonical export against its original document, read-only.

Usage: python3 tools/paea/verify_canonical.py --document FILE --result FILE.
Requires jsonschema. Reads regular files and the versioned repository schema;
compares strict JSON, schema shape and exact paea-json-v1 correspondence using
our existing Python oracle. This does not establish semantic admission, identity,
signature, current authority, effect, completeness of evidence or E2/E3 assurance.
No server, binary, network, policy, credential or project datastore is opened.
Files are read into memory; there is no operator policy or global watchdog here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

from jsonschema import Draft202012Validator
from jsonschema.exceptions import SchemaError
from make_contract_vectors import vector

ROOT = Path(__file__).resolve().parents[2]


class Refusal(ValueError):
    """Static safe diagnostic; raw documents, paths and private values stay unprinted."""


def unique_object(pairs):
    """Build each JSON object without silently overwriting decoded duplicate names."""
    value = {}
    for key, item in pairs:
        if key in value:
            raise Refusal('canonical-json-duplicate-key')
        value[key] = item
    return value


def non_integer(_value):
    """Reject floating/exponent and NaN/Infinity representations before comparison."""
    raise Refusal('canonical-json-number-invalid')


def check_unicode(value):
    """Reject unpaired surrogates anywhere; recursion failures refuse at the CLI boundary."""
    if isinstance(value, str):
        value.encode('utf-8')
    elif isinstance(value, dict):
        for key, item in value.items():
            check_unicode(key)
            check_unicode(item)
    elif isinstance(value, list):
        for item in value:
            check_unicode(item)


def strict_json(raw):
    """Decode one UTF-8 JSON value with exact integers; ambiguous/invalid input raises.

    Python's interpreter integer/depth limits may refuse technical extremes;
    this reader is not a full port of Rust semantic admission or RFC8785.
    """
    value = json.loads(raw.decode('utf-8'), object_pairs_hook=unique_object,
                       parse_float=non_integer, parse_constant=non_integer)
    check_unicode(value)
    return value


def read_regular(path):
    """Read one regular file through its owned descriptor; refuse FIFO/device/directory.

    Nonblocking open prevents FIFO waits; symlinks to regular files are allowed.
    No file is written. IO/close failures propagate; paths never confer authority.
    """
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(descriptor, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise Refusal('canonical-regular-file-required')
        return stream.read()


def verify(document_raw, result_raw, schema):
    """Return byte-bound correspondence only, or a safe Refusal on mismatch.

    All schema fields are mandatory, so the fixture oracle's defaults cannot fill
    omitted context. Request IDs remain excluded by the versioned contract.
    A matching forged pair is possible: authenticity and domain admission are
    explicitly unestablished. Caller input bytes are never mutated or printed.
    """
    document = strict_json(document_raw)
    result = strict_json(result_raw)
    if next(Draft202012Validator(schema).iter_errors(document), None) is not None:
        raise Refusal('canonical-document-shape-invalid')
    expected = vector('export-check', document['proposal']['action'],
                      document['proposal']['request_id'], **document['context'])
    envelope = {
        'schema_version': 1, 'canonicalization': 'paea-json-v1',
        'canonical': expected['canonical'], 'action_digest': expected['action_digest'],
        'intent_id': expected['intent_id'], 'authority': 'not-established',
        'scope': 'offline-data-only',
    }
    if (not isinstance(result, dict) or type(result.get('schema_version')) is not int
            or result != envelope):
        raise Refusal('canonical-correspondence-mismatch')
    return {
        'schema_version': 1, 'status': 'canonical-correspondence-verified',
        'validation': 'schema-and-canonical-correspondence-only',
        'authority': 'not-established', 'effect': 'not-established',
        'document_sha256': hashlib.sha256(document_raw).hexdigest(),
        'result_sha256': hashlib.sha256(result_raw).hexdigest(),
        'action_digest': expected['action_digest'], 'intent_id': expected['intent_id'],
    }


def main(argv=None):
    """Read explicit files and emit one JSON result; exit2 on refusal/IO/output failure.

    Safe static diagnostics omit raw values and paths. A failed output flush may
    have published partial bytes: a nonzero exit never proves absence of output.
    No implicit retries or writes to original/export files occur.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--document', required=True)
    parser.add_argument('--result', required=True)
    args = parser.parse_args(argv)
    try:
        schema = strict_json(read_regular(ROOT / 'schemas/paea/contract-v1.schema.json'))
        Draft202012Validator.check_schema(schema)
        report = verify(read_regular(args.document), read_regular(args.result), schema)
        sys.stdout.buffer.write((json.dumps(report, sort_keys=True) + '\n').encode())
        sys.stdout.buffer.flush()
        return 0
    except SchemaError:
        code = 'canonical-verifier-schema-invalid'
    except Refusal as error:
        code = str(error)
    except (UnicodeError, ValueError, RecursionError):
        code = 'canonical-json-invalid'
    except OSError:
        code = 'canonical-io-unavailable'
    try:
        sys.stderr.write(json.dumps({'status': 'refused', 'code': code}) + '\n')
        sys.stderr.flush()
    except OSError:
        pass  # Primary failure already requires exit2; stderr may also be unavailable.
    return 2


if __name__ == '__main__':
    raise SystemExit(main())
