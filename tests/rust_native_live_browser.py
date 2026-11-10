#!/usr/bin/env python3
"""Live agents dashboard and harness window over a real native `swarm-rust web`.

Private disposable Root only: Works come from the supported CLI; agents, logs and
PTY chunks are fixture rows written before bind (plus one log row during the run
to prove cursor following). No provider, agent process or live mission is used.
No global watchdog. Proves local UI behaviour, not installation or acceptance.
"""
import argparse, json, os, signal, sqlite3, subprocess, sys, tempfile, threading
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_native_http_cli as native
import rust_agent_model_route as retained


def agent_body(id, work, task, status, role, parent='', activity='', usage=None, heartbeat='', facts=None, extra=None):
    """Minimal Go-compatible Agent body accepted by the Rust decoder."""
    body = dict(progress={}, diagnostic=dict(agent_id='', attempt_id='', observed_errors=0, consecutive_error_limit=0, limit_reached=False, consecutive_error_limit_reached=False),
                recovery={}, limits={}, id=id, work_id=work, task_id=task, attempt_id='', provider='fixture', role=role, workspace='/isolated/' + id,
                status=status, activity=activity, started='2026-10-21T10:00:00Z')
    if parent: body['parent'] = parent
    if heartbeat: body['heartbeat'] = heartbeat
    if usage: body['usage'] = usage
    if facts: body['native_provider_facts'] = facts
    if extra: body.update(extra)
    return body


def proc_stamp(pid):
    """Kernel starttime (field 22 of /proc/PID/stat), as the engine records it."""
    raw = Path(f'/proc/{pid}/stat').read_text()
    return raw.rsplit(')', 1)[1].split()[19]


def host_tuple():
    """hostname:boot_id:pid-namespace, the engine's SupervisorIdentity host."""
    import socket
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    return f"{socket.gethostname()}:{boot}:{os.readlink('/proc/self/ns/pid')}"


def seed(binary, root, child):
    """Create two Works through the CLI and fixture agent rows (pre-bind only)."""
    works = []
    for title in ['Refonte facturation', 'Audit sécurité']:
        spec = root / ('work-' + str(len(works)) + '.json')
        retained.publish(spec, dict(schema_version=1, event_id='live-' + str(len(works)), expected_revision=0, title=title, objective='Observe', scope='Fixture', criteria=['Visible']))
        works.append(native.cli(binary, root, ['work', 'create', '--input', str(spec)])['work']['id'])
    # Planned tasks (initial phase) through the public CLI: dependencies and one
    # task that never starts (t5), so the plan graph links tasks to live agents.
    for work, task, title, depends in [(0, 't1', 'Découper les lots', []), (0, 't2', 'Module de facturation', ['t1']), (0, 't3', 'Revue du module', ['t2']), (0, 't5', 'Documentation', ['t2']), (1, 't4', 'Analyse des accès', [])]:
        revision = native.cli(binary, root, ['work', 'show', works[work]])['work']['revision']
        spec = root / ('task-' + task + '.json')
        retained.publish(spec, dict(schema_version=1, event_id='add-' + task, expected_revision=revision, id=task, title=title, deliverable='Livrable ' + task, criteria=['Observé'], depends=depends))
        native.cli(binary, root, ['task', 'add', works[work], '--input', str(spec)])
    # Budget with a small launch reserve (public CLI): the harness shows its use.
    revision = native.cli(binary, root, ['work', 'show', works[0]])['work']['revision']
    spec = root / 'budget.json'
    retained.publish(spec, dict(schema_version=1, event_id='budget', expected_revision=revision, budget=dict(limit_usd=10, reserve_per_launch_usd=0.05, estimate_source='Fixture', reference_date='2026-10-22')))
    native.cli(binary, root, ['budget', 'apply', works[0], '--input', str(spec)])
    db = sqlite3.connect(root / '.swarm' / 'state.db')
    try:
        rows = [('lead', works[0], 't1', 'running', 'responsable', '', 'Planifie les lots', dict(input_tokens=1200, output_tokens=300)),
                ('dev', works[0], 't2', 'running', 'développeur', 'lead', 'Écrit le module de facturation', dict(input_tokens=5400, output_tokens=2100)),
                ('rev', works[0], 't3', 'completed', 'vérificateur', 'lead', 'Revue terminée', None),
                ('audit', works[1], 't4', 'failed', 'auditeur', '', 'Analyse interrompue', None)]
        invalid = dict(schema_version=1, source='fixture', usage_state='invalid', model_state='not_reported', invalid_usage_events=3, first_usage_error='agent-usage-fixture')
        # Real processes for the 'dev' agent: this Python process as supervisor
        # and a live child, both with their exact /proc identities.
        extras = {
            'dev': dict(host=host_tuple(), supervisor_pid=os.getpid(), supervisor_identity=proc_stamp(os.getpid()),
                        child_pid=child.pid, child_identity=proc_stamp(child.pid),
                        limits=dict(observation_mode=0, silence_seconds=0, tool_seconds=0, max_tool_calls=40, max_repeated_calls=0, max_consecutive_errors=0),
                        progress=dict(elapsed_seconds=125, tool_calls=12, tool_results=11, pending_tools=1, last_tool='Bash', action='Exécute les tests'),
                        context=dict(bytes=48213, sha256='', included=['README.md', 'src/lib.rs'], memory=[dict(work='', id='mem-architecture', version=2, sha256='x')])),
            'lead': dict(progress=dict(elapsed_seconds=900, time_threshold_exceeded=True)),
            # Finished attempt with a retained Claude stream capture (terminal-cleaned:
            # one line, values separated by spaces), for the typed transcript.
            'rev': dict(native_execution_result=dict(schema_version=1, capture_truncated=False, capture=' '.join(json.dumps(e) for e in [
                dict(type='system', subtype='init', model='claude-fixture'),
                dict(type='assistant', message=dict(model='claude-fixture', usage=dict(input_tokens=1200, output_tokens=300, cache_read_input_tokens=8000), content=[dict(type='text', text='FIXTURE-ASSISTANT-TEXT'), dict(type='tool_use', id='toolu_01', name='Read', input=dict(path='FIXTURE-PATH'))])),
                dict(type='user', message=dict(content=[dict(type='tool_result', tool_use_id='toolu_01', is_error=False, content='FIXTURE-OUTPUT')])),
                dict(type='assistant', message=dict(content=[dict(type='tool_use', id='toolu_02', name='Bash', input=dict(command='FIXTURE-CMD'))])),
                dict(type='user', message=dict(content=[dict(type='tool_result', tool_use_id='toolu_02', is_error=True, content='FIXTURE-ERR')])),
                dict(type='result', subtype='success', is_error=False, permission_denials=[dict(tool_name='Edit', tool_use_id='toolu_03')])]))),
        }
        for id, work, task, status, role, parent, activity, usage in rows:
            body = json.dumps(agent_body(id, work, task, status, role, parent, activity, usage, '2026-10-21T10:00:00Z', invalid if id == 'audit' else None, extras.get(id)), ensure_ascii=False).encode()
            db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)', (id, work, task, '/isolated/' + id, status, body, b'{}'))
        for i in range(103):
            db.execute("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('dev','2026-10-21T10:00:00Z','message',?)", ('étape %d' % i,))
        db.execute("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('dev','2026-10-21T10:00:30Z','output-limit','Sortie tronquée à la limite')")
        db.execute("INSERT INTO terminal_events(agent_id,data,cols,rows) VALUES('dev',?,80,24)", (b'\x1b[32m$ cargo test\x1b[0m\r\nrunning 4 tests\r\n',))
        # One live review job (task t3) and one applied planning run: minimal
        # stand-ins with the engine column names read by the console (fixture
        # only; the review/planning launch paths are not exercised here).
        db.executescript("CREATE TABLE native_review_jobs(id TEXT PRIMARY KEY, work_id TEXT NOT NULL, task_id TEXT NOT NULL, phase TEXT NOT NULL);"
                         "CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY, work_id TEXT NOT NULL, scope_id TEXT NOT NULL, phase TEXT NOT NULL, prepared_at TEXT NOT NULL);")
        db.execute("INSERT INTO native_review_jobs VALUES('j-review', ?, 't3', 'permitted')", (works[0],))
        db.execute("INSERT INTO native_planning_runs VALUES('c-plan', ?, 'root', 'applied', '2026-10-22T09:00:00Z')", (works[0],))
        db.commit()
    finally:
        db.close()
    return works


def otlp(address, path, token, body, content_type='application/json'):
    """POST one OTLP/JSON request as a machine exporter (no cookie); returns (status, parsed body)."""
    import http.client
    host, port = address.rsplit(':', 1)
    connection = http.client.HTTPConnection(host, int(port))
    try:
        connection.request('POST', path, body=json.dumps(body).encode(), headers={'Content-Type': content_type, 'Authorization': 'Bearer ' + token})
        reply = connection.getresponse(); raw = reply.read()
        # Head refusals are plain-text codes (transport convention); Root replies are JSON.
        json_reply = (reply.getheader('Content-Type') or '').startswith('application/json')
        return reply.status, json.loads(raw) if json_reply else raw.decode()
    finally:
        connection.close()


def seed_telemetry(token_fixture, root, address):
    """Issue a token for 'dev' through the real engine API (stand-in for the
    missing launch issuance) and deliver Claude-Code-shaped OTLP over HTTP.
    Returns the secret (kept in memory only) and status evidence."""
    token = subprocess.run([str(token_fixture), str(root), 'dev'], check=True, capture_output=True, text=True).stdout.strip()
    attr = lambda k, v: dict(key=k, value=dict(stringValue=v))
    event = lambda name, ns, attrs: dict(timeUnixNano=str(ns), body=dict(stringValue='claude_code.' + name), attributes=[attr(k, v) for k, v in attrs])
    logs = dict(resourceLogs=[dict(scopeLogs=[dict(logRecords=[
        event('api_request', 1761040800000000000, [('model', 'claude-fixture'), ('input_tokens', '1800'), ('output_tokens', '420'), ('cache_read_tokens', '9000'), ('cost_usd', '0.042'), ('duration_ms', '3100'), ('prompt', 'FIXTURE-PROMPT-CONTENT')]),
        event('api_request', 1761040805000000000, [('model', 'claude-fixture'), ('input_tokens', '2400'), ('output_tokens', '610'), ('cost_usd', '0.051'), ('duration_ms', '4200')]),
        event('tool_result', 1761040803000000000, [('tool_name', 'Bash'), ('success', 'false'), ('duration_ms', '1250')]),
        event('tool_result', 1761040804000000000, [('tool_name', 'Read'), ('success', 'true'), ('duration_ms', '30')]),
        event('tool_decision', 1761040806000000000, [('tool_name', 'Edit'), ('decision', 'accept')])])])])
    metrics = dict(resourceMetrics=[dict(scopeMetrics=[dict(metrics=[dict(name='claude_code.lines_of_code.count', sum=dict(aggregationTemporality=1, dataPoints=[
        dict(timeUnixNano='1761040806000000000', asDouble=37, attributes=[attr('type', 'added')]),
        dict(timeUnixNano='1761040806000000000', asDouble=5, attributes=[attr('type', 'removed')])])),
        # Codex-shaped latency histogram (delta) and an unrecognized histogram name.
        dict(name='codex.api_request.duration_ms', histogram=dict(aggregationTemporality=1, dataPoints=[
            dict(timeUnixNano='1761040806000000000', count='4', sum=2200.0, max=3000.0, bucketCounts=['1', '2', '1'], explicitBounds=[100.0, 1000.0],
                 attributes=[attr('model', 'gpt-fixture')])])),
        dict(name='codex.sqlite.init.duration_ms', histogram=dict(aggregationTemporality=1, dataPoints=[dict(count='1', sum=1.0)]))])])])
    # Codex-shaped trace: run → turn → command, plus an internal span (dropped).
    span = lambda sid, parent, name, start, end, attrs=(): dict(traceId='ab12', spanId=sid, parentSpanId=parent, name=name,
        startTimeUnixNano=str(1761040800000000000 + start * 1000000), endTimeUnixNano=str(1761040800000000000 + end * 1000000), attributes=[attr(k, v) for k, v in attrs])
    traces = dict(resourceSpans=[dict(scopeSpans=[dict(spans=[span('01', '', 'codex.exec', 0, 5000), span('02', '01', 'session_task.turn', 100, 4800),
        span('03', '02', 'exec_command', 1000, 1192, [('tool_name', 'exec_command'), ('code.file.path', '/PRIVATE/path.rs')]), span('04', '02', 'fs.read_file', 200, 201)])])])
    evidence_traces = otlp(address, '/api/otel/v1/traces', token, traces)
    assert evidence_traces[0] == 200, evidence_traces
    evidence = dict(logs=otlp(address, '/api/otel/v1/logs', token, logs), metrics=otlp(address, '/api/otel/v1/metrics', token, metrics),
                    wrong_token=otlp(address, '/api/otel/v1/logs', 'f' * 64, logs)[0], protobuf=otlp(address, '/api/otel/v1/logs', token, logs, 'application/x-protobuf')[0])
    assert evidence['logs'] == (200, {}) and evidence['metrics'] == (200, {}), evidence
    assert evidence['wrong_token'] == 401 and evidence['protobuf'] == 415, evidence
    # Review job telemetry: its own token (subject review:j-review, live while
    # the job phase is active) and one model call shown only in its run view.
    review = subprocess.run([str(token_fixture), str(root), 'review:j-review'], check=True, capture_output=True, text=True).stdout.strip()
    review_logs = dict(resourceLogs=[dict(scopeLogs=[dict(logRecords=[event('api_request', 1761040810000000000, [('model', 'claude-fixture'), ('input_tokens', '640'), ('output_tokens', '90'), ('cost_usd', '0.004'), ('prompt', 'FIXTURE-REVIEW-PROMPT')])])])])
    evidence['review_logs'] = otlp(address, '/api/otel/v1/logs', review, review_logs)
    assert evidence['review_logs'] == (200, {}), evidence
    # Claude Code trace (shapes from the 2.1.280 probe, no paid call) for the
    # running 'lead': interaction → model call, tool → permission wait.
    lead = subprocess.run([str(token_fixture), str(root), 'lead'], check=True, capture_output=True, text=True).stdout.strip()
    claude = dict(resourceSpans=[dict(scopeSpans=[dict(spans=[
        span('a1', '', 'claude_code.interaction', 0, 900, [('user_prompt', '<REDACTED>')]),
        span('a2', 'a1', 'claude_code.llm_request', 10, 300, [('model', 'claude-fixture'), ('input_tokens', '20'), ('output_tokens', '5'), ('stop_reason', 'tool_use')]),
        span('a3', 'a1', 'claude_code.tool', 320, 700, [('tool_name', 'Bash'), ('bash_argv0', '/PRIVATE/script.sh')]),
        span('a4', 'a3', 'claude_code.tool.blocked_on_user', 320, 340, [('decision', 'accept')])])])])
    evidence['claude_traces'] = otlp(address, '/api/otel/v1/traces', lead, claude)
    assert evidence['claude_traces'][0] == 200, evidence
    return token, evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output-parent', required=True, type=Path, help='Existing directory; a fresh unique evidence child is created')
    parser.add_argument('--token-fixture', type=Path, help='Built telemetry_token_fixture example; enables the OTLP telemetry path')
    parser.add_argument('--ui', default='tests/rust_native_live_ui.cjs', help='Browser scenario script (same stdin contract)')
    a = parser.parse_args()
    binary = a.binary.resolve(); parent = a.output_parent.resolve(); parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='live-ui-', dir=parent))
    root = native.owned_root(binary)
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(600)'])  # fixture provider process, killed below
    works = seed(binary, root, child)
    server = native.Server(binary, root, output, 'live-native'); errors = []; summaries = []; primary = None
    def collect():
        try:
            while True:
                record = server.record()
                if record['event'] == 'web-stopped': summaries.append(record); return
        except BaseException as error: errors.append(error)
    collector = threading.Thread(target=collect); collector.start()
    token = None
    try:
        if a.token_fixture:
            token, evidence = seed_telemetry(a.token_fixture.resolve(), root, server.address)
            retained.publish(output / 'telemetry-ingest.json', evidence)
        payload = json.dumps(dict(address=server.address, key=server.key, telemetry_token=token, db=str(root / '.swarm' / 'state.db'), works=works, output=str(output / 'browser'))).encode()
        with (output / 'browser.stdout').open('xb') as out, (output / 'browser.stderr').open('xb') as err:
            run = subprocess.run(['node', a.ui], input=payload, stdout=out, stderr=err)
        retained.publish(output / 'fixture-child.json', dict(pid=child.pid, alive_after_browser=child.poll() is None, returncode=child.returncode))
        assert run.returncode == 0, ('Browser verification failed; see browser.stderr', run.returncode)
        result = json.loads((output / 'browser/result.json').read_bytes()); assert result['status'] == 'PASS'
        server.process.send_signal(signal.SIGINT); server.stop_sent = True; collector.join()
        assert not errors and len(summaries) == 1, errors
        server.stderr = server.process.stderr.read(); assert server.process.wait() == 0 and not server.stderr
        for file in output.rglob('*'):
            if file.is_file():
                data = file.read_bytes()
                assert server.key.encode() not in data, 'Credential leaked into evidence'
                assert token is None or token.encode() not in data, 'Telemetry token leaked into evidence'
        if token:
            for name in ['state.db', 'telemetry.db']:
                data = (root / '.swarm' / name).read_bytes()
                assert token.encode() not in data and b'FIXTURE-PROMPT-CONTENT' not in data, 'Token or content persisted in ' + name
        print(json.dumps(dict(status='PASS', checks=len(result['checks']), output=str(output))))
    except BaseException as error:
        primary = error
    child.kill(); child.wait()
    native.finish(server, primary)


if __name__ == '__main__':
    main()
