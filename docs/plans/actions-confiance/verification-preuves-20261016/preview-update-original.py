import datetime,hashlib,http.cookiejar,json,os,re,signal,subprocess,tempfile,time
from pathlib import Path
repo=Path('/home/fpizzi/Documents/ChatGPT/swarm');p=repo/'docs/plans/actions-confiance/verification-preuves-20261016';old=json.loads((repo/'docs/plans/actions-confiance/verification-processus-20261015/process-preview-process.json').read_text());proc=Path('/proc')/str(old['pid'])
def sha(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def ticks(proc):return (proc/'stat').read_text().split(') ',1)[1].split()[19]
def stopped(proc):
 try:return (proc/'stat').read_text().split(') ',1)[1].split()[0]=='Z'
 except (FileNotFoundError,ProcessLookupError):return True
assert ticks(proc)==old['proc_start_ticks'] and [x.decode() for x in (proc/'cmdline').read_bytes().split(b'\0')if x]==old['argv'] and sha(Path(old['argv'][0]))==old['binary_sha256']
binary=repo/'bin/swarm-actions-confiance-evidence-labelled';build=json.loads((p/'evidence-labelled-build.json').read_text());assert sha(binary)==build['binary_sha256'];root=Path(old['fixture_root']);state=root/'.swarm/trust-actions.json';before=sha(state);assert before==old['state_sha256_unchanged']
view=json.loads(subprocess.check_output([str(binary),'--root',str(root),'--json','trust','show']));assert len(view['actions'])==2 and len(view['ledger'])==1 and view['evidence_signing']=={'enabled':False,'key_ttl_seconds':86400};assert sha(state)==before
os.kill(old['pid'],signal.SIGTERM)
for _ in range(30):
 if stopped(proc):break
 time.sleep(.1)
else:raise RuntimeError('owned preview did not stop')
fd,log=tempfile.mkstemp(prefix='swarm-evidence-preview-',suffix='.log');os.fchmod(fd,0o600);argv=[str(binary),'--root',str(root),'web','127.0.0.1:36525'];child=subprocess.Popen(argv,stdout=fd,stderr=fd,start_new_session=True);os.close(fd)
for _ in range(100):
 raw=Path(log).read_text()
 if '/session/'in raw:break
 assert child.poll()is None,'owned preview exited'
 time.sleep(.1)
else:raise RuntimeError('owned preview startup expired')
session=re.search(r'http://\S+/session/\S+',raw).group(0);opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()));opener.open(session,timeout=5).read();origin=old['origin'];data=json.loads(opener.open(origin+'/api/v1/trust-actions',timeout=5).read());html=opener.open(origin+'/trust-actions.html',timeout=5).read().decode();js=opener.open(origin+'/trust-evidence.js',timeout=5).read();process_html=opener.open(origin+'/processes.html',timeout=5).read();assert 'pe-policy-form'in html and 'pe-availability'in html and b'pm-create-form'in process_html;assert hashlib.sha256(js).hexdigest()==sha(repo/'web/trust-evidence.js');assert len(data['actions'])==2 and len(data['ledger'])==1 and data['evidence_signing']=={'enabled':False,'key_ttl_seconds':86400};assert sha(state)==before
v={'pid':child.pid,'proc_start_ticks':ticks(Path('/proc')/str(child.pid)),'argv':argv,'binary_sha256':sha(binary),'origin':origin,'fixture_root':str(root),'old_process':{'pid':old['pid'],'identity_verified':True,'stopped':True},'actions':2,'ledger_entries':1,'state_sha256_unchanged':before,'evidence_signing':data['evidence_signing'],'process_policy':data['process_policy'],'public_api_html_js_verified':True,'browser_reload_observed':False,'private_startup_log':log,'at':datetime.datetime.now(datetime.timezone.utc).isoformat()};(p/'evidence-preview-process.json').write_text(json.dumps(v,indent=2)+'\n');print(json.dumps({k:v[k]for k in ['pid','proc_start_ticks','binary_sha256','state_sha256_unchanged']}))
