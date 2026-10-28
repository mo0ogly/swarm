#!/usr/bin/env python3
"""Run current enrollment transport tests and real browser on owned fixtures.

Default Rust profile, no test-fixtures, provider, global watchdog or retry.
The private browser packet travels through stdin and is never retained here.
The declared input snapshot is supplementary to effective compiler provenance.
"""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path

def main():
    """Preserve named outputs and before/after declared sources; stop on failure."""
    parser=argparse.ArgumentParser(description=__doc__);destination=parser.add_mutually_exclusive_group(required=True);destination.add_argument('--output');destination.add_argument('--output-parent',help='Create a fresh owned evidence child under this existing parent');args=parser.parse_args()
    root=Path(__file__).resolve().parent.parent
    if args.output_parent:out=Path(tempfile.mkdtemp(prefix='trust-transport-',dir=Path(args.output_parent).resolve()))
    else:out=Path(args.output).resolve();out.mkdir(parents=True,exist_ok=False)
    sources=[p for pattern in ['rust/Cargo.toml','rust/Cargo.lock','rust/crates/*/Cargo.toml','rust/crates/swarm-engine/src/**/*.rs','rust/crates/swarm-cli/src/**/*.rs','rust/crates/swarm-web/src/**/*.rs','rust/crates/swarm-enrollment-*/src/**/*.rs','rust/crates/*/tests/trust_enrollment.rs','rust/crates/swarm-enrollment-authority/tests/common/authenticated_fixture.rs','web/trust-native.*','web/i18n-en.js','locales/en.json','tests/rust_trust_enrolled_ui.cjs',str(Path(__file__).relative_to(root))] for p in root.glob(pattern) if p.is_file()]
    before={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(root/'.cache/rust-native-version846-target'),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',SWARM_BIBA_BROWSER_OUTPUT=str(out/'browser'))
    commands=[['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-web','--test','trust_enrollment','--','--nocapture'],['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-engine','--lib','root_store::trust_enrollment_call::tests','--','--nocapture'],['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-cli','--test','trust_enrollment','--','--nocapture']]
    commands.append(['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-engine','--test','trust_enrollment','--','--nocapture'])
    (out/'invocation.json').write_text(json.dumps({'commands':commands,'sources_before':before,'global_watchdog':None,'fixture_features':False},indent=2)+'\n')
    results=[]
    for index,command in enumerate(commands):
        with (out/f'{index}.stdout').open('wb') as stdout,(out/f'{index}.stderr').open('wb') as stderr:
            process=subprocess.Popen(command,cwd=root,env=env,stdout=stdout,stderr=stderr)
            print('RUN',index,'pid',process.pid,flush=True);code=process.wait()
        text=(out/f'{index}.stdout').read_text();expected=[3,4,2,3][index]
        result={'step':index,'exit':code,'expected_named_tests':expected,'observed_named_passes':sum(line.startswith('test ') and line.endswith(' ... ok') for line in text.splitlines())};results.append(result)
        assert code==0 and result['observed_named_passes']==expected,result
    after={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources};assert after==before,'declared source changed'
    browser=json.loads((out/'browser/result.json').read_text());assert browser['status']=='PASS' and browser['assertions']>0
    (out/'result.json').write_text(json.dumps({'status':'PASS','steps':results,'browser_assertions':browser['assertions'],'sources_before_after':before,'independent_acceptance':False},indent=2)+'\n');print('TRANSPORT_PASS',flush=True)
if __name__=='__main__':main()
