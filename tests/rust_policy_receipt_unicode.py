"""Go/native UTF16 wire and client confirmation regression, private Roots only.

No live mission/provider/global deadline. JS component proof stays separate from
actual embedded-browser qualification; original raw Go/native replies retained.
"""
from pathlib import Path
import sys,json,subprocess,tempfile,os
sys.path.insert(0,'tests');from rust_agent_model_route import Fixture,publish,digest
import argparse
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=a.binary.resolve();publish(out/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,out,'unicode');raw=b'{"schema_version":1,"event_id":"unicode","expected_revision":0,"values":{"general_ms":0,"snapshot_ms":0,"connection_test_ms":0},"reason":"Owned \\ud800 unicode"}'
pv=f.ok(['run-limits','web-requests','preview'],raw);token=pv['preview_token'];request=json.loads(raw);request['preview_token']=token;transport=raw[:-1]+b',"preview_token":'+json.dumps(token).encode()+b'}';reply=f.ok(['run-limits','web-requests','apply'],transport);assert reply['config']['history'][0]['change']['reason']=='Owned \ufffd unicode'
go=a.go_binary.resolve();root=Path(tempfile.mkdtemp(prefix='swarm-unicode-go-'));env={'PATH':str(root),'USER':'fixture','LANG':'C.UTF-8'}
for argv in [[str(go),'init','--root',str(root),'--json']]:
 r=subprocess.run(argv,env=env,capture_output=True);assert r.returncode==0;publish(out/'go-init.stdout',r.stdout);publish(out/'go-init.stderr',r.stderr)
p=out/'go.input.json';publish(p,raw);r=subprocess.run([str(go),'run-limits','web-requests','apply','--input',str(p),'--root',str(root),'--json'],env=env,capture_output=True);publish(out/'go-apply.stdout',r.stdout);publish(out/'go-apply.stderr',r.stderr);assert r.returncode==0;assert json.loads(r.stdout)['config']['history'][0]['change']['reason']=='Owned \ufffd unicode'
script="require('./web/native-policy-receipt.js');let s='';process.stdin.on('data',b=>s+=b);process.stdin.on('end',()=>{const p=JSON.parse(s);try{const proof=SwarmPolicyReceipt.verifyMutation(p.reply,p.request);const readback=SwarmPolicyReceipt.verifyReadback(p.view,proof);process.stdout.write(JSON.stringify({accepted:true,readback}))}catch(e){process.stdout.write(JSON.stringify({accepted:false,code:e.message}))}})"
r=subprocess.run(['node','-e',script],input=json.dumps(dict(reply=reply,request=request,view=f.ok(['run-limits','web-requests','show']))).encode(),capture_output=True);assert r.returncode==0;value=json.loads(r.stdout);assert value==dict(accepted=True,readback=dict(newer=False,replayed=False));publish(out/'guard.json',value);publish(out/'node-input.json',json.dumps(dict(reply=reply,request=request,view=f.ok(['run-limits','web-requests','show']))).encode());publish(out/'result.json',dict(status='PASS',go_and_native_normalize_utf16=True,known_native_apply=True,client_confirmed=True,binary_sha256=digest(binary.read_bytes()),go_root=str(root)));print('POLICY_RECEIPT_UNICODE_VERIFIED 2')
