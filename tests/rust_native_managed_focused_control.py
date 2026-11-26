"""Exercise the normal native CLI against an isolated scoped control and receipt."""
from pathlib import Path
import json
import os
import sys

source, binary, profile, out = map(Path, sys.argv[1:5])
sys.path.insert(0, str(source/'tests'))
import rust_native_managed_validation as native
out.mkdir(mode=0o700)
script = """from pathlib import Path
assert Path('tracked.txt').read_bytes()==b'Committed planning input\\n'
assert Path('declared.txt').read_bytes()==b'Second declared source\\n'
try:
    Path('unrelated.txt').read_bytes()
except PermissionError:
    pass
else:
    raise AssertionError('undeclared candidate input readable')
print('FOCUSED-CONTROL-EXECUTED')
"""
controls = [dict(id="read-saved-inputs",command=["python3","-c",script],criteria=[1],
    inputs=["tracked.txt","declared.txt"],justification="Check two declared fixture inputs",review_output=True)]
entries = {"declared.txt":b"Second declared source\n","unrelated.txt":b"Unrelated fixture source\n"}
native.managed.worker.subreaper_scope(
    lambda:native.managed.run(binary,profile,out/'workers',project_subdir='app',controls=controls,source_entries=entries),
    lambda error:None,native.managed.parent_death.subreaper)
birth = json.loads((out/'workers/fixture.json').read_bytes())
root = Path(birth['root'])
work = birth['work']
env = {**os.environ,'CODEX_HOME':str(root/'codex-home')}
def call(label,args,value=None,success=True):
    code, result = native.invoke(binary,root,out,label,args,value,environment=env)
    assert (code == 0) == success, (label, result)
    return result
def show(label):
    return call(label,['work','show',work])['work']
def request(version,label,revision):
    return dict(schema_version=version,event_id=label,expected_revision=revision,task_id='task')
current = show('initial')
mission = call('mission',['mission','show',work])
if mission['paused']:
    call('mission-resume',['mission','resume',work],dict(schema_version=1,event_id='resume-mission',
      expected_revision=current['revision'],reason='Isolated fixture scoped controls'))
current=show('plan-before')
if current['planning'].get('paused'):
    call('plan-resume',['planning','resume',work],dict(schema_version=1,event_id='resume-planning',
      expected_revision=current['revision'],reason='Isolated fixture scoped controls'))
current=show('ready')
rev=current['revision']
prepare=call('prepare',['task','validation-candidate',work],request(1,'prepare',rev))
storage=Path(current['planning']['repository']['storage'])
matching=[json.loads(path.read_bytes()) for path in (storage/'candidates').glob('*/candidate.json')
          if json.loads(path.read_bytes())['task']=='task']
assert len(matching)==1
checkpoint=matching[0]
workspace=Path(checkpoint['workspace'])
preview=call('preview',['task','validation-preview',work],request(2,'preview',rev))
assert preview['control_count']==1 and preview['declared_candidate_inputs']==2
assert len(preview['artifacts'])==3 and not preview['execution_authorized']
print('PASS scoped-manifest-two-inputs-and-retained-report',flush=True)
file=workspace/'declared.txt'
original=file.read_bytes()
try:
    file.write_bytes(b'Changed fixture input')
    refused=call('drift',['task','validation-preview',work],request(2,'drift',rev),success=False)
    assert 'task-validation-focused-input-not-committed' in json.dumps(refused)
finally:
    file.write_bytes(original)
assert show('after-drift')==current
print('PASS changed-declared-input-refused-without-work-mutation',flush=True)
execution=request(2,'execute',rev)
result=call('execute',['task','validate',work],execution)
assert result['controls_passed'] and not result['task_accepted']
print('PASS focused-engine-executed-read-only-control',flush=True)
replay=call('replay',['task','validate',work],execution)
assert replay['replayed'] and replay['job_id']==result['job_id']
review=call('review-inputs',['task','review-inputs',work],
    request(1,'review-inputs',result['work']['revision']))
assert review['preview']['native_control_evidence_verified'] and not review['preview']['provider_started']
assert review['managed_protocol']['context']['candidate_commit']==checkpoint['commit']
print('PASS focused-receipt-reopens-for-independent-review',flush=True)
