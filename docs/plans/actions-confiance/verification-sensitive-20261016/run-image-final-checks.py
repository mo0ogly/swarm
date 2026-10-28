import sys,subprocess,json,hashlib,datetime,os
from pathlib import Path
out=Path('docs/plans/actions-confiance/verification-sensitive-20261016');kind=sys.argv[1]
base=json.loads((out/'image-final-inputs.json').read_text())
commands={'race':['go','test','-race','./internal/engine','-run','^(TestTrustFixedHelper|TestKernelCheck|TestActionsConfianceT4(Secret|Sealed)|TestActionsConfianceT6Laboratory|TestLaboratory)' ,'-count=1','-v'], 'effects':['go','test','-race','./internal/engine','-run','^TestActionsConfiance(T2|T6Connector)','-count=1','-v'],'build':['sh','build.sh','bin/swarm-actions-confiance-fixed-image-qualified'],'manifest':['go','test','-race','./internal/engine','-run','^TestGraphDeliveryD03QualificationManifest$','-count=1','-v'],'vet':['go','vet','./internal/engine','./cmd/swarm']}
cmd=commands[kind];receipt={'command':cmd,'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'inputs':base};env=dict(os.environ,CGO_ENABLED='0') if kind=='vet' else dict(os.environ)
with (out/f'image-final-{kind}.out').open('w') as f:
 p=subprocess.Popen(cmd,stdout=f,stderr=subprocess.STDOUT,env=env);receipt['pid']=p.pid;receipt['process_start']=Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split()[19];(out/f'image-final-{kind}-process.json').write_text(json.dumps(receipt,indent=2)+'\n');code=p.wait()
receipt.update(exit_code=code,finished=datetime.datetime.now(datetime.timezone.utc).isoformat(),changed_inputs=[e['path'] for e in base if hashlib.sha256(Path(e['path']).read_bytes()).hexdigest()!=e['sha256']]);(out/f'image-final-{kind}-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'group':kind,'exit':code,'changed_inputs':receipt['changed_inputs']}));sys.exit(code)
