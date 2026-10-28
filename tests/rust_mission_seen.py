#!/usr/bin/env python3
"""Historical mission seen alias on an owned inactive Root, no provider call.

A valid native visit is the control before the unported witness. Compare the
actual bookmark to Go's Visit JSON contract; preserve all nonbookmark SQL and
settings. Output failure must retain the recorded auxiliary effect. No global
watchdog, raw process signal, SQL mutation or live database is used.
"""
import argparse,json,os,sqlite3,subprocess,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest

def run(binary,output,unported):
    """Exercise actual CLI success/refusal/publication and retain each original."""
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes())
    f=Fixture(binary,output,'mission-seen');checks=[];calls=[]
    def call(args,*,machine=True,lang='fr',value=None):
        """Retain argv/stdout/stderr, never infer absence of mutation from Err."""
        argv=[str(binary),*args,'--root',str(f.root),'--lang',lang]
        if machine:argv+=['--json']
        if value is not None:argv+=['--input','-']
        done=subprocess.run(argv,input=None if value is None else json.dumps(value).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=f.environment)
        n=len(calls);publish(output/f'call-{n}.stdout',done.stdout);publish(output/f'call-{n}.stderr',done.stderr);calls.append(dict(argv=argv,exit=done.returncode));publish(output/'commands.json',calls)
        return done
    def sql(exclude_visit=False):
        """Observe all fixture tables readonly, excluding only the bookmark rows."""
        with sqlite3.connect((f.root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True) as c:
            names=[r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
            return {n:list(c.execute('SELECT * FROM "'+n.replace('"','""')+'" ORDER BY rowid')) for n in names if not(exclude_visit and n=='session_visits')}
    def check(name,condition):
        assert condition,name
        checks.append(name);print('PASS '+name,flush=True);publish(output/'checks.json',checks)
    native=f.ok(['visit','mark',f.work]);check('native-visit-valid-control',native['saved'] is True and native['state']['exists'] is True)
    before=sql();business=sql(True);settings=(f.root/'.swarm/engine-settings.json').read_bytes();done=call(['mission','seen',f.work])
    if unported:
        check('unported-seen-refuses',done.returncode!=0)
        check('unported-no-mutation',sql()==before)
        publish(output/'result.json',dict(status='GAP_OBSERVED',checks=checks,binary_sha256=digest(binary.read_bytes())));print('MISSION_SEEN_GAP 3',flush=True);return
    check('historical-json-contract',done.returncode==0 and not done.stderr and set(json.loads(done.stdout))=={'revision','at','operator'} and type(json.loads(done.stdout)['revision']) is int)
    observed=json.loads(done.stdout);state=f.ok(['visit','show',f.work]);check('exact-public-bookmark',state['visit']==observed and str(observed['revision'])==state['work_revision'] and bool(observed['at']) and bool(observed['operator']))
    for lang,text in [('fr','Révision marquée comme vue.\n'),('en','Revision marked as seen.\n')]:
        done=call(['mission','seen',f.work],machine=False,lang=lang);check('plain-'+lang,done.returncode==0 and done.stdout.decode()==text and not done.stderr)
    revision=f.ok(['work','show',f.work])['work']['revision'];f.ok(['task','add',f.work],dict(schema_version=1,event_id='seen-new-task',expected_revision=revision,id='seen-task',title='New revision',deliverable='Recorded task',criteria=['No execution']))
    business=sql(True);done=call(['mission','seen',f.work]);state=f.ok(['visit','show',f.work]);check('marks-current-revision',done.returncode==0 and json.loads(done.stdout)==state['visit'] and state['work_revision']==str(revision+1))
    stable=sql()
    for args,value in [(['mission','seen','unknown-work'],None),(['mission','seen',f.work],{}),(['mission','seen'],None),(['mission','seen','../outside'],None),(['mission','seen',f.work,'--output',str(output/'forbidden-output')],None),(['mission','seen',f.work,'--server'],None),(['mission','seen',f.work,'--task','forged-task'],None),(['mission','seen',f.work,'--phase','entry'],None)]:
        done=call(args,value=value);check('refusal-'+str(len(checks)),done.returncode!=0 and sql()==stable)
    # Only our pipe loses its reader. The process must report the already-saved
    # bookmark and the public read must agree; an IO error is not rollback.
    reader,writer=os.pipe();os.close(reader)
    try:done=subprocess.run([str(binary),'mission','seen',f.work,'--root',str(f.root),'--json'],stdout=writer,stderr=subprocess.PIPE,env=f.environment,stdin=subprocess.DEVNULL)
    finally:os.close(writer)
    publish(output/'broken-output.stderr',done.stderr);error=json.loads(done.stderr)
    check('recorded-effect-after-output-failure',done.returncode!=0 and any(e['kind']=='session_visit' and e['state']=='recorded' and e['work_id']==f.work for e in error['auxiliary_effects']))
    state=f.ok(['visit','show',f.work]);check('output-failure-readback',state['exists'] and state['exact_visit_revision']==str(revision+1))
    check('business-settings-preserved',sql(True)==business and (f.root/'.swarm/engine-settings.json').read_bytes()==settings)
    publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),provider_started=False,live_database=False));print('MISSION_SEEN_VERIFIED '+str(len(checks)),flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.is_dir():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
