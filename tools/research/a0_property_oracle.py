#!/usr/bin/env python3
'''a0_property_oracle.py - offline stdlib-only property oracle for retained A083 evidence.

Purpose:
    Re-derive, per historical case, audit verdicts for the article properties
    PT, PA, PR, PV1/PV2 (explicit 'intention' and 'attempt' projections; the
    unprojected article atoms are reported inconc) and PL, from retained
    evidence bytes only. The oracle never executes evidence: it does not open
    the SQLite stores (hashed as opaque bytes during inventory validation
    only), never reruns the recipe or the receiver, makes no network or
    subprocess calls, and needs no special runtime rights.

Inputs / preconditions:
    * --index PATH: A0-PREUVES.json index (schema in tools/research/
      A0-PROPERTY-ORACLE.md): absolute 'location', 64-hex 'inventory_sha256',
      plus an inline 'inventory' list, an 'inventory_path' manifest,
      or the retained A0-INVENTORY.json dictionary;
      optional 'result_path', 'journey_path'.
    * --output DIR: path that must NOT exist; reserved exclusively.

Outputs / side effects:
    * DIR/a0_oracle_report.json: index/source hashes, per-case input digests,
      event traces, per-property verdicts (pass/fail/inconc/error), coverage
      (triggered vs vacuous), finite violation witnesses, assumptions.
    * DIR/COMPLETE.json: completion marker binding the published artifact hashes
      only; its presence never proves a successful process return, which also
      requires the retained successful exit evidence.
    * DIR/a0_oracle_table.md: one Markdown row per historical result row
      (83 rows for A083).
    * Reads only files listed in the validated inventory, strictly read-only.

Errors:
    OracleError with stable reasons and safe diagnostics (original parser or
    OS messages, quoted) for: missing/corrupt index or manifest, digest
    mismatches, path traversal, symlink components, missing inputs, corrupt
    JSON, existing output path. Such failures never produce a PASS output.

Limitations:
    See ASSUMPTIONS below and the FR/EN docs. Verdicts are evidence-bounded:
    uninferable atoms or orderings yield inconc, never invented facts.
    Historical result.json statuses are recorded but never used as verdicts.
    Execution and acceptance are separate: every generated report binds the
    actual source hashes and retains inconclusive/error verdicts.
'''

import argparse
import hashlib
import json
import os
import re
import stat
import sys
import tempfile
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

TOOL_NAME = 'a0_property_oracle'
TOOL_VERSION = '0.1.0'

REPORT_NAME = 'a0_oracle_report.json'
TABLE_NAME = 'a0_oracle_table.md'

SEVERITY = {'pass': 0, 'inconc': 1, 'error': 2, 'fail': 3}
TRACE_PROPERTIES = ('PT', 'PA', 'PR', 'PV1_intention', 'PV1_attempt',
                    'PV2_intention', 'PV2_attempt')
ALL_PROPERTIES = TRACE_PROPERTIES + ('PV1', 'PV2', 'PL')
APPROVAL_ACTIVE_STATES = ('approved', 'consumed')
EXTRA_NAMES = ('oracle.json', 'initial.observation.json', 'terminal.observation.json')

DEFAULT_PROJECTION_NOTE = (
    'unprojected article atom: the identifier i is unresolved between business '
    'intention (intent_id) and per-attempt identity; see the *_intention and '
    '*_attempt projections; no universal compliance is claimed.')

PL_INCONC_NOTE = ('PL bound Delta is unspecified and request/ack timing is not '
                  'observable in the evidence; no latency is invented.')
PL_VACUOUS_NOTE = 'no withdrawal request in this case; vacuous only.'

ASSUMPTIONS = [
    'A0-projection: a channel recorded in the observed configuration is treated as an '
    'enabled logical channel (presence in configuration = enabled). No OS/network '
    'identity is proven; this is an explicit A0 assumption, not a Swarm proof.',
    'Expiration and real-time validity of approvals are NOT modeled in A0; no result '
    'here speaks about real-time expiry or Swarm behaviour.',
    'PL: the bound Delta is unspecified and request/ack timing is not observable; any '
    'case containing a withdrawal (a revoke request, or an updated response whose '
    'request bytes are unparseable) is inconc; cases without withdrawal are vacuous '
    'only. No latency is invented.',
    'The article identifier i is unresolved between business intention (intent_id) '
    'and per-attempt identity; unprojected PV1/PV2 are reported inconc and two '
    'explicit projections are emitted; no universal compliance is claimed.',
    'Events are derived only from success responses corroborated by persisted '
    'snapshots; commit events come only from new rows in before/after effect diffs. '
    'A claimed response alone is never an event.',
    "Responses with status 'unknown' trigger neither PV1 nor PV2; their coverage is "
    'annotated separately from verdicts, and an unknown-response pass never '
    'establishes recovery quality.',
    "'past' includes the current trace position; each response is evaluated only "
    'against its own past prefix (no retroactive reclassification, e.g. a '
    'before_commit refusal followed by recovery is not an earlier PV2 fail).',
    'Every snapshot is bound to the successful initialization request case_run, '
    'not only to the other snapshots. Initial captures must match the fresh-store '
    'initialization; terminal captures are checked even with an explicit final after-state.',
    'Historical result.json statuses (PASS / EXPECTED_COUNTEREXAMPLE) are recorded '
    'for traceability and are never used as oracle verdicts.',
    'Digest comparisons use stored bytes only; no digest is recomputed for verdicts.',
    'No total timestamp order is invented across actors; ordering is the numbered '
    'sequential command order inside each case.',
    'Vacuous passes are annotated via coverage and never establish triggered '
    'compliance.',
    'An unobserved or corrupt snapshot anywhere in a case is a trace hole: affected '
    'properties become inconc (an earlier fail is never overwritten). Invalid raw '
    'request bytes keep refusals unattributed rather than inventing an identifier.',
]


class OracleError(Exception):
    '''Safe, user-facing failure.

    Purpose: carry a stable machine reason plus a safe detail (evidence-relative
    path, original parser/OS message) to stderr; never secrets, never a PASS.
    Inputs: reason token, optional detail. Side effects: none.
    '''

    def __init__(self, reason: str, detail: str = '') -> None:
        self.reason = reason
        self.detail = detail
        super().__init__(reason + (': ' + detail if detail else ''))


# ---------------------------------------------------------------- basic utils

def _read_bytes(path: str) -> bytes:
    '''Read a whole file as bytes. Errors: OSError propagates to callers that
    wrap it into OracleError. Side effects: none (read-only).'''
    with open(path, 'rb') as handle:
        return handle.read()


def sha256_file(path: str) -> str:
    '''Purpose: streaming SHA-256 of a regular file (no size bound).
    Inputs: pre-validated absolute path. Output: hex digest.
    Errors: OSError propagates. Side effects: none.'''
    digest = hashlib.sha256()
    with open(path, 'rb') as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def _strict_json(data: bytes) -> Any:
    '''Strict JSON decode mirroring the receiver contract.

    Purpose: parse bytes while rejecting duplicate object names (including
    escaped-name duplicates), non-integer numbers and non-JSON constants, so
    the oracle attributes operations exactly like the receiver did.
    Inputs: raw bytes. Output: decoded value. Errors: ValueError (subclass
    UnicodeDecodeError included) with a safe message. Side effects: none.'''

    def unique_object(pairs):
        out = {}
        for key, value in pairs:
            if key in out:
                raise ValueError('duplicate object name %r' % (key,))
            out[key] = value
        return out

    def no_float(_text):
        raise ValueError('non-integer JSON number')

    def no_const(_text):
        raise ValueError('non-JSON constant')

    text = data.decode('utf-8')
    return json.loads(text, object_pairs_hook=unique_object,
                      parse_float=no_float, parse_constant=no_const)


def _safe_json_error(error: Exception) -> str:
    '''Purpose: keep the original parser diagnostic, for stderr and
    the report. Inputs: exception. Output: original diagnostic string. Side effects: none.'''
    return str(error)


def _decode_request(data: bytes) -> Tuple[Optional[Any], Optional[str]]:
    '''Purpose: decode one raw NNN.stdin request.
    Output: (object, None) on success, (None, safe-reason) otherwise; raw
    stdin may legitimately be invalid and is never fatal here.
    Side effects: none.'''
    try:
        return _strict_json(data), None
    except ValueError as error:
        return None, _safe_json_error(error)


def _canonical_json(obj: Any) -> str:
    '''Purpose: canonical serialization used only to compare a parsed request
    action against stored canonical bytes. Side effects: none.'''
    return json.dumps(obj, sort_keys=True, separators=(',', ':'), ensure_ascii=False)


def _parse_action_text(text: Any) -> Optional[Dict[str, Any]]:
    '''Purpose: parse stored canonical action bytes (effects/approvals).
    Output: dict or None; never raises. Limitations: unparseable stored bytes
    degrade dependent checks to inconc.'''
    if not isinstance(text, str):
        return None
    try:
        obj = _strict_json(text.encode('utf-8'))
    except ValueError:
        return None
    return obj if isinstance(obj, dict) else None


def _effect_key(eid: Any) -> Tuple[str, Any]:
    '''Purpose: normalize an effect id for diffing/sorting across snapshots.
    Side effects: none.'''
    if isinstance(eid, bool) or not isinstance(eid, (int, str)):
        return ('s', str(eid))
    if isinstance(eid, int):
        return ('n', eid)
    if eid.isdigit():
        return ('n', int(eid))
    return ('s', eid)


def _pos_name(pos: Tuple[int, int]) -> str:
    '''Purpose: render a snapshot position as 'step:before'/'step:after' for
    notes and witnesses. Side effects: none.'''
    label = 'before' if pos[1] == 0 else 'after'
    return str(pos[0]) + ':' + label


# ------------------------------------------------------- validation machinery

def safe_resolve(root_real: str, rel: str) -> str:
    '''Purpose: resolve a manifest-relative path under the retained root while
    refusing traversal and symlinks.

    Inputs: realpath of the retained root, relative POSIX path.
    Output: absolute path of a regular file.
    Errors: OracleError(path_invalid | path_absolute_or_backslash |
    path_traversal | file_missing | symlink_rejected |
    intermediate_not_directory | not_regular_file | path_escapes_root).
    Side effects: none (stat only). Limitations: TOCTOU between validation and
    read is narrowed by the per-component lstat walk and realpath containment.'''
    if not isinstance(rel, str) or not rel or chr(0) in rel:
        raise OracleError('path_invalid', repr(rel))
    if rel.startswith('/') or chr(92) in rel or re.match(r'^[A-Za-z]:', rel):
        raise OracleError('path_absolute_or_backslash', rel)
    parts = rel.split('/')
    for part in parts:
        if part in ('', '.', '..'):
            raise OracleError('path_traversal', rel)
    current = root_real
    final_stat = None
    for index, part in enumerate(parts):
        current = os.path.join(current, part)
        try:
            final_stat = os.lstat(current)
        except OSError as error:
            raise OracleError('file_missing', rel + ' (' + str(error.strerror) + ')') from error
        if stat.S_ISLNK(final_stat.st_mode):
            raise OracleError('symlink_rejected', rel)
        if index < len(parts) - 1 and not stat.S_ISDIR(final_stat.st_mode):
            raise OracleError('intermediate_not_directory', rel)
    if final_stat is None or not stat.S_ISREG(final_stat.st_mode):
        raise OracleError('not_regular_file', rel)
    real = os.path.realpath(current)
    if os.path.commonpath([root_real, real]) != root_real:
        raise OracleError('path_escapes_root', rel)
    return current


def load_index(index_path: str, raw: Optional[bytes] = None) -> Dict[str, Any]:
    '''Purpose: parse and structurally validate A0-PREUVES.json.
    Inputs: index path; optional raw bytes (when provided, the index is hashed
    and strictly parsed from this single buffer and is never read again).
    Output: index dict.
    Errors: OracleError(index_unreadable | index_corrupt_json | index_shape |
    index_location_invalid | index_location_missing |
    index_inventory_sha_invalid). Side effects: none (read-only).
    The retained layout is the observed A0 index: location, journey, and
    inventory_sha256 bind A0-INVENTORY.json (a relative-path/hash object).
    Explicit inventory_path/result_path and inline test manifests remain
    supported; they never bypass the recorded inventory digest.'''
    if raw is None:
        try:
            raw = _read_bytes(index_path)
        except OSError as error:
            raise OracleError('index_unreadable', str(error)) from error
    try:
        index = _strict_json(raw)
    except ValueError as error:
        raise OracleError('index_corrupt_json', _safe_json_error(error)) from error
    if not isinstance(index, dict):
        raise OracleError('index_shape', 'top-level JSON object expected')
    location = index.get('location')
    if not isinstance(location, str) or not location or not os.path.isabs(location):
        raise OracleError('index_location_invalid',
                          'index key location must be an absolute path')
    if os.path.islink(location) or not os.path.isdir(location):
        raise OracleError('index_location_missing', location)
    recorded = index.get('inventory_sha256')
    if not isinstance(recorded, str) or not re.fullmatch(r'[0-9a-f]{64}', recorded):
        raise OracleError('index_inventory_sha_invalid',
                          'index key inventory_sha256 must be 64 lowercase hex chars')
    for key in ('result_path', 'journey_path', 'inventory_path'):
        value = index.get(key)
        if value is not None and (not isinstance(value, str) or not value):
            raise OracleError('index_shape',
                              'index key ' + repr(key) + ' must be a non-empty string when present')
    if 'journey_path' not in index and 'journey' in index:
        journey = index['journey']
        if not isinstance(journey, str) or not os.path.isabs(journey):
            raise OracleError('index_journey_invalid', 'absolute journey required')
        relative = os.path.relpath(journey, location)
        if relative == '..' or relative.startswith('../'):
            raise OracleError('index_journey_invalid', 'journey outside source location')
        index['journey_path'] = relative
        index.setdefault('result_path', relative + '/result.json')
    return index


def load_manifest_entries(index: Dict[str, Any], root_real: str) -> List[Tuple[str, str]]:
    '''Purpose: obtain the per-file manifest and verify the recorded inventory
    SHA-256 before anything else.

    Inputs: validated index, realpath of the retained root.
    Output: list of (relative path, 64-hex sha256).
    Errors: OracleError(inventory_shape | inventory_digest_mismatch |
    inventory_corrupt_json | inventory_manifest_missing | path/symlink errors
    from safe_resolve). Side effects: reads the manifest file when referenced.
    Limitations: inline inventories are hashed over their canonical JSON
    (sort_keys, compact separators); file manifests over their raw bytes.'''
    expected = index['inventory_sha256']
    inline = index.get('inventory')
    inv_rel = index.get('inventory_path', 'A0-INVENTORY.json')
    if inline is not None:
        if not isinstance(inline, list) or not inline:
            raise OracleError('inventory_shape', 'index key inventory must be a non-empty list')
        canonical = _canonical_json(inline).encode('utf-8')
        got = hashlib.sha256(canonical).hexdigest()
        if got != expected:
            raise OracleError('inventory_digest_mismatch',
                              'inline inventory canonical sha256 ' + got + ' != recorded ' + expected)
        entries = inline
    elif isinstance(inv_rel, str):
        try:
            path = safe_resolve(root_real, inv_rel)
        except OracleError as error:
            if error.reason == 'file_missing' and 'inventory_path' not in index:
                raise OracleError('inventory_manifest_missing', 'A0-INVENTORY.json absent') from error
            raise
        raw = _read_bytes(path)
        got = hashlib.sha256(raw).hexdigest()
        if got != expected:
            raise OracleError('inventory_digest_mismatch',
                              inv_rel + ': sha256 ' + got + ' != recorded ' + expected)
        try:
            entries = _strict_json(raw)
        except ValueError as error:
            raise OracleError('inventory_corrupt_json', _safe_json_error(error)) from error
    else:
        raise OracleError('inventory_manifest_missing',
                          'index needs an inventory list or an inventory_path manifest')
    if isinstance(entries, dict):
        entries = [{'path': rel, 'sha256': digest} for rel, digest in entries.items()]
    if not isinstance(entries, list) or not entries:
        raise OracleError('inventory_shape', 'manifest must contain relative paths and hashes')
    normalized: List[Tuple[str, str]] = []
    for entry in entries:
        if not isinstance(entry, dict):
            raise OracleError('inventory_shape', 'each manifest entry must be an object')
        rel = entry.get('path')
        sha = entry.get('sha256')
        if not isinstance(rel, str) or not rel:
            raise OracleError('inventory_shape', 'manifest entry needs a string path')
        if not isinstance(sha, str) or not re.fullmatch(r'[0-9a-f]{64}', sha):
            raise OracleError('inventory_shape',
                              'manifest entry ' + repr(rel) + ' needs a 64-hex sha256')
        normalized.append((rel, sha))
    return normalized


def validate_inventory(root_real: str, entries: List[Tuple[str, str]]) -> Dict[str, str]:
    '''Purpose: hash-validate every manifest entry before any evaluation.
    Inputs: realpath root, manifest entries. Output: {relative path: absolute
    path}. Errors: OracleError(path_*, symlink_rejected, file_missing,
    file_digest_mismatch, inventory_duplicate_path, inventory_empty).
    Side effects: reads the listed files (read-only). Limitations: files are
    trusted only through this map afterwards.'''
    files: Dict[str, str] = {}
    for rel, sha in entries:
        if rel in files:
            raise OracleError('inventory_duplicate_path', rel)
        path = safe_resolve(root_real, rel)
        got = sha256_file(path)
        if got != sha:
            raise OracleError('file_digest_mismatch',
                              rel + ': recorded ' + sha + ', computed ' + got)
        files[rel] = path
    if not files:
        raise OracleError('inventory_empty', 'no entries validated')
    return files


def load_result_rows(files_map: Dict[str, str], index: Dict[str, Any]) -> Tuple[List[Dict[str, Any]], str]:
    '''Purpose: locate and parse the historical result file (83 rows for A083).
    Output: (rows, relative path). Errors: OracleError(result_missing |
    result_corrupt_json | result_shape). Side effects: none.
    Limitations: the historical 'result' field is recorded, never used as a
    verdict; a top-level {run:{rows:[...]}} wrapper is also accepted.'''
    rel = index.get('result_path') or 'result.json'
    if not isinstance(rel, str) or rel not in files_map:
        raise OracleError('result_missing', str(rel))
    try:
        data = _strict_json(_read_bytes(files_map[rel]))
    except ValueError as error:
        raise OracleError('result_corrupt_json', _safe_json_error(error)) from error
    rows = None
    if isinstance(data, dict):
        rows = data.get('rows')
        if rows is None and isinstance(data.get('run'), dict):
            rows = data['run'].get('rows')
    elif isinstance(data, list):
        rows = data
    if not isinstance(rows, list) or not rows:
        raise OracleError('result_shape', 'a non-empty rows list is required')
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get('case'), str) or not row['case']:
            raise OracleError('result_shape',
                              'each row must be an object with a non-empty string case')
    return rows, rel


# ------------------------------------------------------------ evidence loading

def discover_cases(files_map: Dict[str, str], journey_path: Optional[str]) -> Dict[str, Dict[str, Dict[str, str]]]:
    '''Purpose: group validated inventory paths per case directory.
    Inputs: validated file map, optional journey prefix.
    Output: {case: {'steps': {numstr: {suffix: rel}}, 'extras': {name: rel}}}.
    Errors: none (pure grouping). Side effects: none. Limitations: only the
    documented evidence names are grouped; anything else is ignored.'''
    cases: Dict[str, Dict[str, Dict[str, str]]] = {}
    prefix = (journey_path.rstrip('/') + '/') if journey_path else None
    step_re = re.compile(r'^(.+)/([0-9]+)[.](stdin|result[.]json|process[.]json)$')
    before_re = re.compile(r'^(.+)/before_([0-9]+)[.]observation[.]json$')
    after_re = re.compile(r'^(.+)/after_([0-9]+)[.]observation[.]json$')
    for rel in files_map:
        inner = rel
        if prefix is not None:
            if not rel.startswith(prefix):
                continue
            inner = rel[len(prefix):]
        match = step_re.match(inner)
        if match:
            case, num, suffix = match.group(1), match.group(2), match.group(3)
            slot = cases.setdefault(case, {'steps': {}, 'extras': {}})
            slot['steps'].setdefault(num, {})[suffix] = rel
            continue
        match = before_re.match(inner)
        if match:
            slot = cases.setdefault(match.group(1), {'steps': {}, 'extras': {}})
            slot['steps'].setdefault(match.group(2), {})['before'] = rel
            continue
        match = after_re.match(inner)
        if match:
            slot = cases.setdefault(match.group(1), {'steps': {}, 'extras': {}})
            slot['steps'].setdefault(match.group(2), {})['after'] = rel
            continue
        for name in EXTRA_NAMES:
            if inner.endswith('/' + name):
                case = inner[:-(len(name) + 1)]
                slot = cases.setdefault(case, {'steps': {}, 'extras': {}})
                slot['extras'][name] = rel
                break
    return cases


class _Corrupt(Exception):
    '''Internal marker for corrupt snapshot bytes (safe message carried).'''


def _parse_observation(data: bytes, label: str) -> Dict[str, Any]:
    '''Purpose: parse one observation snapshot into configuration / approvals
    (id-indexed) / effects (effect-id-indexed) maps.
    Errors: _Corrupt with a safe message. Side effects: none.
    Limitations: approval rows must have the 7 documented columns and effect
    rows the 3 documented columns; anything else is corruption, never guessed.'''
    try:
        obj = _strict_json(data)
    except ValueError as error:
        raise _Corrupt(label + ': ' + _safe_json_error(error)) from error
    if not isinstance(obj, dict):
        raise _Corrupt(label + ': JSON object expected')
    approvals: Dict[str, Dict[str, Any]] = {}
    if not all(key in obj for key in ('approvals', 'effects', 'configuration', 'case_run')):
        raise _Corrupt(label + ': required observation field absent')
    raw_approvals = obj.get('approvals')
    if raw_approvals is not None:
        if not isinstance(raw_approvals, list):
            raise _Corrupt(label + ': approvals must be a list')
        for row in raw_approvals:
            if not (isinstance(row, list) and len(row) == 7):
                raise _Corrupt(label + ': malformed approvals row')
            if str(row[0]) in approvals:
                raise _Corrupt(label + ': duplicate approval identifier')
            approvals[str(row[0])] = {'action': row[1], 'digest': row[2],
                                      'presentation': row[3],
                                      'presentation_digest': row[4],
                                      'approver_channel': row[5], 'state': row[6]}
    effects: Dict[Tuple[str, Any], Dict[str, Any]] = {}
    raw_effects = obj.get('effects')
    if raw_effects is not None:
        if not isinstance(raw_effects, list):
            raise _Corrupt(label + ': effects must be a list')
        for row in raw_effects:
            if not (isinstance(row, list) and len(row) == 3):
                raise _Corrupt(label + ': malformed effects row')
            if _effect_key(row[0]) in effects:
                raise _Corrupt(label + ': duplicate effect identifier')
            effects[_effect_key(row[0])] = {'approval_id': row[1], 'action_text': row[2]}
    config = obj.get('configuration')
    if not isinstance(obj.get('case_run'), str) or not obj['case_run']:
        raise _Corrupt(label + ': case identity absent')
    if isinstance(config, dict) and config.get('case_run') is not None and config['case_run'] != obj['case_run']:
        raise _Corrupt(label + ': snapshot belongs to a different case run')
    return {'configuration': config if isinstance(config, dict) else None,
            'approvals': approvals, 'effects': effects, 'case_run': obj.get('case_run')}


@dataclass
class StepEvidence:
    '''Raw per-step evidence bundle (adapter input).

    Inputs: one numbered step of one case. stdin may legitimately be
    unparseable (kept as request_error); corrupt result/process JSON and
    missing required files are recorded, never silently repaired.
    Side effects: none (populated from validated files).'''
    num: int
    numstr: str
    digests: Dict[str, str] = field(default_factory=dict)
    request: Optional[Dict[str, Any]] = None
    request_error: Optional[str] = None
    response: Optional[Dict[str, Any]] = None
    response_error: Optional[str] = None
    process: Optional[Dict[str, Any]] = None
    process_error: Optional[str] = None
    channel: Optional[str] = None
    fault: Optional[str] = None
    exit_code: Optional[int] = None
    streams_closed: Optional[bool] = None
    before: Optional[Dict[str, Any]] = None
    after: Optional[Dict[str, Any]] = None
    before_error: Optional[str] = None
    after_error: Optional[str] = None
    missing: List[str] = field(default_factory=list)
    notes: List[str] = field(default_factory=list)


def _load_bytes(files_map: Dict[str, str], rel: str) -> Tuple[bytes, str]:
    '''Purpose: read one inventory-validated file. Output: (bytes, sha256).
    Errors: OracleError(file_not_in_inventory | file_unreadable).
    Side effects: none beyond reading.'''
    path = files_map.get(rel)
    if path is None:
        raise OracleError('file_not_in_inventory', rel)
    try:
        data = _read_bytes(path)
    except OSError as error:
        raise OracleError('file_unreadable', rel + ' (' + str(error.strerror) + ')') from error
    return data, hashlib.sha256(data).hexdigest()


def load_step(files_map: Dict[str, str], numstr: str, entry: Dict[str, str]) -> StepEvidence:
    '''Purpose: load one numbered step's evidence files.
    Inputs: validated file map, step number string, file-name entry.
    Output: StepEvidence. Errors: OracleError from _load_bytes (validated files
    that vanish mid-run). Side effects: none.
    Limitations: missing before/after observations are allowed here and become
    trace holes upstream; missing stdin/result/process are recorded in missing.'''
    step = StepEvidence(num=int(numstr), numstr=numstr)
    rel = entry.get('stdin')
    if rel is None:
        step.missing.append('stdin')
    else:
        data, dig = _load_bytes(files_map, rel)
        step.digests[rel] = dig
        obj, err = _decode_request(data)
        if obj is not None:
            step.request = obj if isinstance(obj, dict) else None
            if not isinstance(obj, dict):
                step.request_error = 'JSON object expected'
        else:
            step.request_error = err
    rel = entry.get('result.json')
    if rel is None:
        step.missing.append('result.json')
    else:
        data, dig = _load_bytes(files_map, rel)
        step.digests[rel] = dig
        try:
            obj = _strict_json(data)
        except ValueError as error:
            step.response_error = _safe_json_error(error)
        else:
            if isinstance(obj, dict):
                step.response = obj
            else:
                step.response_error = 'JSON object expected'
    rel = entry.get('process.json')
    if rel is None:
        step.missing.append('process.json')
    else:
        data, dig = _load_bytes(files_map, rel)
        step.digests[rel] = dig
        try:
            obj = _strict_json(data)
        except ValueError as error:
            step.process_error = _safe_json_error(error)
        else:
            if not isinstance(obj, dict):
                step.process_error = 'JSON object expected'
            else:
                step.process = obj
                argv = obj.get('argv')
                if isinstance(argv, list):
                    for position, item in enumerate(argv):
                        if item == '--channel' and position + 1 < len(argv) and isinstance(argv[position + 1], str):
                            step.channel = argv[position + 1]
                        elif item == '--fault' and position + 1 < len(argv) and isinstance(argv[position + 1], str):
                            step.fault = argv[position + 1]
                exit_value = obj.get('exit')
                if isinstance(exit_value, int) and not isinstance(exit_value, bool):
                    step.exit_code = exit_value
                closed = obj.get('streams_closed')
                if isinstance(closed, bool):
                    step.streams_closed = closed
    for kind in ('before', 'after'):
        rel = entry.get(kind)
        if rel is None:
            continue
        data, dig = _load_bytes(files_map, rel)
        step.digests[rel] = dig
        try:
            snapshot = _parse_observation(data, kind + '_' + numstr)
        except _Corrupt as error:
            if kind == 'before':
                step.before_error = str(error)
            else:
                step.after_error = str(error)
        else:
            if kind == 'before':
                step.before = snapshot
            else:
                step.after = snapshot
    return step


def build_steps(files_map: Dict[str, str], case_entry: Dict[str, Any]) -> Tuple[List[StepEvidence], List[str]]:
    '''Purpose: load and shape-check all steps of one case.
    Output: (steps in numeric order, shape notes). Shape notes cover numbering
    gaps, missing required files and corrupt result/process JSON; they become
    error instances upstream (never PASS). Side effects: none.
    Limitations: corrupt stdin is expected data, not a shape error.'''
    by_int: Dict[int, str] = {}
    for numstr in case_entry['steps']:
        by_int[int(numstr)] = numstr
    nums = sorted(by_int)
    notes: List[str] = []
    if nums and nums[0] != 1:
        notes.append('first numbered step is ' + str(nums[0]) + ', expected 1')
    for prev, cur in zip(nums, nums[1:]):
        if cur != prev + 1:
            notes.append('numbering gap between steps ' + str(prev) + ' and ' + str(cur))
    steps = [load_step(files_map, by_int[n], case_entry['steps'][by_int[n]]) for n in nums]
    for step in steps:
        for required in ('stdin', 'result.json', 'process.json'):
            if required in step.missing:
                notes.append('step ' + step.numstr + ': required evidence ' + required + ' missing')
        for kind in ('before', 'after'):
            corruption = getattr(step, kind + '_error')
            if corruption:
                notes.append('step ' + step.numstr + ': corrupt ' + kind + ' observation: ' + corruption)
        if step.response_error:
            notes.append('step ' + step.numstr + ': result.json corrupt: ' + step.response_error)
        if step.process_error:
            notes.append('step ' + step.numstr + ': process.json corrupt: ' + step.process_error)
    return steps, notes


def load_extras(files_map: Dict[str, str], extras_entry: Dict[str, str]) -> Tuple[Dict[str, Any], Dict[str, str], List[str]]:
    '''Purpose: load optional per-case files (oracle.json, initial/terminal
    observations). Output: (parsed extras, digests, notes). Corrupt optional
    evidence yields a note (upstream error instance), never silent acceptance.
    Side effects: none.'''
    data: Dict[str, Any] = {}
    digests: Dict[str, str] = {}
    notes: List[str] = []
    for name in EXTRA_NAMES:
        rel = extras_entry.get(name)
        if rel is None:
            continue
        try:
            raw, dig = _load_bytes(files_map, rel)
        except OracleError as error:
            notes.append(name + ': ' + error.reason + ': ' + error.detail)
            continue
        digests[rel] = dig
        try:
            if name == 'oracle.json':
                obj = _strict_json(raw)
                if isinstance(obj, dict):
                    data['oracle'] = obj
                else:
                    notes.append(name + ': JSON object expected')
            else:
                key = 'initial' if name.startswith('initial') else 'terminal'
                data[key] = _parse_observation(raw, name)
        except ValueError as error:
            notes.append(name + ': corrupt JSON: ' + _safe_json_error(error))
        except _Corrupt as error:
            notes.append(name + ': corrupt snapshot: ' + str(error))
    return data, digests, notes


# ------------------------------------------------------------- trace adapter

def derive_trace(steps: List[StepEvidence], extras: Dict[str, Any]) -> Dict[str, Any]:
    '''Trace adapter: per-step evidence -> ordered event trace (no verdicts).

    Purpose: resolve before/after states per step, derive commit events only
    from new effect rows in before/after diffs, derive approve/revoke events
    only from success responses corroborated by persisted snapshots, and record
    every response with its channel, fault and process health.
    Inputs: ordered StepEvidence list, parsed extras.
    Output: trace dict (steps, commits, approves, revokes, snapshots, holes,
    anomalies, effects_uncertain, persistence_conflicts).
    Errors: case_run/initialization OracleError refusals are raised before any diff when the
    snapshots disagree with each other or their initialization request, so no
    commitment is manufactured across runs. Initial/terminal extras are checked
    independently of numbered after-state availability; other corruption is expressed via holes/anomalies/notes.
    Side effects: none. Limitations: one event per trace position; the persisted
    state after step k is after_k, else before_(k+1), else terminal for the
    last step; the state before step 1 may legitimately be unobserved; no
    cross-actor timestamp order is inferred.'''
    trace: Dict[str, Any] = {
        'steps': [], 'commits': [], 'approves': [], 'approve_unconfirmed': [],
        'approve_contradicted': [], 'revokes_effective': [], 'revokes_unconfirmed': [],
        'snapshots': [], 'holes': [], 'anomalies': [], 'effects_uncertain': set(),
        'persistence_conflicts': [],
    }
    if not steps:
        return trace
    runs = set()
    for step in steps:
        for snapshot in (step.before, step.after):
            if snapshot is not None and isinstance(snapshot.get('case_run'), str):
                runs.add(snapshot['case_run'])
    for key in ('initial', 'terminal'):
        snapshot = extras.get(key)
        if isinstance(snapshot, dict) and isinstance(snapshot.get('case_run'), str):
            runs.add(snapshot['case_run'])
    if len(runs) > 1:
        raise OracleError('case_run_mismatch',
                          'snapshots of one case bind several case runs: '
                          + repr(sorted(runs))
                          + '; effect diffs are never derived across runs')
    # Bind observations to the actual successful initialization request, not
    # merely to each other: an entirely coherent foreign run is still foreign.
    initializations = []
    for step in steps:
        if (isinstance(step.request, dict) and step.request.get('op') == 'initialize'
                and isinstance(step.response, dict) and step.response.get('status') == 'initialized'):
            configuration = step.request.get('configuration')
            requested_run = configuration.get('case_run') if isinstance(configuration, dict) else None
            if (not isinstance(requested_run, str) or not requested_run
                    or step.exit_code != 0 or step.streams_closed is not True):
                raise OracleError('initialization_unattributed', 'successful initialization lacks corroborated run input/process')
            echoed_run = step.response.get('case_run')
            if echoed_run is not None and echoed_run != requested_run:
                raise OracleError('initialization_run_mismatch', 'initialization response differs from requested case_run')
            initializations.append((step.num, configuration))
    if not initializations:
        raise OracleError('initialization_unattributed', 'no successful initialization request anchors this case')
    anchor_run = initializations[0][1]['case_run']
    if any(configuration['case_run'] != anchor_run for _, configuration in initializations):
        raise OracleError('initialization_run_mismatch', 'several initialization requests bind different case runs')
    if any(run != anchor_run for run in runs):
        raise OracleError('case_run_initialization_mismatch', 'snapshots differ from the requested initialization case_run')
    initial = extras.get('initial')
    if isinstance(initial, dict):
        if initial['configuration'] != initializations[0][1]:
            raise OracleError('initialization_configuration_mismatch', 'initial snapshot differs from requested configuration')
        # The retained receiver initializes an exclusively created fresh store
        # with an empty effects table. Validate this extra snapshot even when
        # an explicit after-initialize snapshot is also present.
        if initial['effects']:
            raise OracleError('initialization_effects_nonempty', 'initial snapshot contradicts the fresh-store initialization contract')
    trace['initialization_case_run'] = anchor_run
    seen_effects: Dict[Any, Dict[str, Any]] = {}

    def observe_persistence(snapshot, position):
        """Retain immutable effect identity across every ordered snapshot.

        Record contradictions without synthesizing replacement commit events;
        the evaluator reports an evidence error, retaining sound earlier witnesses.
        No IO or verdict computation occurs here.
        """
        if snapshot is None:
            return
        effects = snapshot['effects']
        gone = [eid for eid in seen_effects if eid not in effects]
        rewritten = [eid for eid in effects if eid in seen_effects
                     and effects[eid] != seen_effects[eid]]
        if gone or rewritten:
            trace['persistence_conflicts'].append(position + ': effects gone='
                + repr(gone) + ' rewritten=' + repr(rewritten))
        for eid, row in effects.items():
            seen_effects.setdefault(eid, dict(row))

    prev_after: Optional[Tuple[Dict[str, Any], str]] = None
    for index, step in enumerate(steps):
        k = step.num
        is_last = (index == len(steps) - 1)
        if step.before_error:
            step.notes.append('before_' + step.numstr + ' corrupt: ' + step.before_error)
        if step.after_error:
            step.notes.append('after_' + step.numstr + ' corrupt: ' + step.after_error)
        before = None
        if step.before is not None:
            before = step.before
        elif prev_after is not None:
            before = prev_after[0]
            step.notes.append('before_' + step.numstr + ' absent; using ' + prev_after[1] + ' as the before-state')
        elif step.before_error is not None:
            trace['holes'].append(str(k) + ':before')
        elif k == 1:
            step.notes.append('no before-state for the first command (normal for initialize)')
        else:
            trace['holes'].append(str(k) + ':before')
        after = None
        after_label = None
        if step.after is not None:
            after = step.after
            after_label = 'after_' + step.numstr
        elif k == 1 and isinstance(extras.get('initial'), dict):
            after = extras['initial']
            after_label = 'initial.observation.json'
            step.notes.append('initial snapshot is after initialize, per retained driver')
        elif (isinstance(step.request, dict) and step.request.get('op') == 'present'
              and isinstance(step.response, dict) and step.response.get('status') == 'presented'
              and before is not None and step.exit_code == 0 and step.streams_closed is True):
            after = before
            after_label = 'readonly-present-after-' + step.numstr
            step.notes.append('present is read-only under the retained receiver contract')
        elif index + 1 < len(steps) and steps[index + 1].before is not None:
            after = steps[index + 1].before
            after_label = 'before_' + steps[index + 1].numstr
            step.notes.append('after_' + step.numstr + ' absent; using ' + after_label + ' as the after-state')
        elif is_last and isinstance(extras.get('terminal'), dict):
            after = extras['terminal']
            after_label = 'terminal.observation.json'
            step.notes.append('after-state taken from terminal.observation.json')
        else:
            trace['holes'].append(str(k) + ':after')
        if before is not None:
            trace['snapshots'].append(((k, 0), before))
        if after is not None:
            trace['snapshots'].append(((k, 1), after))
        request = step.request if isinstance(step.request, dict) else None
        response = step.response if isinstance(step.response, dict) else None
        op = request.get('op') if request else None
        op_known = isinstance(op, str)
        status = response.get('status') if response else None
        status = status if isinstance(status, str) else None
        process_ok = (step.exit_code == 0 and step.streams_closed is True)
        events: List[Dict[str, Any]] = []
        # Compare adjacent command boundaries too; an explicit next before-state
        # must not silently rewrite an earlier after-state's immutable effect.
        observe_persistence(before, str(k) + ':before')
        historical_ids = set(seen_effects)
        observe_persistence(after, str(k) + ':after')
        # Commit events: only genuinely new IDs in the observed step, never
        # rewritten/reappearing identifiers from earlier snapshots.
        if before is not None and after is not None:
            new_ids = [eid for eid in after['effects']
                       if eid not in before['effects'] and eid not in historical_ids]
            gone = [eid for eid in before['effects'] if eid not in after['effects']]
            rewritten = [eid for eid in after['effects'] if eid in before['effects']
                         and (before['effects'][eid].get('approval_id') != after['effects'][eid].get('approval_id')
                              or before['effects'][eid].get('action_text') != after['effects'][eid].get('action_text'))]
            if gone or rewritten:
                trace['persistence_conflicts'].append('step ' + str(k) + ': effects gone='
                                                      + repr(gone) + ' rewritten=' + repr(rewritten))
                trace['anomalies'].append('step ' + str(k) + ': effect persistence inconsistent: gone='
                                          + repr(gone) + ' rewritten=' + repr(rewritten))
            for eid in sorted(new_ids, key=_effect_key):
                effect = after['effects'][eid]
                action = _parse_action_text(effect.get('action_text'))
                identity = None
                if step.channel and isinstance(before.get('configuration'), dict):
                    channels = before['configuration'].get('channels')
                    if isinstance(channels, dict):
                        candidate = channels.get(step.channel)
                        if isinstance(candidate, dict):
                            identity = candidate
                trace['commits'].append({
                    'step': k, 'effect_id': eid, 'g': effect.get('approval_id'),
                    'action_text': effect.get('action_text'), 'action': action,
                    'i': action.get('intent_id') if action else None,
                    'p': action.get('policy_revision') if action else None,
                    'channel': step.channel, 'identity': identity})
                events.append({'kind': 'commit', 'effect_id': eid,
                               'approval_id': effect.get('approval_id')})
        else:
            if after is not None and after['effects']:
                trace['effects_uncertain'].add(k)
                step.notes.append('new vs pre-existing effects not separable (before-state unobserved)')
            elif after is None:
                trace['effects_uncertain'].add(k)
                step.notes.append('after-state unobserved; effect outcome unknown for this step')
            else:
                step.notes.append('after-state effect set empty; no commit derivable at this step')
        # Approve events: success response AND persisted row, content from bytes.
        if op_known and op == 'approve':
            approval_id = None
            if response and isinstance(response.get('approval_id'), str):
                approval_id = response['approval_id']
            elif isinstance(request.get('approval_id'), str):
                approval_id = request['approval_id']
            if status == 'approved':
                if not process_ok:
                    trace['approve_unconfirmed'].append({'step': k, 'g': approval_id})
                    step.notes.append('approve success with anomalous process exit/streams; not confirmed')
                elif after is None:
                    trace['approve_unconfirmed'].append({'step': k, 'g': approval_id})
                    step.notes.append('approve persisted state unobserved; approve event not derivable')
                else:
                    row = after['approvals'].get(approval_id) if approval_id is not None else None
                    if row is None:
                        trace['approve_contradicted'].append({'step': k, 'g': approval_id})
                        step.notes.append('approve response claims success but no approval row persisted')
                    else:
                        action = _parse_action_text(row.get('action'))
                        digest_match = None
                        if isinstance(response.get('digest'), str) and isinstance(row.get('digest'), str):
                            digest_match = (response['digest'] == row['digest'])
                        request_match = None
                        if isinstance(request.get('action'), dict) and isinstance(row.get('action'), str):
                            request_match = (_canonical_json(request['action']) == row['action'])
                        inconsistent = (digest_match is False) or (request_match is False)
                        requested_id = request.get('approval_id')
                        if isinstance(requested_id, str) and approval_id is not None and requested_id != approval_id:
                            inconsistent = True
                            step.notes.append('approve response approval_id differs from request approval_id')
                        trace['approves'].append({
                            'step': k, 'g': approval_id, 'action_text': row.get('action'),
                            'action': action,
                            'p': action.get('policy_revision') if action else None,
                            'approver_channel': row.get('approver_channel'),
                            'digest_match': digest_match, 'request_match': request_match,
                            'inconsistent': inconsistent})
                        events.append({'kind': 'approve', 'approval_id': approval_id})
                        if inconsistent:
                            step.notes.append('approve response/persisted-row inconsistency (digest or request binding)')
        # Revoke events: success response AND persisted revoked state.
        if op_known and op == 'revoke':
            approval_id = request.get('approval_id') if isinstance(request.get('approval_id'), str) else None
            if status == 'updated':
                if not process_ok:
                    trace['revokes_unconfirmed'].append({'step': k, 'g': approval_id, 'contradicted': False})
                    step.notes.append('revoke success with anomalous process exit/streams; not confirmed')
                elif after is None:
                    trace['revokes_unconfirmed'].append({'step': k, 'g': approval_id, 'contradicted': False})
                    step.notes.append('revoke persisted state unobserved; effectiveness unknown')
                else:
                    row = after['approvals'].get(approval_id) if approval_id is not None else None
                    if row is not None and row.get('state') == 'revoked':
                        trace['revokes_effective'].append({'step': k, 'g': approval_id})
                        events.append({'kind': 'revoke_effective', 'approval_id': approval_id})
                    else:
                        state_desc = None if row is None else row.get('state')
                        trace['revokes_unconfirmed'].append({'step': k, 'g': approval_id, 'contradicted': True})
                        trace['anomalies'].append('step ' + str(k) + ': revoke response success but persisted state is ' + repr(state_desc))
        # Policy updates are recorded only; revisions are read from snapshots.
        if op_known and op == 'policy' and status == 'updated':
            events.append({'kind': 'policy_update', 'revision': request.get('revision')})
        if op_known and op == 'initialize' and status == 'initialized':
            if (after is not None and isinstance(request.get('configuration'), dict)
                    and isinstance(after.get('configuration'), dict)
                    and request['configuration'] != after['configuration']):
                raise OracleError('initialization_configuration_mismatch',
                                  'initialized configuration differs from persisted configuration')
            if after is not None and after['effects']:
                raise OracleError('initialization_effects_nonempty',
                                  'after-initialize snapshot contradicts fresh-store initialization')
            events.append({'kind': 'initialize'})
        intent = None
        if request and isinstance(request.get('action'), dict):
            candidate = request['action'].get('intent_id')
            if isinstance(candidate, str):
                intent = candidate
        echoed = response.get('approval_id') if response else None
        trace['steps'].append({
            'step': k, 'channel': step.channel, 'fault': step.fault,
            'exit': step.exit_code, 'streams_closed': step.streams_closed,
            'process_ok': process_ok,
            'op': op if op_known else None, 'op_known': op_known,
            'request_error': step.request_error, 'status': status,
            'reason': response.get('reason') if response and isinstance(response.get('reason'), str) else None,
            'approval_id': echoed if isinstance(echoed, str) else None,
            'digest': response.get('digest') if response and isinstance(response.get('digest'), str) else None,
            'intent': intent, 'events': events,
            'missing': list(step.missing), 'notes': list(step.notes)})
        prev_after = (after, after_label) if after is not None else None
    # A terminal capture is evidence in its own right, even when the last
    # command already has an explicit after-state. Never silently discard it.
    terminal = extras.get('terminal')
    if isinstance(terminal, dict):
        extra_ids = set(terminal['effects']) - set(seen_effects)
        if extra_ids:
            trace['holes'].append(str(steps[-1].num) + ':terminal_unattributed_effects')
        observe_persistence(terminal, 'terminal.observation.json')
    trace['snapshots'].sort(key=lambda item: item[0])
    return trace


# ---------------------------------------------------------------- evaluator

def _config_at(trace: Dict[str, Any], bound: Tuple[int, int]) -> Tuple[Optional[Dict[str, Any]], Optional[Tuple[int, int]]]:
    '''Purpose: latest observed configuration at or before a position.
    Output: (configuration, position) or (None, None). Side effects: none.'''
    for pos, snapshot in reversed(trace['snapshots']):
        if pos > bound:
            continue
        config = snapshot.get('configuration')
        if isinstance(config, dict):
            return config, pos
    return None, None


def _lookup_row(trace: Dict[str, Any], bound: Tuple[int, int], approval_id: Optional[str]) -> Tuple[Optional[Dict[str, Any]], Optional[Tuple[int, int]]]:
    '''Purpose: most recent persisted approval row for an id at or before a
    position. Output: (row, position) or (None, None). Side effects: none.'''
    if approval_id is None:
        return None, None
    for pos, snapshot in reversed(trace['snapshots']):
        if pos > bound:
            continue
        row = snapshot.get('approvals', {}).get(approval_id)
        if row is not None:
            return row, pos
    return None, None


def _states_before(trace: Dict[str, Any], bound: Tuple[int, int], approval_id: Optional[str]) -> List[Tuple[Tuple[int, int], Any]]:
    '''Purpose: every observed state of an approval row at or before a
    position (for revoked/consumed evidence). Side effects: none.'''
    out: List[Tuple[Tuple[int, int], Any]] = []
    if approval_id is None:
        return out
    for pos, snapshot in trace['snapshots']:
        if pos > bound:
            continue
        row = snapshot.get('approvals', {}).get(approval_id)
        if row is not None:
            out.append((pos, row.get('state')))
    return out


def evaluate_case(trace: Dict[str, Any], shape_errors: Tuple[str, ...] = ()) -> Dict[str, Any]:
    '''Evaluator: pure function from the derived trace to per-property verdicts.

    Purpose: evaluate PT, PA, PR per commit event; PV1/PV2 projections per
    apply response; PL per case; aggregate with fail > error > inconc > pass so
    an earlier fail is never overwritten by later incomplete events.
    Inputs: trace from derive_trace, optional shape-error notes.
    Output: {'properties': {name: {'verdict','coverage','instances',
    'witnesses','notes'}}, 'case_notes': [str]}.
    Errors: none raised; unexpected shapes degrade to inconc/error instances.
    Side effects: none. Limitations: past includes the current position; trace
    holes force inconc on all trace properties; unprojected PV1/PV2 stay inconc
    with an explicit note; PL is timing-bounded and stays inconc with withdrawal.'''
    props: Dict[str, Dict[str, Any]] = {name: {'instances': [], 'witnesses': [], 'notes': []}
                                       for name in ALL_PROPERTIES}
    case_notes: List[str] = []

    def add(prop, verdict, step=None, note=None, witness=None):
        instance = {'verdict': verdict}
        if step is not None:
            instance['step'] = step
        if note:
            instance['note'] = note
        props[prop]['instances'].append(instance)
        if witness is not None:
            props[prop]['witnesses'].append(witness)

    for message in shape_errors:
        for prop in TRACE_PROPERTIES + ('PL',):
            add(prop, 'error', note='evidence shape problem: ' + message)
    if trace['holes']:
        holes_txt = ', '.join(trace['holes'])
        message = ('trace hole(s): ' + holes_txt +
                   '; commitment history incomplete, affected properties cannot be asserted')
        for prop in ('PT', 'PA', 'PR', 'PV2_intention', 'PV2_attempt'):
            add(prop, 'inconc', note=message)
    if trace.get('persistence_conflicts'):
        persistence_message = ('effect persistence conflict(s): '
                               + '; '.join(trace['persistence_conflicts'])
                               + '; snapshots disagree about the same effect identifiers; '
                                 'affected properties cannot rest on this persistence')
        for prop in TRACE_PROPERTIES:
            add(prop, 'error', note=persistence_message)
    commits = trace['commits']
    groups = {}
    for commit in commits:
        groups.setdefault((commit['step'], commit['g']), []).append(commit)
    for (step, grant), members in groups.items():
        if len(members) > 1:
            add('PA', 'fail', step=step,
                witness={'property':'PA', 'approval_id':grant,
                         'effect_ids':[item['effect_id'] for item in members],
                         'summary':'multiple same-grant commitments in one observation interval; at least one violates Used under every ordering'})
    for commit in commits:
        k = commit['step']
        approval_id = commit['g']
        action = commit['action']
        text = commit['action_text']
        bound = (k, 0)
        derived = [e for e in trace['approves'] if e['step'] <= k and e['g'] == approval_id]
        clean_match = any((e.get('action_text') == text) and not e.get('inconsistent') for e in derived)
        clean_mismatch = any((e.get('action_text') != text) and not e.get('inconsistent') for e in derived)
        row, row_pos = _lookup_row(trace, bound, approval_id)
        # ---- PT: commit(a,g,i,p,x) -> past approve(g,p) with content binding.
        if not isinstance(action, dict):
            add('PT', 'inconc', step=k, note='committed parameters cannot be resolved from canonical bytes')
        elif clean_match:
            add('PT', 'pass', step=k)
        elif clean_mismatch:
            add('PT', 'fail', step=k,
                note='committed canonical action differs from the clean approval recorded for this approval id',
                witness={'property': 'PT',
                         'summary': 'PT: commit step ' + str(k) + ' approval ' + repr(approval_id) + ': committed action differs from approved action',
                         'commit_step': k, 'approval_id': approval_id, 'intent': commit['i'],
                         'committed_action': text,
                         'approved_action_variants': sorted({e.get('action_text') or '' for e in derived})})
        elif any(u.get('g') == approval_id and u.get('step', 0) <= k for u in trace['approve_contradicted']):
            add('PT', 'error', step=k,
                note='approve response claimed success without a persisted approval row')
        elif derived:
            add('PT', 'inconc', step=k,
                note='approval record integrity doubt (response/snapshot inconsistency)')
        elif any(u.get('g') == approval_id and u.get('step', 0) <= k for u in trace['approve_unconfirmed']):
            add('PT', 'inconc', step=k,
                note='approve success response not corroborated by a persisted snapshot')
        elif row is not None and row.get('action') == text and row.get('state') in APPROVAL_ACTIVE_STATES:
            add('PT', 'inconc', step=k,
                note='approval persisted before the commit but its approve event is not derivable from responses; conservative inconc')
        else:
            observed = [{'at': _pos_name(pos), 'state': state_row}
                        for pos, state_row in _states_before(trace, bound, approval_id)]
            prefix_holes = [hole for hole in trace['holes']
                            if (int(hole.split(':', 1)[0]) < k
                                or (int(hole.split(':', 1)[0]) == k
                                    and hole.split(':', 1)[1] == 'before'))]
            if prefix_holes or shape_errors:
                add('PT', 'inconc', step=k,
                    note='no approving evidence observed, but the commit prefix is incomplete ('
                         + ', '.join(prefix_holes + list(shape_errors))
                         + '); absence of an approval is not established')
            else:
                add('PT', 'fail', step=k,
                    note='no approving evidence for this approval id at or before the commit',
                    witness={'property': 'PT',
                             'summary': 'PT: commit step ' + str(k) + ' approval ' + repr(approval_id) + ': no clean approve(g) in past',
                             'commit_step': k, 'approval_id': approval_id, 'intent': commit['i'],
                             'committed_action': text, 'approvals_observed_before_commit': observed})
        # ---- PA: approved at current revision, not revoked, not used, active x.
        config, config_pos = _config_at(trace, bound)
        config_fresh = config_pos is not None and config_pos in ((k, 0), (k - 1, 1))
        components: List[Tuple[str, str, Optional[str]]] = []
        if row is None:
            components.append(('approval_persisted', 'inconc', 'no approval row observed up to the commit'))
        else:
            state = row.get('state')
            if state in APPROVAL_ACTIVE_STATES:
                components.append(('approval_state', 'pass', None))
            elif state == 'revoked':
                components.append(('approval_state', 'fail', 'approval already revoked at or before commit'))
            else:
                components.append(('approval_state', 'inconc', 'unexpected approval state ' + repr(state)))
            if config is None:
                components.append(('approver_role', 'inconc', 'configuration unobserved'))
            else:
                channels = config.get('channels') if isinstance(config.get('channels'), dict) else {}
                approver_ident = channels.get(row.get('approver_channel'))
                if not isinstance(approver_ident, dict) or approver_ident.get('role') != 'approver':
                    components.append(('approver_role', 'fail',
                                       'approver channel ' + repr(row.get('approver_channel')) + ' not recorded as approver in configuration'))
                else:
                    components.append(('approver_role', 'pass', None))
            commit_revision = action.get('policy_revision') if isinstance(action, dict) else None
            row_action = _parse_action_text(row.get('action'))
            row_revision = row_action.get('policy_revision') if isinstance(row_action, dict) else None
            config_revision = config.get('policy_revision') if isinstance(config, dict) else None
            if commit_revision is None or row_revision is None:
                components.append(('revision_current', 'inconc', 'policy revision not resolvable from stored bytes'))
            elif row_revision != commit_revision:
                components.append(('revision_current', 'fail', 'approval revision differs from committed action revision'))
            elif config_revision is None:
                components.append(('revision_current', 'inconc', 'configuration policy revision unobserved'))
            elif commit_revision != config_revision:
                if config_fresh:
                    components.append(('revision_current', 'fail', 'committed revision differs from current configuration revision'))
                else:
                    components.append(('revision_current', 'inconc', 'revision differs from a stale observed configuration'))
            else:
                components.append(('revision_current', 'pass', None))
        if (any(r.get('g') == approval_id and r['step'] <= k for r in trace['revokes_effective'])
                or any(state == 'revoked' for _pos, state in _states_before(trace, bound, approval_id))):
            components.append(('not_revoked_in_history', 'fail', 'effective withdrawal already occurred'))
        earlier = [x for x in commits if x.get('g') == approval_id and x['step'] < k]
        if earlier:
            earlier_step = earlier[0]['step']
            components.append(('not_used_before', 'fail', 'approval already committed at step ' + str(earlier_step)))
        elif len(groups[(k, approval_id)]) > 1:
            components.append(('not_used_before', 'inconc', 'exact order inside the commitment group is unobserved; group violation retained separately'))
        else:
            consumed = [pos for pos, state in _states_before(trace, bound, approval_id) if state == 'consumed']
            if consumed:
                components.append(('not_used_before', 'fail',
                                   'approval already consumed at position ' + _pos_name(consumed[0])))
            else:
                components.append(('not_used_before', 'pass', None))
        channel = commit.get('channel')
        if channel is None:
            components.append(('actif_channel', 'inconc', 'executing channel not recorded in process argv'))
        elif config is None:
            components.append(('actif_channel', 'inconc', 'configuration unobserved at commit'))
        else:
            channels = config.get('channels') if isinstance(config.get('channels'), dict) else {}
            identity = channels.get(channel)
            if not isinstance(identity, dict):
                components.append(('actif_channel', 'fail',
                                   'executing channel ' + repr(channel) + ' absent from configuration (A0 projection)'))
            elif not isinstance(action, dict):
                components.append(('actif_channel', 'inconc', 'committed action bytes unparseable; identity binding not checkable'))
            else:
                mismatch = None
                if identity.get('role') != 'requester':
                    role_value = identity.get('role')
                    mismatch = 'channel role ' + repr(role_value) + ' is not requester'
                elif identity.get('actor') != action.get('requester_id'):
                    mismatch = 'channel actor differs from action requester_id'
                elif identity.get('run') != action.get('run_id'):
                    mismatch = 'channel run differs from action run_id'
                elif identity.get('scope') != action.get('scope_id'):
                    mismatch = 'channel scope differs from action scope_id'
                if mismatch:
                    components.append(('actif_channel', 'fail', mismatch))
                else:
                    components.append(('actif_channel', 'pass', None))
        failures = [c for c in components if c[1] == 'fail']
        unknowns = [c for c in components if c[1] == 'inconc']
        if failures:
            names = ', '.join(n for n, _v, _d in failures)
            add('PA', 'fail', step=k,
                note='; '.join(n + ': ' + d for n, _v, d in failures if d),
                witness={'property': 'PA',
                         'summary': 'PA: commit step ' + str(k) + ' approval ' + repr(approval_id) + ': failed components ' + names,
                         'commit_step': k, 'approval_id': approval_id,
                         'components': [list(c) for c in components]})
        elif unknowns:
            add('PA', 'inconc', step=k,
                note='; '.join(n + ': ' + d for n, _v, d in unknowns if d))
        else:
            add('PA', 'pass', step=k)
        # ---- PR: no past effective revocation of g.
        effective = [r for r in trace['revokes_effective'] if r.get('g') == approval_id and r['step'] <= k]
        revoked_seen = [pos for pos, state in _states_before(trace, bound, approval_id) if state == 'revoked']
        unconfirmed = [r for r in trace['revokes_unconfirmed']
                       if r.get('g') == approval_id and r['step'] <= k and not r.get('contradicted')]
        if effective or revoked_seen:
            add('PR', 'fail', step=k,
                witness={'property': 'PR',
                         'summary': 'PR: commit step ' + str(k) + ' approval ' + repr(approval_id) + ': revocation before commit',
                         'commit_step': k, 'approval_id': approval_id,
                         'effective_revoke_steps': [r['step'] for r in effective],
                         'revoked_state_observed_at': [_pos_name(pos) for pos in revoked_seen]})
        elif unconfirmed:
            add('PR', 'inconc', step=k,
                note='revoke success response not corroborated by a persisted snapshot')
        else:
            add('PR', 'pass', step=k)
    # ---- PV1/PV2 projections per apply response.
    for record in trace['steps']:
        k = record['step']
        status = record['status']
        if status not in ('committed', 'refused', 'unknown'):
            continue
        if status == 'unknown':
            case_notes.append('step ' + str(k) + ": response status 'unknown' triggers neither PV1 nor PV2 (contract); "
                              'coverage annotated, no verdict instance, and no claim about recovery quality')
            continue
        if not record['process_ok']:
            for prop in ('PV1_intention', 'PV1_attempt', 'PV2_intention', 'PV2_attempt'):
                add(prop, 'inconc', step=k,
                    note='process anomaly (exit=' + repr(record['exit']) + ', streams_closed=' + repr(record['streams_closed']) + '); response reliability unclear')
            continue
        if status == 'committed':
            if not record['op_known']:
                add('PV1_intention', 'inconc', step=k, note='request bytes unparseable; intent not resolvable')
            elif record['op'] != 'apply':
                add('PV1_intention', 'inconc', step=k,
                    note='committed response to op ' + repr(record['op']) + '; PV1 not attributed')
            elif record['intent'] is None:
                add('PV1_intention', 'inconc', step=k, note='intent_id not resolvable from raw request')
            elif any(c['i'] == record['intent'] and c['step'] <= k for c in commits):
                add('PV1_intention', 'pass', step=k)
            elif (k in trace['effects_uncertain'] or any(int(h.split(':', 1)[0]) <= k for h in trace['holes'])):
                add('PV1_intention', 'inconc', step=k, note='same-step effect outcome uncertain (snapshot hole)')
            else:
                add('PV1_intention', 'fail', step=k,
                    witness={'property': 'PV1_intention',
                             'summary': 'PV1_intention: committed response step ' + str(k) + ' without past same-intent commit',
                             'response_step': k, 'intent': record['intent'],
                             'commits_in_past': [{'step': c['step'], 'approval_id': c['g'], 'intent': c['i']}
                                                 for c in commits if c['step'] <= k]})
            same = [c for c in commits if c['step'] == k]
            if same:
                add('PV1_attempt', 'pass', step=k)
            elif (k in trace['effects_uncertain'] or any(int(h.split(':', 1)[0]) <= k for h in trace['holes'])):
                add('PV1_attempt', 'inconc', step=k, note='same-step effect observation incomplete')
            else:
                add('PV1_attempt', 'fail', step=k,
                    witness={'property':'PV1_attempt', 'response_step':k,
                             'summary':'applied response without an observed same-attempt commitment'})
        else:  # refused
            if not record['op_known']:
                case_notes.append('step ' + str(k) + ': refusal not attributed; no identifier invented')
                for prop in ('PV2_intention', 'PV2_attempt'):
                    add(prop, 'inconc', step=k, note='request unparseable; response domain unestablished')
            elif record['op'] != 'apply':
                case_notes.append('step ' + str(k) + ': refusal to op ' + repr(record['op']) +
                                  ' is not an application refusal; excluded from PV2')
            else:
                intent = record['intent']
                if intent is None:
                    add('PV2_intention', 'inconc', step=k,
                        note='refused apply: intent_id not resolvable from raw request; not invented')
                elif any(c['i'] == intent and c['step'] <= k for c in commits):
                    past = min((c for c in commits if c['i'] == intent and c['step'] <= k), key=lambda c: c['step'])
                    past_step = past['step']
                    add('PV2_intention', 'fail', step=k,
                        witness={'property': 'PV2_intention',
                                 'summary': 'PV2_intention: refusal step ' + str(k) + ' intent ' + repr(intent) + ' follows commit step ' + str(past_step),
                                 'refused_step': k, 'intent': intent,
                                 'past_commit': {'step': past['step'], 'approval_id': past['g'], 'effect_id': past['effect_id']}})
                else:
                    add('PV2_intention', 'pass', step=k)
                same_step = [c for c in commits if c['step'] == k]
                if same_step:
                    add('PV2_attempt', 'fail', step=k,
                        witness={'property': 'PV2_attempt',
                                 'summary': 'PV2_attempt: refusal step ' + str(k) + ' but new effect(s) at the same step',
                                 'refused_step': k,
                                 'new_effects': [{'effect_id': c['effect_id'], 'approval_id': c['g']} for c in same_step]})
                else:
                    add('PV2_attempt', 'pass', step=k)
    # ---- PL: withdrawal present -> inconc (Delta unspecified); else vacuous.
    withdrawals = [r['step'] for r in trace['steps']
                   if (r['op_known'] and r['op'] == 'revoke')
                   or (r['status'] == 'updated' and not r['op_known'])]
    if withdrawals:
        add('PL', 'inconc', note='withdrawal request(s) at steps ' + repr(withdrawals) + ': ' + PL_INCONC_NOTE)
    else:
        props['PL']['notes'].append(PL_VACUOUS_NOTE)
    for prop in ('PV1', 'PV2'):
        props[prop]['instances'] = [{'verdict': 'inconc', 'note': DEFAULT_PROJECTION_NOTE}]
    out: Dict[str, Any] = {}
    for prop in ALL_PROPERTIES:
        instances = props[prop]['instances']
        if instances:
            verdict = max((i['verdict'] for i in instances), key=lambda v: SEVERITY[v])
            coverage = 'triggered'
        else:
            verdict, coverage = 'pass', 'vacuous'
        entry = {'verdict': verdict, 'coverage': coverage, 'instances': instances,
                 'witnesses': props[prop]['witnesses'], 'notes': list(props[prop]['notes'])}
        if prop in ('PV1', 'PV2'):
            entry['coverage'] = 'n/a'
        elif coverage == 'vacuous':
            entry['notes'].append('vacuous: no triggering instance in this case')
        out[prop] = entry
    return {'properties': out, 'case_notes': case_notes}


# ------------------------------------------------------------ report assembly

def case_error_row(row: Dict[str, Any], error: OracleError) -> Dict[str, Any]:
    '''Purpose: build a fully non-PASS report row when a case cannot be
    evaluated (missing directory, structural impossibility, internal error).
    Output: report row with error verdicts and the safe diagnostic. Side
    effects: none.'''
    diagnostic = error.reason + ': ' + error.detail
    properties: Dict[str, Any] = {}
    for prop in TRACE_PROPERTIES + ('PL',):
        properties[prop] = {'verdict': 'error', 'coverage': 'n/a',
                            'instances': [{'verdict': 'error', 'note': diagnostic}],
                            'witnesses': [], 'notes': []}
    for prop in ('PV1', 'PV2'):
        properties[prop] = {'verdict': 'inconc', 'coverage': 'n/a',
                            'instances': [{'verdict': 'inconc', 'note': DEFAULT_PROJECTION_NOTE}],
                            'witnesses': [], 'notes': []}
    return {'case': row['case'], 'historical_result_recorded': row.get('result'),
            'historical_row': row, 'input_digests': {}, 'trace': {},
            'case_notes': ['case not evaluated: ' + diagnostic],
            'properties': properties, 'witness_summaries': []}


def evaluate_case_row(row: Dict[str, Any], files_map: Dict[str, str], case_entry: Dict[str, Any]) -> Dict[str, Any]:
    '''Purpose: evaluate one historical row from validated evidence.
    Inputs: result row, validated file map, discovered case entry.
    Output: report row (digests, trace, verdicts, witnesses, notes).
    Errors: OracleError(no_steps_found) and any propagation from loaders; the
    caller converts failures into case_error_row. Side effects: none.
    Limitations: verdicts are evidence-bounded per ASSUMPTIONS.'''
    steps, shape_notes = build_steps(files_map, case_entry)
    extras_data, extras_digests, extra_notes = load_extras(files_map, case_entry.get('extras', {}))
    shape_notes.extend(extra_notes)
    if not steps:
        raise OracleError('no_steps_found', row['case'])
    trace = derive_trace(steps, extras_data)
    evaluation = evaluate_case(trace, tuple(shape_notes))
    digests: Dict[str, str] = {}
    for step in steps:
        digests.update(step.digests)
    digests.update(extras_digests)
    trace_public = {
        'steps': trace['steps'],
        'initialization_case_run': trace['initialization_case_run'],
        'commits': [{'step': c['step'], 'effect_id': c['effect_id'], 'approval_id': c['g'],
                     'intent': c['i'], 'channel': c['channel']} for c in trace['commits']],
        'approves': [{'step': e['step'], 'approval_id': e['g'], 'inconsistent': e['inconsistent']}
                     for e in trace['approves']],
        'approve_unconfirmed': trace['approve_unconfirmed'],
        'approve_contradicted': trace['approve_contradicted'],
        'revokes_effective': trace['revokes_effective'],
        'revokes_unconfirmed': trace['revokes_unconfirmed'],
        'holes': trace['holes'], 'anomalies': trace['anomalies'],
        'persistence_conflicts': trace['persistence_conflicts'],
    }
    witnesses: List[Dict[str, Any]] = []
    for prop in TRACE_PROPERTIES:
        witnesses.extend(evaluation['properties'][prop]['witnesses'])
    case_notes = list(evaluation['case_notes'])
    if trace['holes']:
        case_notes.append('trace holes at: ' + ', '.join(trace['holes']))
    if trace['anomalies']:
        case_notes.extend('anomaly: ' + a for a in trace['anomalies'])
    return {'case': row['case'], 'historical_result_recorded': row.get('result'),
            'historical_row': row, 'input_digests': digests, 'trace': trace_public,
            'case_notes': case_notes, 'properties': evaluation['properties'],
            'witness_summaries': [w.get('summary', '') for w in witnesses]}


def build_report(source: Dict[str, Any], rows: List[Dict[str, Any]]) -> Dict[str, Any]:
    '''Purpose: assemble the full JSON report. Inputs: source hashes/metadata,
    evaluated rows. Output: report dict. Errors: none. Side effects: none.'''
    counts = {name: {'pass': 0, 'inconc': 0, 'error': 0, 'fail': 0} for name in ALL_PROPERTIES}
    for row in rows:
        for name, entry in row['properties'].items():
            counts[name][entry['verdict']] = counts[name].get(entry['verdict'], 0) + 1
    return {'tool': TOOL_NAME, 'version': TOOL_VERSION,
            'status': 'evaluated_with_errors' if any(c['error'] for c in counts.values()) else 'evaluated',
            'independent_acceptance': 'not established by this tool',
            'source': source, 'verdict_model': {
                'pass': 'property holds over the whole case trace',
                'fail': 'finite violation witness retained',
                'inconc': 'evidence does not allow a verdict (never upgraded to pass)',
                'error': 'evidence or oracle integrity problem at this granularity',
                'coverage': 'triggered vs vacuous, annotated separately from the verdict',
                'aggregation': 'fail > error > inconc > pass; an earlier fail is never overwritten'},
            'assumptions': list(ASSUMPTIONS), 'rows': rows,
            'summary': {'cases': len(rows), 'properties': counts}}


def _md_cell(text: Any) -> str:
    '''Purpose: make a text safe for one Markdown table cell.
    Side effects: none.'''
    return str(text).replace('|', r'\|').replace(chr(10), ' ')


def render_markdown(report: Dict[str, Any]) -> str:
    '''Purpose: render the audit table (one row per historical result row, 83
    rows for A083) plus counts and assumptions. Output: Markdown text.
    Errors: none. Side effects: none.'''
    props_order = ['PT', 'PA', 'PR', 'PV1_intention', 'PV1_attempt',
                   'PV2_intention', 'PV2_attempt', 'PV1', 'PV2', 'PL']
    heads = ['case', 'recorded (not a verdict)'] + props_order + ['witnesses / notes']
    source = report['source']
    lines = ['# A0 property oracle - per-case results', '',
             'source location: `' + source['location'] + '`',
             'inventory sha256 (validated): `' + source['inventory_sha256_recorded'] + '`',
             'historical rows: ' + str(len(report['rows'])) +
             ' (statuses recorded, never used as verdicts)', '',
             'Verdicts: pass / FAIL / inconc / error; `pass (vacuous)` marks properties with no triggering '
             'instance; coverage is annotated separately from verdicts. See a0_oracle_report.json for traces, '
             'digests and witnesses.', '']
    lines.append('| ' + ' | '.join(heads) + ' |')
    lines.append('|' + ' --- |' * len(heads))

    def cell(entry):
        verdict = entry['verdict']
        label = 'FAIL' if verdict == 'fail' else verdict
        if verdict == 'pass' and entry['coverage'] == 'vacuous':
            label = 'pass (vacuous)'
        return label

    for row in report['rows']:
        props = row['properties']
        extras = list(row.get('witness_summaries') or [])
        if row.get('case_notes'):
            extras.append(str(len(row['case_notes'])) + ' detailed note(s) in the JSON report')
        cells = ([row['case'], str(row.get('historical_result_recorded'))]
                 + [cell(props[name]) for name in props_order]
                 + ['; '.join(extras) if extras else '-'])
        lines.append('| ' + ' | '.join(_md_cell(c) for c in cells) + ' |')
    lines.append('')
    lines.append('## Counts')
    lines.append('')
    lines.append('| property | pass | inconc | error | FAIL |')
    lines.append('| --- | --- | --- | --- | --- |')
    counts = report['summary']['properties']
    for name in props_order:
        c_pass = counts[name]['pass']
        c_inc = counts[name]['inconc']
        c_err = counts[name]['error']
        c_fail = counts[name]['fail']
        lines.append('| ' + name + ' | ' + str(c_pass) + ' | ' + str(c_inc) + ' | ' + str(c_err) + ' | ' + str(c_fail) + ' |')
    lines.append('')
    lines.append('## Assumptions and limits')
    lines.append('')
    for assumption in report['assumptions']:
        lines.append('- ' + assumption)
    lines.append('')
    lines.append('Evaluation executed on the source bytes recorded above. Independent scientific acceptance '
                 'is not established by this tool; vacuous and inconclusive verdicts retain their limits.')
    return chr(10).join(lines) + chr(10)


def write_output(output_dir: str, report: Dict[str, Any], markdown: str) -> None:
    """Reserve a new directory exclusively and publish complete files in it.

    Existing paths are never replaced. Each artifact is written to a fresh
    sibling file, synchronized, then hard-linked without overwrite. The
    completion marker binds the published artifact hashes only; its presence
    never proves that the process returned success. On error the original
    cause and partial output are retained: if the failure follows the marker
    link (final directory fsync), the marker may exist and publication is
    reported uncertain; no rollback or blanket unlink is performed.
    This is local artifact publication, not permission to publish externally.
    """
    if os.path.lexists(output_dir):
        raise OracleError('output_exists', output_dir)
    parent = os.path.dirname(os.path.abspath(output_dir))
    if not os.path.isdir(parent):
        raise OracleError('output_parent_missing', parent)
    try:
        os.mkdir(output_dir, mode=0o700)
    except FileExistsError as error:
        raise OracleError('output_exists', output_dir) from error
    marker_published = False
    def publish(name, payload):
        """Synchronize complete bytes and create a non-overwriting artifact link.

        Return its SHA-256; propagate IO errors and retain temporary bytes.
        """
        nonlocal marker_published
        fd, temporary = tempfile.mkstemp(prefix='.' + name + '-', dir=output_dir)
        with os.fdopen(fd, 'wb') as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, os.path.join(output_dir, name))
        if name == 'COMPLETE.json':
            marker_published = True
        os.unlink(temporary)
        return hashlib.sha256(payload).hexdigest()
    try:
        hashes = {
            REPORT_NAME: publish(REPORT_NAME, json.dumps(report, indent=2, ensure_ascii=False).encode('utf-8') + b'\n'),
            TABLE_NAME: publish(TABLE_NAME, markdown.encode('utf-8')),
        }
        marker = {'state': 'complete', 'sha256': hashes,
                  'verification': 'binds the published artifact hashes only; confirming a '
                                  'successful run also requires the retained successful '
                                  'process exit (terminal receipt)'}
        publish('COMPLETE.json', json.dumps(marker, indent=2).encode('utf-8') + b'\n')
        marker_published = True
        fd = os.open(output_dir, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except OSError as error:
        if marker_published:
            raise OracleError('output_publication_uncertain',
                              'COMPLETE.json was already published when a later publication '
                              'step failed (' + (error.strerror or type(error).__name__)
                              + '); the marker and partial artifacts are retained; the marker '
                                'alone does not prove a successful run') from error
        raise OracleError('output_publication_failed', error.strerror or type(error).__name__) from error


def run_oracle(index_path: str, output_dir: str) -> Dict[str, Any]:
    '''Programmatic entry point.

    Purpose: validate index, inventory and every file digest; evaluate all
    historical rows; publish the JSON report and table as complete files with a completion marker.
    Inputs: index path (A0-PREUVES.json), non-existing output directory.
    Output: the report dict. Side effects: creates the output directory and
    its artifacts and completion marker; partial output is retained on IO
    failure, and a failure after the marker link is reported as uncertain
    publication with the original OSError cause.
    Errors: OracleError with safe diagnostics for every refusal; no output is
    written when validation fails. Limitations: single-threaded, offline,
    evidence-bounded; historical statuses are never verdicts.'''
    try:
        index_raw = _read_bytes(index_path)
    except OSError as error:
        raise OracleError('index_unreadable', str(error)) from error
    index_sha = hashlib.sha256(index_raw).hexdigest()
    index = load_index(index_path, index_raw)
    root_real = os.path.realpath(index['location'])
    entries = load_manifest_entries(index, root_real)
    files = validate_inventory(root_real, entries)
    rows, result_rel = load_result_rows(files, index)
    journey = index.get('journey_path')
    cases = discover_cases(files, journey)
    evaluated: List[Dict[str, Any]] = []
    for row in rows:
        entry = cases.get(row['case'])
        if entry is None:
            missing = case_error_row(row, OracleError('case_directory_not_found', row['case']))
            for prop, item in missing['properties'].items():
                item['verdict'] = 'inconc'
                item['coverage'] = 'unobserved'
                item['instances'] = [{'verdict':'inconc', 'note':'historical row has no attributed event trace'}]
            missing['case_kind'] = 'unattributed_historical_assertion'
            evaluated.append(missing)
            continue
        try:
            evaluated.append(evaluate_case_row(row, files, entry))
        except OracleError as error:
            evaluated.append(case_error_row(row, error))
        except Exception as error:  # defensive: one broken case never aborts the audit
            evaluated.append(case_error_row(row, OracleError('internal_error', repr(error))))
    source = {'index_path': os.path.abspath(index_path), 'index_sha256': index_sha,
              'location': index['location'], 'location_realpath': root_real,
              'inventory_sha256_recorded': index['inventory_sha256'],
              'inventory_digest_verified': True,
              'inventory_entries_validated': len(files),
              'result_path': result_rel,
              'result_sha256': hashlib.sha256(_read_bytes(files[result_rel])).hexdigest(),
              'journey_path': journey}
    expected = dict(entries)
    for row in evaluated:
        for rel, digest in row['input_digests'].items():
            if expected.get(rel) != digest:
                raise OracleError('consumed_input_drift', rel)
    validate_inventory(root_real, entries)
    source['tool_sha256'] = sha256_file(__file__)
    source['final_inventory_revalidated'] = True
    report = build_report(source, evaluated)
    write_output(output_dir, report, render_markdown(report))
    return report


def main(argv: Optional[List[str]] = None) -> int:
    '''CLI entry point.

    Purpose: parse --index/--output, run the oracle, print a one-line summary.
    Output: exit code 0 on success, 2 on refusal with a safe diagnostic on
    stderr. Side effects: see run_oracle. Errors: OracleError handled here;
    argparse errors exit 2 via argparse itself. Limitations: no retries, no
    watchdogs, no network, no subprocess, no sqlite3 import anywhere.'''
    parser = argparse.ArgumentParser(
        prog=TOOL_NAME,
        description='Offline A0 property oracle over retained A083 evidence (read-only; stdlib only).')
    parser.add_argument('--index', required=True, help='path to A0-PREUVES.json')
    parser.add_argument('--output', required=True, help='NEW directory reserved exclusively; COMPLETE.json binds the published artifact hashes only and never proves run success by itself')
    args = parser.parse_args(argv)
    try:
        report = run_oracle(args.index, args.output)
    except OracleError as error:
        print(TOOL_NAME + ': refusal: ' + error.reason + ': ' + error.detail, file=sys.stderr)
        return 2
    counts = report['summary']['properties']
    watched = ('PT', 'PA', 'PR', 'PV1_intention', 'PV1_attempt',
               'PV2_intention', 'PV2_attempt', 'PL')
    parts = []
    for name in watched:
        fail_count = counts[name]['fail']
        if fail_count:
            parts.append(name + '=' + str(fail_count))
    tail = ('; FAIL: ' + ', '.join(parts)) if parts else '; no FAIL verdicts'
    case_count = report['summary']['cases']
    print(TOOL_NAME + ': wrote ' + args.output + ' (' + str(case_count) + ' cases)' + tail)
    return 0


if __name__ == '__main__':
    sys.exit(main())
