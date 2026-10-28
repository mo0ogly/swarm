from pathlib import Path
import json,hashlib,tarfile,os,re,sys,shutil
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-status-unsubmitted1260');out=b/'decision1281';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for name in ['status-engine','cli','clippy','rustdoc']:
 assert json.loads((b/'quality1278'/f'{name}.result.json').read_bytes())['exit']==0,name
profile=json.loads((b/'normal1279/result.json').read_bytes());review=json.loads((out/'source-review.json').read_bytes());assert all(sha(n)==entry['sha256'] for n,entry in review['files'].items())
recipes=[];cache=[]
for name in ['mission-worker-completion-recovery','mission-status-review']:
 d=b/'recipes1280'/name;assert json.loads((d/'result.json').read_bytes())['status']=='PASS'
 assert json.loads((b/'recipes1280'/f'{name}.verify.stdout').read_bytes())['integrity']=='VERIFIED'
 recipes.append(str(d))
 for p in d.glob('step*.stdout'):
  cache += [Path(line.removeprefix('REPLAY_EVIDENCE ')) for line in p.read_text().splitlines() if line.startswith('REPLAY_EVIDENCE ')]
worker=next(p for p in cache if p.name.startswith('worker-recovery-'));public=json.loads((worker/'run/fault/result.json').read_bytes());assert public['status_read_cases']==10
result=dict(status='LOCAL_COMPLETED_BLOCKED_HISTORICAL_STATUS_VERIFIED_GLOBAL_REVIEW_OPEN',binary=profile['binary'],sha256=profile['sha256'],effective_inputs=339,changed_compiler_inputs=3,artifacts=110,no_fixture_feature=True,glm_calls=2,glm_complete=1,glm_incomplete=1,glm_declared_tokens=46135,automatic_retries=0,api_model_requested_reported='glm-5.2',backend_version_user_declared='GLM5.3, not independently attested',engine_status_tests=105,cli_tests=44,clippy='PASS',rustdoc_engine_cli_web='PASS',recipes=recipes,historical_status_read_cases=10,settings_zero_to_two_observed=True,unknown_path_refused_then_restored=True,read_mutation=False,report_submitted_by_status=False,task_accepted_by_status=False,runtime_provider='inert local fixture',global_watchdog=None,installed=False,restarted=False,committed=False,pushed=False,independent_candidate_acceptance=False,global_migration_complete=False,scope='Nonmanaged planned completed Agent / blocked Task without incomplete-delivery prefix, existing review branches preserved',remaining=['managed repository settlement','multi-task journey','incomplete-delivery and other historical status branches','status read may refuse coherent snapshot when supervision changes concurrently; no retry introduced','independent/global qualification, remaining G1–G12, R6/R7 and RETEX'])
atomic(out/'result.json',json.dumps(result,ensure_ascii=False,indent=2)+'\n')
for name,para in [('docs/architecture/NATIVE-WORKER-RECOVERY.md','''\nVérification locale1283 : [preuves et limites](../plans/migration-rust/evidence/20261024-status-unsubmitted1260/decision1281/result.json).\n105 tests moteur de statut et44tests CLI, Clippy et Rustdoc moteur/CLI/web passent.\nDeux recettes publiques (recovery+attribution et revues/acceptation explicite)\npassent avec intégrité vérifiée. Dix lectures CLI/HTTP vérifient les résultats\net marge persistée0→2 sans mutation. Le conducteur de la fixture est arrêté\npar mission stop avant ces lectures : un heartbeat concurrent peut toujours\nfaire refuser le snapshot cohérent, comme le montre le1269 conservé. Normal1279\nsans feature fixture, non installé ; revue indépendante/globale ouverte.\n'''),('docs/en/NATIVE-WORKER-RECOVERY.md','''\nLocal verification1283: [evidence and limits](../plans/migration-rust/evidence/20261024-status-unsubmitted1260/decision1281/result.json).\n105 engine status tests and44CLI tests, Clippy and engine/CLI/web Rustdoc pass.\nTwo public recipes (recovery+attribution and reviews/explicit acceptance) pass\nwith verified evidence integrity. Ten CLI/HTTP reads prove result classification\nand saved margin0→2 without mutation. Public mission stop quiesces the fixture\nbefore these reads: a concurrent heartbeat may still refuse the coherent snapshot,\nas retained1269 demonstrates. Normal profile1279 has no fixture feature and is\nnot installed; independent/global review remains open.\n''')]:
 p=Path(name);atomic(p,p.read_text()+para)
p=Path('docs/plans/migration-rust/PLAN.md');s=p.read_text();start=s.index('\n###1260 —');atomic(out/'execution-history.md',s[start:]);s=s[:start]+'''\n###1260–1283 — statut historique terminé/bloqué, vérifié avec appui GLM\nCLI/HTTP lisibles après vraie reprise : rapport Root absent/frais/ambigu/ancien/\nvide/restauré ; marge persistée0→2 consommée ; chemin externe refusé puis repris.\nDix lectures sans mutation/soumission/acceptation ; revues existantes préservées.\n105tests moteur+44CLI,Clippy,Rustdoc moteur/CLI/web et2recettes PASS/intégrité.\nNormal1279 :339entrées/110artefacts/4cores sans fixture ; binaire61f08c…0796,\nnon installé. [Preuves](evidence/20261024-status-unsubmitted1260/decision1281/result.json),\n[revue locale](evidence/20261024-status-unsubmitted1260/decision1281/review-notes.md),\n[historique](evidence/20261024-status-unsubmitted1260/decision1281/execution-history.md).\nGLM :2appels/1complet/1tronqué,46135tokens déclarés,0retry automatique ;\n[arbitrages](evidence/20261024-status-unsubmitted1260/ASSESSMENT.md).\nRED1262,concurrence1269,anciennes attentes1268 etClippy1274 conservés ; aucune\ngarde affaiblie. Revue indépendante/globale ouverte ; pas de Go/LIVE/UX/restart/commit.\nSuite : règlement planning.repository et parcours multi-tâches, autres branches\nde statut/G1–G12,R6/R7/RETEX. [Apprentissage de méthode](evidence/20261024-status-unsubmitted1260/METHOD-RETEX.md) : contexte GLM ciblé avec vrai caller,\nClippy avant recette finale, distinguer attribution au repos et snapshot concurrent.\n'''
s=s.replace('| G1 Cockpit | suivre mission et décisions | lectures/logs/status final, avis négatifs et acceptation explicite localement vérifiés555/556/558/560/563, revue bornée601 ; retry/closed/managed/cockpit/revue globale ouverts |','| G1 Cockpit | suivre mission et décisions | lectures/logs/status, avis négatifs/acceptation explicite555–563 et résultat terminé/bloqué1283 localement vérifiés ; retry/closed/managed/cockpit/revue globale ouverts |')
lines=s.splitlines(True)
for i,line in enumerate(lines):
 if line.startswith('| G3 Supervision |'):
  lines[i]='| G3 Supervision | conduire planner→worker→contrôles→reviewer | conducteur CLI/web1218, reprise queued/registered et journal1256, statut terminé/bloqué1283 localement vérifiés ; concurrence Go complète, règlement repository, multi-tâches et acceptation globale ouverts |\n'
atomic(p,''.join(lines))
roots=set()
for base in cache:
 for p in base.rglob('*.json'):
  if p.is_symlink():continue
  for val in re.findall(r'/tmp/swarm-model83-[A-Za-z0-9_-]+',p.read_text(errors='replace')):
   d=Path(val)
   if d.is_dir() and d.stat().st_uid==os.getuid() and d.stat().st_mode&0o077==0:roots.add(d)
retained=[];excluded=[]
with tarfile.open(out/'owned-runtime-fixtures.tar.gz','w:gz') as archive:
 for base in sorted(set(cache)|roots):
  for p in sorted(base.rglob('*')):
   if p.is_symlink() or not p.is_file():continue
   rel=p.relative_to(base)
   if '.git' in rel.parts or any(x in p.name.lower() for x in ['web-session','credential','auth.json']) or p.suffix=='.key':excluded.append(str(p));continue
   archive.add(p,arcname=str(Path(base.name)/rel),recursive=False);retained.append(dict(root=str(base),path=str(rel),sha256=sha(p)))
atomic(out/'runtime-retention.json',json.dumps(dict(roots=[str(p) for p in sorted(roots)],outputs=[str(p) for p in cache],files=retained,excluded_credential_paths=excluded),indent=2)+'\n')
methods=out/'methods';methods.mkdir(mode=0o700)
for src in ['/tmp/swarm-prepare1260.py','/tmp/swarm-focus1261.py','/tmp/swarm-test1262.py','/tmp/swarm-port1263.py','/tmp/swarm-tests1264.py','/tmp/swarm-public1266.py','/tmp/swarm-doc1267.py','/tmp/swarm-quality1268.py','/tmp/swarm-replan1270.py','/tmp/swarm-align1272.py','/tmp/swarm-review1275.py','/tmp/swarm-recipes1276.py','/tmp/swarm-bool1277.py','/tmp/swarm-quality1278.py','/tmp/swarm-recipes1280.py','/tmp/swarm-review1281.py',__file__]:
 if Path(src).exists():shutil.copyfile(src,methods/Path(src).name)
files=['tests/rust_mission_worker_recovery.py','tools/verification/recipes/mission-worker-completion-recovery.json','docs/architecture/NATIVE-WORKER-RECOVERY.md','docs/en/NATIVE-WORKER-RECOVERY.md','docs/plans/migration-rust/REPLAYS.md','docs/plans/migration-rust/PLAN.md']
atomic(out/'closure-inputs.json',json.dumps({n:sha(n) for n in files},indent=2)+'\n')
print(result['status'])
