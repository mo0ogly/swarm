"""Actual SQL-tail parity plus explicit Go integrity counterexamples."""
import copy
import json

def vectors():
    out=[]
    def work(revision,title='candidate'):
        return dict(schema_version=1,id='work',revision=revision,title=title,updated='2026-10-18T00:00:00Z',tasks=[])
    def raw(w):return json.dumps(w,separators=(',',':'),ensure_ascii=False).encode().hex()
    def frame(create):return dict(Name='',Existing=''if create else raw(work(1)),Candidate=raw(work(1 if create else 2)),Expected=0 if create else 1,Event='create'if create else'update',Kind='work.create'if create else'work.update',Request=b'{"request":true}'.hex(),Fault='')
    def add(name,f,kind='parity',control=None):
        if control is not None:out.append(dict(copy.deepcopy(control),Name=name+'-control',classification='parity',control=True))
        out.append(dict(f,Name=name,classification=kind,control=control is None and kind=='parity'))
    for create in [True,False]:
        valid=frame(create);label='create'if create else'update';add(label,valid)
        for payload in [b'',b'null',b'{}',b'[]',b'{ "x":1,"x":2 }',b'PRIVATE invalid',b'"\xff"',b'x'*65536]:add(label+'-request-'+str(len(out)),dict(valid,Request=payload.hex()))
        for title in ['', '<>&\u2028\u2029', 'é漢字', 'quote"slash\\\t\x00', 'a'*65536]:
            w=work(1 if create else 2,title);add(label+'-title-'+str(len(out)),dict(valid,Candidate=raw(w)))
        for fault in ['event-abort','event-rollback']+(['work-ignore']if create else[]):add(label+'-'+fault,dict(valid,Fault=fault),control=valid)
        for fault in ['event-ignore','event-tamper','work-tamper']:add(label+'-'+fault,dict(valid,Fault=fault),kind='integrity',control=valid)
        if not create:
            for expected in [-1,0,2,9223372036854775807]:add('stale-'+str(expected),dict(valid,Expected=expected),control=valid)
            add('missing-current',dict(valid,Existing=''),control=valid)
            add('event-collision',dict(valid,Event='seed'),control=valid)
    maximum=9223372036854775807
    near=dict(frame(False),Existing=raw(work(maximum-1)),Candidate=raw(work(maximum)),Expected=maximum-1)
    add('maximum-valid',near)
    overflow=dict(frame(False),Existing=raw(work(maximum)),Candidate=raw(work(maximum)),Expected=maximum,BeforeRevision=maximum)
    add('revision-overflow',overflow,kind='integrity',control=near)
    return out
