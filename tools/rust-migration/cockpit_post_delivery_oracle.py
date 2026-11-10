#!/usr/bin/env python3
"""Retain actual Go acceptanceSource/cost-text results without installing Go or opening Root.

A fresh test overlay exercises the real private Go producer on owned data. All
listed package/test/embedded source bytes are hashed before and after execution;
the archive retains those originals. This is a pure serialization oracle, not
freshness, process/identity permission, full compiler qualification or acceptance.
The test watchdog is explicitly disabled. No provider or live datastore is used.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import traceback
from current_cli_inventory import publish_new

ROOT = Path(__file__).resolve().parents[2]
TEST = 'TestCodexCockpitAcceptanceSourceOracle'


def publish(path, value):
    """Publish fresh retained bytes through the shared no-replace writer.

    IO/close/sync errors propagate, including a potentially visible linked file.
    Existing evidence is never refreshed or silently replaced.
    """
    raw = value if isinstance(value, bytes) else (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()
    publish_new(path, raw)


def command(argv, out, label, env):
    """Run one recorded local command without a shell or whole-run deadline.

    Retain both streams and exit status before raising on failure. No unchanged
    retry, output truncation, provider invocation or live Root access is added.
    """
    publish(out / (label + '-argv.json'), argv)
    result = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True)
    publish(out / (label + '.stdout'), result.stdout)
    publish(out / (label + '.stderr'), result.stderr)
    publish(out / (label + '-exit.json'), dict(returncode=result.returncode))
    if result.returncode:
        raise RuntimeError(label + '-failed')
    return result.stdout


def objects(raw):
    """Decode every concatenated Go package JSON document; partial input fails."""
    decoder = json.JSONDecoder()
    text = raw.decode()
    while text.strip():
        value, end = decoder.raw_decode(text.lstrip())
        yield value
        text = text.lstrip()[end:]


def sources(packages, virtual, backing):
    """Resolve actual listed Go/test/embed files, including overlay replacement.

    Duplicate logical entries retain one exact path. Missing files refuse rather
    than disappearing; generated test-main inputs are retained if Go lists them.
    Source equality is a before/after observation, not atomic or ABA protection.
    """
    found = {}
    for package in packages:
        if package.get('Error') or package.get('DepsErrors'):
            raise ValueError('go-package-incomplete')
        directory = Path(package.get('Dir', ''))
        for key in ['GoFiles', 'CgoFiles', 'TestGoFiles', 'XTestGoFiles', 'EmbedFiles']:
            for name in package.get(key, []):
                logical = Path(name) if Path(name).is_absolute() else directory / name
                actual = backing if logical == virtual else logical
                if not actual.is_file():
                    raise ValueError('listed-go-source-missing: ' + str(logical))
                try:
                    label = logical.relative_to(ROOT).as_posix()
                except ValueError:
                    label = 'external-go/' + hashlib.sha256(str(logical).encode()).hexdigest() + '/' + logical.name
                if label in found and found[label] != actual:
                    raise ValueError('go-source-alias-conflict')
                found[label] = actual
    # Module selection/checksum inputs belong to this oracle too, not only .go.
    modules = {ROOT / 'go.mod', ROOT / 'go.sum'}
    for package in packages:
        mod = package.get('Module') or {}
        if mod.get('GoMod'):
            modules.add(Path(mod['GoMod']))
    for path in modules:
        if not path.is_file():
            raise ValueError('go-module-input-missing')
        try:
            label = path.relative_to(ROOT).as_posix()
        except ValueError:
            label = 'external-go/' + hashlib.sha256(str(path).encode()).hexdigest() + '/' + path.name
        found[label] = path
    if not found or virtual.relative_to(ROOT).as_posix() not in found:
        raise ValueError('oracle-overlay-not-selected')
    return found


def observe(found):
    """Hash complete original selected bytes; IO failure is never empty proof."""
    return {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in sorted(found.items())}


def run(inputs, out, write_fixture, check_fixture):
    """Generate or check a golden source oracle with actual named Go subtests.

    Output and an optional initial golden file must be fresh. Checking compares
    actual results, source hashes and Go environment without rewriting the golden.
    No readiness, source freshness, SQL state or budget is inferred from hashes.
    """
    if not out.is_dir() or any(out.iterdir()):
        raise ValueError('oracle-output-not-owned-empty')
    cases = json.loads(inputs.read_bytes())
    if not isinstance(cases, list) or not cases or len({c['name'] for c in cases}) != len(cases):
        raise ValueError('oracle-cases-invalid')
    runner_inputs = [Path(__file__).resolve(), inputs, Path(__file__).with_name('current_cli_inventory.py')]
    if check_fixture:
        runner_inputs.append(check_fixture)
    expected_fixture = json.loads(check_fixture.read_bytes()) if check_fixture else None
    runtime_before = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in runner_inputs}
    publish(out / 'runner-inputs.before.json', runtime_before)
    publish(out / 'runner.py', Path(__file__).read_bytes())
    # Carry raw task JSON as a string through both JSON transports. Never parse
    # and reserialize a RawMessage or copy a Go-remarshaled numeric argument.
    wire_cases = [dict(c, task_json=c.get('task_json') or json.dumps(c['task'], ensure_ascii=False, separators=(',', ':'))) if c['kind']=='source' else c for c in cases]
    body = json.dumps(wire_cases, ensure_ascii=False, separators=(',', ':')).encode()
    encoded = base64.b64encode(body).decode()
    program = '''package engine
import ("bytes"; "encoding/base64"; "encoding/json"; "fmt"; "testing")
func TestCodexCockpitAcceptanceSourceOracle(t *testing.T) {
 raw,err:=base64.StdEncoding.DecodeString("INPUT_BASE64");if err!=nil{t.Fatal(err)}
 var cases []struct{Name string `json:"name"`; Kind string `json:"kind"`; Cost CostTotal `json:"cost"`; Work string `json:"work_id"`; TaskJSON string `json:"task_json"`}
 if err=json.Unmarshal(raw,&cases);err!=nil{t.Fatal(err)}
 for _,c:=range cases{t.Run(c.Name,func(t *testing.T){
  result:=map[string]any{"name":c.Name,"kind":c.Kind}
  switch c.Kind{
  case "source":
   var task Task;decoder:=json.NewDecoder(bytes.NewReader([]byte(c.TaskJSON)));decoder.DisallowUnknownFields();if err:=decoder.Decode(&task);err!=nil{t.Fatal(err)}
   result["work_id"]=c.Work;result["task_json"]=c.TaskJSON;result["expected"]=acceptanceSource(Work{ID:c.Work},task)
  case "cost":result["cost"]=c.Cost;result["expected_text"]=c.Cost.Text()
  default:t.Fatal("unknown fixture kind")
  }
  raw,err:=json.Marshal(result);if err!=nil{t.Fatal(err)}
  fmt.Printf("CODEX_SOURCE_ORACLE %s\\n",raw)
 })}
}
'''.replace('INPUT_BASE64', encoded)
    backing = out / 'oracle_test.go.txt'
    virtual = ROOT / 'internal/engine/zz_codex_cockpit_source_oracle_test.go'
    if virtual.exists():
        raise ValueError('oracle-virtual-path-already-owned')
    publish(backing, program.encode())
    overlay = out / 'overlay.json'
    publish(overlay, {'Replace': {str(virtual): str(backing)}})
    env = {**os.environ, 'GOTOOLCHAIN': 'local', 'GOPROXY': 'off'}
    settings = json.loads(command(['go', 'env', '-json', 'GOVERSION', 'GOOS', 'GOARCH', 'GOFLAGS', 'GOEXPERIMENT', 'CGO_ENABLED'], out, 'go-env', env))
    packages = list(objects(command(['go', 'list', '-deps', '-test', '-json', '-overlay', str(overlay), './internal/engine'], out, 'go-list', env)))
    found = sources(packages, virtual, backing)
    before = observe(found)
    publish(out / 'sources.before.json', before)
    with tarfile.open(out / 'go-sources.tar.gz', 'x:gz', dereference=True) as archive:
        for name, path in sorted(found.items()):
            archive.add(path, arcname=name, recursive=False)
    with tarfile.open(out / 'go-sources.tar.gz') as archive:
        if set(archive.getnames()) != set(before):
            raise ValueError('go-source-archive-members-changed')
        for member in archive:
            if not member.isfile():
                raise ValueError('go-source-archive-not-retained-bytes')
            stream = archive.extractfile(member)
            if stream is None:
                raise ValueError('go-source-archive-member-missing')
            with stream:
                if hashlib.sha256(stream.read()).hexdigest() != before[member.name]:
                    raise ValueError('go-source-archive-bytes-changed')
    events = [json.loads(line) for line in command(['go', 'test', '-json', '-count=1', '-timeout=0', '-overlay', str(overlay), '-run', '^' + TEST + '$', './internal/engine'], out, 'go-test', env).splitlines()]
    after = observe(found)
    publish(out / 'sources.after.json', after)
    if before != after:
        raise ValueError('go-sources-changed-during-oracle')
    runtime_after = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in runner_inputs}
    publish(out / 'runner-inputs.after.json', runtime_after)
    if runtime_before != runtime_after:
        raise ValueError('oracle-runner-inputs-changed')
    passed = {e.get('Test') for e in events if e.get('Action') == 'pass' and e.get('Test')}
    expected = {TEST, *(TEST + '/' + c['name'] for c in cases)}
    if passed != expected:
        raise ValueError('oracle-named-tests-missing')
    prefix = 'CODEX_SOURCE_ORACLE '
    answers = [json.loads(e['Output'][len(prefix):]) for e in events if e.get('Action') == 'output' and e.get('Output', '').startswith(prefix)]
    by_name = {c['name']: c for c in answers}
    if len(answers) != len(cases) or len(by_name) != len(cases):
        raise ValueError('oracle-answers-incomplete')
    ordered = []
    for case in wire_cases:
        answer = by_name[case['name']]
        if case['kind'] == 'source':
            if answer['task_json'] != case['task_json'] or answer['work_id'] != case['work_id']:
                raise ValueError('go-source-arguments-changed')
            ordered.append(dict(kind='source', name=case['name'], work_id=case['work_id'], task_json=case['task_json'], expected=answer['expected']))
        else:
            # Go serializes negative float zero as -0; Python's integer parser
            # would lose its sign. Retain the actual original typed argument.
            ordered.append(dict(kind='cost', name=case['name'], cost=case['cost'], expected_text=answer['expected_text']))
    golden = dict(schema_version=2, scope='Go acceptanceSource and CostTotal.Text only; not readiness or acceptance', cases=ordered, go_environment=settings, local_go_sources={k: v for k, v in before.items() if not k.startswith('external-go/')})
    publish(out / 'oracle.json', golden)
    if write_fixture:
        publish(write_fixture, golden)
    if check_fixture and expected_fixture != golden:
        raise ValueError('golden-oracle-or-source-binding-changed')
    publish(out / 'result.json', dict(status='PASS', cases=len(cases), named_tests=len(passed), source_files=len(before), source_stable=True, provider_started=False, datastore_opened=False))
    print('GO_COCKPIT_SOURCE_ORACLE_VERIFIED', len(cases), flush=True)


def main():
    """Validate exclusive generate/check arguments and retain any original failure."""
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write-fixture', type=Path)
    mode.add_argument('--check-fixture', type=Path)
    a = p.parse_args()
    out = a.output.resolve()
    # Atomic mkdir occurs outside recovery: never append to an earlier run.
    out.mkdir(parents=True, exist_ok=False, mode=0o700)
    try:
        run(a.inputs.resolve(), out, a.write_fixture.resolve() if a.write_fixture else None, a.check_fixture.resolve() if a.check_fixture else None)
    except BaseException as error:
        try:
            publish(out / 'failure.json', dict(status='FAIL', cause=''.join(traceback.format_exception(error))))
        except BaseException as retention:
            raise BaseExceptionGroup('oracle failure and evidence retention', [error, retention])
        raise


if __name__ == '__main__':
    main()
