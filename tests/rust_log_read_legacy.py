#!/usr/bin/env python3
"""Old-settings historical reads without implicit configuration adoption.

Fixture settings are built by the real versioned document constructor, either
compiled from an explicit profile or reused as an explicitly selected, hashed
retained constructor. This constructor is not the production candidate. Only
owned inert Root files change; no provider, live mission, history reset or retry.
"""
import argparse,json,subprocess,traceback
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_agent_exchanges import logical

def run(binary,profile,output,constructor=None):
    """Preserve valid schema15 read, absent/default observation, write refusal.
    Present source with no adopted file cap refuses rather than unsafe read.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'method.py',Path(__file__).read_bytes());
    if constructor is None:
        records=[json.loads(l) for l in (profile.parent/'cargo.stdout').read_text().splitlines() if l.startswith('{')];libs={}
        for name in ['swarm_engine','serde_json']:
            a=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']==name];assert len(a)==1;libs[name]=Path(next(s for s in a[0]['filenames'] if s.endswith('.rlib')))
        source='fn main(){let v:swarm_engine::EngineSettingsValues=serde_json::from_reader(std::io::stdin()).unwrap();assert_eq!(v.schema_version(),15);assert!(v.agent_provider_config_max_bytes.is_none());assert!(v.agent_launch_policy().is_err());let d=swarm_engine::EngineSettingsDocument::initial(v,"fixture-legacy-read".into(),"2026-01-01T00:00:00Z".into()).unwrap();use std::io::Write;std::io::stdout().write_all(&swarm_engine::encode_model(&d).unwrap()).unwrap();}';publish(output/'settings-fixture.rs',source.encode());argv=['rustc','--edition','2021',str(output/'settings-fixture.rs'),'-L','dependency='+str(libs['swarm_engine'].parent),'-o',str(output/'settings-fixture')]
        for name,lib in libs.items():argv+=['--extern',name+'='+str(lib)]
        hashes={k:digest(v.read_bytes()) for k,v in libs.items()};publish(output/'compile-inputs.json',dict(argv=argv,library_hashes=hashes,profile=str(profile),scope='Owned fixture constructor, not production executable'));r=subprocess.run(argv,capture_output=True);publish(output/'compile.stdout',r.stdout);publish(output/'compile.stderr',r.stderr);assert r.returncode==0;assert hashes=={k:digest(v.read_bytes()) for k,v in libs.items()}
    else:
        assert constructor.is_file()
        publish(output/'settings-fixture',constructor.read_bytes());(output/'settings-fixture').chmod(0o700)
        publish(output/'constructor-input.json',dict(path=str(constructor),sha256=digest(constructor.read_bytes()),scope='Previously verified settings fixture constructor only; not current production profile'))
    f=Fixture(binary,output,'legacy-log-read');agent=f.prepare()['agent']['id'];settings=f.root/'.swarm/engine-settings.json';policy=f.root/'.swarm/log-read-policy.json';original=settings.read_bytes();publish(output/'original-settings.json',original);data=json.loads(original);catalogue=json.loads(Path('rust/crates/swarm-engine/config/engine-settings-catalog.json').read_bytes());versions={v['name']:v['introduced_schema_version'] for v in catalogue['fields']};assert all(n in versions for n in data['values']);values={k:v for k,v in data['values'].items() if versions[k]<=15};publish(output/'legacy-values.json',values);r=subprocess.run([str((output/'settings-fixture').resolve())],input=json.dumps(values).encode(),capture_output=True);publish(output/'fixture.stdout',r.stdout);publish(output/'fixture.stderr',r.stderr);assert r.returncode==0;publish(settings,r.stdout);legacy=settings.read_bytes();before=logical(f.root);checks=[]
    rows=f.ok(['agent','logs',agent,'-1']);assert rows and settings.read_bytes()==legacy and not policy.exists();view=f.ok(['run-limits','logs','show']);assert view['source']=='versioned_defaults' and not view['configuration_cap_adopted'] and view['configuration_max_bytes'] is None;page=f.ok(['logs','query',f.work,agent]);assert page['entries']==rows;checks.append('actual-valid-schema15-cli-history-query-and-observed-unadopted-defaults')
    intent=dict(schema_version=1,event_id='legacy-preview',expected_revision=0,reason='Must not implicitly adopt',values=view['values']);code,value=f.call(['run-limits','logs','preview'],intent);assert code==2 and value['failure']['code']=='log-read-file-cap-not-configured';assert settings.read_bytes()==legacy and not policy.exists();checks.append('proposed-save-requires-file-cap-without-implicit-adoption')
    present=dict(schema_version=1,revision=0,values=view['values'],history=[]);publish(output/'unadopted-present-source.json',present);publish(policy,present);raw=policy.read_bytes();code,value=f.call(['agent','logs',agent,'-1']);assert code==2 and value['failure']['code']=='log-read-file-cap-not-configured' and policy.read_bytes()==raw;policy.unlink();assert f.ok(['agent','logs',agent,'-1'])==rows and logical(f.root)==before and settings.read_bytes()==legacy and not list(f.root.rglob('model-received.json'));checks.append('present-private-source-refuses-unbounded-read-then-absent-source-recovers')
    publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),settings_sha256=digest(legacy),schema_version=15,provider_started=False,live_database=False));print('LOG_READ_LEGACY_VERIFIED 3',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile-result',type=Path);p.add_argument('--settings-constructor',type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:
        assert (a.profile_result is None)!=(a.settings_constructor is None),'Exactly one fixture constructor source is required'
        run(a.binary.resolve(),a.profile_result.resolve() if a.profile_result else None,a.output.resolve(),a.settings_constructor.resolve() if a.settings_constructor else None)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
