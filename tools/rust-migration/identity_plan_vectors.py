"""Data-only finite corpus derived from the actual Go omission/layout rules."""
import copy
import json


def vectors(layout):
    base=dict(schema_version=1,id='work-a',revision=1,title='recorded contract',tasks=[dict(id='task-a',owner='agent-a',status='todo',launch_profile=dict(provider='local',actor='operator',updated='then'))],launch_profile=dict(provider='local',actor='operator',updated='then'),planning=dict(scopes=[dict(id='scope-a',role='worker',state='ready')],repository=dict(candidate_commit='old',workspace='fixture'),judge=dict(verdict='old',policy='retained')))
    out=[]
    def encoded(value):return json.dumps(value,ensure_ascii=True,separators=(',',':')).encode()
    control=encoded(base)
    def add(name,raw,kind='blob',guard=True):
        if guard:out.append(dict(name=name+'-valid-control',raw=control.hex(),kind=kind if kind in ['blob','text']else'blob',control=True))
        out.append(dict(name=name,raw=raw.hex(),kind=kind,control=False))
    def owner(value,role):
        return {'object':lambda:value,'task':lambda:value['tasks'][0],'planning':lambda:value['planning'],'scope':lambda:value['planning']['scopes'][0]}[role]()
    choices=[None,False,True,0,1,-1,1.5,'changed',[],{},[None],{'future':'retained'}]
    for role,fields in sorted(layout['omits'].items()):
        for field in fields:
            for i,val in enumerate(choices):
                value=copy.deepcopy(base);owner(value,role)[field]=val;add(role+'-'+field+'-'+str(i),encoded(value))
        for i,val in enumerate(choices):
            value=copy.deepcopy(base);owner(value,role)['future_unknown']=val;add(role+'-unknown-'+str(i),encoded(value))
    for item in layout['nested']:
        role,key,fields=item['owner'],item['key'],item['omit']
        for i,val in enumerate(choices):
            value=copy.deepcopy(base);owner(value,role)[key]=val;add(role+'-'+key+'-type-'+str(i),encoded(value))
        for field in fields:
            for i,val in enumerate(choices):
                value=copy.deepcopy(base);owner(value,role)[key]={field:val,'unknown':'retained'};add(role+'-'+key+'-'+field+'-'+str(i),encoded(value))
    for char in list(range(256))+[0xd800,0xdfff,0x2028,0x2029,0x10000]:
        for kind in ['blob','text']:add('character-'+str(char)+'-'+kind,encoded({'unknown':chr(char),chr(char):'key'}),kind)
    numbers=['0','-0','0.0','-0.0','1.0','1e0','1e+0001','1e-0000','9007199254740992','9007199254740993','18446744073709551615','-9223372036854775809','1e9999','-1e9999','1e-9999','5e-324','1.7976931348623157e308','9'*4096]
    for i,token in enumerate(numbers):
        for kind in ['blob','text']:add('number-'+str(i)+'-'+kind,b'{"unknown":{"number":'+token.encode()+b'}}',kind)
    roots=[b'{}',b'null',b'[]',b'true',b'1',b'"text"',b'',b'{}{}',b'{"x":"\xff"}',b'{"\xff":1}',b'{"x":"\\ud800"}',b'{"x":1,"x":2}',b'{"tasks":[null]}',b'{"tasks":null}',b'{"tasks":[]}',b'{"tasks":[1]}',b'{"tasks":1,"tasks":[]}',b'{"planning":null}',b'{"planning":{}}',b'{"planning":{"scopes":null}}',b'{"planning":{"scopes":[null]}}',b'{"planning":{"scopes":1}}',b'{"planning":{"scopes":[]}}',b'{"planning":1,"planning":{"scopes":[]}}',b'{"planning":{"scopes":[],"repository":false}}',b'{"planning":{"scopes":[],"judge":[]}}',b'{"tasks":[{"launch_profile":1}]}',b'{"launch_profile":null}',b'{"launch_profile":1}',b'{"revision":1,"Revision":2}',b'{"unknown":{"same":1,"same":null}}']
    for i,raw in enumerate(roots):
        for kind in ['blob','text']:add('root-'+str(i)+'-'+kind,raw,kind)
    for kind in ['null','integer','real']:add('sql-parameter-'+kind,b'{}',kind)
    # Go's existing syntax depth, not a new operator/test deadline.
    for depth in [127,128,129,9998,9999,10000]:
        raw=b'{"unknown":'+b'['*depth+b'0'+b']'*depth+b'}';add('syntax-depth-'+str(depth),raw)
    add('omitted-deep-git',b'{"git":'+b'['*9999+b'0'+b']'*9999+b'}')
    assert len({v['name']for v in out})==len(out)
    return out
