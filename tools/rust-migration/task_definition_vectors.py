"""Contract helpers, policy data and attributed instruction composition."""
import copy,json
def vectors():
    cases=[]
    task=dict(id='task',title='Old',deliverable='report',criteria=['proof'],status='todo',attempts=[dict(id='prior')])
    work=dict(schema_version=1,id='work',revision=3,tasks=[task])
    control=dict(id='check',command=['go','version'],criteria=[1],justification='Objective evidence')
    policy=dict(mode='automatic',controls=[control])
    instructions=dict(task,instruction_base='base',next='new learning\n\nold learning\n\nbase',instruction_revisions=[dict(state='applied',addition='old learning'),dict(state='rejected',addition='rejected'),dict(state='applied',addition='new learning')])
    def frame(name,mode,body=None,request=None,p=None,positive=False,classification='parity'):
        if body is None:body=policy if mode=='policy'else task if mode=='coverage'else instructions if mode=='instructions'else work
        request=dict(schema_version=1,id='task',title='New')if request is None else request
        encode=lambda value:json.dumps(value,ensure_ascii=False,separators=(',',':')).encode().hex()
        return dict(Name=name,Mode=mode,Body=encode(body),Request=encode(request),Policy=encode(policy if p is None else p),control=positive,classification=classification)
    def pair(name,mode,body=None,request=None,p=None,classification='parity'):
        valid=dict(schema_version=1,id='task',next='replacement')if mode=='instructions'else None
        cases.append(frame(name+'-control',mode,request=valid,positive=True));cases.append(frame(name,mode,body,request,p,classification=classification))
    for mode in ['definition','metadata']:
        for field,values in [
            ('title',['',' ','\u0085\u3000','\ufeff','é <&>\u2028','New']),
            ('deliverable',['',' ','report-new']),
            ('criteria',[None,[],[''],[' '],['ok'],['ok']*12,['ok']*13]),
            ('depends',[None,[],['task'],['missing'],['prior'],['prior','prior']]),
            ('max_attempts',[-1,0,1,3,4,9223372036854775807]),
            ('max_tool_calls',[-1,0,1,100,101,9223372036854775807]),
        ]:
            for v in values:pair(mode+'-'+field+'-'+str(len(cases)),mode,request=dict(schema_version=1,id='task',**{field:v}))
        for state in ['todo','blocked','running','submitted','accepted','waived','abandoned','unknown']:
            for definition in [False,True]:
                w=copy.deepcopy(work);w['tasks'][0]['status']=state;r=dict(schema_version=1,id='task',owner='owner',next='next',blocker='blocker')
                if definition:r['title']='New'
                pair(mode+'-status-'+str(len(cases)),mode,w,r)
        for state in ['queued','running','passed','changes_requested','unknown']:
            w=copy.deepcopy(work);w['tasks'][0].update(independent_review=dict(id='review',state=state),gate={},automatic_validation={},manual_override={},revalidation={},plan_checks={'plan-criterion-9':'validation','custom':'delivery'},previous_reviews=[dict(id='older')])
            for r in [dict(title='New'),dict(max_tool_calls=20),dict(title='Old'),dict(criteria=['new']),dict(owner='other')]:pair(mode+'-review-'+str(len(cases)),mode,w,dict(schema_version=1,id='task',**r))
        for old in [None,{},dict(check='delivery'),{'plan-criterion-8':'validation','plan-entry':'custom','custom':'delivery'}]:
            for criteria in [None,['new'],['a','b']]:
                w=copy.deepcopy(work);w['tasks'][0]['plan_checks']=old;pair(mode+'-checks-'+str(len(cases)),mode,w,dict(schema_version=1,id='task',title='Changed',criteria=criteria))
        for other in ['todo','running','submitted']:
            w=copy.deepcopy(work);w['tasks'].append(dict(id='prior',status=other));pair(mode+'-other-'+other,mode,w)
        w=copy.deepcopy(work);w['tasks'].append(dict(id='prior',depends=['task']));pair(mode+'-cycle',mode,w,dict(schema_version=1,id='task',depends=['prior']))
        pair(mode+'-missing-task',mode,request=dict(schema_version=1,id='absent',title='New'))
        for p in [dict(mode='human'),dict(mode='automatic',controls=[]),dict(policy,actor='untrusted',authorized_at='untrusted')]:pair(mode+'-policy-'+str(len(cases)),mode,request=dict(schema_version=1,id='task',validation_policy=p))
    # Coverage runs before generated plan-check rebuilding, as in actual Go.
    for p in [policy,dict(mode='human'),dict(mode='bad'),dict(policy,controls=[dict(control,criteria=[0])]),dict(policy,controls=[dict(control,criteria=[2])])]:
        for criteria in [None,[],['one'],['one','two']]:
            for checks in [None,{},dict(check='delivery'),dict(custom='delivery')]:pair('coverage-'+str(len(cases)),'coverage',dict(task,criteria=criteria,plan_checks=checks),p=p)
    for mode in ['','human','automatic',' automatic\u3000','Human','\ufeffhuman']:
        for controls in [None,[],[control]]:pair('policy-mode-'+str(len(cases)),'policy',dict(mode=mode,controls=controls,actor='untrusted',authorized_at='untrusted'))
    for field,values in [
        ('id',['',' ',' check ','deliverable','café','x'*100,'x'*101]),
        ('command',[None,[],['sh','-c','true'],['/usr/bin/go'],['Go'],['go',''],['go','\u0085'],['go','nul\0'],['go']*32,['go']*33]),
        ('criteria',[None,[],[0],[-1],[1,1],[1,2],[9223372036854775807]]),
        ('justification',['','short','x'*8,'x'*500,'x'*501,'é'*500,'é'*501,' \u0085objective evidence\u3000 ']),
        ('inputs',[None,[],['.'],['a/../proof'],['../out'],['a/../../out'],['/out'],[''],['nul\0'],['x']*64,['x']*65]),
        ('dir',['','relative','..','a/../../out','/outside']),
        ('timeout_seconds',[-1,0,1,60,300,301,9223372036854775807]),
        ('environment_exit_codes',[None,[],[0],[1],[255],[256],[7,7],[-1]]),
    ]:
        for v in values:pair('policy-'+field+'-'+str(len(cases)),'policy',dict(policy,controls=[dict(control,**{field:v})]))
    for size in [1,5,6,8,9]:pair('policy-count-'+str(size),'policy',dict(policy,controls=[dict(control,id='c'+str(i))for i in range(size)]))
    for a,b in [(150,150),(151,150),(1,299)]:pair('policy-total-'+str(a)+'-'+str(b),'policy',dict(policy,controls=[dict(control,id='a',timeout_seconds=a),dict(control,id='b',timeout_seconds=b)]))
    for next_value in ['replacement',instructions['next'],'operator\n\n'+instructions['next'],'new learning\n\nold learning\n\nchanged','new learning\n\nrelocated old learning tail','', 'new learning']:
        pair('instructions-next-'+str(len(cases)),'instructions',request=dict(next=next_value))
    for state in ['todo','blocked','running','accepted']:
        for base in [None,'','base']:
            for operator in [None,'','operator']:
                body=copy.deepcopy(instructions);body.update(status=state,instruction_base=base,instruction_operator_base=operator)
                pair('instructions-state-'+str(len(cases)),'instructions',body,dict(next='replacement'))
    for legacy in [False,True]:
        body=copy.deepcopy(instructions)
        if legacy:body['next']='legacy\n\n'+body['next']
        classification='atomicity'if legacy else 'parity'
        pair('instructions-ambiguous-'+str(len(cases)),'instructions',body,dict(next='other old learning misplaced'),classification=classification)
        w=copy.deepcopy(work);w['tasks'][0]=body
        pair('metadata-partial-'+str(len(cases)),'metadata',w,dict(schema_version=1,id='task',title='Changed',owner='Changed',next='other old learning misplaced'),classification='atomicity')
    # Graph traversal alone, including duplicate ID behavior and long chains.
    for tasks in [[],[task],[dict(task,depends=['task'])],[dict(task,depends=['absent'])],[dict(task,depends=['prior','prior']),dict(id='prior')],[task,dict(task,depends=['absent'])]]:
        pair('graph-'+str(len(cases)),'graph',dict(work,tasks=tasks),dict(id='task',depends=None))
    deep=[dict(id='n'+str(i),depends=['n'+str(i+1)]if i<4095 else[])for i in range(4096)];w=dict(work,tasks=deep)
    for deps in [None,[],['n0'],['missing']]:pair('graph-deep-'+str(len(cases)),'graph',w,dict(id='n4095',depends=deps))
    assert len(cases)==len({v['Name']for v in cases});return cases
