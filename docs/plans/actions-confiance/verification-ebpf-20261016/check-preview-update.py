import datetime,hashlib,http.cookiejar,json,os,re,signal,subprocess,tempfile,time,urllib.request
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=repo/'docs/plans/actions-confiance/verification-ebpf-20261016'
old=json.loads((repo/'docs/plans/actions-confiance/verification-referentiels-20261016/controls-preview-process.json').read_text());binary=repo/'bin/swarm-actions-confiance-kernel-public';dest=p/'check-preview-process.json';assert not dest.exists(),'Existing process receipt: inspect and resume it rather than spawning again'
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def ticks(pid):return (Path('/proc')/str(pid)/'stat').read_text().split(') ',1)[1].split()[19]
def stopped(pid):
 try:return (Path('/proc')/str(pid)/'stat').read_text().split(') ',1)[1].split()[0]=='Z'
 except (FileNotFoundError,ProcessLookupError):return True
proc=Path('/proc')/str(old['pid']);assert ticks(old['pid'])==old['proc_start_ticks'];assert [x.decode()for x in(proc/'cmdline').read_bytes().split(b'\0')if x]==old['argv'];assert sha(Path(old['argv'][0]))==old['binary_sha256'];assert sha(binary)==json.loads((p/'check-verification.json').read_text())['binary_sha256']
root=Path(old['fixture_root']);state=root/'.swarm/trust-actions.json';before=sha(state);assert before==old['state_sha256_unchanged'];view=json.loads(subprocess.check_output([str(binary),'--root',str(root),'--json','trust','show']));assert len(view['actions'])==2 and len(view['ledger'])==1
os.kill(old['pid'],signal.SIGTERM)
for _ in range(30):
 if stopped(old['pid']):break
 time.sleep(.1)
else:raise RuntimeError('identified owned preview did not stop')
fd,log=tempfile.mkstemp(prefix='swarm-kernel-preview-',suffix='.log');os.fchmod(fd,0o600);argv=[str(binary),'--root',str(root),'web','127.0.0.1:36525'];child=subprocess.Popen(argv,stdout=fd,stderr=fd,start_new_session=True);os.close(fd)
v={'pid':child.pid,'argv':argv,'proc_start_ticks':None,'binary_sha256':sha(binary),'fixture_root':str(root),'origin':old['origin'],'state_sha256_unchanged':before,'old_process':{'pid':old['pid'],'identity_verified_before_stop':True,'stopped':True},'verification_status':'pending','private_startup_log':log,'browser_reload_observed':False};dest.write_text(json.dumps(v,indent=2)+'\n');v['proc_start_ticks']=ticks(child.pid);dest.write_text(json.dumps(v,indent=2)+'\n')
for _ in range(100):
 raw=Path(log).read_text()
 if '/session/'in raw:break
 assert child.poll()is None,'owned preview exited'
 time.sleep(.1)
else:raise RuntimeError('owned preview startup expired')
session=re.search(r'http://\S+/session/\S+',raw).group(0);opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()));opener.open(session,timeout=5).read();origin=old['origin'];data=json.loads(opener.open(origin+'/api/v1/trust-actions',timeout=5).read());html=opener.open(origin+'/trust-actions.html',timeout=5).read().decode();js=opener.open(origin+'/trust-evidence.js',timeout=5).read();assert 'controls-title'in html and 'connector-form'in html and 'pe-archive-create-form'in html and 'evidence_trusted_checkpoint'in html and 'lab-policy-form'in html and 'lab-result-summary'in html;labjs=opener.open(origin+'/trust-laboratory.js',timeout=5).read();assert hashlib.sha256(labjs).hexdigest()==sha(repo/'web/trust-laboratory.js');assert data['laboratory']=={'enabled':False,'timeout_seconds':30,'max_concurrent_runs':1};assert hashlib.sha256(js).hexdigest()==sha(repo/'web/trust-evidence.js');assert len(data['actions'])==2 and len(data['ledger'])==1 and data['evidence_signing']=={'enabled':False,'key_ttl_seconds':86400};assert sha(state)==before
connectorjs=opener.open(origin+'/trust-connectors.js',timeout=5).read();assert hashlib.sha256(connectorjs).hexdigest()==sha(repo/'web/trust-connectors.js');assert len(data['connector_catalog']['connectors'])==2 and data['access_ledger']==[]
matrixbytes=opener.open(origin+'/trust-controls.json',timeout=5).read();assert hashlib.sha256(matrixbytes).hexdigest()==sha(repo/'web/trust-controls.json');assert json.loads(matrixbytes)['claims']=={'certification':False,'global_compliance':False,'independent_assessment':False}
controlsjs=opener.open(origin+'/trust-controls.js',timeout=5).read();assert hashlib.sha256(controlsjs).hexdigest()==sha(repo/'web/trust-controls.js')
v.update(controls_matrix_verified=True,connector_catalog_verified=True,actions=2,ledger_entries=1,laboratory=data['laboratory'],evidence_signing=data['evidence_signing'],process_policy=data['process_policy'],public_api_html_js_verified=True,verification_status='pending-kernel',at=datetime.datetime.now(datetime.timezone.utc).isoformat());dest.write_text(json.dumps(v,indent=2)+'\n')

kerneljs=opener.open(origin+'/trust-kernel.js',timeout=5).read();assert hashlib.sha256(kerneljs).hexdigest()==sha(repo/'web/trust-kernel.js');assert 'trust-kernel-check' in html;assert data['kernel_check']=={'enabled':False,'timeout_seconds':15,'ring_bytes':65536};assert sha(state)==before;v.update(kernel_check=data['kernel_check'],kernel_assets_verified=True,verification_status='verified');dest.write_text(json.dumps(v,indent=2)+'\n')

print(json.dumps({k:v[k]for k in ['pid','proc_start_ticks','binary_sha256','verification_status']}))
