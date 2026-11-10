from pathlib import Path
import json,hashlib,sys,tarfile,difflib
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
b=Path('docs/plans/migration-rust/evidence/20261024-worker-recovery1230');out=b/'decision1253';out.mkdir(mode=0o700)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
notes='''# Revue locale1230–1253 : finalisation native sous bail

Périmètre : diff normal1211 → normal1249, huit entrées compilées, plus recette
permanente et docs FR/EN. Revue Root, pas revue indépendante ni acceptation globale.
Aucun nouveau GLM lancé pour ce lot. Aucun Go/LIVE/UX/serveur installé modifié.

## Politiques et preuve de configuration
Aucune nouvelle politique opérateur. PID>0, roleworker, originconducteur,
schema1, booléens gate/reap/release, états starting/running/terminal et event
schema1 sont invariants de protocole/propriété, pas réglages d'exploitation.
Box<Agent>, Option<fence>, transaction Immediate/Deferred sont représentation et
frontière transactionnelle. None conserve les chemins manuel/owner existants.
Un fenceSome appartient au seul type MissionLease fourni par Root : pas de JSON
client d'autorité. Le bail provient des paramètres persistés validés/claim history ;
fence.check lit le propriétaire, instance et fraîcheur en transaction. Le cycle
utilise sa cadence sauvegardée ; agent_storage_retry relit la politique actuelle
sous réservation settings. Aucun plafond/retry/délai nouveau ni hausse de budget.
Les valeurs fixture (slots1, budget1, DDL, version2 invalide) sont entrées de test.
Trois recettes indépendantes en parallèle est leur nombre déclaré, pas plafond
produit. Pas de watchdog de vérification, de provider payant ni reset consommation.

## Fonctions et documentation
agent_completion_journal : verify_original_facts extrait sans changement le bloc
WaitFacts/provenance de observe_terminal et le réutilise avant recovery automatique.
recover conserve le chemin manuel ; recover_original partage journal/projection,
settings reservation et vérification terminale, avec fence optionnel en snapshot.
recover_under_mission distingue uniquement absence propre et owner encore présent
comme attente ; format/faits invalides et autres erreurs restent fatals. L'attente
ne prouve ni propriétaire authentifié ni validité complète de la projection SQL :
le contrôle intégral recovery_owner est requis avant toute mutation, jamais un
PID absent seul. Waiting n'autorise aucune nouvelle opération.
agent_execution_store : complete/complete_recovered délèguent None ; nouveau
complete_recovered_fenced et complete_with_owner vérifient bail avant écritures
et après readbacks ; nouveau finish reçoit sa provenance dans le même tx.
MissionWorkerRecovery et RootStore::recover_mission_worker : Root writable et
binding, heartbeat courant, bon Work/origine/rôle/enregistrement, puis original.
Exports lib/root_store/mission_supervision_store n'ajoutent aucun comportement.
CLI continuation valide TOUS les siblings avant recovery ; recover_registered
ne traite pas les Child locaux, conserve cause waiting et marque connu lifecycle/
Work si erreur de lecture/publication après terminal. drive relit après finalisation.
Les fonctions de recette snapshot/watch/fault/main/cases ont contrats, ressources
possédées et limites. Scénario réel, fixture isolée seulement ; sorties/canaux/fautes
conservés. Rustdoc du candidat et docs FR/EN décrivent la portée partielle.

## Erreurs et reprise
read_journal conserve les causes IO/codec/binding. Seul missing direct propre
retourne Waiting, jamais close/cleanup ou erreur imbriquée. verify_original_facts
refuse reap/release/gate inconnus ; départ du propriétaire vérifié par même
host/boot/namespace et stamp. recovery_owner relit l'Agent/ownership exacts,
sauf intention stop autorisée ; reservation/work/FIFO restent ceux du producteur
existant. Échec fence/provenance/settlement/readback suit rollback commun ;
erreurs indépendantes et commit incertain ne deviennent pas succès vide.
Terminal replay : snapshot explicite, vérification et rollback avec finished,
pas nouveau finish ni provider. Fenced terminal snapshot utilise Immediate pour
sérialiser l'observation du bail ; aucune mutation n'est commitée dans ce cas.
CLI attend sans adopter/annuler queued ; tous les autres échecs arrêtent la
session via wrappers existants. Échec de publication Waiting conserve sa cause ;
après Finalized, lifecycle connu reste signalé et la révision connue est conservée
si sa lecture a réussi. emit flush existant. Erreur du Child possédé reste fatale,
pas masquée par une récupération de son journal. Le serveur hérite du même drive.
Les règles originales de cleanup et panics restent dans le conducteur existant,
pas prétendues intégralement auditées ou renforcées ici.

## Preuves et limites
RED1246 : même fixture saine puis refus SQL réel, journal durable, helper récolté ;
ancien watch synchronise sans finaliser. Condition discriminante : première
synchronisation, pas durée. RED1240 est échec de sérialisation fixture, pas RED
produit.1242 manquait la barrière sync ; arrêt du seul watch authentifié conservé.
Recipe1244 a atteint récupération/refus mais échoué au statut HTTP historique.
Scopes réels waiting/ready : hypothèse closed de1245 réfutée et corrigée1248 ;
planned_pair refuse completed/blocked sans revue courante. La recette vérifie
explicitement ce500 et conserve la lacune G3/status, pas de parité revendiquée.
Profil1249 :339entrées,110artefacts,4cores sans features/fixture, non installé.
Recipes1252 : recovery avec quatre mutants, original absent/restauration,
réservation estimée et FIFO libéré une seule fois, lien finish sous génération
supérieure ; registered worker vivant inchangé ; web start/6writers/concurrence/
shutdown et held planner TERM réel. Vérification d'intégrité par recipe.verify.
Tests1251 et Clippy/Rustdoc doivent être lus dans leurs résultats réels avant
qualification finale. Résultats1243 conservés mais invalidés pour fichiers changés
par Box ; aucune compilation verte héritée ne remplace1251.
Ne pas prétendre tester publiquement chaque variante stalelease/wrongWork,
owner-live-avec-reçu ou erreur de sortie après commit dans ce lot. Gardes existantes
lease/procfs et source revues ; ces scénarios complets restent couverture distincte.
Règlement planning.repository, multi-tâches, candidat indépendant, G1–G12/R6/R7
et RETEX global restent ouverts. Même UID hostile, identité métier et permission
provider réelle ne sont pas démontrés par ces fixtures locales inertes.
'''
atomic(out/'review-notes.md',notes)
prev=Path('docs/plans/migration-rust/evidence/20261024-web-conductor1190/normal1211/after.json');cur=b/'normal1249/after.json'
a=json.loads(prev.read_bytes());c=json.loads(cur.read_bytes());changed=sorted(n for n in a.keys()|c.keys() if a.get(n)!=c.get(n) and not n.startswith(('external-go/','external-rust/')))
ref=dict(path=str(out/'review-notes.md'),sha256=sha(out/'review-notes.md'))
review=dict(schema_version=1,reviewer='Root semantic self-review, not independent acceptance',baseline_sha256=sha(prev),current_sha256=sha(cur),files={n:dict(sha256=c.get(n),axes={ax:dict(state='reviewed',reason=f'{ax}: exact changed source examined in file-specific function/error/configuration inventory; scope and untested variants explicitly retained.',evidence=[ref]) for ax in ['policies','functions','errors','documentation']}) for n in changed})
atomic(out/'source-review.json',json.dumps(review,indent=2)+'\n')
extras=['tests/rust_mission_worker_recovery.py','tools/verification/recipes/mission-worker-completion-recovery.json','docs/architecture/NATIVE-WORKER-RECOVERY.md','docs/en/NATIVE-WORKER-RECOVERY.md','docs/plans/migration-rust/REPLAYS.md']
atomic(out/'reviewed-files.json',json.dumps({n:sha(n) for n in changed+extras},indent=2)+'\n')
with tarfile.open(out/'reviewed-sources.tar.gz','w:gz') as archive:
 for n in changed+extras:archive.add(n,arcname=n,recursive=False)
patch=''
for n in changed:
 orig=b/'originals'/n
 if orig.exists():patch+=''.join(difflib.unified_diff(orig.read_text().splitlines(True),Path(n).read_text().splitlines(True),fromfile='originals/'+n,tofile=n))
atomic(out/'owned-increment.patch',patch)
print(json.dumps(dict(changed_compiler_inputs=len(changed),extra_reviewed=len(extras),review=str(out/'source-review.json'))))
