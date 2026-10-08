#!/usr/bin/env python3
"""Bounded, reproducible T5 checks. Never operates on the live Swarm database."""
import hashlib, json, os, pathlib, shutil, subprocess, sys, tempfile
ROOT = pathlib.Path(__file__).resolve().parents[2]
COMMIT = '7110a0b805caf4be1bb09a57b776176b8dc52a68'
RUN = '36719933441'

def run(args, cwd=ROOT, timeout=90):
    p = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT, timeout=timeout, env={**os.environ, 'CHROME_BIN':'/usr/bin/google-chrome'})
    return p

def identity():
    for name in ['.github/workflows/ci.yml', 'tests/i18n_ui.cjs', 'package.json', 'package-lock.json']:
        p = subprocess.run(['git','show',COMMIT+':'+name],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
        assert p.stdout == (ROOT/name).read_bytes(), 'CI input differs: '+name
    workflow=(ROOT/'.github/workflows/ci.yml').read_text()
    step=workflow.split('- run: npm run test:i18n-ui',1)
    assert len(step)==2 and 'continue-on-error' not in step[1].split('  install:',1)[0]
    print('PASS exact CI inputs and mandatory browser step',COMMIT)

def ci():
    identity()
    p=run(['gh','run','view',RUN,'--repo','mo0ogly/swarm','--json','status,conclusion,headSha,jobs'],timeout=30)
    assert p.returncode==0,'Cannot read GitHub run'
    data=json.loads(p.stdout)
    assert data['status']=='completed' and data['conclusion']=='success' and data['headSha']==COMMIT
    steps=[s for j in data['jobs'] if j['name']=='checks' for s in j['steps'] if s['name']=='Run npm run test:i18n-ui']
    assert len(steps)==1 and steps[0]['conclusion']=='success'
    p=run(['gh','run','view',RUN,'--repo','mo0ogly/swarm','--log'],timeout=30)
    assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout
    print('PASS GitHub run',RUN,COMMIT,'browser step success, runner log confirmed')

def browser(mutate=False):
    # Copy the current candidate, including new Go files; isolate mutations.
    with tempfile.TemporaryDirectory(prefix='swarm-t5-check-') as temp:
        dst=pathlib.Path(temp)
        for p in ROOT.glob('*.go'): shutil.copy2(p,dst/p.name)
        for name in ['go.mod','go.sum','package.json']: shutil.copy2(ROOT/name,dst/name)
        for name in ['web','locales','contracts','scripts','tools/agent-workflows','.claude/skills']:
            shutil.copytree(ROOT/name,dst/name)
        (dst/'node_modules').symlink_to(ROOT/'node_modules',target_is_directory=True)
        (dst/'tests').mkdir();shutil.copy2(ROOT/'tests/i18n_ui.cjs',dst/'tests/i18n_ui.cjs')
        def build():
            p=run(['go','build','-o','swarm','.'],dst)
            assert p.returncode==0,p.stdout[-3000:]
        def check():
            return run(['node','tests/i18n_ui.cjs','swarm','screens'],dst,timeout=60)
        build(); p=check();assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout,p.stdout[-3000:]
        if mutate:
            path=dst/'web/i18n-en.js'; original=path.read_text()
            old='"Ajouter une IA": "Add an AI connection"'
            assert old in original
            path.write_text(original.replace(old,'"Ajouter une IA": "REGRESSION-T5-INJECTED"',1))
            build();p=check()
            assert p.returncode!=0 and 'REGRESSION-T5-INJECTED' in p.stdout and 'AssertionError' in p.stdout, p.stdout[-3000:]
            path.write_text(original);build();p=check()
            assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout,p.stdout[-3000:]
            print('PASS browser green/red/green; injected English-label regression fails assertion; restored candidate passes')
        else:
            for lang in ['fr','en']:
                for theme in ['etat','sombre']:
                    assert (dst/'screens'/f'graph-{lang}-{theme}.png').is_file()
            print('PASS browser FR/EN x etat/sombre; real Chrome; current isolated candidate')

if __name__=='__main__':
    if len(sys.argv)>2:
        assert hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()==sys.argv[2], 'Authorized check script changed'
    mode=sys.argv[1]
    if mode=='identity': identity()
    elif mode=='ci': ci()
    elif mode=='mutation': browser(True)
    elif mode=='browser': browser()
    else: raise SystemExit('Unknown check')
