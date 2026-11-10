#!/usr/bin/env python3
"""Exercise served historical dialogue only; no model call, adoption or live Root.

Fixture contract: the private historical Root is published with an empty
provider configuration (providers{}, atomic) immediately after fixture birth,
before the server and the witness turns, because the shared Fixture helper
otherwise writes a CLI-eligible fixture provider. The real public
/api/v1/preparations/capabilities endpoint is asserted disabled with no
provider before the browser starts; no interception fabricates this
precondition. The real Node stdout/stderr are retained before every wrapper
assertion. The configured-calls journey is covered by the preparation-calls
recipes, not by this read-only scenario.
"""
import argparse,json,sqlite3,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native
from rust_preparation_dialogue import turn_fixture


def main():
    """Keep the browser on an owned historical fixture with observable no-write.

    Precondition: provider configuration is emptied through the same atomic
    publisher the Fixture uses, before the HTTP server and the synthetic
    witness turns exist; the real capabilities endpoint then proves
    enabled=false with zero providers, so the UI keeps the strict readonly
    reader. Node's own exit status and output stay primary evidence.
    """
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'dialogue-browser');fixture.publish(f.providers,dict(schema_version=1,providers={}));f.settings_change(preparation_write_enabled='1');p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='history-browser',expected_revision=0,title='History fixture',text='Need'));method=next(m for m in f.ok(['prepare','methods']) if m['id']==p['method']);first=turn_fixture(p,method);second={**first,'id':'turn-second','source_revision':0,'status':'failed','answer':None,'error':'Recorded provider refusal'}
    with sqlite3.connect(f.root/'.swarm/state.db') as db:
        for t in [first,second]:db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)',(t['id'],p['id'],t['status'],json.dumps(t)))
    def sql():
        """Read complete fixture state; no live mission database is used."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return '\n'.join(db.iterdump())
    before=sql();server=native.Server(f.binary,f.root,out,'dialogue-browser',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        capability=server.request('/api/v1/preparations/capabilities',observe=False);assert capability['enabled'] is False and capability['providers']==[],capability
        cfg=dict(base='http://'+server.address,cookie=server.cookie,preparation=p,output=str(out))
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_dialogue_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        print((out/'browser.stdout').read_text(),end='');print((out/'browser.stderr').read_text(),end='')
        assert r.returncode==0,'Historical dialogue browser failed';assert before==sql(),'Historical dialogue mutated fixture SQL';print('DIALOGUE_UI_PASS read-only-state-preserved')
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
if __name__=='__main__':main()
