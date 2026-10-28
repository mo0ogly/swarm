#!/usr/bin/env python3
"""Probe of Claude Code's OTLP export, traces included (LIVE-UX, no paid call).

Runs the installed `claude` CLI in print mode against a LOCAL fake Anthropic
Messages API (streaming SSE) and a LOCAL OTLP/JSON sink on the same port.
Temporary HOME and config dir: the user's ~/.claude is never read; the API key
is a fake string, so no account is used and nothing is billed. The fake model
first asks for one Bash tool call (`echo swarm-probe`), then answers.

Output directory (argument 1, must not exist): `sink.jsonl` (one line per
request: path, content type, signal names) and the raw OTLP bodies
`body-<n>-<signal>.json`, for the span/attribute inventory in README.md.
Exit 0 when the CLI exited and at least one trace export arrived; the waits
are on the CLI process and on the observed export, no global watchdog.
"""
import http.server, json, os, subprocess, sys, tempfile, threading, time
from pathlib import Path

out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=False)
log = (out / 'sink.jsonl').open('a')
state = dict(calls=0, traces=0, n=0)
lock = threading.Lock()


def sse(handler, events):
    handler.send_response(200)
    handler.send_header('Content-Type', 'text/event-stream')
    handler.end_headers()
    for name, data in events:
        handler.wfile.write(('event: %s\ndata: %s\n\n' % (name, json.dumps(data))).encode())
    handler.wfile.flush()


def message(blocks, usage):
    start = dict(type='message_start', message=dict(id='msg_probe', type='message', role='assistant', model='claude-sonnet-4-5', content=[],
                 stop_reason=None, stop_sequence=None, usage=dict(input_tokens=usage[0], output_tokens=1, cache_read_input_tokens=usage[1], cache_creation_input_tokens=0)))
    events = [('message_start', start)]
    for i, (block, delta) in enumerate(blocks):
        events += [('content_block_start', dict(type='content_block_start', index=i, content_block=block)),
                   ('content_block_delta', dict(type='content_block_delta', index=i, delta=delta)),
                   ('content_block_stop', dict(type='content_block_stop', index=i))]
    stop = 'tool_use' if blocks[0][0]['type'] == 'tool_use' else 'end_turn'
    events += [('message_delta', dict(type='message_delta', delta=dict(stop_reason=stop, stop_sequence=None), usage=dict(output_tokens=usage[2]))),
               ('message_stop', dict(type='message_stop'))]
    return events


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        size = int(self.headers.get('Content-Length') or 0)
        body = self.rfile.read(size) if size else b''
        path = self.path.split('?')[0]
        with lock:
            state['n'] += 1
            n = state['n']
        if path.startswith('/v1/messages'):
            request = json.loads(body or b'{}')
            log.write(json.dumps(dict(path=path, stream=request.get('stream'), model=request.get('model'))) + '\n'); log.flush()
            if path.endswith('count_tokens'):
                self._json(dict(input_tokens=10)); return
            if not request.get('stream'):
                self._json(dict(id='msg_side', type='message', role='assistant', model=request.get('model'), content=[dict(type='text', text='ok')],
                                stop_reason='end_turn', stop_sequence=None, usage=dict(input_tokens=5, output_tokens=1))); return
            with lock:
                state['calls'] += 1
                first = state['calls'] == 1
            if first:
                sse(self, message([(dict(type='tool_use', id='toolu_probe', name='Bash', input={}),
                                    dict(type='input_json_delta', partial_json=json.dumps(dict(command='echo swarm-probe', description='probe'))))], (20, 0, 5)))
            else:
                sse(self, message([(dict(type='text', text=''), dict(type='text_delta', text='done'))], (10, 4, 3)))
            return
        signal = path.rsplit('/', 1)[-1]
        (out / ('body-%04d-%s.json' % (n, signal))).write_bytes(body)
        names = []
        try:
            d = json.loads(body)
            for rs in d.get('resourceSpans', []):
                for ss in rs.get('scopeSpans', []):
                    names += ['span:' + s['name'] for s in ss.get('spans', [])]
            for rl in d.get('resourceLogs', []):
                for sl in rl.get('scopeLogs', []):
                    names += ['log:' + (r.get('body', {}).get('stringValue') or '?') for r in sl.get('logRecords', [])]
            for rm in d.get('resourceMetrics', []):
                for sm in rm.get('scopeMetrics', []):
                    names += ['metric:' + m['name'] for m in sm.get('metrics', [])]
        except Exception as error:
            names = ['unparsed:' + repr(error)[:80]]
        log.write(json.dumps(dict(path=path, ctype=self.headers.get('Content-Type'), names=names)) + '\n'); log.flush()
        if signal == 'traces':
            with lock:
                state['traces'] += 1
        self._json({})

    do_GET = do_POST

    def _json(self, value):
        raw = json.dumps(value).encode()
        self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(raw))); self.end_headers()
        self.wfile.write(raw)

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
base = 'http://127.0.0.1:%d' % server.server_address[1]
home = Path(tempfile.mkdtemp(prefix='claude-home-', dir=out))
work = Path(tempfile.mkdtemp(prefix='claude-work-', dir=out))
env = dict(PATH=os.environ['PATH'], HOME=str(home), CLAUDE_CONFIG_DIR=str(home / '.claude'),
           ANTHROPIC_BASE_URL=base, ANTHROPIC_API_KEY='sk-ant-fake-not-a-secret', CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC='1',
           CLAUDE_CODE_ENABLE_TELEMETRY='1', CLAUDE_CODE_ENHANCED_TELEMETRY_BETA='1',
           OTEL_TRACES_EXPORTER='otlp', OTEL_LOGS_EXPORTER='otlp', OTEL_METRICS_EXPORTER='otlp',
           OTEL_EXPORTER_OTLP_PROTOCOL='http/json', OTEL_EXPORTER_OTLP_ENDPOINT=base,
           OTEL_METRIC_EXPORT_INTERVAL='1000', OTEL_LOGS_EXPORT_INTERVAL='500', OTEL_BSP_SCHEDULE_DELAY='500',
           OTEL_LOG_USER_PROMPTS='0')
version = subprocess.run(['claude', '--version'], capture_output=True, text=True, env=env).stdout.strip()
run = subprocess.run(['claude', '-p', 'Run echo swarm-probe with Bash, then say done.', '--allowedTools', 'Bash(echo:*)', '--model', 'claude-sonnet-4-5'],
                     cwd=work, env=env, capture_output=True, text=True, stdin=subprocess.DEVNULL)
(out / 'cli.json').write_text(json.dumps(dict(version=version, exit=run.returncode, stdout=run.stdout[-400:], stderr=run.stderr[-800:]), indent=2))
server.shutdown()
print(json.dumps(dict(version=version, exit=run.returncode, model_calls=state['calls'], trace_exports=state['traces'], output=str(out))))
sys.exit(0 if run.returncode == 0 and state['traces'] > 0 else 1)
