"""Go mutation-prefix observations, not full-command permission/acceptance."""
import copy
import json

def vectors():
    result=[]
    def cell(kind,data=b''):return dict(Kind=kind,Data=data.hex()if isinstance(data,bytes)else str(data))
    body=json.dumps(dict(schema_version=1,id='work',revision=2,title='current',tasks=[]),separators=(',',':')).encode()
    valid=dict(Name='valid',Version=37,ID='work',Kind='work.update',Event='event',Request=b'{}'.hex(),Expected=1,Works=[dict(ID='work',Revision=2,Raw=body.hex())],Events=[[cell('text',b'event'),cell('text',b'work'),cell('text',b'work.update'),cell('blob',b'{}')]])
    def add(name,frame,control=False):
        if not control:result.append(dict(copy.deepcopy(valid),Name=name+'-control',control=True))
        result.append(dict(frame,Name=name,control=control))
    add('valid',valid,True)
    for expected in [-1,0,1,2,9223372036854775807]:add('ignored-expected-'+str(expected),dict(valid,Expected=expected),True)
    for identifier in ['', 'work','other','bad work',"work' OR 1=1"]:add('work-filter-'+str(len(result)),dict(valid,ID=identifier))
    for name in ['', 'bad event', 'é','a'*100,'a'*101,'new-event',"event' OR 1=1"]:add('event-name-'+str(len(result)),dict(valid,Event=name))
    for version in [-1,0,1,36,37,38,2147483647,2147483648]:
        add('version-'+str(version),dict(valid,Version=version))
        add('version-name-priority-'+str(version),dict(valid,Version=version,Event='bad event'))
    for kind in ['', 'work.create','task.update','work.update']:add('kind-'+str(len(result)),dict(valid,Kind=kind))
    for request in [b'',b'{}',b'{ }',b'null',b'\xff',b'PRIVATE invalid']:
        add('request-'+str(len(result)),dict(valid,Request=request.hex()))
    for empty in [False,True]:
        f=copy.deepcopy(valid);f['Events']=[];f['ID']='missing'if empty else'bad work';f['Request']=b'bad'.hex();f['Kind']='unknown'
        if empty:f['Works']=[]
        add('new-is-not-permission-'+str(empty),f,True)
    for raw in [b'',b'null',b'[]',b'{',b'PRIVATE invalid',body.replace(b'"work"',b'"other"'),body.replace(b'"revision":2',b'"revision":1'),body.replace(b'"schema_version":1',b'"schema_version":2'),body.replace(b'current',b'bad\xfftext')]:
        f=copy.deepcopy(valid);f['Works'][0]['Raw']=raw.hex();add('bad-current-'+str(len(result)),f)
    add('missing-current',dict(valid,Works=[]))
    for col in range(4):
        for kind,values in [('null',[b'']),('text',[b'',b'{}',b'work',b'work.update',b'\xff',b'PRIVATE']),('blob',[b'',b'{}',b'work',b'work.update',b'\xff']),('integer',[0,1,1000000]),('real',['1','1.5','1e6','1e-5','NaN','Inf','-Inf'])]:
            for value in values:
                f=copy.deepcopy(valid);f['Events'][0][col]=cell(kind,value);add('event-cell-'+str(len(result)),f)
                if col==3:
                    normalized=(b''if kind=='null'or value=='NaN'else value if isinstance(value,bytes)else str(value).encode())
                    if kind=='real':normalized={'1':b'1','1.5':b'1.5','1e6':b'1e+06','1e-5':b'1e-05','NaN':b'','Inf':b'+Inf','-Inf':b'-Inf'}[value]
                    add('request-normalized-'+str(len(result)),dict(f,Request=normalized.hex()))
    return result
