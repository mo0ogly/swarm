#!/usr/bin/env python3
"""Portable fresh UI component proof plus failed-script/integrity regressions.

HTTP double only. Receipt hashes declared outputs, not browser caches, ownership,
concurrent hostile writes, native backend, independent acceptance or protection.
Outer recipe stdout anchors the receipt SHA and integrity-guard observations.
"""
from pathlib import Path
import hashlib,json,os,re,shutil,stat,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1]
FILES=['normal.stdout','normal.stderr','fault.stdout','fault.stderr','normal/result.json','normal/paea-linux-prerequisites-v2.json','normal/fr-etat-1180.png','normal/fr-etat-390.png','normal/en-sombre-1180.png','normal/en-sombre-390.png','fault/result.json']

def digest(raw):
    """Hash bytes only; never infer producer identity from a matching digest."""
    return hashlib.sha256(raw).hexdigest()

def regular(root,name):
    """Fixed declared regular files and real intermediate directories only.

    Private quiescent fixture assumption; this is no adversarial-writer lease.
    """
    path=root
    for i,part in enumerate(Path(name).parts):
        if part in ('..','.')or Path(name).is_absolute():raise ValueError('artifact-name')
        path/=part;mode=path.lstat().st_mode
        if i==len(Path(name).parts)-1:
            if not stat.S_ISREG(mode):raise ValueError('artifact-file-kind')
        elif not stat.S_ISDIR(mode):raise ValueError('artifact-directory-kind')
    return path.read_bytes()

def verify(root,expected):
    """Verify anchored metadata kind/hash, exact declared output table and content.

    No current-worktree or independent-authorship claim; source hashes are retained
    observations bound separately to outer recipe source archives.
    """
    assert re.fullmatch('[a-f0-9]{64}',expected)
    raw=regular(root,'receipt.json')
    if digest(raw)!=expected:raise ValueError('receipt-hash')
    receipt=json.loads(raw)
    if set(receipt)!={'schema_version','scope','files','browser_checks','presentation_cases'}or receipt['schema_version']!=1 or receipt['scope']!='workspace-browser-component-http-double':raise ValueError('receipt-shape')
    if set(receipt['files'])!=set(FILES)or receipt['browser_checks']!=111 or receipt['presentation_cases']!=8:raise ValueError('receipt-inventory')
    for name,expected_hash in receipt['files'].items():
        if digest(regular(root,name))!=expected_hash:raise ValueError('artifact-hash')
    normal=json.loads(regular(root,'normal/result.json'));fault=json.loads(regular(root,'fault/result.json'))
    if len(normal['checks'])!=111 or len(set(normal['checks']))!=111 or len(normal['cases'])!=8 or normal['errors']:raise ValueError('normal-outcome')
    if not fault['errors']or not any('fixture-prerequisite-script-failure'in e for e in fault['errors']):raise ValueError('script-refusal')
    expected_cases={f'{lang}-{theme}-{width}'for lang in ['fr','en']for theme in ['etat','sombre']for width in [1180,390]}
    if {case['key']for case in normal['cases']}!=expected_cases:raise ValueError('presentation-matrix')
    if not isinstance(normal.get('browser_version'),str)or not re.fullmatch('[a-f0-9]{64}',normal.get('browser_executable_sha256')or''):raise ValueError('browser-identity')
    if b'UI_PREREQUISITES_PASS checks=111 cases=8'not in regular(root,'normal.stdout'):raise ValueError('missing-named-outcome')
    if b'fixture-prerequisite-script-failure'not in regular(root,'fault.stderr'):raise ValueError('missing-native-diagnostic')
    return receipt

def run():
    """Collect one normal witness, failed startup, then private-copy guard cases."""
    if os.environ.get('SWARM_PAEA_PREREQ_FAULT')is not None:raise ValueError('unexpected-ambient-fixture-fault')
    parent=Path(os.environ['SWARM_PAEA_PREREQ_UI_PARENT']).resolve();assert parent.is_dir()
    proof=Path(tempfile.mkdtemp(prefix='linux-ui-proof-',dir=parent));os.chmod(proof,0o700)
    print('UI_PREREQUISITES_EVIDENCE='+str(proof),flush=True)
    script=ROOT/'tests/paea_linux_ui_test.cjs'
    for mode in ['normal','fault']:
        env=os.environ.copy()
        if mode=='fault':env['SWARM_PAEA_PREREQ_FAULT']='script'
        result=subprocess.run(['node',str(script),str(proof/mode)],cwd=ROOT,env=env,capture_output=True)
        (proof/(mode+'.stdout')).write_bytes(result.stdout);(proof/(mode+'.stderr')).write_bytes(result.stderr)
        if mode=='normal':
            sys.stdout.buffer.write(result.stdout);sys.stdout.flush()
            assert result.returncode==0,(mode,result.returncode,result.stderr.decode())
        else:
            assert result.returncode!=0 and b'fixture-prerequisite-script-failure'in result.stderr
            print('UI_STARTUP_SCRIPT_REFUSAL_PASS',flush=True)
    receipt={'schema_version':1,'scope':'workspace-browser-component-http-double','files':{n:digest(regular(proof,n))for n in FILES},'browser_checks':111,'presentation_cases':8}
    raw=(json.dumps(receipt,indent=2)+'\n').encode();(proof/'receipt.json').write_bytes(raw);anchor=digest(raw)
    verify(proof,anchor);guards=[]
    for case in ['content','metadata-symlink','artifact-symlink']:
        copy=proof/'guards'/case;copy.mkdir(parents=True)
        for name in ['receipt.json',*FILES]:
            dest=copy/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(regular(proof,name))
        verify(copy,anchor)
        target=copy/('receipt.json'if case=='metadata-symlink'else'normal/paea-linux-prerequisites-v2.json')
        if case=='content':target.write_bytes(target.read_bytes()+b'\n')
        else:
            kept=target.with_name(target.name+'.actual');target.rename(kept);target.symlink_to(kept.name)
        try:verify(copy,anchor)
        except ValueError as error:guards.append({'case':case,'valid_control':True,'refused':True,'reason':str(error)})
        else:raise AssertionError('alteration accepted:'+case)
        verify(proof,anchor)
    (proof/'guard-results.json').write_text(json.dumps(guards,indent=2)+'\n')
    print('UI_RECEIPT_SHA256='+anchor,flush=True)
    print('UI_INTEGRITY_GUARDS='+json.dumps(guards,sort_keys=True),flush=True)
    print('UI_INTEGRITY_GUARDS_PASS controls=3 refusals=3',flush=True)
    print('UI_COMPONENT_RECIPE_PASS checks=111 cases=8',flush=True)

if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='--verify-sha':
        verify(Path(os.environ['SWARM_PAEA_PREREQ_UI_EVIDENCE']).resolve(),sys.argv[2]);print('UI_NESTED_DECLARED_OUTPUT_INTEGRITY_PASS')
    elif len(sys.argv)==1:run()
    else:raise SystemExit('Use no args to collect or --verify-sha HASH with explicit evidence env')
