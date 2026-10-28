"""Byte-bound laboratory evidence, never authenticated authority or E2/E3 proof.

POSIX descriptor traversal refuses symlinks, special files and unstable reads.
Only retained report/recipe/observations are checked; nothing in them is executed.
The recipe verifier reads a private temporary copy of captured regular-file bytes,
not paths from untrusted originals. Same-UID writers are not confined by this tool.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import tempfile

from tools.verification.recipe import verify_evidence

CASES = (
    'test_t04_substitution_then_equivalent_json_recovery',
    'test_t06_t21_crash_new_attempt_exact_marker_and_reconciliation',
    'test_t05_t17_ordered_revocation_first_use_and_opposite_order',
    'test_unavailable_oracle_stays_unknown_then_recovers_without_creation',
    'test_git_stale_and_symbolic_target_refuse_then_recover',
    'test_output_failure_after_effect_is_unknown_and_reconciles',
    'test_invoice_alias_is_a_counterexample_without_authoritative_registry',
    'test_resolved_invoice_aliases_converge_on_one_effect',
)
INVENTORY = 'paea-lab-evidence.json'
REPORT = 'paea-lab-report.json'


class Refusal(ValueError):
    """Safe static code; original errors remain chained, never printed with paths/data."""


def digest(raw):
    """Return SHA256 of exact bytes; a hash alone authenticates no issuer."""
    return hashlib.sha256(raw).hexdigest()


def strict_json(raw):
    """Decode exact UTF-8 JSON; reject duplicate keys, floats, constants and surrogates.

    Interpreter depth/integer constraints are technical limits, not operator policy.
    Malformed input keeps its cause and produces a static refusal code.
    """
    def unique(pairs):
        """Reject decoded duplicate object names instead of overwriting evidence."""
        value = {}
        for key, item in pairs:
            if key in value:
                raise Refusal('laboratory-json-duplicate-key')
            value[key] = item
        return value

    def number(_):
        """Refuse inexact/constant numeric representations in this integer format."""
        raise Refusal('laboratory-json-number-invalid')

    def unicode(value):
        """Require scalar Unicode in every string/key before rendering or path use."""
        if isinstance(value, str):
            value.encode('utf-8')
        elif isinstance(value, dict):
            for key, item in value.items():
                unicode(key)
                unicode(item)
        elif isinstance(value, list):
            for item in value:
                unicode(item)
    try:
        value = json.loads(raw.decode('utf-8'), object_pairs_hook=unique,
                           parse_float=number, parse_constant=number)
        unicode(value)
        return value
    except (ValueError, UnicodeError, RecursionError) as error:
        raise Refusal('laboratory-json-invalid') from error


def valid_hash(value):
    """Recognise the versioned lowercase SHA256 representation, not its authority."""
    return isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value) is not None


def relative_name(value):
    """Require an unambiguous relative POSIX name, without traversal or empty parts."""
    return (isinstance(value, str) and bool(value) and '\\' not in value and '\0' not in value
            and not PurePosixPath(value).is_absolute()
            and all(part not in ('', '.', '..') for part in value.split('/'))
            and not any(0xD800 <= ord(c) <= 0xDFFF for c in value))


def fingerprint(value):
    """Capture inode/content metadata to detect a change during an owned read."""
    return (value.st_dev, value.st_ino, value.st_mode, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def read_file(parent, name):
    """Read one regular child through a no-follow/nonblocking descriptor.

    FIFO/device/link/directory or changed metadata refuses. All bytes are retained;
    there is no product size ceiling. No original is written or path escaped.
    """
    fd = os.open(name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode):
            raise Refusal('laboratory-regular-file-required')
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            raw = stream.read()
        if fingerprint(before) != fingerprint(os.fstat(fd)):
            raise Refusal('laboratory-file-changed-during-read')
        return raw
    finally:
        os.close(fd)


def tree(parent, name, capture=False):
    """Inventory an opened subtree, retaining bytes only for requested files.

    capture=True captures the recipe snapshot; a set captures selected observation
    files. Directory entries are traversed relative to owned descriptors, never
    through symlinks. Changed listings/metadata or special files refuse. Directory
    recursion is subject to interpreter/OS limits, not an invented policy cap.
    """
    hashes, content = {}, {}

    def walk(fd, prefix=''):
        """Read each regular descendant and verify its directory stayed stable."""
        before = os.fstat(fd)
        names = sorted(os.listdir(fd))
        for child in names:
            key = prefix + child
            if not relative_name(key):
                raise Refusal('laboratory-relative-path-invalid')
            metadata = os.stat(child, dir_fd=fd, follow_symlinks=False)
            if stat.S_ISDIR(metadata.st_mode):
                nested = os.open(child, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
                try:
                    if fingerprint(os.fstat(nested)) != fingerprint(metadata):
                        raise Refusal('laboratory-directory-changed')
                    walk(nested, key + '/')
                finally:
                    os.close(nested)
            elif stat.S_ISREG(metadata.st_mode):
                raw = read_file(fd, child)
                if fingerprint(os.stat(child, dir_fd=fd, follow_symlinks=False)) != fingerprint(metadata):
                    raise Refusal('laboratory-file-changed-during-read')
                hashes[key] = digest(raw)
                if capture is True or isinstance(capture, set) and key in capture:
                    content[key] = raw
            else:
                raise Refusal('laboratory-regular-tree-required')
        if names != sorted(os.listdir(fd)) or fingerprint(before) != fingerprint(os.fstat(fd)):
            raise Refusal('laboratory-directory-changed')

    fd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
    try:
        walk(fd)
    finally:
        os.close(fd)
    return hashes, content


def validate_report(raw):
    """Check the unchanged closed report v1, exact cases and non-authority markers."""
    value = strict_json(raw)
    fields = {'schema_version', 'scope', 'recipe_id', 'binary_sha256', 'recipe_result_sha256',
              'source_inventory_sha256', 'calls_sha256', 'tests', 'refuted_claim', 'authority',
              'isolation', 'independent_evaluation', 'protected_invariants'}
    markers = {'scope': 'isolated-laboratory', 'recipe_id': 'paea-pilot-lab',
               'refuted_claim': 'raw-invoice-identifiers-guarantee-global-debt-uniqueness',
               'authority': 'simulated', 'isolation': 'same-uid',
               'independent_evaluation': 'not-established', 'protected_invariants': 'not-established'}
    if (not isinstance(value, dict) or set(value) != fields or type(value['schema_version']) is not int
            or value['schema_version'] != 1 or any(value[k] != v for k, v in markers.items())
            or any(not valid_hash(value[k]) for k in ('binary_sha256', 'recipe_result_sha256', 'source_inventory_sha256', 'calls_sha256'))
            or value['tests'] != [{'id': name, 'status': 'PASS'} for name in CASES]):
        raise Refusal('laboratory-report-invalid')
    return value


def validate_calls(raw, observed):
    """Require v2 call records and every declared original input/output hash.

    Standard calls retain stdin/file inputs, stderr and stdout except an explicit
    stdout fault. Workers retain their document/stdout/stderr. Missing or changed
    raw oracle output is thus refused even before first inventory publication.
    Recorded argv is data only and is never executed. This is not proof of truthful
    recording by a hostile same-UID writer or completeness of an unrecorded effect.
    """
    calls = strict_json(raw)
    if not isinstance(calls, list) or not calls:
        raise Refusal('laboratory-calls-missing')
    for index, call in enumerate(calls, 1):
        prefix = 'call-%04d' % index
        if (not isinstance(call, dict) or call.get('recording_version') != 2
                or type(call.get('recording_version')) is not int
                or type(call.get('id')) is not int or call['id'] != index
                or type(call.get('exit')) is not int or not isinstance(call.get('artifacts'), dict)):
            raise Refusal('laboratory-call-record-incomplete')
        if 'fixture_worker' in call:
            required = {prefix + suffix for suffix in ('.stdout', '.stderr', '.document.json')}
        else:
            if (type(call.get('stdout_fault')) is not bool or type(call.get('input_supplied')) is not bool
                    or not isinstance(call.get('input_files'), list)
                    or not isinstance(call.get('argv'), list) or not call['argv']
                    or any(not isinstance(arg, str) for arg in call['argv'])):
                raise Refusal('laboratory-call-record-incomplete')
            required = {prefix + '.stderr'}
            if not call['stdout_fault']:
                required.add(prefix + '.stdout')
            if call['input_supplied']:
                required.add(prefix + '.input')
            expected_files = [prefix + '.file-%04d.input' % n for n in range(1, len(call['input_files']) + 1)]
            if call['input_files'] != expected_files:
                raise Refusal('laboratory-call-record-incomplete')
            required.update(expected_files)
        if set(call['artifacts']) != required or any(not valid_hash(h) or observed.get(n) != h for n, h in call['artifacts'].items()):
            raise Refusal('laboratory-call-artifact-missing-or-changed')
    return calls


def check_recipe(files):
    """Verify retained recipe bytes in private scratch, never execute supplied evidence.

    Call the maintained verifier on copies of captured regular files. Temporary
    writes are removed on return/failure; originals stay untouched. A recorded
    failure, corrupt/missing artifact/source or wrong case contract refuses.
    """
    with tempfile.TemporaryDirectory(prefix='paea-recipe-snapshot-') as temporary:
        target = Path(temporary)
        for name, raw in files.items():
            if not relative_name(name):
                raise Refusal('laboratory-relative-path-invalid')
            path = target / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        try:
            verified = verify_evidence(target)
        except Exception as error:
            raise Refusal('laboratory-recipe-invalid') from error
    if verified != {'integrity': 'VERIFIED', 'recorded_status': 'PASS', 'recipe': 'paea-pilot-lab'}:
        raise Refusal('laboratory-recipe-not-passing')
    recipe = strict_json(files['recipe.original.json'])
    if len(recipe['steps']) != 1 or not set(CASES).issubset(recipe['steps'][0]['required_stdout']):
        raise Refusal('laboratory-case-contract-mismatch')


def collect(campaign, report_raw, directory_fd=None):
    """Bind one report to stable recipe/observation snapshots, or refuse without writes.

    Caller supplies exact report bytes and optionally an already owned root fd.
    The caller retains that descriptor; otherwise this function opens/closes it.
    Two complete inventories
    detect ordinary changes across verification. Root/children use owned descriptors;
    same-UID adversarial immutability or current protected authority is not proven.
    """
    report = validate_report(report_raw)
    fd = directory_fd if directory_fd is not None else os.open(campaign, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        recipes, recipe_bytes = tree(fd, 'recipe', capture=True)
        observations, selected = tree(fd, 'observations', capture={'calls.json'})
        if ('result.json' not in recipe_bytes or 'inputs.before.json' not in recipe_bytes
                or 'calls.json' not in selected or recipes['result.json'] != report['recipe_result_sha256']
                or recipes['inputs.before.json'] != report['source_inventory_sha256']
                or observations['calls.json'] != report['calls_sha256']):
            raise Refusal('laboratory-report-evidence-mismatch')
        validate_calls(selected['calls.json'], observations)
        check_recipe(recipe_bytes)
        if tree(fd, 'recipe')[0] != recipes or tree(fd, 'observations')[0] != observations:
            raise Refusal('laboratory-evidence-changed-during-verification')
        return {'schema_version': 1, 'scope': 'retained-laboratory-evidence',
                'report_sha256': digest(report_raw), 'recipe_files': recipes, 'observation_files': observations}
    finally:
        if directory_fd is None:
            os.close(fd)


def verify(campaign, expected):
    """Require an explicitly supplied inventory hash, then check all retained bytes.

    No trust-on-first-use, invocation of recorded commands or original mutation.
    Extra root diagnostics are outside the stated scope; exact recipe/observation
    membership is checked. External pinning detects changes since that inventory,
    but a caller-supplied digest alone authenticates no author or observation.
    """
    if not valid_hash(expected):
        raise Refusal('laboratory-inventory-hash-invalid')
    fd = os.open(campaign, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        inventory_raw = read_file(fd, INVENTORY)
        report_raw = read_file(fd, REPORT)
        if digest(inventory_raw) != expected:
            raise Refusal('laboratory-inventory-hash-mismatch')
        inventory = strict_json(inventory_raw)
        fields = {'schema_version', 'scope', 'report_sha256', 'recipe_files', 'observation_files'}
        if (not isinstance(inventory, dict) or set(inventory) != fields
                or type(inventory['schema_version']) is not int or inventory['schema_version'] != 1
                or inventory['scope'] != 'retained-laboratory-evidence' or not valid_hash(inventory['report_sha256'])):
            raise Refusal('laboratory-inventory-invalid')
        for key in ('recipe_files', 'observation_files'):
            if not isinstance(inventory[key], dict) or not inventory[key] or any(not relative_name(n) or not valid_hash(h) for n, h in inventory[key].items()):
                raise Refusal('laboratory-inventory-invalid')
        actual = collect(campaign, report_raw, directory_fd=fd)
        if inventory != actual:
            raise Refusal('laboratory-inventory-content-mismatch')
        if read_file(fd, INVENTORY) != inventory_raw or read_file(fd, REPORT) != report_raw:
            raise Refusal('laboratory-evidence-changed-during-verification')
    finally:
        os.close(fd)
    return {'schema_version': 1, 'status': 'laboratory-evidence-consistent',
            'scope': 'retained-bytes-only', 'inventory_sha256': expected,
            'report_sha256': actual['report_sha256'], 'observation_files': len(actual['observation_files']),
            'recipe_files': len(actual['recipe_files']), 'authority': 'simulated', 'isolation': 'same-uid',
            'independent_evaluation': 'not-established', 'protected_invariants': 'not-established'}
