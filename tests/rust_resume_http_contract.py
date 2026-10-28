"""Public resume observation guards; owned Root only, no view publication."""
import hashlib,subprocess
import rust_native_http_cli as native


def verify(server,binary,root,work,csrf):
    """Compare real French/English HTTP summaries with readonly CLI work show.
    Witness refusals after success and verify views, Work, events and budget stay
    unchanged. Raises on mismatch; uses no hidden timeout/provider/datastore write."""
    checks=[]
    def check(name,value):
        assert value,name
        checks.append(name);print('PASS resume-http-'+name,flush=True)
    def views():
        return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'.swarm/views').glob('*.md')}
    before=native.cli(binary,root,['work','show',work]);files=views()
    check('retained-view-file-witness',bool(files))
    budget=server.request('/api/v1/work-budget?work='+work,observe=False)
    base='/api/v1/work-resume?work='+work
    fr=server.request(base+'&lang=fr',observe=False)
    check('actual-identity-revision',fr['schema_version']==1 and fr['work_id']==work and fr['revision']==before['work']['revision'] and fr['language']=='fr')
    check('French-CLI-same-markdown',fr['markdown']==before['resume_markdown'] and bool(fr['markdown'].strip()))
    recorded=server.request('/api/v1/work-recorded-resume?work='+work+'&lang=fr',observe=False)
    check('recorded-explicit-mode-not-calculated',recorded['observation']=='recorded' and recorded['work_id']==work and recorded['revision']==fr['revision'])
    check('recorded-not-acceptance', 'validation non recalculée' in recorded['markdown'] and 'Task <b>plain</b>' in recorded['markdown'])
    en=server.request(base+'&lang=en',observe=False)
    cli=subprocess.run([str(binary),'--root',str(root),'--lang','en','work','show',work],capture_output=True,check=True)
    check('English-CLI-same-markdown',cli.stdout.decode()==en['markdown']+'\n' and en['language']=='en' and en['markdown']!=fr['markdown'])
    for name,query in [('missing-language',base),('missing-Work','/api/v1/work-resume?lang=fr'),('duplicate-Work',base+'&work='+work+'&lang=fr'),('duplicate-language',base+'&lang=fr&lang=en'),('unknown-key',base+'&lang=fr&unknown=1'),('unknown-language',base+'&lang=de'),('traversal','/api/v1/work-resume?work=../escape&lang=fr')]:
        failure=server.request(query,status=400,observe=False);check(name,failure['failure']['code']=='work-resume-query-invalid')
    server.request(base+'&lang=fr',authorized=False,status=403,observe=False)
    server.request(base+'&lang=fr',body={},status=403,observe=False)
    server.request(base+'&lang=fr',body={},csrf=csrf,status=405,observe=False)
    check('refusals-no-view-publication',views()==files)
    check('no-Work-or-event-mutation',native.cli(binary,root,['work','show',work])==before)
    check('no-budget-mutation',server.request('/api/v1/work-budget?work='+work,observe=False)==budget)
    print('RESUME_HTTP_PASS '+str(len(checks))+' named assertions',flush=True)
