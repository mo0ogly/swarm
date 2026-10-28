#!/usr/bin/env python3
"""Run actual served source attachment UI in four languages/themes fixtures."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Retain actual source add/remove browser requests and persisted readback."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'sources-browser');f.settings_change(preparation_write_enabled='1');(f.root/'docs').mkdir();text='<img src=x onerror="globalThis.sourceExecuted=true">\nDocumentary text';(f.root/'docs/ui.md').write_text(text);cases=[]
    for lang in ['fr','en']:
        for theme in ['etat','sombre']:
            prep=f.ok(['prepare','create'],dict(version=1,action='create',event_id='source-ui-'+lang+'-'+theme,expected_revision=0,title='Sources UI',text='Need'));cases.append(dict(lang=lang,theme=theme,preparation=prep,text=text))
    s=native.Server(f.binary,f.root,out,'source-browser',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out),cases=cases)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_sources_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Source browser failed';print((out/'browser.stdout').read_text(),end='')
        for c in cases:
            saved=f.ok(['prepare','show',c['preparation']['id']]);assert not saved.get('sources') and saved['revision']==c['preparation']['revision']+2,'Source mutation not retained'
        assert not (f.root/'model-received.json').exists(),'Provider unexpectedly executed';print('PREPARATION_SOURCES_UI_PASS persisted-add-remove')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
if __name__=='__main__':main()
