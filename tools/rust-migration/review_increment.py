"""Check declared migration reviews against exact qualified input snapshots.

This guards coverage and freshness; lexical candidates and human declarations do
not establish semantic correctness, independent review or runtime acceptance.
"""
import argparse
import hashlib
import io
import tarfile
import json
import os
from pathlib import Path
import re
import tempfile

AXES = ('policies', 'functions', 'errors', 'documentation')
SHA = re.compile(r'[0-9a-f]{64}')


def digest(path):
    """Hash actual bytes; propagate IO errors rather than invent an empty input."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def local(root, name):
    """Resolve a relative reference inside root; reject traversal and escaping links."""
    path = Path(name)
    if path.is_absolute() or '..' in path.parts or not path.parts:
        raise ValueError('review-path-invalid')
    result = (root / path).resolve()
    if not result.is_relative_to(root.resolve()):
        raise ValueError('review-path-escape')
    return result


def go_test_inventory(executed, listed):
    """Bind successful named top-level Go tests to a nonempty listed inventory.

    Empty/skip/fail/cached, duplicate or mismatched results are incomplete evidence.
    This checks original text only, without executing/retrying or inferring review.
    """
    from collections import Counter
    actual=Counter(line.split()[2] for line in executed.splitlines() if line.startswith('--- PASS: '))
    expected=Counter(line for line in listed.splitlines() if line.startswith('Test'))
    if (not actual or actual!=expected or any(count!=1 for count in actual.values())
            or '(cached)' in executed or '--- FAIL:' in executed or '--- SKIP:' in executed):
        raise ValueError('go-test-inventory-incomplete')
    return sorted(actual)


def retained_parity(root, reference):
    """Resolve an explicit retained parity result without a guessed folder name.

    Require a versioned exact result hash and an in-root non-traversing path;
    unknown fields, missing/moved/tampered results and escaping links refuse.
    This reads data only: no command, identity or independent approval is inferred.
    """
    if (not isinstance(reference, dict) or set(reference) != {'schema_version', 'path', 'result_sha256'}
            or type(reference.get('schema_version')) is not int or reference['schema_version'] != 1
            or not isinstance(reference.get('path'), str) or not reference['path']
            or not isinstance(reference.get('result_sha256'), str)
            or not SHA.fullmatch(reference['result_sha256'])):
        raise ValueError('parity-reference-invalid')
    folder = local(root, reference['path'])
    result = local(root, str(Path(reference['path']) / 'result.json'))
    if not folder.is_dir() or not result.is_file() or digest(result) != reference['result_sha256']:
        raise ValueError('parity-reference-changed-or-missing')
    return folder


def candidates(text):
    """Return lexical prompts with line numbers, never completeness or verdicts.

    Includes comments and test literals intentionally: the reviewer classifies
    protocol, test, technical and operator choices. Multiline/generated syntax
    needs semantic review beyond this search.
    """
    expressions = {
        'policies': r'\b(?:true|false|[0-9]+|timeout|retry|retries|limit|default|mandatory)\b',
        'functions': r'^\s*(?:(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+|func\s+|(?:async\s+)?def\s+|(?:async\s+)?function\s+)',
        'errors': r'(?:\b(?:unwrap|expect|panic|recover|catch|except|ignore|Err|error|fallback)\b|let\s+_\s*=|_\s*=|\?)',
    }
    return {axis: [{'line': n, 'text': line} for n, line in enumerate(text.splitlines(), 1)
                   if re.search(expression, line)] for axis, expression in expressions.items()}


def check(root, baseline_path, current_path, review):
    """Validate coverage, immutable references and all four review axes.

    Snapshots must come from the effective-input collector, not a handwritten
    file list. Local membership changes are compared; external toolchain inputs
    stay the qualification collector's responsibility. Deleted inputs require an
    explicit removal review. A still-existing unselected input requires an
    explicit reason and unchanged original source hash; it is never called deleted.
    No writes, executions or implicit approvals occur.
    Open findings return BLOCKED; malformed/stale declarations raise ValueError.
    """
    if not isinstance(review, dict) or review.get('schema_version') != 1 or not isinstance(review.get('reviewer'), str) or not review['reviewer'].strip():
        raise ValueError('review-header-invalid')
    for key, path in [('baseline_sha256', baseline_path), ('current_sha256', current_path)]:
        if review.get(key) != digest(path):
            raise ValueError('review-snapshot-stale')
    before, after = (json.loads(path.read_bytes()) for path in (baseline_path, current_path))
    for snapshot in (before, after):
        if not isinstance(snapshot, dict) or not snapshot or any(not isinstance(v, str) or not SHA.fullmatch(v) for v in snapshot.values()):
            raise ValueError('review-snapshot-invalid')
    names = {name for name in before.keys() | after.keys()
             if not name.startswith(('external-go/', 'external-rust/')) and before.get(name) != after.get(name)}
    entries = review.get('files')
    if not isinstance(entries, dict) or not names or set(entries) != names:
        raise ValueError('review-coverage-incomplete')
    findings, inventory = [], {}
    for name in sorted(names):
        path, entry = local(root, name), entries[name]
        if not isinstance(entry, dict):
            raise ValueError('review-entry-invalid: ' + name)
        expected = after.get(name)
        retained_unselected = False
        selection = entry.get('selection')
        if expected is None and path.exists():
            if (not isinstance(selection, dict) or selection.get('state') != 'unselected'
                    or not isinstance(selection.get('reason'), str) or not selection['reason'].strip()
                    or selection.get('sha256') != before.get(name)
                    or digest(path) != selection.get('sha256')):
                raise ValueError('review-unselected-input-invalid: ' + name)
            retained_unselected = True
        elif selection is not None:
            raise ValueError('review-selection-invalid: ' + name)
        if entry.get('sha256') != expected or (expected is not None and digest(path) != expected):
            raise ValueError('review-source-stale: ' + name)
        if not isinstance(entry.get('axes'), dict) or set(entry['axes']) != set(AXES):
            raise ValueError('review-axes-incomplete: ' + name)
        for axis, outcome in entry['axes'].items():
            if not isinstance(outcome, dict):
                raise ValueError('review-outcome-invalid: ' + name + ':' + axis)
            state = outcome.get('state')
            if state not in ('reviewed', 'not_applicable', 'open') or (not isinstance(outcome.get('reason'), str) or not outcome['reason'].strip()):
                raise ValueError('review-outcome-invalid: ' + name + ':' + axis)
            references = outcome.get('evidence')
            if not isinstance(references, list) or not references:
                raise ValueError('review-evidence-missing: ' + name + ':' + axis)
            for reference in references:
                if not isinstance(reference, dict) or not isinstance(reference.get('path'), str) or not isinstance(reference.get('sha256'), str) or not SHA.fullmatch(reference['sha256']):
                    raise ValueError('review-evidence-invalid: ' + name + ':' + axis)
                if digest(local(root, reference['path'])) != reference.get('sha256'):
                    raise ValueError('review-evidence-stale: ' + name + ':' + axis)
            if state == 'open':
                if any(not isinstance(outcome.get(field), str) or not outcome[field].strip() for field in ('owner', 'resume_when')):
                    raise ValueError('review-open-followup-missing: ' + name + ':' + axis)
                findings.append({'path': name, 'axis': axis, **outcome})
        if retained_unselected:
            inventory[name] = {'unselected': True, 'retained_sha256': selection['sha256'],
                               'reason': selection['reason']}
        else:
            inventory[name] = candidates(path.read_text(errors='replace')) if expected else {'removed': True}
    return {'coverage_status': 'PASS', 'contract_status': 'BLOCKED' if findings else 'DECLARED_REVIEW_COMPLETE',
            'files': len(names), 'open_findings': findings, 'lexical_candidates': inventory,
            'independent_review': False, 'semantic_guarantee': False, 'runtime_acceptance': False}


def publish(path, value):
    """Create one atomic report; refuse an existing report and retain originals.

    Caller supplies an owned output directory. IO failures propagate and the
    temporary file is removed; this does not delete or replace previous evidence.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    temporary = Path(name)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2, ensure_ascii=False)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)



def archive_references(root, output, review):
    """Retain exact declared reference bytes before later document revisions.

    Called only after coverage/hash validation. Recheck each actual read to close
    a reference-change race, then create an atomic, exclusive archive and manifest
    in the owned output. Errors preserve original files and propagate; no source
    is edited or receipt/authority invented. A partial archive publication blocks
    qualification and stays as evidence; existing artifacts are never overwritten.
    """
    references = {ref['path']: ref['sha256']
                  for entry in review['files'].values()
                  for axis in entry['axes'].values() for ref in axis['evidence']}
    fd, name = tempfile.mkstemp(dir=output, prefix='.review-originals.')
    temporary = Path(name)
    os.close(fd)
    try:
        with tarfile.open(temporary, 'w:gz') as bundle:
            for name, expected in sorted(references.items()):
                raw = local(root, name).read_bytes()
                if hashlib.sha256(raw).hexdigest() != expected:
                    raise ValueError('review-reference-changed-before-archive')
                entry = tarfile.TarInfo(name=name)
                entry.size = len(raw)
                entry.mode = 0o600
                bundle.addfile(entry, io.BytesIO(raw))
        with temporary.open('rb') as stream:
            os.fsync(stream.fileno())
        archive = output / 'review-reference-originals.tar.gz'
        os.link(temporary, archive)
        publish(output / 'review-reference-originals.json', {
            'scope': 'Exact declared reference originals, not execution or acceptance',
            'archive_sha256': digest(archive), 'files': references})
    finally:
        temporary.unlink(missing_ok=True)


class ReviewRefused(RuntimeError):
    """A missing/stale/open review prevents qualification, without granting authority."""


def resolve_baseline(root, environ):
    """Require an explicit existing prior snapshot inside the repository.

    The environment selects a qualification baseline, never a runtime operator
    policy. No default or new baseline is invented. Missing/escaping input raises
    ValueError before expensive checks; the caller preserves original diagnostics.
    """
    value = environ.get('SWARM_MIGRATION_REVIEW_BASELINE')
    if not isinstance(value, str) or not value.strip():
        raise ValueError('review-baseline-required: SWARM_MIGRATION_REVIEW_BASELINE')
    path = local(root, value)
    if not path.is_file():
        raise ValueError('review-baseline-not-a-file')
    return path


def require_review(root, output, baseline):
    """Publish a fresh review report and refuse invalid or open declarations.

    Uses the collector's inputs-before.json and a source-review.json supplied in
    its owned output directory. Source/proof hashes are checked after execution;
    no agent, provider, reviewer or policy authorization follows from this check.
    Exact reference originals are archived before later documentation edits.
    Original inputs/reports remain intact; IO errors propagate. On review errors,
    writes FAIL then raises ReviewRefused retaining the cause. Open findings are
    preserved with owner/resumption and also raise, never a successful empty set.
    """
    review_path = output / 'source-review.json'
    try:
        review = json.loads(review_path.read_bytes())
        result = check(root, baseline, output / 'inputs-before.json', review)
        archive_references(root, output, review)
        result['declaration_sha256'] = digest(review_path)
        result['baseline_sha256'] = digest(baseline)
        result['current_sha256'] = digest(output / 'inputs-before.json')
    except (ValueError, OSError, KeyError, TypeError) as error:
        publish(output / 'source-review-result.json', {'coverage_status': 'FAIL', 'cause': str(error)})
        raise ReviewRefused('review-declaration-invalid-or-missing') from error
    publish(output / 'source-review-result.json', result)
    if result['contract_status'] != 'DECLARED_REVIEW_COMPLETE':
        raise ReviewRefused('review-open-findings')
    return result


def main():
    """Emit a fresh report and exit nonzero on open, invalid or stale reviews.

    Required CLI paths make the chosen baseline and declaration explicit. Error
    diagnostics identify review infrastructure, not runtime data; original input
    files are never changed. This is a local method check, not an engine gate.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('root', 'baseline', 'current', 'review', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    try:
        result = check(args.root, args.baseline, args.current, json.loads(args.review.read_bytes()))
    except (ValueError, OSError, KeyError, TypeError) as error:
        publish(args.output, {'coverage_status': 'FAIL', 'cause': str(error)})
        raise SystemExit(2) from error
    publish(args.output, result)
    print(result['contract_status'], 'files', result['files'], 'open', len(result['open_findings']))
    raise SystemExit(1 if result['open_findings'] else 0)


if __name__ == '__main__':
    main()
