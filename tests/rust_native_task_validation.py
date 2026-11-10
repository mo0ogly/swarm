#!/usr/bin/env python3
"""Execute preauthorized native task controls after a fully public worker flow.

No SQL seeding, Go, paid provider, automatic mission, reviewer or acceptance.
The actual control writes a create-only witness and output. Original command
responses, helper/source/default-binary hashes and failures remain retained.
"""
import argparse,json,sys,subprocess,traceback,hashlib,os
from pathlib import Path
import rust_agent_model_route as fixture
import rust_planning_worker_public_flow as public

CONTROL="""from pathlib import Path
import os,json,re,hashlib
raw=Path('handoff.md').read_bytes()
assert re.search(rb'Input SHA256: [0-9a-f]{64}',raw)
with Path('validation-executed.json').open('x') as f:
 json.dump(dict(pid=os.getpid(),ppid=os.getppid(),cwd=os.getcwd(),report_sha256=hashlib.sha256(raw).hexdigest()),f)
 f.flush();os.fsync(f.fileno())
print('CONTROL-PROBE-SUCCESS',flush=True)
"""

def invoke(binary,root,out,name,args,request=None,environment=None):
    """Retain public argv/channels without retry; optional caller environment
    selects an owned fixture cache. Only explicit CODEX_HOME override is recorded,
    never the inherited environment or credentials; None preserves prior behavior.
    """
    cmd=[str(binary),*args,'--root',str(root),'--json']
    if request is not None:
        p=out/(name+'.input.json');fixture.publish(p,request);cmd+=['--input',str(p)]
    fixture.publish(out/(name+'.command.json'),dict(argv=cmd,global_watchdog=None,explicit_fixture_cache={'CODEX_HOME':environment['CODEX_HOME']} if environment is not None and 'CODEX_HOME' in environment else None))
    r=subprocess.run(cmd,capture_output=True,stdin=subprocess.DEVNULL,env=environment);fixture.publish(out/(name+'.stdout'),r.stdout);fixture.publish(out/(name+'.stderr'),r.stderr)
    raw=r.stdout if r.returncode==0 else r.stderr
    return r.returncode,json.loads(raw)

def scenario(binary,out):
    """Require actual execution, pending review and replay without another child.

    The control was explicitly authorized before decomposition. A successful
    controller result is not independent review or acceptance. No invented PID,
    duration, zero price or replacement production attempt is permitted.
    """
    setup=public.flow(binary,out,control_script=CONTROL);root=Path(setup['root']);work=setup['work_id'];assert not (root/'validation-executed.json').exists()
    request=dict(schema_version=1,event_id='native-validation',expected_revision=setup['revision'],task_id='task')
    code,result=invoke(binary,root,out,'validation',['task','validate',work],request)
    assert code==0,('native task validation not implemented or refused',result)
    assert result['controls_passed'] and result['task_accepted'] is False and not result['replayed']
    witness=json.loads((root/'validation-executed.json').read_bytes());assert witness['pid']>0 and witness['ppid']>0 and witness['cwd']==str(root) and witness['report_sha256']==fixture.digest((root/'handoff.md').read_bytes())
    code,shown=invoke(binary,root,out,'after',['work','show',work]);assert code==0;task=shown['work']['tasks'][0];receipt=task['automatic_validation'];assert task['status']=='submitted' and receipt['state']=='pending_review' and receipt['attempt_id']==setup['attempt_id'] and len(task['attempts'])==1
    controls=receipt['controls'];assert len(controls)==1 and controls[0]['executed'] and controls[0]['passed'] and controls[0]['exit_code']==0 and controls[0]['review_output']=='CONTROL-PROBE-SUCCESS\n'
    assert receipt['artifacts'][receipt['receipt']]==fixture.digest((root/receipt['receipt']).read_bytes())
    before=(root/'validation-executed.json').read_bytes();code,replayed=invoke(binary,root,out,'replay',['task','validate',work],request);assert code==0 and replayed['replayed'] and (root/'validation-executed.json').read_bytes()==before
    return dict(status='PASS',root=str(root),actual_control=True,replay_without_execution=True,task_accepted=False,independent_review=False)

def main():
    """Keep stable helper/default-binary inputs and actual failure evidence."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve()
    sources=list(dict.fromkeys([Path(__file__),Path(public.__file__),Path(fixture.__file__),Path(public.feedback.__file__),Path(public.mission.__file__),Path(public.feedback.native.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [public.request,public.feedback.fresh_work,public.feedback.ledger,public.feedback.apply_policy]]));inputs=dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(p):fixture.digest(p.read_bytes()) for p in sources});fixture.publish(out/'inputs.json',inputs)
    try:
        result=scenario(binary,out);assert fixture.digest(binary.read_bytes())==inputs['binary_sha256'] and all(fixture.digest(Path(p).read_bytes())==h for p,h in inputs['sources'].items());fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise

if __name__=='__main__':main()
