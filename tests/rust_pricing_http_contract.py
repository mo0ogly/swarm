"""Pricing public HTTP guards on one owned Root, no provider/mission mutation."""
import rust_native_http_cli as native


def verify(server,binary,root,work,csrf):
    """Witness empty/read, invalid/CSRF/stale, append/quote/persist through actual
    routes. No direct datastore writes; budget witness must remain unchanged.
    Raises on wrong semantics; no local timeout, retry or ignored result."""
    checks=[]
    def check(name,value):
        assert value,name
        checks.append(name)
        print('PASS pricing-http-'+name,flush=True)
    path='/api/v1/pricing';before=server.request('/api/v1/work-budget?work='+work,observe=False)
    empty=server.request(path,observe=False);check('empty-no-price-default',empty['rates']==[] and empty['schema_version']==1)
    change=dict(schema_version=1,expected_digest=empty['digest'],rate=dict(provider='fixture',model='http-witness',currency='USD',source='Inert HTTP fixture',reference_date='2026-10-24',input_per_million=2,output_per_million=3,cache_read_per_million=None,cache_write_per_million=0,version='caller-version',actor='caller-actor',updated='caller-time'))
    server.request(path,change,status=403,observe=False)
    bad={**change,'rate':{**change['rate'],'reference_date':'2026-02-30'}}
    server.request(path,bad,csrf,status=400,observe=False)
    server.request(path+'?extra=1',status=400,observe=False)
    check('refusals-no-write',server.request(path,observe=False)==empty)
    saved=server.request(path,change,csrf,observe=False);check('engine-metadata',saved['version']!='caller-version' and saved['actor']!='caller-actor' and saved['updated']!='caller-time')
    after=server.request(path,observe=False);check('append-reread',len(after['rates'])==1 and after['rates'][0]==saved and after['digest']!=empty['digest'])
    server.request(path,change,csrf,status=409,observe=False);check('stale-no-duplicate',server.request(path,observe=False)==after)
    q=dict(rate_version=saved['version'],non_cached_input_tokens=1000000,output_tokens=2000000,cache_read_tokens=0,cache_write_tokens=0)
    quote=server.request(path+'/estimate',q,csrf,observe=False);check('explicit-quote',quote['status']=='estimated' and quote['estimated_amount']==8)
    unknown=server.request(path+'/estimate',{**q,'cache_read_tokens':1},csrf,observe=False);check('unknown-not-free',unknown['status']=='unknown' and unknown['estimated_amount'] is None and unknown['missing_rates']==['cache_read'])
    server.request(path+'/estimate',{**q,'output_tokens':-1},csrf,status=400,observe=False)
    server.request(path+'/estimate',{**q,'rate_version':'not-recorded'},csrf,status=404,observe=False)
    check('quotes-no-write',server.request(path,observe=False)==after)
    persisted=native.cli(binary,root,['pricing','list']);check('CLI-persisted-same-catalogue',persisted==after)
    check('mission-budget-unchanged',server.request('/api/v1/work-budget?work='+work,observe=False)==before)
    print('PRICING_HTTP_PASS '+str(len(checks))+' named assertions',flush=True)
