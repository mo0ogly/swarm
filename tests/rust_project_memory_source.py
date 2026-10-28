#!/usr/bin/env python3
"""Verify profile memory guards on an owned inert Root through public CLI/HTTP.

Synthetic legacy Work records are explicit fixtures, never live state or authority.
No model/provider, global watchdog, consumption reset or acceptance is involved.
"""
import argparse, hashlib, json, sqlite3, os, subprocess, tempfile
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native


def seed_work(root, work_id, raw):
    """Use the retained native fixture writer, preserving real identity triggers.

    Only a marked /tmp fixture is allowed. Retain candidate, argv and raw channels;
    the sidecar decodes Work, restricts fields and CAS-checks revision. No
    live state, model, consumption or acceptance is involved; errors propagate.
    """
    root=Path(root).resolve()
    assert root.parent==Path('/tmp') and root.name.startswith('swarm-model83-')
    seeder=Path(os.environ['SWARM_PROJECT_MEMORY_SEEDER']).resolve()
    fixture.publish(root/'.swarm/memory-source-fixture-owner',f'swarm-memory-source-fixture-v1\n{root}\n{work_id}\n'.encode())
    fd,name=tempfile.mkstemp(prefix='memory-source-fixture-',suffix='.json',dir=root/'.swarm')
    with os.fdopen(fd,'wb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())
    candidate=Path(name);argv=[str(seeder),str(root),work_id,str(candidate)]
    fixture.publish(candidate.with_suffix('.argv.json'),argv)
    result=subprocess.run(argv,capture_output=True)
    fixture.publish(candidate.with_suffix('.stdout'),result.stdout);fixture.publish(candidate.with_suffix('.stderr'),result.stderr)
    assert result.returncode==0,('native fixture writer refused',result.returncode,result.stderr.decode())


def seed_lesson(root, work_id, *, imported=False, project=None):
    """Set one synthetic legacy source through the native inert fixture writer.

    Read original Work bytes without mutation, change only fixture memory/RETEX,
    and delegate validation/CAS/identity projection to seed_work. Return original
    bytes for restoration. This is not product version admission or acceptance.
    """
    root=Path(root).resolve()
    with sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        raw=db.execute('SELECT body FROM works WHERE id=?',(work_id,)).fetchone()[0]
    value=json.loads(raw);assert value['id']==work_id
    value['retex']=[dict(id='memory-guard-retex',lesson='.claude/field-guide/g.md')]
    value['project_memory']=[dict(id='memory-guard-entry',imported=imported,versions=[dict(version=1,action='withdraw',source_retex='memory-guard-retex',project_sha256=project or hashlib.sha256(str(root).encode()).hexdigest())])]
    seed_work(root,work_id,json.dumps(value).encode())
    return raw


def refusal(value, reason):
    """Read stable CLI cause codes or the HTTP safe reason, never display text."""
    failure=value.get('failure',{})
    return failure.get('reason')==reason or reason in failure.get('causes',[])

def main():
    """Assert unmanaged admission, managed refusal, aliases and exact recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,mode=0o700,exist_ok=False)
    f=fixture.Fixture(Path(a.binary).resolve(),out,'profile-memory-source');checks=[]
    def check(name,value):
        """Retain each named assertion as soon as its semantic outcome is known."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    guide=f.root/'.claude/field-guide';guide.mkdir(parents=True)
    fixture.publish(f.root/'ordinary.md','Ordinary project instructions')
    fixture.publish(guide/'index.md','Ordinary unmanaged guide')
    fixture.publish(guide/'g.md','Synthetic legacy lesson')
    def request(path):
        """Build an explicit five-role profile without applying native permissions."""
        return dict(expected_sha256='',profile=dict(version=1,name='Memory source fixture',source='custom',instructions=[dict(path=path,roles=['preparation','planner','subplanner','worker','reviewer'])]))
    check('ordinary-source-positive-control',len(f.ok(['project-profile','preview'],request('ordinary.md'))['roles'])==5)
    rc,value=f.call(['project-profile','preview'],request('.claude/field-guide/index.md'))
    if a.expect_unported:
        check('unmanaged-guide-baseline-refuses',rc!=0 and refusal(value,'versioned-memory-source-guard-not-ported'))
        print('GAP_OBSERVED UNMANAGED_GUIDE_UNPORTED',flush=True);return
    check('unmanaged-guide-five-role-admission',rc==0 and len(value['roles'])==5)
    original=seed_lesson(f.root,f.work)
    check('managed-inventory-unreferenced-index-allowed',len(f.ok(['project-profile','preview'],request('.claude/field-guide/index.md'))['roles'])==5)
    for body in ['[lesson](g.md)','[lesson](%67.md)','Historical .claude/field-guide/g.md']:
        fixture.publish(guide/'index.md',body)
        rc,error=f.call(['project-profile','preview'],request('.claude/field-guide/index.md'))
        check('managed-index-refuses-'+str(len(checks)),rc!=0 and refusal(error,'project-source-memory-managed'))
    rc,error=f.call(['project-profile','preview'],request('.claude/field-guide/g.md'))
    check('withdrawn-lesson-remains-managed',rc!=0 and refusal(error,'project-source-memory-managed'))
    (f.root/'alias.md').symlink_to('.claude/field-guide/g.md')
    rc,error=f.call(['project-profile','preview'],request('alias.md'))
    check('physical-alias-cannot-bypass-managed-source',rc!=0 and refusal(error,'project-source-memory-managed'))
    fixture.publish(guide/'index.md','Ordinary unmanaged guide')
    seed_lesson(f.root,f.work,imported=True)
    check('imported-versions-do-not-own-local-lesson',len(f.ok(['project-profile','preview'],request('.claude/field-guide/g.md'))['roles'])==5)
    seed_lesson(f.root,f.work,project='other-project')
    check('foreign-project-versions-do-not-own-local-lesson',len(f.ok(['project-profile','preview'],request('.claude/field-guide/g.md'))['roles'])==5)
    seed_lesson(f.root,f.work)
    server=native.Server(f.binary,f.root,out,'profile-memory-http',environment=f.environment);primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        check('http-unmanaged-guide-admission',len(server.request('/api/v1/project-profile/preview',request('.claude/field-guide/index.md'),csrf=csrf,observe=False)['roles'])==5)
        blocked=server.request('/api/v1/project-profile/preview',request('alias.md'),csrf=csrf,status=400,observe=False)
        check('http-managed-refusal-safe',blocked['failure'].get('reason')=='project-source-memory-managed' and 'causes' not in blocked['failure'])
        check('http-recovery-after-managed-refusal',len(server.request('/api/v1/project-profile/preview',request('ordinary.md'),csrf=csrf,observe=False)['roles'])==5)
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    seed_work(f.root,f.work,original)
    check('source-preview-never-applied-profile',not(f.root/'swarm.project.json').exists())
    applied=f.ok(['project-profile','apply'],request('.claude/field-guide/index.md'))
    check('unmanaged-guide-applies-under-exclusive-settings-lock',applied['enabled'] and len(applied['roles'])==5 and (f.root/'swarm.project.json').is_file())
    check('applied-guide-reopens-with-five-role-contexts',len(f.ok(['project-profile','show'])['roles'])==5)
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,root=str(f.root),binary_sha256=fixture.digest(f.binary.read_bytes()),provider_calls=0))
    print('PROJECT_MEMORY_SOURCE_PASS '+str(len(checks)),flush=True)

if __name__=='__main__':main()
