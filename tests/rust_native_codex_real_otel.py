#!/usr/bin/env python3
"""LIVE-UX-03 telemetry with the REAL `codex` CLI launched by Swarm (no paid call).

Private owned Root only. The installed `codex` binary is registered as the
`codex` provider; its model provider is a local fake (Responses API over SSE)
configured in a temporary CODEX_HOME, so no network model or account is used
and the user's ~/.codex is never read. The fake model asks once for
`exec_command` (`echo swarm-probe`) then answers. Swarm injects the exporter
(`-c otel.…` arguments, token in OTEL_EXPORTER_OTLP_HEADERS) and its local web
server receives the OTLP export. Checks on the live agent view: one model call
with the mapped tokens (uncached input = 10 − 4), the tool call, and no prompt,
argument or output content stored. Codex exports by batches: a missing batch
at process exit is reported as such, not retried. Requires `codex` on PATH;
records its version. No global watchdog: waits are on observable steps.
Witness: SWARM_PROBE_NO_TELEMETRY=1 runs the same launch with telemetry off.
Status 2026-10-22 (codex-cli 0.163.0): BLOCKED before telemetry — `agent run`
fails with agent-execution-process / process-control then
agent-launch-commit-refused (sqlite busy), identically with telemetry off;
reported to the launch owner in docs/plans/migration-rust/PLAN.md. Not part of
the `live-monitor-native` recipe until that launch path works.
"""
import argparse, http.server, json, os, shutil, signal, subprocess, sys, tempfile, threading, traceback
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_native_http_cli as native
import rust_agent_model_route as retained
from rust_native_telemetry_launch import tv, settings


class FakeModel(http.server.BaseHTTPRequestHandler):
    """Responses API stand-in: first call requests one tool, the next answers."""
    calls = 0

    def do_POST(self):
        self.rfile.read(int(self.headers.get('Content-Length') or 0))
        if not self.path.endswith('/responses'):
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.end_headers(); self.wfile.write(b'{}'); return
        FakeModel.calls += 1
        usage = {"input_tokens": 10, "input_tokens_details": {"cached_tokens": 4}, "output_tokens": 3,
                 "output_tokens_details": {"reasoning_tokens": 1}, "total_tokens": 13}
        item = ({"type": "function_call", "id": "fc1", "call_id": "call1", "name": "exec_command",
                 "arguments": json.dumps({"cmd": "echo swarm-probe"}), "status": "completed"} if FakeModel.calls == 1 else
                {"type": "message", "role": "assistant", "id": "m1", "status": "completed",
                 "content": [{"type": "output_text", "text": "done", "annotations": []}]})
        events = [('response.created', {"type": "response.created", "response": {"id": "r", "status": "in_progress", "output": []}}),
                  ('response.output_item.done', {"type": "response.output_item.done", "output_index": 0, "item": item}),
                  ('response.completed', {"type": "response.completed", "response": {"id": "r", "status": "completed", "output": [], "usage": usage}})]
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        for name, data in events:
            self.wfile.write(('event: %s\ndata: %s\n\n' % (name, json.dumps(data))).encode())
        self.wfile.flush()

    do_GET = do_POST

    def log_message(self, *args):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path, help='Fresh evidence directory')
    a = parser.parse_args()
    codex = shutil.which('codex')
    assert codex, 'codex CLI not found on PATH: this proof needs the real binary'
    binary = a.binary.resolve(); out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    version = subprocess.run([codex, '--version'], capture_output=True, text=True, stdin=subprocess.DEVNULL).stdout.strip()
    model = http.server.ThreadingHTTPServer(('127.0.0.1', 0), FakeModel)
    threading.Thread(target=model.serve_forever, daemon=True).start()
    home = Path(tempfile.mkdtemp(prefix='codex-home-', dir=out))
    (home / 'config.toml').write_text('model = "fake-model"\nmodel_provider = "fake"\n[model_providers.fake]\nname = "fake"\n'
        'base_url = "http://127.0.0.1:%d/v1"\nwire_api = "responses"\nenv_key = "FAKE_KEY"\nrequest_max_retries = 0\nstream_max_retries = 0\n' % model.server_address[1])
    (home / 'models_cache.json').write_text(json.dumps({'models': [{'slug': 'fake-model', 'visibility': 'list', 'supported_reasoning_levels': []}]}))
    root = native.owned_root(binary)
    code, work = tv.invoke(binary, root, out, 'work-create', ['work', 'create'], dict(schema_version=1, event_id='create', expected_revision=0, title='Real codex telemetry', objective='Observe', scope='Fixture', criteria=['Visible']))
    assert code == 0, work
    work = work['work']['id']
    code, shown = tv.invoke(binary, root, out, 'show-t1', ['work', 'show', work]); assert code == 0, shown
    code, added = tv.invoke(binary, root, out, 'add-t1', ['task', 'add', work], dict(schema_version=1, event_id='add-t1', expected_revision=shown['work']['revision'], id='t1', title='Echo probe', deliverable='Evidence', criteria=['Observed, not accepted']))
    assert code == 0, added
    providers = root / '.swarm/providers.json'
    retained.publish(providers, dict(schema_version=1, providers=dict(codex=dict(
        command=str(Path(codex).resolve()), args=['exec', '--json', '--skip-git-repo-check', '-'], env_allow=['CODEX_HOME', 'FAKE_KEY', 'HOME', 'PATH'],
        preflight_ttl_seconds=300,  # test input within the saved ceiling (300): real codex preflight is slow
        # Swarm requires a saved model policy for the codex adapter; every level is the fake model.
        model_policy=dict(version=1, billing='external', work_level='standard', page_level='simple',
                          levels={level: dict(model='fake-model', effort='') for level in ['simple', 'standard', 'exigeant']})))))
    providers.chmod(0o600)
    settings(binary, root, out, 'enable', [] if os.environ.get('SWARM_PROBE_NO_TELEMETRY') else ['codex'])  # witness switch
    os.environ.update(CODEX_HOME=str(home), FAKE_KEY='fake-not-a-secret')
    server = native.Server(binary, root, out, 'codex-web'); primary = None; summaries = []; errors = []
    def collect():
        try:
            while True:
                record = server.record()
                if record['event'] == 'web-stopped': summaries.append(record); return
        except BaseException as error: errors.append(error)
    collector = threading.Thread(target=collect); collector.start()
    checks = dict(codex_version=version)
    try:
        published = root / '.swarm/telemetry-endpoint.json'
        while not published.exists():
            assert server.process.poll() is None, 'web server exited before publishing its telemetry endpoint'
            threading.Event().wait(0.05)
        code, shown = tv.invoke(binary, root, out, 'show-run', ['work', 'show', work]); assert code == 0, shown
        code, prepared = tv.invoke(binary, root, out, 'prepare', ['agent', 'prepare', work], dict(schema_version=1, event_id='real-codex', expected_revision=shown['work']['revision'], task_id='t1', provider='codex'))
        assert code == 0, prepared
        code, ran = tv.invoke(binary, root, out, 'run', ['agent', 'run', 'real-codex']); assert code == 0, ran
        agent = ran['agent']['id']
        checks['run_status'] = ran['agent']['status']
        checks['model_calls_served'] = FakeModel.calls
        view = server.request('/api/v1/live/agent?agent=' + agent + '&work=' + work, observe=False)['agent']['telemetry']
        retained.publish(out / 'telemetry-view.json', view)
        assert view['state'] == 'received', view
        calls = [c['fields'] for c in view['calls']]
        assert calls and all(c.get('input_tokens') == '6' and c.get('cache_read_tokens') == '4' and c.get('output_tokens') == '3' for c in calls), calls
        assert [t['tool'] for t in view['tool_calls']] == ['exec_command'], view['tool_calls']
        stored = (root / '.swarm/telemetry.db').read_bytes()
        assert b'swarm-probe' not in stored, 'tool argument/output never stored'
        checks.update(calls=len(calls), tools=len(view['tool_calls']), latencies=[l['name'] for l in view.get('latencies', [])],
                      unknown_names=len(view.get('unknown', {})), rejected=view.get('rejected'))
        server.process.send_signal(signal.SIGINT); server.stop_sent = True; collector.join()
        assert not errors and len(summaries) == 1, errors
        retained.publish(out / 'result.json', dict(status='PASS', checks=checks, scope='Real codex CLI with a local fake model; not a billed run, not acceptance'))
        print(json.dumps(dict(status='PASS', checks=checks, output=str(out))))
    except BaseException as error:
        traceback.print_exc(); primary = error
    finally:
        model.shutdown()
    native.finish(server, primary)


if __name__ == '__main__':
    main()
