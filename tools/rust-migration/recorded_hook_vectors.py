"""Existing SQL-tail cases plus trusted-hook order, refusal and contract faults."""
import copy
from recorded_change_vectors import vectors as previous_vectors
def vectors():
    out=previous_vectors()
    for create in [True,False]:
        baseline=copy.deepcopy(next(v for v in out if v['Name']==('create'if create else'update')))
        baseline['Hook']='observe';baseline['Name']=('create'if create else'update')+'-hook'
        baseline['control']=True;out.append(baseline)
        for hook in ['refuse','sql-refuse','work-tamper','version-tamper','commit','rollback']:
            name=baseline['Name']+'-callback-'+hook
            out.append(dict(copy.deepcopy(baseline),Name=name+'-control'))
            classification='integrity'if hook in ['work-tamper','version-tamper','commit']or(hook=='rollback'and not create)else'parity'
            out.append(dict(copy.deepcopy(baseline),Name=name,Hook=hook,classification=classification,control=False))
        for fault in ['event-abort','event-rollback','event-ignore','event-tamper','work-tamper']:
            name=baseline['Name']+'-trigger-'+fault;out.append(dict(copy.deepcopy(baseline),Name=name+'-control'))
            out.append(dict(copy.deepcopy(baseline),Name=name,Fault=fault,classification='integrity'if fault in ['event-ignore','event-tamper','work-tamper']else'parity',control=False))
    assert len(out)==len({v['Name']for v in out});return out
