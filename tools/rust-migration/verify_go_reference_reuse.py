"""Positive, stale-proof refusals and recovery for the exact reuse implementation."""
import copy
import json
import shutil
from reuse_go_reference import reuse

def check(baseline,fresh,old,current,corpus,work):
    root=work/'reuse-guard';root.mkdir(mode=0o700);a=root/'baseline';b=root/'fresh';a.mkdir();b.mkdir()
    baseline_files=['go-vet.stdout','go-vet.stderr','result.json','inputs-after.json','inputs-before.json','commands.json','go-original-mutation-tests.stdout','go-concurrent-writer-race.stdout','command-corpus-go.stdin','command-corpus-go.stdout','build-environment.json','rust-version.stdout','go-version.stdout','cc-version.stdout']
    baseline_files+=['go-env.stdout','go-inputs-normal.stdout','go-inputs-race.stdout']
    for name in baseline_files:shutil.copyfile(baseline/name,a/name)
    for name in ['build-environment.json','rust-version.stdout','go-version.stdout','cc-version.stdout','go-env.stdout','go-inputs-normal.stdout','go-inputs-race.stdout']:shutil.copyfile(fresh/name,b/name)
    reference,_=reuse(a,b,old,current,corpus);assert reference
    extra=copy.deepcopy(current);extra['docs/method-fixture-extra.py']='different-method-input';assert reuse(a,b,old,extra,corpus)[0]==reference
    cases=[]
    def fault(name,change,restore):
        values=copy.deepcopy(current);raw=corpus;raw=change(values,raw)or raw
        try:reuse(a,b,old,values,raw)
        except (AssertionError,KeyError):pass
        else:raise AssertionError('stale proof accepted: '+name)
        restore();assert reuse(a,b,old,current,corpus)[0]==reference;cases.append(name)
    for name in ['internal/engine/store.go','tools/rust-migration/command-reference/main.go','tools/rust-migration/recorded_command_vectors.py','rust/Cargo.toml','rust/Cargo.lock']:
        fault(name,lambda values,raw,name=name:values.__setitem__(name,'stale'),lambda:None)
    fault('corpus',lambda values,raw:raw+b'\n',lambda:None)
    for filename in ['rust-version.stdout','go-version.stdout','cc-version.stdout']:
        original=(b/filename).read_bytes()
        fault(filename,lambda values,raw,filename=filename:(b/filename).write_bytes(b'stale')and None,lambda filename=filename,original=original:(b/filename).write_bytes(original))
    original=(b/'build-environment.json').read_bytes();changed=json.loads(original);changed['RUSTFLAGS']='--cfg stale_proof'
    fault('environment',lambda values,raw:(b/'build-environment.json').write_text(json.dumps(changed))and None,lambda:(b/'build-environment.json').write_bytes(original))
    original=(a/'commands.json').read_bytes();changed=json.loads(original)
    for command in changed:
        if command['label']=='command-corpus-go':command['exit']=2
    fault('failed-original-command',lambda values,raw:(a/'commands.json').write_text(json.dumps(changed))and None,lambda:(a/'commands.json').write_bytes(original))
    original=(a/'go-original-mutation-tests.stdout').read_bytes()
    fault('unexecuted-test',lambda values,raw:(a/'go-original-mutation-tests.stdout').write_bytes(b'[no tests to run]\n')and None,lambda:(a/'go-original-mutation-tests.stdout').write_bytes(original))
    original=(b/'go-env.stdout').read_bytes();changed=json.loads(original);changed['GOFLAGS']='-tags=changed'
    fault('go-environment',lambda values,raw:(b/'go-env.stdout').write_text(json.dumps(changed))and None,lambda:(b/'go-env.stdout').write_bytes(original))
    # A new root embed must participate, even when its path resembles tooling.
    from qualification_inputs import stream
    original=(b/'go-inputs-normal.stdout').read_bytes();items=list(stream(original));root_package=next(p for p in items if p['ImportPath']=='swarm.local/companion');root_package['EmbedFiles'].append('docs/new-runtime-asset.py');modified=b''.join(json.dumps(p).encode()+b'\n'for p in items)
    fault('root-embed-membership',lambda values,raw:(values.__setitem__('docs/new-runtime-asset.py','new'),(b/'go-inputs-normal.stdout').write_bytes(modified),None)[-1],lambda:(b/'go-inputs-normal.stdout').write_bytes(original))
    return dict(status='PASS',valid_control=True,non_go_method_input_not_misclassified=True,refused=cases,recovery_after_each=True,original_proofs_unchanged=True)
