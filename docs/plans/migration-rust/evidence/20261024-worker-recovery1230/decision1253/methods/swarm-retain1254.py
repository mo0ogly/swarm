from pathlib import Path
import os,json,re,tarfile,hashlib
b=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230');out=b/'decision1253'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
cache=[]
for p in (b/'recipes1252').glob('*/step*.stdout'):
 for line in p.read_text().splitlines():
  if line.startswith('REPLAY_EVIDENCE '):cache.append(Path(line.removeprefix('REPLAY_EVIDENCE ')))
roots=set()
for base in cache:
 for p in base.rglob('fixture*.json'):
  if p.is_symlink():continue
  for val in re.findall(r'/tmp/swarm-model83-[A-Za-z0-9_-]+',p.read_text(errors='replace')):
   d=Path(val)
   if d.is_dir() and d.stat().st_uid==os.getuid() and d.stat().st_mode&0o077==0:roots.add(d)
# Other fixture ownership paths are retained in source-bound setup outputs.
retained=[];excluded=[]
with tarfile.open(out/'owned-runtime-fixtures.tar.gz','w:gz') as archive:
 for base in sorted(set(cache)|roots):
  for p in sorted(base.rglob('*')):
   if p.is_symlink() or not p.is_file():continue
   relative=p.relative_to(base)
   if '.git' in relative.parts or any(x in p.name.lower() for x in ['web-session','credential','auth.json']) or p.suffix=='.key':excluded.append(str(p));continue
   archive.add(p,arcname=str(Path(base.name)/relative),recursive=False);retained.append(dict(root=str(base),path=str(relative),sha256=sha(p)))
(out/'runtime-retention.json').write_text(json.dumps(dict(roots=[str(p) for p in sorted(roots)],source_bound_outputs=[str(p) for p in cache],files=retained,excluded_credentials=excluded,symlinks_and_nonregular_not_followed=True),indent=2)+'\n')
print(json.dumps(dict(runtime_roots=len(roots),recipe_outputs=len(cache),files=len(retained),excluded=len(excluded))))
