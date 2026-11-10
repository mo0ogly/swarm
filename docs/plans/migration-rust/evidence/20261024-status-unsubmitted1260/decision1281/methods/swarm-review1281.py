from pathlib import Path
import json,hashlib,tarfile,sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260');out=b/'decision1281';out.mkdir(mode=0o700)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
notes=(b/'decision1275/review-notes.md').read_text()+'''\n## Actualisation finale1281\nClippy1274 a refusé nonminimal_bool.1277 nomme completed_supported : disjonction\nsubmitted/accepted OU revue présentée blocked OU attribution observée de la\nforme étroite. Le refus est completed ET NON completed_supported ; mêmes\nconditions, pas allow ni nouvelle politique. Clippy1278 PASS ; autres résultats\n1278/1280 doivent être lus avant clôture, pas déduits. Normal1279 nouveau binaire\n61f08c…0796 ; relecture publique exigée sur ces octets,1276 pas reconduit seul.\n'''
atomic(out/'review-notes.md',notes)
prev=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230/normal1249/after.json');cur=b/'normal1279/after.json';a=json.loads(prev.read_bytes());c=json.loads(cur.read_bytes());changed=sorted(n for n in a.keys()|c.keys() if a.get(n)!=c.get(n) and not n.startswith(('external-rust/','external-go/')))
ref=dict(path=str(out/'review-notes.md'),sha256=sha(out/'review-notes.md'))
review=dict(schema_version=1,reviewer='Root source self-review, not independent acceptance',baseline_sha256=sha(prev),current_sha256=sha(cur),files={n:dict(sha256=c.get(n),axes={axis:dict(state='reviewed',reason='Changed bytes and actual consumed contracts examined in retained file-specific review; tested scope and unverified criteria explicit.',evidence=[ref]) for axis in ['policies','functions','errors','documentation']}) for n in changed})
atomic(out/'source-review.json',json.dumps(review,indent=2)+'\n')
extras=['tests/rust_mission_worker_recovery.py','tools/verification/recipes/mission-worker-completion-recovery.json','docs/architecture/NATIVE-WORKER-RECOVERY.md','docs/en/NATIVE-WORKER-RECOVERY.md','docs/plans/migration-rust/REPLAYS.md']
atomic(out/'reviewed-files.json',json.dumps({n:sha(n) for n in changed+extras},indent=2)+'\n')
with tarfile.open(out/'reviewed-sources.tar.gz','w:gz') as archive:
 for n in changed+extras:archive.add(n,arcname=n,recursive=False)
print('reviewed inputs',len(changed))
