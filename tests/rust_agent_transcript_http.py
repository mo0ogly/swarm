"""Protected typed-transcript checks on the existing owned CLI log fixture.
No raw SQL writes/provider account; caller supplies actual queued/completed stages.
"""
import json


def before_execution(server,work):
    """Witness the recorded queued Agent without inventing a capture or liveness."""
    v=server.request('/api/v1/agent-transcript?work='+work+'&agent=session',observe=False)
    assert v==dict(schema_version=1,observation='recorded',work_id=work,agent_id='session',recorded_status='queued',state='unavailable',reason='receipt-absent',capture_truncated=None,capture_invalid_utf8=None,output_complete=None,transcript=None),v
    print('PASS transcript-http-queued-explicitly-unavailable',flush=True)


def verify(server,work,other,csrf):
    """Check known 230-frame fixture's redacted metadata and guarded read recovery.
    Raw output text stays omitted, true receipt flags retained, and malformed
    requests never turn into an empty transcript. Return witness for UI checks.
    """
    checks=[]
    def check(name,value):
        assert value,name
        checks.append(name);print('PASS transcript-http-'+name,flush=True)
    base='/api/v1/agent-transcript?work='+work+'&agent=session';v=server.request(base,observe=False)
    check('recorded-association-and-state',v['schema_version']==1 and v['observation']=='recorded' and v['work_id']==work and v['agent_id']=='session' and v['recorded_status']=='completed' and v['state']=='available' and v['reason'] is None)
    t=v['transcript'];check('actual-stream-not-provider-alias',t['format']=='claude' and t['counts']['assistant_message']==231 and t['counts']['model_call']==1 and len(t['entries'])==234)
    check('no-assistant-or-tool-content','log witness' not in json.dumps(v) and 'local model route fixture' not in json.dumps(v) and '<b>literal' not in json.dumps(v))
    check('receipt-completeness-flags',v['capture_truncated'] is False and v['capture_invalid_utf8'] is False and v['output_complete'] is True and t['truncated'] is False)
    check('only-metadata-entry-fields',all(set(e)<=set(['kind','tool','id','error','bytes','tokens','label']) for e in t['entries']))
    for name,path in [('missing-work','/api/v1/agent-transcript?agent=session'),('missing-agent','/api/v1/agent-transcript?work='+work),('duplicate-work',base+'&work='+work),('duplicate-agent',base+'&agent=session'),('unknown-key',base+'&lang=fr'),('unsafe-agent','/api/v1/agent-transcript?work='+work+'&agent=../escape')]:
        bad=server.request(path,status=400,observe=False);check(name,bad['failure']['code']=='agent-transcript-query-invalid')
    bad=server.request('/api/v1/agent-transcript?work='+other+'&agent=session',status=400,observe=False);check('cross-work',bad['failure']['code']=='agent-transcript-work-mismatch')
    server.request(base,authorized=False,status=403,observe=False);server.request(base,body={},status=403,observe=False);server.request(base,body={},csrf=csrf,status=405,observe=False)
    check('recovery-same-observation',server.request(base,observe=False)==v)
    print('TRANSCRIPT_HTTP_PASS '+str(len(checks)+1)+' named assertions including queued witness',flush=True)
    return v
