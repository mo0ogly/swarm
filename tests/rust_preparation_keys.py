#!/usr/bin/env python3
"""Retained reproducer for Go preparation keys (128 bytes) versus agent keys
(100 bytes). Seed only owned historical rows, then exercise the actual CLI.
--legacy reproduces the old refusal; its success is evidence of the defect,
not acceptance. No arbitrary timeout/provider/live database operation.
"""
import argparse,json
import rust_native_http_cli as native
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish


def main():
    """Use valid 100-byte control before 101/128 and invalid 129-byte input."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);p.add_argument('--legacy',action='store_true');a=p.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);publish(out/'driver.py',Path(__file__).read_bytes());d=Driver(Path(a.binary).resolve(),out);d.ok(['init']);checks=[]
    for size in (100,101,128,129):
        identifier='p'*size;body=dict(id=identifier,title='Historical Go key witness',method='apex',revision=1,created_at='2026-10-25T00:00:00Z',updated_at='2026-10-25T00:00:00Z',documents={},plan_ready=False)
        d.write_fixture('INSERT INTO preparations(id,work_id,revision,body) VALUES(?,NULL,1,?)',(identifier,json.dumps(body).encode()))
        before=d.database();code,stdout,stderr=d.call(['prepare','show',identifier]);allowed=size<= (100 if a.legacy else 128)
        assert code==(0 if allowed else 2),(size,code,stderr);assert d.database()==before
        if allowed:assert json.loads(stdout)['id']==identifier and not stderr
        else:assert not stdout and json.loads(stderr)['failure']['code']=='preparation-id-invalid'
        checks.append(dict(bytes=size,accepted=allowed));print('PASS preparation-key-'+str(size),flush=True)
    server=native.Server(d.binary,d.root,out,'keys');server.capture_async(out/'observer.raw');primary=None
    try:
        before=d.database()
        for size in (100,101,128,129):
            allowed=size<=(100 if a.legacy else 128)
            result=server.request('/api/v1/preparations/show?id='+'p'*size,status=200 if allowed else 400,observe=False)
            assert result.get('id')=='p'*size if allowed else result['failure']['code']=='preparation-query-invalid'
            checks.append(dict(http_bytes=size,accepted=allowed));print('PASS preparation-http-key-'+str(size),flush=True)
        assert d.database()==before
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='REPRODUCED_LEGACY_DEFECT' if a.legacy else 'PASS',checks=checks,root=str(d.root)))
    print('PREPARATION_KEYS_'+('LEGACY_REPRODUCED' if a.legacy else 'PASS'),flush=True)

if __name__=='__main__':main()
