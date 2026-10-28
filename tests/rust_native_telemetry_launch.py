#!/usr/bin/env python3
"""LIVE-UX-03 E2E-2: real native worker launch exports OTLP to the real web server.

Private owned Root only. A local Python program is registered as the `claude`
provider (no paid call, no real Claude Code). Through public CLI routes the
scenario saves settings v30 `agent_telemetry_providers=["claude"]`, starts
`swarm-rust web` (which publishes `.swarm/telemetry-endpoint.json`), prepares
and runs one agent. The provider reads ONLY the injected OTEL variables from its
environment and POSTs one OTLP/JSON log record with the injected bearer token.
Checks: the provider saw endpoint + token, ingest answered 200, the live agent
view shows the received call, a second launch with telemetry disabled gets no
variable, and neither the token nor prompt content reaches evidence or the
mission database. A fake `codex` (executable script, same argv as the real
CLI) checks the leading `-c otel.…` arguments, the token in the environment
only, and the Codex call mapping. The ended attempt also blocks its task: one alert evaluation
fires `task-blocked` once (L5). Finally a restarted server with the v35
alert cadence at 1 s opens the alert of the second blocked task with no
console request (server loop). No global watchdog: every wait is on an
observable step.
"""
import argparse, json, signal, sqlite3, sys, threading, time, traceback
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_native_http_cli as native
import rust_agent_model_route as retained
import subprocess


class tv:
    """Public CLI call with retained channels (same contract as rust_native_task_validation.invoke)."""
    @staticmethod
    def invoke(binary, root, out, name, args, request=None):
        cmd = [str(binary), *args, '--root', str(root), '--json']
        if request is not None:
            p = out / (name + '.input.json'); retained.publish(p, request); cmd += ['--input', str(p)]
        retained.publish(out / (name + '.command.json'), dict(argv=cmd, global_watchdog=None))
        r = subprocess.run(cmd, capture_output=True, stdin=subprocess.DEVNULL)
        retained.publish(out / (name + '.stdout'), r.stdout); retained.publish(out / (name + '.stderr'), r.stderr)
        return r.returncode, json.loads(r.stdout if r.returncode == 0 else r.stderr)

# Fake provider: records which OTEL names it received (never values), posts one
# api_request event with the injected token, then prints a minimal Claude
# stream-json result so the native stream parser settles the attempt.
PROVIDER = r'''
import json, os, sys, urllib.request
sys.stdin.buffer.read()
proof = sys.argv[1]
names = sorted(k for k in os.environ if k.startswith('OTEL_') or k == 'CLAUDE_CODE_ENABLE_TELEMETRY')
result = dict(names=names, status=None)
endpoint = os.environ.get('OTEL_EXPORTER_OTLP_ENDPOINT')
header = os.environ.get('OTEL_EXPORTER_OTLP_HEADERS', '')
if endpoint and header.startswith('Authorization='):
    # Two calls: with the saved per-attempt cap of 1, the second is refused and counted.
    body = json.dumps({"resourceLogs": [{"scopeLogs": [{"logRecords": [{"timeUnixNano": str(1761040800000000000 + i),
        "body": {"stringValue": "claude_code.api_request"},
        "attributes": [{"key": "model", "value": {"stringValue": "claude-launch-fixture"}},
                       {"key": "input_tokens", "value": {"stringValue": str(321 + i)}},
                       {"key": "prompt", "value": {"stringValue": "LAUNCH-PROMPT-CONTENT"}}]} for i in range(2)]}]}]}).encode()
    request = urllib.request.Request(endpoint + '/v1/logs', data=body, method='POST',
        headers={'Content-Type': 'application/json', 'Authorization': header.split('=', 1)[1]})
    with urllib.request.urlopen(request) as reply:
        result['status'] = reply.status
        result['reply'] = json.loads(reply.read())
    result['endpoint_path'] = endpoint.split('/', 3)[3]
json.dump(result, open(proof, 'w'))
print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "result": "ok"}))
'''

# Fake Codex: an executable script so the engine's leading `-c otel.…` overrides
# reach it exactly as they reach `codex` (argv[1:]). It records argv and the
# OTEL variable names, takes the endpoint from `-c otel.exporter=…` and the
# token from OTEL_EXPORTER_OTLP_HEADERS, and posts Codex-shaped records
# (shape observed from codex-cli 0.163.0: zero timeUnixNano, string values,
# content inside tool_result).
CODEX = r'''#!PYTHON
import json, os, re, sys, urllib.request
sys.stdin.buffer.read()
proof = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'codex-proof.json')
args = sys.argv[1:]
result = dict(args=args, names=sorted(k for k in os.environ if k.startswith('OTEL_')), status=None)
logs = [a.split('=', 1)[1] for a in args if a.startswith('otel.exporter=')]
header = os.environ.get('OTEL_EXPORTER_OTLP_HEADERS', '')
try:
  if logs and header.startswith('Authorization='):
    endpoint = re.search(r'endpoint="([^"]+)"', logs[0]).group(1)
    def rec(event, ns, extra):
        attrs = [{"key": "event.name", "value": {"stringValue": event}}, {"key": "model", "value": {"stringValue": "codex-fixture"}}]
        attrs += [{"key": k, "value": {"stringValue": v}} for k, v in extra]
        return {"timeUnixNano": "0", "observedTimeUnixNano": str(ns), "attributes": attrs}
    body = json.dumps({"resourceLogs": [{"scopeLogs": [{"logRecords": [
        rec("codex.api_request", 1761040800000000000, [("duration_ms", "900"), ("http.response.status_code", "200")]),
        rec("codex.sse_event", 1761040800100000000, [("event.kind", "response.completed"), ("input_token_count", "500"),
            ("cached_token_count", "200"), ("output_token_count", "40"), ("reasoning_token_count", "7")]),
        rec("codex.tool_result", 1761040800200000000, [("tool_name", "exec_command"), ("duration_ms", "146"), ("success", "true"),
            ("arguments", "CODEX-ARGUMENT-CONTENT"), ("output", "CODEX-OUTPUT-CONTENT")])]}]}]}).encode()
    request = urllib.request.Request(endpoint, data=body, method='POST',
        headers={'Content-Type': 'application/json', 'Authorization': header.split('=', 1)[1]})
    with urllib.request.urlopen(request) as reply:
        result['status'] = reply.status
    result['path'] = endpoint.split('/', 3)[3]
except Exception as error:
    result['error'] = repr(error)
json.dump(result, open(proof, 'w'))
print(json.dumps({"type": "turn.completed", "usage": {"input_tokens": 500, "cached_input_tokens": 200, "output_tokens": 40}}))
'''


def setup(binary, root, out, proof):
    """Work + task through the public CLI; provider registry written as the existing fixtures do."""
    code, work = tv.invoke(binary, root, out, 'work-create', ['work', 'create'], dict(schema_version=1, event_id='create', expected_revision=0, title='Telemetry launch', objective='Observe', scope='Fixture', criteria=['Visible']))
    assert code == 0, work
    work = work['work']['id']
    for task in ['t1', 't2', 't3']:
        code, shown = tv.invoke(binary, root, out, 'show-' + task, ['work', 'show', work]); assert code == 0, shown
        code, added = tv.invoke(binary, root, out, 'add-' + task, ['task', 'add', work], dict(schema_version=1, event_id='add-' + task, expected_revision=shown['work']['revision'], id=task, title='Run ' + task, deliverable='Evidence', criteria=['Observed, not accepted']))
        assert code == 0, added
    providers = root / '.swarm/providers.json'
    fake_codex = out / 'fake-codex'
    retained.publish(fake_codex, CODEX.replace('#!PYTHON', '#!' + sys.executable).encode())
    fake_codex.chmod(0o755)
    retained.publish(providers, dict(schema_version=1, providers=dict(
        claude=dict(command=sys.executable, args=['-c', PROVIDER, str(proof)]),
        codex=dict(command=str(fake_codex), args=['exec', '--json', '-']))))
    providers.chmod(0o600)
    return work


def settings(binary, root, out, name, providers, cap=None, alert_seconds=None):
    """Explicit public preview/apply of the v30 list, v33 cap and v35 server
    alert cadence (None = loop off, so console evaluations stay deterministic)."""
    code, current = tv.invoke(binary, root, out, name + '-show', ['engine-settings', 'show']); assert code == 0, current
    values = dict(current['values']); values['agent_telemetry_providers'] = providers
    values['agent_telemetry_max_events_per_attempt'] = cap
    values['agent_alert_evaluation_seconds'] = alert_seconds
    request = dict(schema_version=current['schema_version'], event_id=name, expected_revision=current['revision'], values=values, reason='Telemetry launch fixture adoption', preview_token='')
    code, preview = tv.invoke(binary, root, out, name + '-preview', ['engine-settings', 'preview'], request); assert code == 0, preview
    request['preview_token'] = preview['preview_token']
    code, saved = tv.invoke(binary, root, out, name + '-apply', ['engine-settings', 'apply'], request); assert code == 0, saved
    assert saved['document']['schema_version'] == current['schema_version'] and saved['document']['values']['agent_telemetry_providers'] == providers, saved


def launch(binary, root, out, work, session, task, provider='claude'):
    """Prepare then run one attempt through the public agent CLI."""
    code, shown = tv.invoke(binary, root, out, 'show-' + session, ['work', 'show', work]); assert code == 0, shown
    code, prepared = tv.invoke(binary, root, out, 'prepare-' + session, ['agent', 'prepare', work], dict(schema_version=1, event_id=session, expected_revision=shown['work']['revision'], task_id=task, provider=provider))
    assert code == 0, prepared
    code, ran = tv.invoke(binary, root, out, 'run-' + session, ['agent', 'run', session]); assert code == 0, ran
    return ran


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path, help='Fresh evidence directory')
    a = parser.parse_args()
    binary = a.binary.resolve(); out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    root = native.owned_root(binary); proof = out / 'provider-proof.json'
    work = setup(binary, root, out, proof)
    # (Enabled without a live server injects nothing: engine unit test.)
    settings(binary, root, out, 'enable', ['claude'], cap='1')
    server = native.Server(binary, root, out, 'telemetry-web'); primary = None; summaries = []; errors = []
    def collect():
        try:
            while True:
                record = server.record()
                if record['event'] == 'web-stopped': summaries.append(record); return
        except BaseException as error: errors.append(error)
    collector = threading.Thread(target=collect); collector.start()
    checks = {}
    try:
        # `web-bound` precedes runtime startup (runtime_ready false); the endpoint
        # is published by the runtime afterwards: wait for that observable file,
        # failing if the server exits first (no deadline).
        published = root / '.swarm/telemetry-endpoint.json'
        while not published.exists():
            assert server.process.poll() is None, 'web server exited before publishing its telemetry endpoint'
            time.sleep(0.05)
        endpoint = json.loads(published.read_text())
        assert endpoint['address'] == server.address and endpoint['pid'] == server.process.pid, endpoint
        ran = launch(binary, root, out, work, 'session-on', 't1')
        seen = json.loads(proof.read_text())
        assert 'OTEL_EXPORTER_OTLP_ENDPOINT' in seen['names'] and 'OTEL_EXPORTER_OTLP_HEADERS' in seen['names'], seen
        assert seen['status'] == 200 and seen['endpoint_path'] == 'api/otel', seen
        assert seen['reply'] == {'partialSuccess': {'rejectedLogRecords': '1', 'errorMessage': 'telemetry-attempt-cap-reached'}}, seen['reply']
        agent = ran['agent']['id']
        view = server.request('/api/v1/live/agent?agent=' + agent + '&work=' + work, observe=False)
        telemetry = view['agent']['telemetry']
        assert telemetry['state'] == 'received' and len(telemetry['calls']) == 1 and telemetry['calls'][0]['fields']['input_tokens'] == '321', telemetry
        assert telemetry['rejected'] == 1, 'capped point counted, not hidden'
        policy = server.request('/api/v1/live/alerts', observe=False)['retention']['policy']
        assert policy == {'days': 30, 'max_events_per_attempt': 1}, policy
        assert 'LAUNCH-PROMPT-CONTENT' not in json.dumps(telemetry)
        checks['enabled'] = dict(names=seen['names'], ingest=seen['status'], state=telemetry['state'], run_status=ran['agent']['status'])
        checks['attempt_cap'] = dict(stored=1, rejected=1)
        # L5: the engine blocks the task when the attempt ends; the alert
        # evaluation (session + CSRF) fires task-blocked once, by public routes only.
        csrf = server.request('/api/v1/native-session', observe=False)['csrf_token']
        first = server.request('/api/v1/live/alerts/evaluate', {}, csrf=csrf, observe=False)
        subject = work + '/t1'
        fired = [a for a in first['sync']['fired'] if a['rule'] == 'task-blocked' and a['subject'] == subject]
        assert len(fired) == 1 and any(a['subject'] == subject for a in first['alerts']['open']), first
        again = server.request('/api/v1/live/alerts/evaluate', {}, csrf=csrf, observe=False)
        assert again['sync'] == {'fired': [], 'resolved': []}, again['sync']
        checks['alert_task_blocked'] = dict(rule='task-blocked', fired_once=True)
        # After the attempt ends its token no longer authorizes anything (implicit revocation).
        db = sqlite3.connect('file:' + str(root / '.swarm/telemetry.db') + '?mode=ro', uri=True)
        try: tokens = db.execute("SELECT count(*) FROM tokens WHERE agent_id=?", (agent,)).fetchone()[0]
        finally: db.close()
        assert tokens == 1, tokens
        # Codex: endpoint as leading global `-c otel.…` arguments, token only in the
        # environment; its completed response becomes the call (input − cached).
        settings(binary, root, out, 'codex-on', ['codex'])
        ran_codex = launch(binary, root, out, work, 'session-codex', 't3', provider='codex')
        cx = json.loads((out / 'codex-proof.json').read_text())
        assert cx['args'][:8] == ['-c', 'otel.exporter={otlp-http={endpoint="http://%s/api/otel/v1/logs",protocol="json"}}' % server.address,
            '-c', 'otel.metrics_exporter={otlp-http={endpoint="http://%s/api/otel/v1/metrics",protocol="json"}}' % server.address,
            '-c', 'otel.trace_exporter={otlp-http={endpoint="http://%s/api/otel/v1/traces",protocol="json"}}' % server.address,
            '-c', 'otel.log_user_prompt=false'] and 'exec' in cx['args'][8:], cx['args']
        assert cx['names'] == ['OTEL_EXPORTER_OTLP_HEADERS'] and cx['status'] == 200 and cx['path'] == 'api/otel/v1/logs', cx
        codex_agent = ran_codex['agent']['id']
        cview = server.request('/api/v1/live/agent?agent=' + codex_agent + '&work=' + work, observe=False)['agent']['telemetry']
        call = cview['calls'][0]['fields']
        assert len(cview['calls']) == 1 and call['input_tokens'] == '300' and call['cache_read_tokens'] == '200' and call['output_tokens'] == '40', cview
        assert [t['tool'] for t in cview['tool_calls']] == ['exec_command'], cview['tool_calls']
        assert 'CODEX-ARGUMENT-CONTENT' not in json.dumps(cview) and 'CODEX-OUTPUT-CONTENT' not in json.dumps(cview)
        # Estimated cost from a rate saved through the public pricing CLI.
        code, listed = tv.invoke(binary, root, out, 'pricing-list', ['pricing', 'list']); assert code == 0, listed
        rate = dict(provider='codex', model='codex-fixture', currency='USD', input_per_million=2, output_per_million=10,
                    cache_read_per_million=0.5, source='fixture', reference_date='2026-10-22')
        code, saved = tv.invoke(binary, root, out, 'pricing-save', ['pricing', 'save'], dict(schema_version=1, expected_digest=listed['digest'], rate=rate)); assert code == 0, saved
        priced = server.request('/api/v1/live/agent?agent=' + codex_agent + '&work=' + work, observe=False)['agent']['telemetry']['estimated_cost']
        assert priced['priced_calls'] == 1 and priced['currency'] == 'USD' and abs(priced['amount'] - 0.0011) < 1e-12, priced
        checks['codex'] = dict(args=8, env=cx['names'], call_input=call['input_tokens'], tool='exec_command', estimated_usd=priced['amount'])
        # Disabled again: next launch receives no telemetry variable.
        settings(binary, root, out, 'disable', [])
        proof.unlink()
        launch(binary, root, out, work, 'session-off', 't2')
        off = json.loads(proof.read_text())
        assert off['names'] == [] and off['status'] is None, off
        checks['disabled'] = dict(names=off['names'])
        server.process.send_signal(signal.SIGINT); server.stop_sent = True; collector.join()
        assert not errors and len(summaries) == 1, errors
        assert not (root / '.swarm/telemetry-endpoint.json').exists(), 'endpoint retracted at stop'
        checks['endpoint_retracted'] = True
        # No token or prompt content in evidence or mission state.
        secrets = [row[0] for row in sqlite3.connect(str(root / '.swarm/telemetry.db')).execute('SELECT digest FROM tokens')]
        # (The witness string also lives in the provider script, hence in the
        # registry and agent record; only telemetry storage must not hold it.)
        assert b'LAUNCH-PROMPT-CONTENT' not in (root / '.swarm/telemetry.db').read_bytes()
        for witness in [b'CODEX-ARGUMENT-CONTENT', b'CODEX-OUTPUT-CONTENT']:
            assert witness not in (root / '.swarm/telemetry.db').read_bytes(), witness
        for file in [*out.rglob('*'), root / '.swarm/state.db']:
            if file.is_file():
                assert b'Authorization=Bearer' not in file.read_bytes(), 'token header leaked into ' + str(file)
        # v35 server loop: a restarted server evaluating every second opens
        # the alert of task t2 (blocked by the last attempt, never evaluated
        # yet) with no console request; waits are on the observable view.
        settings(binary, root, out, 'alert-loop', [], alert_seconds='1')
        server = native.Server(binary, root, out, 'loop-web'); summaries.clear()
        collector = threading.Thread(target=collect); collector.start()
        subject = work + '/t2'
        while True:
            alerts = server.request('/api/v1/live/alerts', observe=False)
            if any(a['subject'] == subject and a['rule'] == 'task-blocked' for a in alerts['alerts']['open']): break
            assert alerts['server_evaluation'] is None or alerts['server_evaluation']['state'] in ('starting', 'running'), alerts['server_evaluation']
            time.sleep(0.2)
        loop = alerts['server_evaluation']
        assert loop['state'] == 'running' and loop['interval_seconds'] == 1 and loop['last_ms'] > 0, loop
        server.process.send_signal(signal.SIGINT); server.stop_sent = True; collector.join()
        assert not errors and len(summaries) == 1, errors
        checks['server_alert_loop'] = dict(state=loop['state'], interval_seconds=1, opened=subject)
        retained.publish(out / 'result.json', dict(status='PASS', checks=checks, token_digests=len(secrets), scope='Private fixture Root; fake claude provider; not real Claude Code, not acceptance'))
        print(json.dumps(dict(status='PASS', checks=sorted(checks), output=str(out))))
    except BaseException as error:
        traceback.print_exc(); primary = error
    native.finish(server, primary)


if __name__ == '__main__':
    main()
