#!/usr/bin/env python3
"""Real CLI guard: rejected readiness must never receive raw fixture material."""
import argparse,hashlib,http.server,json,os,subprocess,threading
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
received=[];prepared=[]
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_POST(self):
  self.rfile.read(int(self.headers.get('Content-Length','0')))
  if 'secret-ready' in self.path:prepared.append(True);reply=b'{"ready":false}'
  else:received.append(True);reply=b'{"secret_ref":"unexpected"}'
  self.send_response(200);self.send_header('Content-Length',str(len(reply)));self.end_headers();self.wfile.write(reply)
server=http.server.HTTPServer(('127.0.0.1',0),Handler);thread=threading.Thread(target=server.serve_forever);thread.start()
try:
 binary=a.binary.resolve();env=dict(os.environ,SWARM_WEB_SESSION='session-0123456789abcdef01234567',SWARM_TRUST_TOKEN='admin-token-fixture-012345')
 r=subprocess.run([str(binary),'--server','http://127.0.0.1:'+str(server.server_port),'--json','trust','secret-load','--input','-'],input='public-readiness-fixture',text=True,capture_output=True,env=env,timeout=15)
 result={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'preparation_calls':len(prepared),'ready':False,'raw_sent_after_refusal':bool(received),'exit':r.returncode}
 a.evidence.parent.mkdir(parents=True,exist_ok=True);a.evidence.write_text(json.dumps(result,indent=2)+'\n')
 assert len(prepared)==1 and not received and r.returncode==4 and not r.stdout
 assert json.loads(r.stderr)['failure']['code']=='authorization_required'
 print(json.dumps(result))
finally:server.shutdown();thread.join();server.server_close()
