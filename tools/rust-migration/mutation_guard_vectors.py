"""Guard vectors only: never a model of the mutation closure or actor authority."""
import copy
import hashlib
import json

def raw(value):
    return json.dumps(value,ensure_ascii=False,separators=(',',':')).encode()

def contract(task):
    def encoded(v):
        return raw(v).replace(b'&',b'\\u0026').replace(b'<',b'\\u003c').replace(b'>',b'\\u003e').replace('\u2028'.encode(),b'\\u2028').replace('\u2029'.encode(),b'\\u2029')
    first=hashlib.sha256(encoded([task.get('title',''),task.get('deliverable',''),task.get('criteria'),task.get('depends'),task.get('plan_brief_hash','')])).hexdigest()
    if task.get('memory_selections'):
        refs=[dict(work=m.get('work',''),id=m.get('id',''),version=m.get('version',0),sha256=m.get('sha256',''))for m in task['memory_selections']]
        return hashlib.sha256(encoded([first,refs])).hexdigest()
    return first

def vectors():
    cases=[]
    work=dict(schema_version=1,id='work',revision=4,tasks=[])
    task=dict(id='task',title='T <&>\u2028',deliverable='D',criteria=['old'],depends=[],scope_id='root',status='blocked')
    planned=dict(work,tasks=[task],planning=dict(scopes=[dict(id='root',state='ready')]))
    amendment=dict(id='task',criteria=['new'],confirm_contract_revision=True,contract_revision_reason='Repair the blocked criterion',expected_contract=contract(task))
    def frame(name,w=work,kind='task.update',request=None,sql='',expected=4,event='fresh',identifier='work',want='existing'):
        return dict(Name=name,Body=raw(w).hex()if w is not None else '',ID=identifier,Kind=kind,Event=event,Expected=expected,Request=(raw(request)if not isinstance(request,bytes)else request).hex(),SQL=sql,want=want,control=False)
    def add(name,**kw):
        f=frame(name,request={},**kw);cases.append(f);return f
    def pair(name,control,**changes):
        valid=copy.deepcopy(control);valid['Name']=name+'-control';valid['control']=True;cases.append(valid)
        f=copy.deepcopy(control);f.update(changes);f['Name']=name;f['control']=False;cases.append(f)
    baseline=frame('base',request={})
    hier=frame('hier',w=planned,request=amendment,sql="INSERT INTO cockpit_controls VALUES('work',1)")
    for kind in ['work.update','task.add','task.update','task.submit','checkpoint','unknown.kind']:
        f=frame('kind-'+kind,kind=kind,request={});f['control']=True;cases.append(f)
    pair('stale-revision',baseline,Expected=3,want='revision_conflict')
    pair('invalid-event',baseline,Event='../event',want='format')
    pair('wrong-storage-version',baseline,SQL='PRAGMA user_version=38',want='storage_version_conflict')
    creation=frame('create',w=None,kind='work.create',identifier='',expected=0,request=b'not-json',want='create')
    pair('create-nonzero-revision',creation,Expected=1,want='format')
    pair('missing-work',baseline,ID='missing',want='format')
    for kind in ['work.update','task.add']:
        pair('immutable-'+kind,frame('flat',kind=kind,request={}),Body=raw(planned).hex(),want='format')
    for state in ['queued','starting','running','stopping','completed','failed','interrupted','','RUNNING']:
        for kind,request,other_active in [('work.update',{},'format'),('task.update',dict(id='task',depends=[]),'active_agent'),('task.submit',dict(id='task',criteria=[]),'active_agent'),('task.update',dict(id='task',status='blocked'),'existing'),('task.update',dict(id='task',owner='metadata'),'existing')]:
            sql="INSERT INTO agents VALUES('work','other','"+state+"')"
            f=frame('active-'+state+'-'+kind+'-'+str(len(cases)),kind=kind,request=request)
            pair(f['Name'],f,SQL=sql,want=other_active if state in ['queued','starting','running','stopping']else 'existing')
        pair('same-task-'+state,frame('same-task',request=dict(id='task',status='submitted')),SQL="INSERT INTO agents VALUES('work','task','"+state+"')",want='active_agent'if state in ['queued','starting','running','stopping']else 'existing')
    for key,v in [('title','X'),('deliverable','X'),('criteria',[]),('depends',[]),('max_attempts',-1),('max_tool_calls',1),('validation_policy',{})]:
        pair('definition-'+key,frame('definition',request=dict(id='task',**{key:v})),SQL="INSERT INTO agents VALUES('work','other','running')",want='active_agent')
    for key,v in [('title',''),('criteria',None),('depends',None),('max_attempts',0),('max_tool_calls',0),('validation_policy',None),('objective','not-a-task-definition'),('owner','next'),('next','next')]:
        f=frame('metadata-'+key,request=dict(id='task',**{key:v}),sql="INSERT INTO agents VALUES('work','task','running')");f['control']=True;cases.append(f)
    for key,v in [('confirm_contract_revision',False),('contract_revision_reason','short'),('expected_contract','stale'),('criteria',[]),('criteria',None),('depends',[]),('status','blocked'),('title','X'),('deliverable','X'),('max_attempts',1),('max_tool_calls',1),('validation_policy',{})]:
        r=dict(amendment,**{key:v});pair('hier-request-'+key+'-'+str(len(cases)),hier,Request=raw(r).hex(),want='existing'if key=='criteria'and v is None else 'format')
    for reason in ['a'*15,'a'*16,'a'*2000,'a'*2001,'\u0085'+'é'*8+'\u3000',' '*30+'a'*15+' '*30,'\ufeff'+'a'*13,'\u200b'+'a'*13,'🦀'*4]:
        r=dict(amendment,contract_revision_reason=reason);accepted=len(reason.strip().encode())>=16 and len(reason.encode())<=2000
        pair('hier-reason-'+str(len(cases)),hier,Request=raw(r).hex(),want='existing'if accepted else 'format')
    variants=[]
    for change in ['repository','task-missing','todo','scope-empty','scope-missing','scope-closed','review-running','other-review-running']:
        w=copy.deepcopy(planned)
        if change=='repository':w['planning']['repository']={}
        elif change=='task-missing':w['tasks']=[]
        elif change=='todo':w['tasks'][0]['status']='todo'
        elif change=='scope-empty':w['tasks'][0]['scope_id']=''
        elif change=='scope-missing':w['planning']['scopes']=[]
        elif change=='scope-closed':w['planning']['scopes'][0]['state']='closed'
        elif change=='review-running':w['tasks'][0]['independent_review']=dict(state='running')
        else:w['tasks'].append(dict(id='other',independent_review=dict(state='running')))
        variants.append((change,w));pair('hier-'+change,hier,Body=raw(w).hex(),want='format')
    pair('hier-submission',hier,Kind='task.submit',want='format')
    for value,want in [('0','format'),('1','existing'),('2','format'),('NULL','format'),('1.0','format'),('0.0','format')]+[("'"+v+"'",'existing'if v in ['1','t','T','TRUE','true','True']else 'format')for v in ['1','t','T','TRUE','true','True','0','f','F','FALSE','false','False',' true','true ','TrUe','yes','01','+1','']]+[("x'"+v.hex()+"'",'existing'if v in [b'1',b't',b'T',b'TRUE',b'true',b'True']else 'format')for v in [b'1',b't',b'T',b'TRUE',b'true',b'True',b'0',b'false',b'\xff',b'',b'1\x00']]:
        pair('pause-'+str(len(cases)),hier,SQL="INSERT INTO cockpit_controls VALUES('work',"+value+")",want=want)
    pair('pause-absent',hier,SQL='',want='format')
    for table in ['cockpit_controls','agents','preparation_launch_locks']:
        f=hier if table=='cockpit_controls'else frame('sql',request=dict(id='task',status='running'))
        pair('missing-table-'+table,f,SQL='DROP TABLE '+table,want='format')
    running=frame('running',request=dict(id='task',status='running'))
    for identifier,task_id,released in [('work','task',0),('work','task',1),('work','other',0),('other','task',0)]:
        pair('launch-'+identifier+'-'+task_id+'-'+str(released),running,SQL=f"INSERT INTO preparation_launch_locks VALUES('{identifier}','{task_id}',{released})",want='format'if identifier=='work'and task_id=='task'and released==0 else 'existing')
    for request,want in [(b'null','existing'),(b'{}','existing'),(b'{"unknown":{"nested":true}}','existing'),(b'{"TITLE":null}','existing'),(b'{"depends":null}','existing'),(b'{"depends":[],"depends":null}','existing'),(b'{"depends":null,"depends":[]}', 'active_agent'),(b'[]','format'),(b'','format'),(b'{}{}','format'),(b'{"title":1}','format'),(b'{"max_attempts":1.0}','format'),(b'{"criteria":[null]}','active_agent'),(b'{"title":"\\ud800"}','active_agent'),(b'{"title":"\xff"}','active_agent')]:
        f=frame('decode-'+str(len(cases)),request=request,sql="INSERT INTO agents VALUES('work','other','running')",want=want);cases.append(f)
    for nils in [None,[],['X <&>\u2028','é','🦀']]:
        for dependencies in [None,[],['dep']]:
            for refs in [None,[],[dict(work='work',id='memory',version=1,sha256='abc')],[dict(work='first',id='m',version=9223372036854775807,sha256='<&>'),dict(work='second',id='n',version=2,sha256='def')]]:
                w=copy.deepcopy(planned);t=w['tasks'][0];t.update(criteria=nils,depends=dependencies,memory_selections=refs)
                r=dict(amendment,criteria=['new']*len(nils or []),expected_contract=contract(t))
                f=frame('contract-'+str(len(cases)),w=w,request=r,sql=hier['SQL']);f['control']=True;cases.append(f)
    # Identical recorded request bypasses stale revision, invalid JSON and new guards.
    for request in [b'bad',b'\xff',b'',raw(amendment)]:
        for w in [work,planned]:
            sql="INSERT INTO events VALUES('recorded','work','task.update',x'"+request.hex()+"');DROP TABLE agents;DROP TABLE cockpit_controls;DROP TABLE preparation_launch_locks"
            f=frame('replay-'+str(len(cases)),w=w,request=request,event='recorded',expected=-1,sql=sql,want='replay');f['control']=True;cases.append(f)
            pair('conflict-'+str(len(cases)),f,Request=(request+b' ').hex(),want='event_conflict')
    assert cases and len({f['Name']for f in cases})==len(cases)
    return cases
