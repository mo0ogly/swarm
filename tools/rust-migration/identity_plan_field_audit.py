"""Cross-check omission keys against hash-bound compiled Go DTO producers."""
import hashlib
from pathlib import Path

def audit(root: Path, work: dict, plan: dict) -> dict:
    types=work['types'];records=[]
    roles={'object':'Work','task':'Task','planning':'PlanningState','scope':'PlanningScope'}
    for name,definition in types.items():
        assert hashlib.sha256((root/definition['source']).read_bytes()).hexdigest()==definition['source_sha256'],name
    for source,digest in plan['sources'].items():
        assert hashlib.sha256((root/source).read_bytes()).hexdigest()==digest,source
    for role,typename in roles.items():
        fields={f['json_tag'].split(',')[0]:f['go_name'] for f in types[typename]['fields']}
        missing=sorted(set(plan['omits'][role])-fields.keys())
        assert missing==(['auto_validation','override'] if role=='task' else []),(role,missing)
        records.append(dict(role=role,producer=typename,keys=plan['omits'][role],historical_unknown_keys=missing))
    nested={'launch_profile':'LaunchProfile','repository':'ManagedRepository','judge':'JudgeState'}
    for entry in plan['nested']:
        typename=nested[entry['key']];fields={f['json_tag'].split(',')[0] for f in types[typename]['fields']}
        assert set(entry['omit'])<=fields,(typename,entry)
        records.append(dict(role=entry['owner']+'.'+entry['key'],producer=typename,keys=entry['omit'],historical_unknown_keys=[]))
    task_fields={f['go_name']:f['json_tag'].split(',')[0] for f in types['Task']['fields']}
    assert task_fields['AutoValidation']=='automatic_validation' and task_fields['Override']=='manual_override'
    return dict(status='PASS',groups=records,v2_additions=[task_fields['AutoValidation'],task_fields['Override']],scope='Producer-key coverage, not a claim all runtime fields should be excluded or that authority is valid')
