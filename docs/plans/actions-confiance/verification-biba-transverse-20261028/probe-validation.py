"""Public before-worker/current-control handoff on two owned synthetic Roots.
A valid same-runner control precedes an I0 report-code fault. No accepted task,
live database, real account/provider or original secret is read or modified.
"""
import sys,json,os
from pathlib import Path
sys.path.insert(0,str(Path('tests').resolve()))
import rust_native_task_validation as native
import rust_manual_mission as mission
base=Path(__file__).resolve().parent;out=base/'validation-before';out.mkdir(exist_ok=False)
before=base/'baseline-image';after=Path(json.loads((base/'candidate-fioclex/result.json').read_text())['binary'])
control="""from pathlib import Path
text=Path('handoff.md').read_text()
code=text.split('\\nFIXTURE_CODE\\n',1)[1].split('\\nEND_FIXTURE_CODE\\n',1)[0]
exec(compile(code,'<owned_i0_report>','exec'),{})
"""+native.CONTROL
results=[]
for label,payload in [('valid',"print('OWNED_LOW_INPUT_OK')"),('fault',"from pathlib import Path\nwith Path('.swarm/biba-validation-canary').open('xb') as f:f.write(b'OWNED_LOW_I0_CONTROL')")]:
 directory=out/label;directory.mkdir();f=native.fixture.Fixture(before,directory,label)
 marker="os.replace(name,'handoff.md')";assert mission.PROGRAM.count(marker)==1
 suffix='\nFIXTURE_CODE\n'+payload+'\nEND_FIXTURE_CODE\n'
 program=mission.PROGRAM.replace(marker,marker+"\nwith open('handoff.md','a') as fixture_file:fixture_file.write("+repr(suffix)+")")
 native.fixture.publish(f.root/'codex',program.encode());(f.root/'codex').chmod(0o700)
 w=f.ok(['work','show',f.work])['work']
 policy=dict(mode='human',controls=[dict(id='owned-report-test',command=['python3','-c',control],criteria=[1],inputs=['handoff.md'],review_output=True,justification='Run owned report test code, never accept the task')])
 w=f.ok(['task','update',f.work],dict(schema_version=1,event_id='authorize',expected_revision=w['revision'],id='task',validation_policy=policy))['work']
 f.request['expected_revision']=w['revision'];prepared=f.prepare();f.run_and_assert(prepared);mission.submit(f)
 f.binary=after;w=f.ok(['work','show',f.work])['work'];canary=f.root/'.swarm/biba-validation-canary';assert not canary.exists()
 request=dict(schema_version=1,event_id='control',expected_revision=w['revision'],task_id='task')
 preview=f.ok(['task','validation-preview',f.work],request);assert preview['authority_mode']=='operator-task-policy' and not preview['execution_authorized']
 result=f.ok(['task','validate',f.work],request);assert result['controls_passed'] and not result['task_accepted']
 observed=canary.exists();assert observed==(label=='fault')
 if observed:assert canary.read_bytes()==b'OWNED_LOW_I0_CONTROL'
 proof=dict(label=label,root=str(f.root),private_write=observed,controls_passed=True,accepted=False,before_sha256=native.fixture.digest(before.read_bytes()),after_sha256=native.fixture.digest(after.read_bytes()))
 native.fixture.publish(directory/'result.json',proof);results.append(proof);print('OBSERVED '+label+' private_write='+str(observed),flush=True)
native.fixture.publish(out/'result.json',dict(status='REPRODUCED_CONTROL_WRITE_UP',cases=results,scope='Owned synthetic report code; no real state/secret or acceptance'))
