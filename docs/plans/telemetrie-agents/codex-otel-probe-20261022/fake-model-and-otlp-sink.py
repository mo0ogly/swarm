import http.server, json, sys, threading
log=open(sys.argv[2],'a')
class H(http.server.BaseHTTPRequestHandler):
    def _h(self):
        n=int(self.headers.get('Content-Length') or 0); body=self.rfile.read(n) if n else b''
        names=[]
        try:
            d=json.loads(body)
            for rl in d.get('resourceLogs',[]):
                for sl in rl.get('scopeLogs',[]):
                    for r in sl.get('logRecords',[]):
                        b=r.get('body',{}).get('stringValue'); ev=[a['value'].get('stringValue') for a in r.get('attributes',[]) if a['key']=='event.name']
                        names.append(b or (ev[0] if ev else '?'))
            for rm in d.get('resourceMetrics',[]):
                for sm in rm.get('scopeMetrics',[]):
                    names+=['metric:'+m['name'] for m in sm.get('metrics',[])]
        except Exception as e: names=["err:"+repr(e)[:80]]
        if self.path.endswith("/logs") or self.path.endswith("/metrics") or self.path.endswith("/responses"):
            import os,time; open(os.path.join(os.path.dirname(sys.argv[2]),"body-%d-%s.json"%(time.time_ns(),self.path.rsplit("/",1)[1])),"wb").write(body)
        log.write(json.dumps(dict(path=self.path,auth=self.headers.get('Authorization'),ctype=self.headers.get('Content-Type'),names=names))+'\n');log.flush()
        if self.path.startswith('/v1/responses') or 'chat' in self.path:
            import time; time.sleep(2)
            global CALLS
            CALLS=globals().get('CALLS',0)+1
            if CALLS==1:
                ev=[('response.created',{"type":"response.created","response":{"id":"r0","status":"in_progress","output":[]}}),
                    ('response.output_item.done',{"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc1","call_id":"call1","name":"exec_command","arguments":json.dumps({"cmd":"echo swarm-probe"}),"status":"completed"}}),
                    ('response.completed',{"type":"response.completed","response":{"id":"r0","status":"completed","output":[],"usage":{"input_tokens":20,"input_tokens_details":{"cached_tokens":0},"output_tokens":5,"output_tokens_details":{"reasoning_tokens":0},"total_tokens":25}}})]
                self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
                for n,d in ev: self.wfile.write(('event: %s\ndata: %s\n\n'%(n,json.dumps(d))).encode())
                self.wfile.flush();return
            ev=[('response.created',{"type":"response.created","response":{"id":"r1","status":"in_progress","output":[]}}),
                ('response.output_item.done',{"type":"response.output_item.done","output_index":0,"item":{"type":"message","role":"assistant","id":"m1","status":"completed","content":[{"type":"output_text","text":"hi","annotations":[]}]}}),
                ('response.completed',{"type":"response.completed","response":{"id":"r1","status":"completed","output":[],"usage":{"input_tokens":10,"input_tokens_details":{"cached_tokens":4},"output_tokens":3,"output_tokens_details":{"reasoning_tokens":1},"total_tokens":13}}})]
            self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
            for n,d in ev: self.wfile.write(('event: %s\ndata: %s\n\n'%(n,json.dumps(d))).encode())
            self.wfile.flush();time.sleep(6);return
        self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(b'{}')
    do_POST=_h; do_GET=_h
    def log_message(self,*a):pass
s=http.server.ThreadingHTTPServer(('127.0.0.1',int(sys.argv[1])),H);s.serve_forever()
