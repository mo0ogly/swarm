#!/usr/bin/env python3
"""Expand a hashed public-entry inventory into an explicitly incomplete matrix.

Reads Git/Python/source evidence only, never Go or an application. CLI names and
HTTP paths are entry rows, not behavior parity. Mission and planning operations
are separately reviewed slices; unknown mappings stay open. Requires a fresh output
directory and unchanged inventoried dispatch bytes. Retains source originals,
hashes and the normal profile; does not qualify the current combined checkout.
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


def publish(path, data):
    """Publish UTF-8/bytes atomically, preserving errors including postrename sync.

    Requires an existing owned parent directory. A failure before replace leaves
    the destination intact; a directory-sync error after replace is not rollback.
    Temporary cleanup errors propagate rather than imply successful publication.
    """
    raw = data.encode() if isinstance(data, str) else data
    fd, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        parent = os.open(path.parent, os.O_DIRECTORY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def digest(raw):
    """Hash exact retained bytes; hashes establish equality, not authenticity."""
    return hashlib.sha256(raw).hexdigest()


def build(inventory_path, normal_path, output):
    """Retain entry rows and reviewed mission gaps or refuse stale prerequisites.

    Inputs are a source inventory and qualified normal-profile directory. Writes
    owned evidence only, no Root/provider/build/runtime state. IO, Git, JSON,
    source drift and missing reviewed patterns raise with no PASS result. Partial
    output survives for diagnosis; it must not be reused as a fresh destination.
    """
    output.mkdir(parents=True, exist_ok=False)
    publish(output / 'method.py', Path(__file__).read_bytes())
    original = inventory_path.read_bytes()
    inventory = json.loads(original)
    sources = {name: Path(name).read_bytes()
               for name in inventory['current_source_sha256']}
    for name, raw in sources.items():
        if digest(raw) != inventory['current_source_sha256'][name]:
            raise ValueError('inventoried source changed: ' + name)
    mission_name = 'rust/crates/swarm-cli/src/mission.rs'
    sources[mission_name] = Path(mission_name).read_bytes()
    normal_inputs = json.loads((normal_path / 'after.json').read_bytes())
    cli_name = 'rust/crates/swarm-cli/src/lib.rs'
    if any(normal_inputs.get(n) != digest(sources[n]) for n in [cli_name, mission_name]):
        raise ValueError('CLI or mission differs from qualified normal profile')
    commit = inventory['reference_commit']
    historical_name = 'internal/engine/mission_cli.go'
    historical = subprocess.check_output(['git', 'show', commit + ':' + historical_name])
    go = historical.decode()
    native = sources[mission_name].decode()
    expected_usage = 'status|changes|spending|seen|recovery|preview|start|pause|resume|stop|watch'
    if expected_usage not in go or 'if op == "watch" { return Err(CliError::Unsupported); }' not in native:
        raise ValueError('reviewed mission dispatch changed; reexamine mappings')
    rows = []
    for family, key in [('cli', 'same_name'), ('http', 'same_path')]:
        surface = inventory[family]
        declared = surface[key + '_present'] + surface[key + '_absent']
        if not declared or len(declared) != surface['reference_count']:
            raise ValueError('empty or inconsistent surface: ' + family)
        for present in [True, False]:
            names = surface[key + ('_present' if present else '_absent')]
            for name in names:
                row = dict(id=family + ':' + name, surface=family, reference=name,
                           state='ENTRY_PRESENT_PARITY_OPEN' if present else 'SAME_ENTRY_ABSENT_REPLACEMENT_OPEN',
                           owner='Codex', parity_accepted=False,
                           remaining='Map methods/inputs/outputs/refusals/effects/recovery and renamed equivalents',
                           rust_source=cli_name if family == 'cli' else 'rust/crates/swarm-web/src/service.rs')
                if family == 'http':
                    origin = next(r for r in surface['rows'] if r['path'] == name)
                    row['go_source'] = origin['go_source']
                    row['go_line'] = origin['go_line']
                else:
                    row['go_source'] = 'internal/engine/main.go'
                rows.append(row)
    mission = []
    for operation in expected_usage.split('|'):
        if operation in {'start', 'pause', 'resume', 'stop'}:
            if '"' + operation + '"' not in native:
                raise ValueError('reviewed native operation missing: ' + operation)
            state, equivalent = 'PARTIAL_CONTRACT_DIFFERENT', 'mission ' + operation
            gap = 'Native audited revision/event request and Work output differ from Go LaunchProfile/missionStatus; exact JSON/plain/refusal/effect parity remains open'
        elif operation == 'status':
            state, equivalent = 'PARTIAL_RENAMED_VIEW', 'mission show'
            gap = 'show exposes controls, not the complete Go missionStatus aggregate/guidance/changes/spending/supervision view'
        elif operation == 'watch':
            state, equivalent = 'EXPLICITLY_UNPORTED', None
            gap = 'Complete reconcile/dispatch/watch loop and interruption semantics absent'
        else:
            state, equivalent = 'NO_NATIVE_OPERATION', None
            gap = 'No same operation; map any reusable internal components before porting public contract'
        mission.append(dict(id='cli:mission:' + operation, reference='mission ' + operation,
                            state=state, rust_equivalent=equivalent, remaining=gap,
                            owner='Codex', parity_accepted=False,
                            go_source=historical_name, rust_source=mission_name))
    planning_api = subprocess.check_output(['git', 'show', commit + ':internal/engine/planning_api.go'])
    planning_engine = subprocess.check_output(['git', 'show', commit + ':internal/engine/planning.go'])
    api = planning_api.decode().split('func (s *Store) registerPlanning(', 1)[0]
    engine = planning_engine.decode().split('func (s *Store) planningChange(', 1)[1].split('\nfunc ', 1)[0]
    usage = re.search(r'usage : planning ([^ ]+) WORK', api)
    if usage is None:
        raise ValueError('historical planning usage changed')
    legacy_planning = set(usage[1].split('|'))
    legacy_planning.update(re.findall(r'pos\[1\] == "([a-z-]+)"', api))
    legacy_planning.update(re.findall(r'action == "([a-z-]+)"', engine))
    cli = sources[cli_name].decode()
    control = cli.split('fn execute_control(', 1)[1].split('    let input =', 1)[0]
    native_planning = set(re.findall(r'"([a-z-]+)" => (?:None|Some\("planning\.)', control))
    routed = re.search(r'matches!\(action, ([^)]*)\)', control)
    if routed is None or 'CliError::PlanningUnsupported' not in control:
        raise ValueError('reviewed native planning dispatch changed')
    native_planning.update(re.findall(r'"([a-z-]+)"', routed[1]))
    planning = [dict(id='cli:planning:' + operation, reference='planning ' + operation,
                     state='ENTRY_PRESENT_PARITY_OPEN' if operation in native_planning else 'NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN',
                     rust_equivalent='planning ' + operation if operation in native_planning else None,
                     go_source=['internal/engine/planning_api.go', 'internal/engine/planning.go'],
                     rust_source=cli_name, owner='Codex', parity_accepted=False,
                     remaining='Map request schema, operation-specific Root branches, outputs/refusals, effects and recovery; public CLI absence does not prove internal feature absence')
                for operation in sorted(legacy_planning)]
    if len(rows) != inventory['cli']['reference_count'] + inventory['http']['reference_count']:
        raise ValueError('entry count mismatch')
    if len({r['id'] for r in rows + mission + planning}) != len(rows) + len(mission) + len(planning):
        raise ValueError('duplicate matrix row')
    for name, raw in sources.items():
        if Path(name).read_bytes() != raw:
            raise ValueError('source drift during collection: ' + name)
        publish(output / (name.replace('/', '__') + '.source'), raw)
    publish(output / 'mission-reference.go.source', historical)
    publish(output / 'planning-api-reference.go.source', planning_api)
    publish(output / 'planning-engine-reference.go.source', planning_engine)
    publish(output / 'inventory.json', original)
    publish(output / 'normal-profile.json', (normal_path / 'result.json').read_bytes())
    result = dict(schema_version=1, reference_commit=commit,
                  scope='Entry matrix plus reviewed mission/planning operation slices; NOT exhaustive behavior denominator',
                  inventory_sha256=digest(original), source_sha256={n: digest(r) for n, r in sources.items()},
                  collector_limit='Reviewed syntax only; unfamiliar forms may be missed. Every new source hash needs semantic review, not scanner-only acceptance.',
                  historical_mission_sha256=digest(historical), entry_rows=rows, mission_rows=mission,
                  historical_planning_sha256=[digest(planning_api), digest(planning_engine)], planning_rows=planning,
                  counts=dict(entries=len(rows), cli=inventory['cli']['reference_count'],
                              http=inventory['http']['reference_count'], mission_operations=len(mission),
                              mission_states=dict(collections.Counter(r['state'] for r in mission)),
                              planning_operations=len(planning),
                              planning_states=dict(collections.Counter(r['state'] for r in planning))),
                  globally_accepted_rows=0, exhaustive=False, installed=False)
    publish(output / 'matrix.json', json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result['counts']))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--normal-profile-directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    arguments = parser.parse_args()
    build(arguments.inventory, arguments.normal_profile_directory, arguments.output)
