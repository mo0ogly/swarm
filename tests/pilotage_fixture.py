"""Isolated browser corpus. Historical agent rows are fixtures, never live processes."""
import json, sqlite3, subprocess, tempfile, uuid, sys, hashlib
from pathlib import Path
root=Path(tempfile.mkdtemp(prefix='swarm-pilotage-corpus-')); binary=sys.argv[1]
def call(*args, data=None):
    p=subprocess.run([binary,'--root',str(root),'--json',*args,*(['--input','-'] if data is not None else [])],input=json.dumps(data) if data else None,text=True,capture_output=True,check=True)
    return json.loads(p.stdout)
def mutate(*args, revision=0, **fields):
    return call(*args,data=dict(schema_version=1,event_id=uuid.uuid4().hex,expected_revision=revision,**fields))['work']
call('init'); works={}
for size in [0,1,12,50,200]:
    w=mutate('work','create',title=f'Corpus {size}',objective='Mesurer les parcours sur des données de recette identifiées.',scope='Recette isolée',criteria=['Navigation fidèle'],next='Examiner')
    call('autonomy',w['id'],'manuel')
    for i in range(size):
        w=mutate('task','add',w['id'],revision=w['revision'],id=f't{i}',title=f'Mission {i} — vérifier les dépendances et les preuves du livrable',deliverable=f'docs/t{i}.md',criteria=['Contrôler le rapport'],owner='recette',next='Examiner',depends=[] if i==0 else [f't{(i-1)//3}'])
    works[str(size)]=w['id']
(root/'docs').mkdir();(root/'docs/t0-handoff.md').write_text('# Conclusions de recette\nRapport consultable sans validation implicite.\n')
report='docs/t0-handoff.md'
(root/'docs/t0.evidence.json').write_text(json.dumps(dict(method_version='2',scope_id='t0',artifacts={report:hashlib.sha256((root/report).read_bytes()).hexdigest()},domains={'quality':1},checks=[dict(id='review',domain='quality',mandatory=True,gate='delivery',penalty=100,max_penalty=100,severity='major')],results=[dict(id='review',status='PASS',count=0,evidence=[report])])) )
# More than 200 historical sessions: oldest remains reachable via previous/parent.
db=sqlite3.connect(root/'.swarm/state.db'); wid=works['12']
for i in range(2):
    a=dict(id=f'orphan-{i}',work_id=wid,task_id=f't{i+2}',attempt_id=f'orphan-attempt-{i}',provider='recette',role='worker',status='starting',started='2026-09-15T07:00:00Z',workspace=str(root/f'orphan-{i}'),host='fixture-other-host',progress={})
    db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)',(a['id'],wid,a['task_id'],a['workspace'],a['status'],json.dumps(a),'{}'))
for i in range(205):
    task='t0' if i in [0,203,204] else 't1'
    a=dict(id=f'archive-{i:03d}',work_id=wid,task_id=task,attempt_id=f'attempt-{i:03d}',provider='recette',role='worker',status='failed' if i==0 else 'completed',started='2026-09-15T08:00:00Z',workspace=str(root/'archive'),host='fixture-other-host',progress={})
    if i==203:a['previous']='archive-000'
    if i==204:a['previous']='archive-203';a['parent']='archive-001';a['usage']={'provider_reported_cost_usd':0.23}
    db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)',(a['id'],wid,task,a['workspace'],a['status'],json.dumps(a),'{}'))
for i in range(16):
    a=dict(id=f'intention-{i}',work_id=works['200'],task_id=f't{i}',attempt_id=f'intention-attempt-{i}',provider='recette',role='worker',status='running',started='2026-09-15T08:00:00Z',workspace=str(root/f'intention-{i}'),host='fixture-other-host',progress={})
    db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)',(a['id'],a['work_id'],a['task_id'],a['workspace'],a['status'],json.dumps(a),'{}'))
db.commit();db.close()
print(json.dumps(dict(root=str(root),works=works)))
