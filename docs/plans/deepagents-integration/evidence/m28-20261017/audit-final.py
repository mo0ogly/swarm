"""Audit original M28 receipts and current inputs without rerunning application tests."""
from pathlib import Path
import ast
import datetime
import hashlib
import json
import re
import subprocess

ROOT = Path.cwd()
OUT = Path(__file__).resolve().parent


def read(name):
    return json.loads((OUT / name).read_text())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


# Reuse only the input inventory function, not the runner's execution body.
runner = OUT / 'run_checks.py'
tree = ast.parse(runner.read_text())
node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'inputs')
scope = dict(ROOT=ROOT, Path=Path, subprocess=subprocess, json=json,
             __file__=str(runner), digest=digest)
exec(compile(ast.Module(body=[node], type_ignores=[]), str(runner), 'exec'), scope)
current_inputs = scope['inputs']()
tested_inputs = read('candidate-inputs-after.json')
assert current_inputs == tested_inputs, 'Current local Go input set differs from tested candidate'

runs = {}
for label in ['candidate', 'race', 'storage-guard']:
    record = read(label + '-run.json')
    before, after = read(label + '-inputs-before.json'), read(label + '-inputs-after.json')
    assert record['status'] == 'PASS' and before == after and record['inputs_unchanged']
    assert not record['failed'] and not record['skipped']
    assert all(step['exit_code'] == 0 for step in record['steps'])
    events = []
    for line in (OUT / (label + '-tests.jsonl')).read_text().splitlines():
        try:
            events.append(json.loads(line))
        except ValueError:
            pass
    passed = [e['Test'] for e in events if e.get('Action') == 'pass' and 'Test' in e]
    assert passed == record['passed'] and passed
    assert not any(e.get('Action') in ['fail', 'skip'] for e in events)
    assert any(e.get('Action') == 'pass' and 'Test' not in e for e in events)
    # Recipe file differs for race; every inventoried Go/module/embed input must agree.
    without_recipe = lambda values: {n: h for n, h in values.items()
                                    if not (n.startswith(str(OUT.relative_to(ROOT))) and n.endswith('.py'))}
    assert without_recipe(after) == without_recipe(tested_inputs)
    runs[label] = dict(parents=sum('/' not in n for n in passed),
                       subcases=sum('/' in n for n in passed),
                       passed=passed, inputs_unchanged=True)

required = {
    'TestWorkRowStoreGetRejectsInvalidRows', 'TestWorkRowStoreListRejectsPartialResult',
    'TestWorkRowCommonMutationRejectsBeforeCallbacks', 'TestWorkRowPublicReadersRefuseIncoherentWork',
    'TestWorkRowReplayReturnsCurrentConsistentState', 'TestWorkRowReadEnvelopePreservesLegacyAndCauses',
    'TestMemoryWorkRejectsInvalidSourceRows', 'TestMemoryWorkDoesNotReturnPartialRowsAndClosesReaders',
    'TestMemoryWorkMissingSourceAndSQLFailures', 'TestMemoryWorkRefusalIdentifiesSafeStoredSource',
    'TestMemoryWorkRefusalDoesNotEchoInvalidStoredKey', 'TestMemoryWorkIdentityMismatchRefusesPublicCatalog',
    'TestProjectMemoryReplacementPersistenceAndStaleSource', 'TestProjectMemoryConcurrentReplacement',
    'TestProjectMemoryArchiveDoesNotImportAdmission', 'TestProjectMemoryMigrationPreservesOriginals',
    'TestProjectMemoryLegacyGuideAndPreparedGuard', 'TestProjectMemoryPublicLifecycle',
    'TestMemorySummarySelectionActualProviderInputAndReopen', 'TestMemorySummarySelectionReceiptDriftPreventsProvider',
    'TestResumeDurableAndNoAutostart', 'TestIdempotencyAndRevision', 'TestConcurrentWriters',
    'TestCrashRollsBack', 'TestCLIJSONAndExitCodes', 'TestReadOnlyShowAndMalformedStatus',
    'TestExportImportMissingAndNoOverwrite', 'TestRejectHostileArchive', 'TestImportMissingPieceAndUnknownVersion',
    'TestInspectionCLIRefusesImplicitStorageUpgrade', 'TestInspectionDoesNotCreateMissingStore',
    'TestLaunchAtomicIdempotentAndWorkspaceExclusive', 'TestMigrationOnePreservesWork',
    'TestSupervisorSuccessIsNotAcceptance', 'TestPlanningArchiveAndMigration',
    'TestAgentWorkflowReachesWorkerAndSurvivesReopen',
    'TestLaunchWaitsForConcurrentWriterWithoutDuplicate',
    'TestReviewRetryConnectedWriterRejectsChangedStorageBeforeMutation',
}
unique = set().union(*(r['passed'] for r in runs.values()))
assert required == {n for n in unique if '/' not in n}, 'Expected named test coverage differs'
assert 'TestLaunchWaitsForConcurrentWriterWithoutDuplicate' in runs['race']['passed']
assert [(runs[x]['parents'], runs[x]['subcases']) for x in runs] == [(36, 20), (17, 15), (1, 0)]

red = read('red-run.json')
red_current = read('red-current-run.json')
assert red['status'] == 'FAIL' and not red['passed'] and not red['inputs_unchanged']
assert red_current['status'] == 'FAIL' and not red_current['passed'] and red_current['inputs_unchanged']
assert len([n for n in red_current['failed'] if n.startswith('Test')]) == 11
assert digest(OUT / 'red-current-test.go.txt') == read('red-current-inputs-before.json')['internal/engine/work_row_test.go']

baseline = read('baseline.json')
for name, sha in baseline['read_only'].items():
    assert digest(ROOT / name) == sha == tested_inputs[name], name
for name, sha in read('documentation-baseline.json')['preimages'].items():
    assert digest(OUT / 'before' / name) == sha
binary = Path(read('candidate-run.json')['binary'])
assert digest(binary) == read('candidate-run.json')['binary_sha256']
assert [s['kind'] for s in read('candidate-run.json')['steps']] == ['tests', 'vet', 'build']

owned = read('owned-files.json')
assert len(owned) == 14 and len(set(owned)) == 14
go_owned = [n for n in owned if n.endswith('.go')]
assert subprocess.check_output(['gofmt', '-l', *go_owned], text=True) == ''
assert not any(n in tested_inputs for n in owned if n.endswith('.md')), 'Changed doc is an inventoried embed input'
links_checked = 0
for name in owned:
    if not name.endswith('.md'):
        continue
    content = (ROOT / name).read_text()
    for destination in re.findall(r'\]\(([^)]+)\)', content):
        if re.match(r'^(?:[A-Za-z][A-Za-z0-9+.-]*:|#)', destination):
            continue
        target = destination.split('#')[0].split('?')[0]
        assert (ROOT / name).parent.joinpath(target).exists(), (name, target)
        links_checked += 1

inventory = read('callers-current-summary.json')
correction = read('callers-inventory-correction.json')
assert inventory['inputs_unchanged'] and inventory['literal_works_sql_sites'] == 27
assert inventory['selector_sites'] == 168 and len(correction['false_matches']) == 9
assert len(correction['original_exact_works_sites']) == 31
for name, sha in inventory['inputs_sha256'].items():
    assert digest(Path(name)) == sha

diff_check = subprocess.run(['git', 'diff', '--check'], stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)  # no execution watchdog
assert diff_check.returncode == 0, diff_check.stdout
proof = dict(status='PASS', at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
             local_review='self-review by implementing author; not independent acceptance',
             runs=runs, total_pass_occurrences=sum(len(r['passed']) for r in runs.values()),
             unique_names=len(unique), unique_parents=len(required), unique_subcases=len(unique)-len(required),
             named_roots_required=sorted(required), current_inputs_equal_candidate=True,
             inventoried_local_inputs=len(current_inputs),
             current_inputs_sha256=digest(OUT / 'candidate-inputs-after.json'),
             original_red_source_verified=True, read_only_boundaries_unchanged=list(baseline['read_only']),
             binary=str(binary), binary_sha256=digest(binary),
             owned_files={name:digest(ROOT / name) for name in owned},
             owned_diff_sha256=digest(OUT / 'owned.diff'), audit_recipe_sha256=digest(Path(__file__)),
             gofmt='PASS', git_diff_check='PASS', local_markdown_links_checked=links_checked,
             inventory_correction=dict(original_false_workspace_matches=9, original_works_sites=31,
                                       current_works_sites=27, current_selector_sites=168),
             not_tested=['Mermaid rendering', 'browser UI', 'deployed or running binary',
                         'native agents or real providers', 'all direct SQL readers',
                         'all caller error propagation', 'public M20 rights compliance',
                         'independent review'])
with (OUT / 'audit-final.json').open('x') as stream:
    json.dump(proof, stream, indent=2)
    stream.write('\n')
print(json.dumps({k:proof[k] for k in ['status', 'total_pass_occurrences', 'unique_names',
                                      'unique_parents', 'unique_subcases', 'inventoried_local_inputs',
                                      'gofmt', 'git_diff_check', 'local_markdown_links_checked']}))
