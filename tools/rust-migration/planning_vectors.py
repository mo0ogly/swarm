"""Planning data and clock vectors. No actor, provider or acceptance oracle."""
import copy
import json

def base_work():
    return dict(schema_version=1,id='work',revision=1,criteria=['Global'],tasks=[dict(id='task',scope_id='child',plan_role='worker',requirements=['req-1'],status='accepted',attempts=[dict(id='attempt')])],planning=dict(version=1,max_tasks=1,max_decisions=1,max_activations=1,scopes=[dict(id='root',revision=1,state='closed'),dict(id='child',revision=1,parent='root',state='closed',requirements=['req-1'])]))

def vectors():
    cases=[];base=base_work()
    def frame(name,w=None,mode='validate',before=None,event='transition',at='2026-10-18T00:00:00Z',control=False,classification='parity'):
        body=json.dumps(base if w is None else w,ensure_ascii=False,separators=(',',':')).encode().hex()
        return dict(Name=name,Mode=mode,Body=body,Before={'task':'running'}if before is None else before,Event=event,At=at,control=control,classification=classification)
    def pair(name,w):
        cases.append(frame(name+'-control',control=True));cases.append(frame(name,w))
    for field,values in [('version',[0,1,2]),('max_tasks',[-1,0,1,2,9223372036854775807]),('max_scopes',[-1,0,1,2,3]),('max_depth',[-1,0,1,2,30]),('max_decisions',[-1,0,1,2]),('max_activations',[-1,0,1,2]),('activations',[-1,0,1,2]),('decisions',[-1,0,1,2])]:
        for v in values:
            w=copy.deepcopy(base);w['planning'][field]=v;pair('policy-'+field+'-'+str(v),w)
    for role in ['', 'worker','planner','reviewer','WORKER']:
        w=copy.deepcopy(base);w['tasks'][0]['plan_role']=role;pair('role-'+role,w)
    changes={
        'scopes-null':lambda w:w['planning'].update(scopes=None),
        'scopes-empty':lambda w:w['planning'].update(scopes=[]),
        'root-parent':lambda w:w['planning']['scopes'][0].update(parent='child'),
        'missing-root':lambda w:w['planning']['scopes'][0].update(id='another'),
        'scope-invalid-name':lambda w:w['planning']['scopes'][1].update(id='../bad'),
        'scope-zero-revision':lambda w:w['planning']['scopes'][1].update(revision=0),
        'scope-duplicate':lambda w:w['planning']['scopes'].append(copy.deepcopy(w['planning']['scopes'][1])),
        'parent-missing':lambda w:w['planning']['scopes'][1].update(parent='missing'),
        'cycle-self':lambda w:w['planning']['scopes'][1].update(parent='child'),
        'cycle-pair':lambda w:(w['planning']['scopes'][1].update(parent='another'),w['planning']['scopes'].append(dict(id='another',revision=1,parent='child'))),
        'owner-duplicate':lambda w:w['planning']['scopes'][0].update(requirements=['req-1']),
        'owner-unknown':lambda w:w['planning']['scopes'][1].update(requirements=['req-2']),
        'owner-absent':lambda w:w['planning']['scopes'][1].update(requirements=[]),
        'criteria-empty':lambda w:w.update(criteria=[]),
        'task-owner-missing':lambda w:w['tasks'][0].update(scope_id='missing'),
        'task-requirements-empty':lambda w:w['tasks'][0].update(requirements=[]),
        'task-requirements-null':lambda w:w['tasks'][0].update(requirements=None),
        'task-outside':lambda w:w['tasks'][0].update(requirements=['req-2']),
        'reviewer-context-negative':lambda w:w['planning'].update(reviewer=dict(input_context_tokens=-1)),
        'inbox-bad-id':lambda w:w['planning'].update(inbox=[dict(id='../bad',scope='root')]),
        'inbox-bad-scope':lambda w:w['planning'].update(inbox=[dict(id='valid',scope='missing')]),
        'inbox-duplicate':lambda w:w['planning'].update(inbox=[dict(id='valid',scope='root'),dict(id='valid',scope='child')]),
    }
    for name,change in changes.items():
        w=copy.deepcopy(base);change(w);pair(name,w)
    # No prerequisites means the Go prerequisite validator intentionally does no work.
    for prerequisites in [None,{},dict(),{'req-1':None},{'req-1':[]},{'req-1':['req-1']},{'missing':[]},{'req-1':['missing']},{'req-1':['req-2','req-2']},{'req-1':['req-2'],'req-2':[]},{'req-1':['req-2'],'req-2':['req-1']}]:
        w=copy.deepcopy(base);w['criteria']=['First','Second'];w['planning']['scopes'][1]['requirements']=['req-1','req-2'];w['requirement_prerequisites']=prerequisites;pair('prerequisite-'+str(len(cases)),w)
    for depth in [1,2,21,32,127,512]:
        w=copy.deepcopy(base);w['planning']['scopes']=[dict(id='root',revision=1,requirements=['req-1'])]+[dict(id='s-'+str(i),revision=1,parent='root'if i==1 else 's-'+str(i-1))for i in range(1,depth)];w['tasks'][0]['scope_id']='root'
        cases.append(frame('depth-unlimited-'+str(depth),w,control=True))
        for limit in [depth-1,depth,depth+1]:
            v=copy.deepcopy(w);v['planning']['max_depth']=limit;cases.append(frame('depth-'+str(depth)+'-limit-'+str(limit),v))
    # Durable judge histories: current and renewed cycles; no evidence freshness claim.
    def judge(verdict='continue',at='2026-10-18T00:00:00Z',reason='Reason',evidence='proof'):
        return dict(cycle=1,max_cycles=0,verdict=verdict,reason=reason,at=at,evidence_ref=evidence,closed=verdict=='end',history=[dict(cycle=1,verdict=verdict,reason=reason,at=at,evidence_ref=evidence)])
    for j in [None,dict(cycle=1),dict(cycle=0),dict(cycle=-1),dict(cycle=1,max_cycles=-1),dict(cycle=2,max_cycles=1),dict(cycle=9223372036854775807),judge(),judge('end'),judge('invalid'),dict(cycle=2,history=[dict(cycle=1,verdict='continue',reason='Renewed',at='2026-10-18T00:00:00Z',renewed_at='2026-10-18T01:00:00Z')])]:
        w=copy.deepcopy(base);w['planning']['judge']=j;pair('judge-state-'+str(len(cases)),w)
    for field,value in [('cycle',2),('reason','changed'),('verdict','end'),('at','invalid'),('renewed_at','2026-10-18T00:00:00Z')]:
        j=judge();j['history'][0][field]=value;w=copy.deepcopy(base);w['planning']['judge']=j;pair('judge-history-'+field,w)
    for reason in ['', ' '*20,'\u0085\u3000','\ufeff','é'*2000,'é'*2001]:
        w=copy.deepcopy(base);w['planning']['judge']=judge(reason=reason);pair('judge-reason-'+str(len(cases)),w)
    clocks=set(['','2026-10-18T00:00:00Z','2026-10-18T1:02:03Z','2026-10-18T01:02:03,123Z','0000-01-01T00:00:00+24:60','9999-12-31T23:59:59-24:60'])
    for date in ['2026-10-18','2024-02-29','2023-02-29','0000-01-01','0001-01-01','9999-12-31','10000-01-01','2026-1-01','2026-01-1']:
        for clock in ['00:00:00','1:02:03','23:59:59','24:00:00','01:60:00','01:02:60','01:2:03']:
            for zone in ['Z','+00:00','-24:60','+25:00','+00:61','z']:
                clocks.add(date+'T'+clock+zone)
    for fraction in ['.0',',0','.123456789','.12345678901234567890',',1234567890123','.','..1','.+1']:
        for zone in ['Z','+24:60','-24:60','+01:30']:
            clocks.add('2026-10-18T01:02:03'+fraction+zone)
    for at in sorted(clocks):
        cases.append(frame('clock-'+str(len(cases)),mode='clock',at=at))
        w=copy.deepcopy(base);w['planning']['judge']=judge(at=at);cases.append(frame('judge-date-'+str(len(cases)),w))
    for old,status in [('', 'accepted'),('running','accepted'),('accepted','blocked'),('accepted','accepted'),('running','running'),('todo','blocked')]:
        for task_count in [1,3]:
            w=copy.deepcopy(base);w['tasks'][0]['status']=status;w['planning']['max_tasks']=task_count
            if task_count>1:w['tasks']=[dict(w['tasks'][0],id='task-'+str(i))for i in range(task_count)]
            before={t['id']:old for t in w['tasks']};cases.append(frame('signals-'+str(len(cases)),w,mode='signals',before=before,control=True))
    # Same signal key twice: Go helper mutates then rejects; Rust retains the old plan.
    import hashlib
    signal='plan-'+hashlib.sha256(json.dumps(['transition','validation','task'],separators=(',',':')).encode()).hexdigest()
    w=copy.deepcopy(base);w['planning']['inbox']=[dict(id=signal,scope='child',kind='existing',message='existing',at='old')]
    cases.append(frame('signals-duplicate',w,mode='signals',classification='atomicity'))
    cases.append(frame('unsafe-valid-parent-control',mode='unsafe-signals',control=True))
    w=copy.deepcopy(base);w['planning']['scopes'][1]['parent']='missing'
    cases.append(frame('unsafe-missing-parent',w,mode='unsafe-signals',classification='panic'))
    assert cases and len({v['Name']for v in cases})==len(cases)
    return cases
