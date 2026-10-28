import sys,json,sqlite3,tempfile,subprocess,pathlib,datetime,uuid
binary=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); results=[]
for tail in [('agent','resume-dialogue-preview','absent'),('mission','recovery','WORK','absent'),('mission','changes','WORK'),('mission','spending','WORK')]:
 root=pathlib.Path(tempfile.mkdtemp(prefix='swarm-m14-storage-preview-'))
 def cli(args,body=None): return subprocess.run([str(binary),'--root',str(root),*args],input=json.dumps(body) if body else None,text=True,capture_output=True)
 r=cli(['init']); assert r.returncode==0,r.stderr
 request={'schema_version':1,'event_id':str(uuid.uuid4()),'expected_revision':0,'title':'Storage preview fixture','objective':'Read without implicit upgrade','scope':'Isolated fixture','criteria':['No implicit migration'],'next':'Inspect'}
 r=cli(['--json','work','create','--input','-'],request); assert r.returncode==0,(r.stdout,r.stderr)
 data=json.loads(r.stdout);work=data.get('work',data)['id']; db=root/'.swarm/state.db'
 with sqlite3.connect(db) as conn: conn.execute('PRAGMA user_version=34')
 args=[work if x=='WORK' else x for x in tail]
 r=cli(['--json',*args])
 with sqlite3.connect(db) as conn: version=conn.execute('PRAGMA user_version').fetchone()[0]
 passed=r.returncode==2 and 'storage_upgrade_required' in r.stdout+r.stderr and version==34
 results.append({'argv':[str(binary),'--root',str(root),'--json',*args],'exit_code':r.returncode,'stdout':r.stdout,'stderr':r.stderr,'version_before':34,'version_after':version,'backup_count':len(list((root/'.swarm').glob('state-pre-v35-*.db'))),'result':'PASS' if passed else 'FAIL'})
out.write_text(json.dumps({'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'cases':results,'model_calls':0},ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'result':'PASS' if all(x['result']=='PASS' for x in results) else 'FAIL','cases':len(results),'versions_after':[x['version_after'] for x in results]}))
sys.exit(0 if all(x['result']=='PASS' for x in results) else 1)
