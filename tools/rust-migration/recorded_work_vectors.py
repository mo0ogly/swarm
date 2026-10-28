"""SQL representation/envelope cases, with valid controls before fault cases."""
import copy
import json

def vectors():
    out=[]
    def body(key='work-a',revision=1):return json.dumps(dict(schema_version=1,id=key,revision=revision,title='SQL comparison',tasks=[]),separators=(',',':')).encode()
    def row(key='work-a',revision='1',kind='integer',raw=None,id_kind='text'):
        if isinstance(key,str):key=key.encode()
        return dict(ID=key.hex(),IDKind=id_kind,Revision=revision,RevisionKind=kind,Raw=(body()if raw is None else raw).hex())
    valid=row()
    def frames(name,rows,identifier='work-a'):
        return [dict(Name=name+'-'+op,Operation=op,ID=identifier,Rows=rows)for op in ['read','list']]
    def add(name,rows,identifier='work-a',control=False):
        if not control:out.extend(dict(f,control=True)for f in frames(name+'-control',[valid]))
        out.extend(dict(f,control=control)for f in frames(name,rows,identifier))
    add('valid',[valid],control=True)
    add('empty',[])
    add('newest-first',[valid,row('work-b',raw=body('work-b'))],control=True)
    for key,values in [('schema_version',[0,2,None,'1']),('id',['other','bad name','',None,1]),('revision',[0,-1,2,None,'1',1.0])]:
        for value in values:
            b=json.loads(body());b[key]=value
            add('body-'+key+'-'+str(len(out)),[row(raw=json.dumps(b,separators=(',',':')).encode()),row('work-b',raw=body('work-b'))])
    for raw in [b'',b'null',b'[]',b'{',b'{}junk',b'\xff',body()+b'\xff',body().replace(b'SQL comparison',b'bad\xfftext'),br'{"schema_version":"bad","schema_version":1,"id":"work-a","revision":1}',br'{"schema_version":1,"id":"wrong","id":"work-a","revision":1}',body().replace(b'"tasks":[]',b'"tasks":[],"unknown":{"x":1,"x":2}')]:
        add('raw-'+str(len(out)),[row(raw=raw),row('work-b',raw=body('work-b'))])
    decimal=['1','+1','0001','-0','+0','-1','1.0','1e0',' 1','1 ','','0x1','1_0','9223372036854775807','9223372036854775808','-9223372036854775809','١','−1']
    for kind in ['text','blob']:
        for value in decimal:
            raw=body(revision=9223372036854775807 if value=='9223372036854775807'else 1)
            add('revision-'+kind+'-'+str(len(out)),[row(revision=value if kind=='text'else value.encode().hex(),kind=kind,raw=raw)])
    for value in [b'\xff',b'1\0',b'+01',b'\t1']:
        add('revision-blob-bytes-'+str(len(out)),[row(revision=value.hex(),kind='blob')])
    for value in ['1','2','999999','1000000','-0','0','-1','1.5','0.00001','1e20','NaN','Inf','-Inf']:
        n=int(float(value))if value not in ['NaN','Inf','-Inf']else 1
        add('revision-real-'+str(len(out)),[row(revision=value,kind='real',raw=body(revision=n))])
    for value in ['0','-1','1','2','1000000','9223372036854775807']:
        add('revision-int-'+value,[row(revision=value,raw=body(revision=int(value)))])
    add('revision-null',[row(kind='null')])
    for key in [b'work-a',b'bad name',b'\xff',b"work-a' OR 1=1",'é'.encode()]:
        for kind in ['text','blob']:add('id-column-'+str(len(out)),[row(key,id_kind=kind)])
    for identifier in ['missing','bad name',"work-a' OR 1=1",'é']:
        add('lookup-'+str(len(out)),[valid],identifier)
    # Read only its selected row, whereas list must reject any bad row.
    add('read-good-after-bad',[row(raw=b'{}'),row('work-b',raw=body('work-b'))],'work-b')
    return out
