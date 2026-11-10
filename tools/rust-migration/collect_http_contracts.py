#!/usr/bin/env python3
"""Inventory historical registered HTTP contracts without claiming parity.

Reads the exact hashed surface inventory, Git objects and current Rust sources.
The bounded lexical collector records visible syntax, not effective Go behavior:
delegated handlers, variable queries and transport/middleware remain open. Only
four common route adapters receive a source-reviewed semantic note. No compiler,
Go, server, database or provider is executed. Fresh output preserves originals;
malformed inventories, duplicate rows, stale sources and uncertain registrations
raise rather than produce a successful empty inventory.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess


SERVICE = 'rust/crates/swarm-web/src/service.rs'
EXTRA = ['rust/crates/swarm-web/src/service/planning.rs',
         'rust/crates/swarm-web/src/service/quotas.rs']


def sha(raw):
    """Hash exact bytes for comparison; this does not establish authenticity."""
    return hashlib.sha256(raw).hexdigest()


def masked(text, mask_literals=True):
    """Mask Go strings/comments, preserving length/newlines for delimiter offsets.

    Supports quoted/rune/raw strings and both Go comment forms. When
    mask_literals=False only comments are replaced, for visible-token scans.
    Unterminated syntax raises. This is a lexical helper, not a Go parser.
    """
    out = list(text)
    i = 0
    while i < len(text):
        start = i
        if text.startswith('//', i):
            end = text.find('\n', i)
            i = len(text) if end < 0 else end
        elif text.startswith('/*', i):
            end = text.find('*/', i + 2)
            if end < 0:
                raise ValueError('unterminated block comment')
            i = end + 2
        elif text[i] in ['"', "'", '`']:
            quote = text[i]
            i += 1
            while i < len(text):
                if text[i] == quote:
                    i += 1
                    break
                if text[i] == '\\' and quote != '`':
                    i += 1
                i += 1
            else:
                raise ValueError('unterminated quoted literal')
            if not mask_literals:
                continue
        else:
            i += 1
            continue
        for n in range(start, i):
            if out[n] != '\n':
                out[n] = ' '
    return ''.join(out)


def balanced_end(mask, start, opening='(', closing=')'):
    """Return the end offset of one balanced lexical region or raise."""
    if start >= len(mask) or mask[start] != opening:
        raise ValueError('missing opening delimiter')
    depth = 0
    for i in range(start, len(mask)):
        if mask[i] == opening:
            depth += 1
        elif mask[i] == closing:
            depth -= 1
            if depth == 0:
                return i + 1
    raise ValueError('unbalanced registration')


def registration(text, line):
    """Read the Handle/HandleFunc statement at an inventoried one-based line.

    Refuses non-integer/out-of-range/incorrect anchors; returns exact statement
    including inline closure. Dynamic path suffixes stay expressions, never
    inferred new registrations.
    """
    lines = text.splitlines(keepends=True)
    if type(line) is not int or line < 1 or line > len(lines):
        raise ValueError('invalid registration line')
    start = sum(map(len, lines[:line - 1]))
    match = re.search(r'\b\w+\.Handle(?:Func)?\s*\(', lines[line - 1])
    if match is None:
        raise ValueError('registration anchor changed')
    opening = start + match.end() - 1
    end = balanced_end(masked(text), opening)
    return text[start + match.start():end]


def go_observations(statement):
    """Extract visible evidence candidates, retaining unresolved expressions.

    Method comparisons are observations, not accepted-method sets. No comparison
    means no *local visible* guard, not unrestricted access through middleware.
    Query aliases and delegated calls are explicitly outside semantic inference.
    """
    statement = masked(statement, mask_literals=False)
    methods = sorted(set(re.findall(r'\b\w+\.Method\s*(?:==|!=)\s*(?:"([A-Z]+)"|http\.Method(\w+))', statement)))
    methods = sorted({literal or word.upper() for literal, word in methods})
    query = sorted(set(re.findall(r'\.Get\("([^"]+)"\)', statement)))
    calls = sorted(set(re.findall(r'\bs\.([A-Za-z_]\w*)\s*\(', statement)))
    return dict(method_comparison_candidates=methods,
                local_method_guard='VISIBLE_COMPARISONS_REVIEW_REQUIRED' if methods else 'NO_LOCAL_COMPARISON_OBSERVED',
                get_key_candidates=query,
                method_conditions=[line.strip() for line in statement.splitlines() if re.search(r'\.Method\b', line)],
                query_conditions=[line.strip() for line in statement.splitlines() if '.Get(' in line],
                body_type_candidates=re.findall(r'\bvar\s+\w+\s+(\w+)', statement),
                delegated_store_calls=calls,
                delegated_handler_expression=statement.split(',', 1)[1].strip() if ', func(' not in statement else None,
                unresolved=['Visible comparisons/keys/calls require control-flow and producer review; delegated implementation and middleware not expanded'])


def validate_inventory(inventory, sources):
    """Refuse malformed, duplicate, inconsistent or stale HTTP prerequisites.

    sources maps inventoried current paths to actual bytes. Unrelated CLI source
    need not match: this scope checks the HTTP dispatcher, not CLI parity.
    """
    if (type(inventory.get('schema_version')) is not int or inventory['schema_version'] != 1 or not isinstance(inventory.get('reference_commit'), str)
            or re.fullmatch(r'[0-9a-f]{7,40}', inventory['reference_commit']) is None):
        raise ValueError('invalid inventory version/reference')
    http = inventory.get('http')
    if not isinstance(http, dict) or not isinstance(http.get('rows'), list) or not http['rows']:
        raise ValueError('missing HTTP rows')
    rows = http['rows']
    for row in rows:
        if (not isinstance(row, dict) or not isinstance(row.get('path'), str)
                or not row['path'].startswith('/api/v1/') or not isinstance(row.get('go_source'), str)
                or not row['go_source'].startswith('internal/engine/')
                or '..' in Path(row['go_source']).parts
                or type(row.get('go_line')) is not int or row['go_line'] < 1):
            raise ValueError('malformed HTTP row')
    paths = [r['path'] for r in rows]
    if len(set(paths)) != len(paths):
        raise ValueError('duplicate HTTP path')
    if type(http.get('reference_count')) is not int or http['reference_count'] != len(rows):
        raise ValueError('HTTP count mismatch')
    present, absent = http.get('same_path_present'), http.get('same_path_absent')
    if not isinstance(present, list) or not isinstance(absent, list) or sorted(present + absent) != sorted(paths):
        raise ValueError('HTTP partition mismatch')
    for row in rows:
        if row.get('same_path_rust_dispatch') is not (row['path'] in present):
            raise ValueError('HTTP presence mismatch')
    expected = inventory.get('current_source_sha256', {}).get(SERVICE)
    if not expected or SERVICE not in sources or sha(sources[SERVICE]) != expected:
        raise ValueError('inventoried HTTP dispatcher changed')


def family(path):
    """Group historical routes by explicit path vocabulary, not engine parity."""
    part = path.removeprefix('/api/v1/').split('/')[0]
    if part.startswith('preparation') or part in {'project-profile', 'skills'}:
        return 'preparation'
    if part.startswith('planning') or part in {'control-diagnostic', 'retex-evolution'}:
        return 'planning'
    if part.startswith('trust') or part in {'processes', 'terminal'}:
        return 'trust-process-terminal'
    if part.startswith('provider') or part in {'pricing'}:
        return 'provider-pricing'
    if part in {'graph-drafts', 'graph-performance', 'run-limits', 'web-requests'}:
        return 'configuration-graph'
    if part in {'assist', 'automation'}:
        return part
    return 'cockpit-work-observation'


def delegated_observation(text, statement):
    """Resolve only two reviewed local delegation forms, leaving others open.

    A local name assigned one closure and a Store handler-factory call can be
    retained with their entire exact lexical body. This is not call-graph
    traversal or proof that every factory return follows the same method policy.
    """
    expression = statement.split(',', 1)[1].strip().removesuffix(')').strip()
    if re.fullmatch(r'\w+', expression):
        pattern = r'\b' + re.escape(expression) + r'\s*:=\s*func\('
    else:
        call = re.fullmatch(r's\.(\w+)\([^\n]*\)', expression)
        if call is None:
            return None
        pattern = r'func\s+\(s\s+\*Store\)\s+' + re.escape(call[1]) + r'\('
    matches = list(re.finditer(pattern, masked(text, mask_literals=False)))
    if len(matches) != 1:
        return None
    start = matches[0].start()
    brace = masked(text).find('{', matches[0].end())
    if brace < 0:
        raise ValueError('delegated handler body missing')
    end = balanced_end(masked(text), brace, '{', '}')
    raw = text[start:end]
    return dict(source=raw, sha256=sha(raw.encode()), observations=go_observations(raw),
                limit='One reviewed local declaration only; nested called implementations remain open')


def reviewed_common(sources):
    """Bind four semantically reviewed adapter notes to required current anchors.

    Notes are same-context source analysis only. Changed required routing shapes
    refuse; whole exact source hashes accompany the report. They establish no
    execution, authorization completeness or HTTP response compatibility.
    """
    main = sources[SERVICE].decode()
    planning = sources[EXTRA[0]].decode()
    quotas = sources[EXTRA[1]].decode()
    required = [(main, 'action.kind != "task"'), (main, '"task.update"'),
                (main, 'self.list().map(RouteBody::plain)'),
                (main, 'HttpServiceError::Refused("web-route-unported")'),
                (planning, '"planning-query-invalid"'),
                (planning, '"web-operation-unported"'),
                (quotas, '"quota-query-invalid"'), (quotas, '"quotas.configure"')]
    if any(anchor not in text for text, anchor in required):
        raise ValueError('reviewed Rust adapter changed; reexamine mappings')
    actions = re.findall(r'Some\("([^"]+)"\)\s*=>\s*Ok\("planning\.[^"]+"\)', planning)
    expected_actions = ['enable', 'claim', 'lease-renew', 'decide', 'judge', 'renew', 'pause', 'resume']
    if actions != expected_actions:
        raise ValueError('reviewed planning vocabulary changed')
    return {
        '/api/v1/works': dict(methods=['GET', 'HEAD'], consumers=['RootStore::list_works', 'encode_model', 'RootStore::close'],
            query='No route-local query validation observed; list does not consume query',
            actions=[], gap='Historical inline list has no local method guard; Rust restricts GET/HEAD. List/output/error/middleware parity not established.'),
        '/api/v1/action': dict(methods=['POST'], consumers=['task_action', 'projection::project', 'NativeHttpService::task', 'RootStore::execute_external(task.update)'],
            query='No query consumed in reviewed task adapter', actions=['task'],
            gap='Only kind=task admitted; historical webAction has many other kinds. Root transaction/revision semantics exist but exact request/response/effect parity not accepted.'),
        '/api/v1/planning': dict(methods=['GET', 'HEAD', 'POST'], consumers=['planning::query', 'planning::kind', 'RootStore::read_work', 'RootStore::execute_external'],
            query='Exactly one safe work, optional nonempty action; duplicate/unknown keys refuse. GET/HEAD no body and action absent/show only.',
            actions=['show'] + actions,
            gap='Historical GET scope-preview/recovery-preview/review-dossier/review-cost/fragment-preview/requalify-preview and POST scope-patch plus remaining planningChange actions not exposed. Historical GET fallback differs from explicit Rust refusal.'),
        '/api/v1/quotas': dict(methods=['GET', 'HEAD'], consumers=['quotas::work_query', 'RootStore::quotas', 'encode_model', 'RootStore::close'],
            query='Exactly one safe work; duplicate/unknown/empty keys and read body refuse', actions=[],
            gap='Adapter compiled only target_os=linux. Historical GET only, Rust also HEAD; exact DTO/error/middleware parity open. New preview/apply routes are additions, not historical extra route completion.')}


def collect(inventory_path, output, repository=Path('.')):
    """Retain a fresh exact-source contract inventory or propagate refusal.

    Writes only the newly created evidence directory; partial evidence survives
    failures. Rechecks live source bytes after collection before publishing PASS.
    Uses Git show only; no application execution. Atomic writer is the existing
    maintained matrix publisher; its IO and cleanup errors propagate.
    """
    publisher_spec = importlib.util.spec_from_file_location('http_matrix_publish', repository / 'tools/rust-migration/build_behavior_matrix.py')
    publisher = importlib.util.module_from_spec(publisher_spec)
    publisher_spec.loader.exec_module(publisher)
    output.mkdir(parents=True, exist_ok=False)
    write = publisher.publish
    raw_inventory = inventory_path.read_bytes()
    inventory = json.loads(raw_inventory)
    sources = {name: (repository / name).read_bytes() for name in [SERVICE] + EXTRA + ['tools/rust-migration/build_behavior_matrix.py']}
    validate_inventory(inventory, sources)
    commit = subprocess.check_output(['git', 'rev-parse', inventory['reference_commit'] + '^{commit}'], cwd=repository).decode().strip()
    historical = {}
    for name in sorted({r['go_source'] for r in inventory['http']['rows']}):
        raw = subprocess.check_output(['git', 'show', commit + ':' + name], cwd=repository)
        previous = inventory['historical_inputs_sha256'].get(name)
        if previous is not None and sha(raw) != previous:
            raise ValueError('historical source hash mismatch: ' + name)
        historical[name] = raw
    common = reviewed_common(sources)
    rows = []
    for origin in inventory['http']['rows']:
        text = historical[origin['go_source']].decode()
        try:
            statement = registration(text, origin['go_line'])
        except ValueError as cause:
            raise ValueError('registration refused: ' + origin['go_source'] + ':' + str(origin['go_line'])) from cause
        path_expression = statement.split(',', 1)[0].split('(', 1)[1]
        literal = re.match(r'"([^"\n]+)"', path_expression)
        constant = None
        if literal is None and re.fullmatch(r'\w+', path_expression.strip()):
            matches = re.findall(r'\b' + re.escape(path_expression.strip()) + r'\s*=\s*"([^"\n]+)"', masked(text, mask_literals=False))
            if len(matches) == 1:
                constant = matches[0]
        if not (literal is not None and origin['path'].startswith(literal[1]) or constant == origin['path']):
            raise ValueError('route literal no longer matches inventory')
        row = dict(origin, family=family(origin['path']), state='ENTRY_PRESENT_PARITY_OPEN' if origin['same_path_rust_dispatch'] else 'SAME_ROUTE_ABSENT_REPLACEMENT_OPEN',
                   parity_accepted=False, go_registration=statement, go_registration_sha256=sha(statement.encode()),
                   path_expression=path_expression, constant_path_resolution=constant, observations=go_observations(statement),
                   dynamic_suffix_resolution='Declared by inherited inventory; suffix not independently recomputed by this collector' if '+' in path_expression else None,
                   delegated_local=delegated_observation(text, statement),
                   rust=common.get(origin['path']),
                   replacement='No same path in inspected dispatcher; internal Root/CLI code and Live/static/OTEL additions are not route parity')
        if row['rust'] is not None:
            row['replacement'] = 'Reviewed partial adapter; no runtime or global acceptance in this collector'
        rows.append(row)
    web = historical['internal/engine/web_server.go'].decode()
    start = web.index('func (s *Store) webAction(')
    brace = web.index('{', start)
    action_source = web[start:balanced_end(masked(web), brace, '{', '}')]
    condition_actions = set(re.findall(r'\br\.Kind\s*==\s*"([^"]+)"', action_source))
    case_actions = {value for clause in re.findall(r'case ([^:\n]+):', action_source) for value in re.findall(r'"([^"]+)"', clause)}
    for name, raw in {**historical, **sources}.items():
        path = output / 'sources' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        write(path, raw)
    write(output / 'inventory.original.json', raw_inventory)
    write(output / 'collector.original.py', Path(__file__).read_bytes())
    write(output / 'web-action.original.go', action_source)
    for name, raw in sources.items():
        if (repository / name).read_bytes() != raw:
            raise ValueError('source changed during collection: ' + name)
    result = dict(schema_version=1, state='SOURCE_INVENTORY_COMPLETE_PARITY_OPEN',
                  reference_commit=commit, inventory_sha256=sha(raw_inventory),
                  inherited_historical_hashes_checked=sorted(set(historical) & set(inventory['historical_inputs_sha256'])),
                  newly_retained_historical_hashes=sorted(set(historical) - set(inventory['historical_inputs_sha256'])),
                  sources_sha256={name: sha(raw) for name, raw in {**historical, **sources}.items()},
                  counts=dict(routes=len(rows), same_path=sum(r['same_path_rust_dispatch'] for r in rows),
                              absent=sum(not r['same_path_rust_dispatch'] for r in rows),
                              family=dict(collections.Counter(r['family'] for r in rows))),
                  historical_action_kind_candidates=sorted(condition_actions | case_actions),
                  rust_dispatch_paths=sorted(set(re.findall(r'"(/api/v1/[^"\n]+)"\s*=>', sources[SERVICE].decode().split('    fn route(', 1)[1].split('    fn list(', 1)[0]))),
                  historical_action_scan_limit='Only r.Kind == literals and switch case literals within exact webAction; no complete interprocedural action schema claim',
                  scope='Same-context bounded source analysis; no compiler/Go/provider/server execution, independent review, running-binary qualification, or parity acceptance',
                  collector_sha256=sha(Path(__file__).read_bytes()), rows=rows)
    write(output / 'contracts.json', json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    return result


def main():
    """Run read-only-source collection into a required fresh evidence path."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = collect(args.inventory, args.output)
    print(json.dumps(dict(state=result['state'], counts=result['counts'])))


if __name__ == '__main__':
    main()
