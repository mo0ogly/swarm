#!/usr/bin/env python3
"""Retain a reviewed source-only CLI operation inventory for eight families.

Reads the immutable Go reference through Git and current Rust dispatch sources;
never compiles, executes Go/Rust, opens a Root or contacts a provider. Every row
keeps parity open. Exact reviewed hashes refuse source drift, including unfamiliar
syntax. The finite named inventory excludes Go providers' open-ended show
fallback, recorded separately. A new source hash requires semantic review.
"""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

REFERENCE = '553593a2ffa3823d16784a6fedc291eebf390837'
GO_MAIN = 'internal/engine/main.go'
RUST_MAIN = 'rust/crates/swarm-cli/src/lib.rs'
GO_FILES = [GO_MAIN, 'internal/engine/console_cli.go', 'internal/engine/pricing.go',
            'internal/engine/quotas.go', 'internal/engine/budget_commands.go',
            'internal/engine/task_model.go']
RUST_FILES = [RUST_MAIN] + ['rust/crates/swarm-cli/src/' + name + '.rs' for name in
    ['agent', 'provider_configuration', 'provider_policy', 'quotas', 'task_model',
     'task_review', 'task_validation']]
# Exact bytes reviewed with the collector's mappings. Not a runtime/version claim.
REVIEWED_SHA256 = {'internal/engine/main.go': '648dff6e6e19516dd1b03b35ab817015b825bf40d056c05dcdbdedd84bf95151', 'internal/engine/console_cli.go': '6457c47961a1e2f97504ea91624781f362fc2aaeaa94e921d8f976347e9a2f49', 'internal/engine/pricing.go': '4a0ba60ad5d57990eb161389a555ebfb7d3c193a7bc3281665c36a2c909973b8', 'internal/engine/quotas.go': '91138131f7215dfe4678d65ee1b451cb566f52f47ec0da13ddc6b9260aac7077', 'internal/engine/budget_commands.go': '093433dfe585e77f936cfca47403efe2c612b560aedbda9d1ca325be7670d7f9', 'internal/engine/task_model.go': '7ad0ed951d0f3a0611a2f57074cef3194f385ea94f1aba37788a09dcf6524055', 'rust/crates/swarm-cli/src/lib.rs': '34264b22e6f32b7419ae83feba38ebfed9e8f30120f2c6a5af2e424800f292d9', 'rust/crates/swarm-cli/src/agent.rs': '27c5819d7e6b072cec794b202520599ba4d5e2d7130d91ab6fdd45e78b5d7167', 'rust/crates/swarm-cli/src/provider_configuration.rs': '74a765a829ab3b16b87963db0a032f385b3da744b5da8138ebfaed79f10abb5d', 'rust/crates/swarm-cli/src/provider_policy.rs': '8f3f1542bc51964f4fb3460f7a8ce2fea07f9e1967c5730c3f83e14c431363e3', 'rust/crates/swarm-cli/src/quotas.rs': '1d91d507ec753d11694eb375c97c3a98431cdb25a5ecbcd5807dad8ef7cc2ac4', 'rust/crates/swarm-cli/src/task_model.rs': '9c3e25699335016c64be3e046bcd194e19ac05d095f952d110dbe4e2f1f9a774', 'rust/crates/swarm-cli/src/task_review.rs': '719a58f9640b05e84ac3e86b80ce902e09e246ecd78cbb50753e9dce5bf7ac9a', 'rust/crates/swarm-cli/src/task_validation.rs': 'fd07533a1daaa633a5034a5fa5703bfaee56839f7ffd18d85a283457dade6d77'}
EXPECTED = {
    'work': {'create', 'update', 'list', 'show'}, 'task': {'add', 'update'},
    'agent': {'resume-dialogue', 'resume-dialogue-preview', 'prepared', 'resume-launch',
              'preflight', 'attach', 'list', 'history', 'show', 'start', 'context-preview',
              'stop', 'reconcile', 'logs'},
    'providers': {'init', 'show', 'relay show', 'relay decide', 'cooldown show', 'cooldown clear'},
    'pricing': {'list', 'save', 'estimate'}, 'quotas': {'show', 'preview', 'apply'},
    'budget': {'show', 'preview', 'apply'}, 'task-model': {'show', 'preview', 'apply'},
}


def sha(raw):
    """Hash retained bytes for equality; digest establishes no authenticity."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, raw):
    """Atomically publish owned evidence, retaining pre-replace originals on error.

    Parent must exist. IO/fsync/replace/cleanup errors propagate; a directory sync
    error after replace is an uncertain publication, never a successful rollback.
    """
    if isinstance(raw, str):
        raw = raw.encode()
    fd, name = tempfile.mkstemp(prefix='.' + path.name, dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
        directory = os.open(path.parent, os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def checked_sources(sources, expected):
    """Refuse missing/extra/stale reviewed sources before producing classifications.

    Inputs map relative source names to exact bytes and reviewed SHA256 values.
    No file writes. Malformed hash inventories and byte mismatches raise ValueError.
    """
    if not expected or set(sources) != set(expected):
        raise ValueError('reviewed source inventory missing or inconsistent')
    for name, value in expected.items():
        if not isinstance(value, str) or re.fullmatch('[0-9a-f]{64}', value) is None:
            raise ValueError('malformed reviewed digest: ' + name)
        if sha(sources[name]) != value:
            raise ValueError('source needs semantic reexamination: ' + name)


def unique_names(names, family):
    """Require a nonempty unique named dispatch projection; reject silent zeroes.

    Names must be canonical lowercase command words, with spaces for nested paths.
    This validates inventory data only, never command input or runtime authority.
    """
    names = list(names)
    if not names or any(not isinstance(n, str) or re.fullmatch('[a-z]+(?:[- ][a-z]+)*', n) is None for n in names):
        raise ValueError('empty or malformed operation projection: ' + family)
    if len(names) != len(set(names)):
        raise ValueError('duplicate operation projection: ' + family)
    return set(names)


def between(text, begin, end=None):
    """Select one reviewed literal source region or refuse a changed boundary.

    This is a syntax-specific projection, not a Go/Rust parser. Reviewed hashes
    guard unrecognized source changes. Returns text; no runtime side effects.
    """
    if text.count(begin) != 1:
        raise ValueError('missing or ambiguous reviewed boundary: ' + begin)
    rest = text.split(begin, 1)[1]
    if end is None:
        return rest
    if end not in rest:
        raise ValueError('missing reviewed end boundary: ' + end)
    return rest.split(end, 1)[0]


def historical_operations(go):
    """Extract reachable named operations from reviewed real Go dispatch branches.

    Requires main routes plus helper dispatch bytes, including Linux console_cli.
    Help strings alone do not establish reachability. Provider show is an explicit
    documented invocation accepted by its fallback; unknown spellings remain an
    open-ended diagnostic outside the named denominator. Changed forms refuse.
    """
    main = go[GO_MAIN].decode()
    console = go['internal/engine/console_cli.go'].decode()
    routes = {'agent': 'agentCLI(s, pos', 'providers': 'agentCLI(s, pos',
              'pricing': 's.pricingCLI(pos', 'quotas': 's.quotasCLI(pos',
              'budget': 's.budgetCLI(pos', 'task-model': 's.taskModelCLI(pos'}
    for family, call in routes.items():
        if 'pos[0] == "' + family + '"' not in main or call not in main:
            raise ValueError('historical main route missing: ' + family)
    result = {}
    agent = between(console, '\tcase "agent":')
    case_names = re.findall(r'^\t\tcase (.*):$', agent, re.M)
    result['agent'] = unique_names([n for case in case_names for n in re.findall('"([a-z-]+)"', case)], 'agent')
    work = between(main, '\tswitch kind {', '\n\tb, e := readInput(input)')
    mutations = re.findall(r'"((?:work|task)\.[a-z]+)"', work)
    for family in ['work', 'task']:
        names = [n.split('.')[1] for n in mutations if n.startswith(family + '.')]
        if family == 'work':
            for name in ['list', 'show']:
                if 'kind == "work.' + name + '"' not in main:
                    raise ValueError('historical work read route changed')
                names.append(name)
        result[family] = unique_names(names, family)
    for family, path, function in [('pricing', 'pricing.go', 'pricingCLI'),
                                    ('quotas', 'quotas.go', 'quotasCLI'),
                                    ('budget', 'budget_commands.go', 'budgetCLI'),
                                    ('task-model', 'task_model.go', 'taskModelCLI')]:
        text = go['internal/engine/' + path].decode()
        block = between(text, 'func (s *Store) ' + function + '(').split('\nfunc ', 1)[0]
        result[family] = unique_names(sorted(set(re.findall(r'pos\[1\] [!=]= "([a-z-]+)"', block))), family)
    providers = between(console, '\tcase "providers":', '\tcase "agent":')
    names = []
    for group, stop in [('relay', '\n\t\tif arg(1) == "cooldown"'), ('cooldown', '\n\t\tif arg(1) == "init"')]:
        block = between(providers, '\t\tif arg(1) == "' + group + '"', stop)
        names.extend(group + ' ' + n for n in re.findall(r'case "([a-z-]+)":', block))
    if 'if arg(1) == "init"' not in providers or 'p, e := s.providers()' not in providers:
        raise ValueError('historical provider fallback changed')
    result['providers'] = unique_names(names + ['init', 'show'], 'providers')
    if result != EXPECTED:
        raise ValueError('historical operations require semantic reexamination')
    return result


def locate(text, anchor):
    """Return first one-based consumer line; absent reviewed consumer is an error.

    The line witnesses source presence only; no runtime or parity assertion.
    """
    if anchor not in text:
        raise ValueError('reviewed consumer missing: ' + anchor)
    return text[:text.index(anchor)].count('\n') + 1


def rows_for(go, rust):
    """Map reviewed reachable Go operations to actual Rust dispatch consumers.

    Requires exact reviewed inputs (caller checks hashes). Returned rows separate
    same-name, renamed/partial, and absent entries; all behavioral parity remains
    unaccepted. Source evidence cannot establish runtime behavior or permissions.
    """
    operations = historical_operations(go)
    root = 'rust/crates/swarm-cli/src/'
    mappings = {}
    for op, consumer in [('create', '"work.create"'), ('update', '"work.update"'), ('list', 'execute_work_list(a, out)'), ('show', '"work.show"')]:
        mappings[('work', op)] = (RUST_MAIN, consumer, 'work ' + op)
    for op in ['add', 'update']:
        mappings[('task', op)] = (RUST_MAIN, '"task.' + op + '"', 'task ' + op)
    for op, consumer in [('show', '.read_agent('), ('stop', '.stop_agent('), ('reconcile', '.reconcile_agent(')]:
        mappings[('agent', op)] = (root + 'agent.rs', consumer, 'agent ' + op)
    for old, new in [('start', 'prepare + agent run (two distinct operations)'), ('context-preview', 'preview'), ('preflight', 'preview (not Go preflight DTO)')]:
        mappings[('agent', old)] = (root + 'agent.rs', '.prepare_agent_launch(', 'agent ' + new)
    for op in ['init', 'show']:
        mappings[('providers', op)] = (root + 'provider_configuration.rs', '.initialize_provider_configuration(' if op == 'init' else '.provider_configuration(', 'providers config ' + op)
    for op, consumer in [('list', '.rate_catalogue('), ('save', '.save_rate('), ('estimate', '.quote(')]:
        mappings[('pricing', op)] = (RUST_MAIN, consumer, 'pricing ' + op)
    for family, file, calls in [('quotas', 'quotas.rs', {'show': '.quotas(', 'preview': '.preview_quotas(', 'apply': '"quotas.configure"'}),
                                ('task-model', 'task_model.rs', {'show': '.read_work(', 'preview': '.preview_task_model(', 'apply': '"task.model"'})]:
        for op, consumer in calls.items():
            mappings[(family, op)] = (root + file, consumer, family + ' ' + op)
    mappings[('budget', 'apply')] = (RUST_MAIN, 'Some("budget.configure")', 'budget apply')
    gaps = {
        'work': 'Request fields, rendered view/events, exact JSON/plain output, revision/replay, refusal and postcommit output recovery require behavioral qualification.',
        'task': 'Task fields/dependencies/evaluations and mutation/replay/refusal/output contracts require behavioral qualification; Root presence is not parity.',
        'agent': 'Native lifecycle is not Go dialogue/terminal/prepared-launch protocol; schemas, process start/stop, durable effects and recovery require separate qualification.',
        'providers': 'Configuration is renamed; Go cooldown and relay actions are not routed here. Native provider policies are a separate contract, not proven equivalents.',
        'pricing': 'Catalogue/save/quote routes exist; exact units, rational/float representations, missing rates, conflicts, output and recovery parity remain open.',
        'quotas': 'Show routes to quota projections in both implementations; exact fields, preview/apply effects and error/output/replay contracts remain open.',
        'budget': 'Native CLI exposes only apply. Go show includes revision, budget and coverage; preview reports remaining/unlimited/new-launch advice. Internal cost APIs do not supply these CLI contracts.',
        'task-model': 'Native show emits Work; Go show adds available_models to flattened Work fields. Preview/apply validation and actual launch consumption require qualification.',
    }
    go_consumers = {
        'work': {'create': 's.executeRequest(', 'update': 's.executeRequest(', 'list': 's.list()', 'show': 's.get(id)'},
        'task': {'add': 's.executeRequest(', 'update': 's.executeRequest('},
        'agent': {'resume-dialogue': 's.resumeDialogue(', 'resume-dialogue-preview': 's.resumeDialogue(',
                  'prepared': 's.preparedLaunchForTask(', 'resume-launch': 's.resumePreparedLaunch(',
                  'preflight': 's.preflightLaunch(', 'attach': 's.attachTerminal(', 'list': 's.cockpitSnapshot(',
                  'history': 'localFile(s.root, ".swarm/imports/"', 'show': 's.agent(arg(2))',
                  'start': 's.prepare(arg(2), r)', 'context-preview': 's.prepareLaunch(',
                  'stop': 's.stopAgent(', 'reconcile': 's.reconcile(', 'logs': 's.logs('},
        'providers': {'init': 's.initProviders()', 'show': 's.providers()', 'relay show': 's.providerRelayView(',
                      'relay decide': 's.decideProviderRelay(', 'cooldown show': 's.providerCooldown(', 'cooldown clear': 's.clearProviderCooldown('},
        'pricing': {'list': 's.rateCatalogue()', 'save': 's.saveRate(', 'estimate': 's.quoteRate('},
        'quotas': {'show': 'quotaView(w)', 'preview': 's.previewQuotas(', 'apply': 's.configureQuotas('},
        'budget': {'show': 's.budget(pos[2])', 'preview': 's.previewBudget(pos[2],', 'apply': 's.configureBudget(pos[2],'},
        'task-model': {'show': 'result["available_models"]', 'preview': 's.previewTaskModel(', 'apply': 's.configureTaskModel('},
    }
    rows = []
    go_paths = {'work': GO_MAIN, 'task': GO_MAIN, 'agent': 'internal/engine/console_cli.go', 'providers': 'internal/engine/console_cli.go', 'pricing': 'internal/engine/pricing.go', 'quotas': 'internal/engine/quotas.go', 'budget': 'internal/engine/budget_commands.go', 'task-model': 'internal/engine/task_model.go'}
    for family in EXPECTED:
        for operation in sorted(operations[family]):
            mapping = mappings.get((family, operation))
            row = dict(id='cli:' + family + ':' + operation.replace(' ', ':'), family=family, operation=operation,
                       go_source=go_paths[family], go_dispatch='run → ' + ('agentCLI' if family in ['agent', 'providers'] else ('executeRequest/read/list' if family in ['work', 'task'] else {'task-model': 'taskModelCLI'}.get(family, family + 'CLI'))),
                       state='NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN', rust_equivalent=None, rust_consumer=None,
                       remaining=gaps[family], parity_accepted=False, owner='Codex')
            go_consumer = go_consumers[family][operation]
            row.update(go_consumer=go_consumer, go_consumer_line=locate(go[go_paths[family]].decode(), go_consumer))
            if mapping:
                name, consumer, equivalent = mapping
                text = rust[name].decode()
                row.update(state='ENTRY_PRESENT_PARITY_OPEN' if equivalent == family + ' ' + operation else 'RENAMED_PARTIAL_CONTRACT_PARITY_OPEN', rust_equivalent=equivalent,
                           rust_source=name, rust_consumer=consumer, rust_consumer_line=locate(text, consumer))
            else:
                row['rust_source'] = [RUST_MAIN, root + ('agent.rs' if family == 'agent' else 'provider_configuration.rs')] if family in ['agent', 'providers'] else RUST_MAIN
            rows.append(row)
    if len({r['id'] for r in rows}) != len(rows):
        raise ValueError('duplicate matrix row')
    return rows


def collect(output, baseline=None):
    """Collect into a fresh evidence directory, refusing drift and stale baseline.

    Baseline, when supplied, is the retained matrix195 source digest map; the lib
    dispatcher must match. Other modules are exact reviewed current sources, not
    a qualified binary profile. Writes originals/method/report only. All IO/Git/
    JSON/source errors propagate, leaving partial evidence and no success report.
    """
    output.mkdir(parents=True, exist_ok=False)
    publish(output / 'collector.py', Path(__file__).read_bytes())
    go = {n: subprocess.check_output(['git', 'show', REFERENCE + ':' + n]) for n in GO_FILES}
    rust = {n: Path(n).read_bytes() for n in RUST_FILES}
    checked_sources(go | rust, REVIEWED_SHA256)
    if baseline is not None:
        raw = baseline.read_bytes()
        data = json.loads(raw)
        if data['reference_commit'] != REFERENCE or data['source_sha256'][RUST_MAIN] != sha(rust[RUST_MAIN]):
            raise ValueError('baseline reference or CLI source stale')
        publish(output / 'baseline.json', raw)
    rows = rows_for(go, rust)
    for name, raw in rust.items():
        if Path(name).read_bytes() != raw:
            raise ValueError('source drift during collection: ' + name)
    for name, raw in (go | rust).items():
        publish(output / (name.replace('/', '__') + '.source'), raw)
    result = dict(schema_version=1, reference_commit=REFERENCE, source_sha256={n: sha(b) for n, b in (go | rust).items()},
        scope='Eight reviewed named Linux CLI families; source inspection only, not exhaustive behavior or binary qualification',
        rows=rows, counts=dict(named_operations=len(rows), families=len(EXPECTED), per_family=dict(collections.Counter(r['family'] for r in rows)), states=dict(collections.Counter(r['state'] for r in rows))),
        native_additions_not_in_historical_denominator=['agent preview/prepare/run/guard/limits', 'task submit/review*/validation*', 'providers config inspect/backups/preview/apply', 'providers policies export/preview/apply'],
        findings=[dict(id='providers-open-ended-show-fallback', state='OBSERVED_SOURCE', detail='Go providers unknown-name falls through to s.providers(); not counted as infinitely many named operations. Native config rejects unknown operations.'),
                  dict(id='linux-scope', state='LIMIT', detail='Go console_cli has linux build tag; non-Linux agents_unsupported is outside this projection.'),
                  dict(id='source-only', state='LIMIT', detail='Exact source consumers, not execution tests, qualified normal binary, authentication, API/UX or global acceptance.')],
        exhaustive=False, globally_accepted_rows=0, installed=False,
        semantic_review='Same-agent review of exact retained dispatch bytes and consumer call sites; no independent review. New hashes require reexamination.')
    publish(output / 'operations.json', json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result['counts'], sort_keys=True))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--baseline', type=Path)
    args = parser.parse_args()
    collect(args.output, args.baseline)
