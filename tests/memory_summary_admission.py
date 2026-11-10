#!/usr/bin/env python3
"""Actual M18 source flow -> explicit M19 choice -> CLI/API -> scripted worker input."""
import argparse
import copy
import hashlib
import http.cookiejar
import json
import os
from pathlib import Path
import re
import select
import subprocess
import sys
import time
import traceback
import urllib.error
import urllib.request
import uuid


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();binary=args.binary.resolve();out=args.output.resolve()
    out.mkdir(mode=0o700,parents=True,exist_ok=False)
    repo=Path(__file__).resolve().parents[1];root=None;server=None;commands=[];checks=[]
    sha=lambda raw:hashlib.sha256(raw).hexdigest()
    def save(name,value):
        raw=value if isinstance(value,bytes) else (json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode()
        with (out/name).open('xb') as file:os.chmod(file.name,0o600);file.write(raw)
    def tool(argv,code=0):
        result=subprocess.run([sys.executable,*map(str,argv)],cwd=repo,text=True,capture_output=True,timeout=30)
        commands.append({'argv':list(map(str,argv)),'exit_code':result.returncode})
        assert result.returncode==code,(argv,result.returncode,result.stderr)
    def cli(argv,payload=None,code=0):
        command=[str(binary),'--root',str(root),'--json',*argv]+(['--input','-'] if payload is not None else [])
        result=subprocess.run(command,input=json.dumps(payload) if payload is not None else None,text=True,capture_output=True,timeout=20)
        commands.append({'argv':argv,'exit_code':result.returncode})
        assert result.returncode==code,(argv,result.returncode,result.stderr)
        return json.loads(result.stdout if code==0 else result.stderr)
    def prepare(name,packet,annotations):
        save(name+'-source.json',packet);save(name+'-annotations.json',annotations)
        tool(['tools/memory-evaluation/swarm_source.py','--source',out/(name+'-source.json'),'--annotations',out/(name+'-annotations.json'),'--output-dir',out/(name+'-bundle')])
        history=out/(name+'-bundle/history.json')
        tool(['tools/memory-evaluation/summarization.py','--history',history,'--reference-cutoff',4,'--output',out/(name+'-candidate.json')])
        tool(['tools/memory-evaluation/summarization.py','--history',history,'--candidate',out/(name+'-candidate.json'),'--output',out/(name+'-reference.json')])
        tool(['tools/memory-evaluation/admission.py','--source',out/(name+'-source.json'),'--annotations',out/(name+'-annotations.json'),'--reference',out/(name+'-reference.json'),'--reason','Operator selected attributed history; inspect current evidence','--output',out/(name+'-choice.json')])
        return json.loads((out/(name+'-choice.json')).read_text())
    try:
        tool(['tests/memory_summary_source.py',binary,out/'source-flow'])
        root=Path(json.loads((out/'source-flow/fixture.json').read_text())['root'])
        save('fixture.json',{'root':str(root),'binary':str(binary)})
        source=json.loads((out/'source-flow/source-current.json').read_text())
        annotations=json.loads((out/'source-flow/annotations-current.json').read_text())
        # One historical answer deliberately remains outside the protected prefix.
        annotations['protected']=[a for a in annotations['protected'] if a['id']!='prior']
        choice=prepare('first',source,annotations)
        checks.append('M18 actual persisted source flow passes; verified M17 reference prepares a declarative choice without source text')
        provider_dir=root/'worker';provider_dir.mkdir();provider=provider_dir/'claude'
        provider.write_text('#!'+sys.executable+'\n'+'''import json,os,pathlib,sys
p=pathlib.Path(__file__)
if "--swarm-preflight" in sys.argv:
 print(json.dumps({"schema_version":1,"capabilities":{"process":"verified","workspace_read":"verified","workspace_write":"verified"}}));sys.exit(0)
raw=sys.stdin.read()
with p.with_suffix(".prompt").open("x") as f:os.chmod(f.name,0o600);f.write(raw)
with p.with_suffix(".calls").open("a") as f:f.write("call\\n")
print(json.dumps({"type":"result","result":"Owned worker fixture completed; no acceptance claimed","usage":{"input_tokens":10,"output_tokens":2}}))
''');provider.chmod(0o700)
        config_path=root/'.swarm/providers.json';config=json.loads(config_path.read_text())
        config['providers']['consumer']={'command':str(provider),'preflight_args':['--swarm-preflight'],'preflight_kind':'provider-context','preflight_capabilities':['process','workspace_read','workspace_write']}
        config_path.write_text(json.dumps(config))
        work=cli(['work','show',json.loads(source['payload'])['scope']['work']])['work'];before=copy.deepcopy(work)
        launch={'schema_version':1,'event_id':str(uuid.uuid4()),'expected_revision':work['revision'],'task_id':'t1','provider':'consumer','role':'worker','capture_output':True,'summary_selection':choice}
        preview=cli(['agent','context-preview',work['id']],launch)['agent'];save('preview-cli.json',preview)
        assert cli(['work','show',work['id']])['work']==before and not provider.with_suffix('.calls').exists()
        assert 'work_criteria' not in preview['prompt'] and preview['context']['summary']['operator_snapshot_injected'] is False
        assert len(preview['context']['summary']['omitted_message_ids'])==1
        checks.append('CLI preview is read-only, reports one omitted original and never injects the operator task snapshot')
        server=subprocess.Popen([str(binary),'--root',str(root),'web','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        announced=b'';match=None;deadline=time.monotonic()+15
        while match is None and time.monotonic()<deadline:
            assert server.poll() is None
            ready,_,_=select.select([server.stdout],[],[],.1)
            if ready:announced+=os.read(server.stdout.fileno(),4096);match=re.search(r'http://\S+/session/\S+',announced.decode())
        assert match,'No fixture session address'
        address=match.group();base=address.split('/session/',1)[0]
        opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        opener.open(address,timeout=10).read();csrf=json.load(opener.open(base+'/api/v1/session',timeout=10))['csrf']
        def api_request(value,want=200):
            request=urllib.request.Request(base+'/api/v1/action',data=json.dumps(value).encode(),headers={'Origin':base,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'},method='POST')
            try:response=opener.open(request,timeout=10);status=response.status;raw=response.read()
            except urllib.error.HTTPError as error:status=error.code;raw=error.read()
            assert status==want,(status,raw.decode())
            return json.loads(raw)
        def api_launch(kind,value):
            return {**{k:v for k,v in value.items() if k not in ['schema_version','task_id']},'kind':kind,'work':work['id'],'task':value['task_id']}
        api_preview=api_request(api_launch('agent-context-preview',launch));save('preview-api.json',api_preview)
        assert api_preview['prompt']==preview['prompt'] and api_preview['context']['sha256']==preview['context']['sha256']
        cli(['agent','start',work['id']],launch,2)
        changed_reason=copy.deepcopy(launch);changed_reason['context_hash']=preview['context']['sha256'];changed_reason['summary_selection']['reason']='Changed after preview'
        api_request(api_launch('start',changed_reason),400)
        assert cli(['work','show',work['id']])['work']==before and not provider.with_suffix('.calls').exists()
        checks.append('Authenticated API preview equals CLI; missing confirmation and changed selection reason refuse without launch or mutation')
        changed=cli(['task','update',work['id']],{'schema_version':1,'event_id':str(uuid.uuid4()),'expected_revision':work['revision'],'id':'t1','criteria':['Current contract changed after preview; recheck before consuming']})['work']
        launch['context_hash']=preview['context']['sha256'];api_request(api_launch('start',launch),409)
        assert not provider.with_suffix('.calls').exists()
        payload=json.loads(source['payload']);q={'schema_version':1,'work':work['id'],'task':'t1','expected_revision':changed['revision'],'dialogues':choice['dialogues']}
        fresh=cli(['memory','summary-source'],q);annotations['source_sha256']=fresh['sha256'];choice=prepare('current',fresh,annotations)
        checks.append('Public criteria update makes prior confirmation stale; no worker invocation occurs and current sources are prepared again')
        launch['expected_revision']=changed['revision'];launch['summary_selection']=choice;launch.pop('context_hash')
        preview=cli(['agent','context-preview',work['id']],launch)['agent'];save('preview-current.json',preview)
        launch['context_hash']=preview['context']['sha256'];started=api_request(api_launch('start',launch));save('started.json',started)
        deadline=time.monotonic()+20;agent=None
        while time.monotonic()<deadline:
            agent=cli(['agent','show',started['id']])['agent']
            if agent['status'] in ['completed','failed','interrupted']:break
            time.sleep(.1)
        assert agent['status']=='completed',(agent['status'],agent.get('activity'))
        save('agent-after-reopen.json',agent);received=provider.with_suffix('.prompt').read_bytes();save('provider-input.txt',received)
        assert received.decode()==agent['prompt'] and sha(received)==agent['context']['sha256']
        receipt=agent['context']['summary'];assert receipt['checked_before_start_at'] and receipt['checked_source_sha256']==receipt['queued_source_sha256']
        assert agent['prepared_context']['summary'].get('checked_before_start_at') is None
        assert len(receipt['received_message_ids'])==3 and len(receipt['omitted_message_ids'])==1
        assert 'work_criteria' not in agent['prompt'] and provider.with_suffix('.calls').read_text()=='call\n'
        reference=json.loads((out/'current-reference.json').read_text());block=json.loads(agent['prompt'].split('\nEXTRAITS DE CADRAGE — ',1)[1].split('\n',1)[1].split('\n',1)[0])
        expected=[e for e in reference['summary_entries'] if e['message']!='current-task-snapshot']
        assert [(e['message'],e['text'],e['kind'],e['state']) for e in block['entries']]==[(e['message'],e['text'],e['kind'],e['state']) for e in expected]
        assert [e['text'] for e in block['recent_messages']]==[e['content'] for e in reference['recent_messages']]
        assert api_request(api_launch('start',launch))['id']==agent['id'] and provider.with_suffix('.calls').read_text()=='call\n'
        current=cli(['work','show',work['id']])['work'];save('work-after.json',current)
        target=next(t for t in current['tasks'] if t['id']=='t1');assert target['status']!='accepted' and 'summary_selection' not in target['launch_profile']
        for old in before['tasks']:
            if old.get('brainstorm'):assert old==next(t for t in current['tasks'] if t['id']==old['id'])
        assert (root/'claude.calls').read_text()=='call\ncall\n'
        checks.append('Actual API launch checks queued sources; scripted worker receives exact persisted prompt and reference quotes; replay, originals, profile and acceptance boundaries hold')
        save('result.json',{'status':'PASS','checks':checks,'m18_public_checks':5,'source_scripted_invocations':2,'worker_scripted_invocations':1,'model_calls':0,'root':str(root),'binary_sha256':sha(binary.read_bytes()),'script_sha256':sha(Path(__file__).read_bytes())})
        print(json.dumps({'status':'PASS','checks':len(checks),'m18_public_checks':5,'scripted_invocations':3,'model_calls':0}))
    except Exception:
        save('failure.json',{'root':str(root),'checks':checks,'error':re.sub(r'/session/[^\s/]+','/session/[private]',traceback.format_exc())})
        raise
    finally:
        save('commands.json',commands)
        if server:
            server.terminate()
            try:server.wait(timeout=5)
            except subprocess.TimeoutExpired:server.kill();server.wait(timeout=5)


if __name__=='__main__':main()
