# Task result — plan-115c11e8f8-T4 (REQ-QW4 — Reprise ciblée après refus ou blocage)

## Outcome en deux phrases

Vérification seule (aucun changement de code) du complément T4 déjà livré par le superviseur : les 3 commandes prescrites (tests Go ciblés, test Node, `git diff --check`) passent sans erreur sur le candidat non committé actuel. req-13 et req-14 sont démontrés par ces contrôles automatisés ; req-15 (parcours réel web+CLI) reste attesté uniquement par le document du superviseur, non ré-exécuté par cette tentative car les captures navigateur lui sont réservées.

## Identité et scope

- Mission / tâche / agent / tentative : `w-115c11e8f802a4f98c3def32` / `plan-115c11e8f8-T4` / `3500cb7c-3823-48cf-a454-295a032b309e` / `a-9b4502ef490c0a9b2227676b` ; départ 2/2 ; révision travail 175 (mémoire moteur : 176).
- Rôle et scope assigné : worker, **vérification seule** du complément T4 déjà terminé — lecture bornée à 3 fichiers, exécution de 3 commandes prescrites, aucun changement applicatif, aucun autre lot.
- Base / candidat : SHA `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, arbre **dirty** (candidat non committé). `git status --short` au début de la tentative montre des modifications hors scope T4 (ex. `GUIDE-UTILISATEUR.md`, `agents_store.go`, `console.go`, `dispatcher.go`, `web/cockpit.js`, etc.) déjà présentes avant cette tentative — non touchées ici.
- État : vérification terminée pour req-13/req-14 ; req-15 partiellement attesté (document fourni, non ré-exécuté).

## Attribution exacte

- **Première tentative** `auto-f6aa5085c5f63a1e7fad` (attempt `a-4060d356d9f554c29087fad8`) : **interrupted** après 25 appels d'outils (7 lectures, 3 écritures, 15 non classés, selon le récapitulatif du superviseur). A commencé le prototype moteur (`recoveryPreview`, `printRecoveryPreview`, édition de `mission_insights.go`) sans rejouer les tests ni confirmer un build propre.
- **Superviseur** (humain, hors moteur Swarm) : a terminé le moteur, le CLI, l'interface web, les traductions et les tests ; a rédigé `docs/plans/clear-launch-recovery/T4-supervisor-recipe.md` ; a réalisé seul les captures navigateur réelles (FR/EN, thèmes clair/sombre, port 18792) et l'observation CLI `T4-recovery-observation.json` citées dans ce document.
- **Présente vérification** (cette tentative, agent `3500cb7c-...`, attempt `a-9b4502ef490c0a9b2227676b`) : n'a lu que les 3 fichiers prescrits (`T4-supervisor-recipe.md`, `recovery_evidence_test.go`, `tests/recovery_evidence_test.cjs`), exécuté les 3 commandes prescrites, constaté les résultats ci-dessous, et rédigé ce handoff. N'a ni exécuté ni observé elle-même le parcours navigateur/CLI réel décrit par le superviseur ; n'a vérifié l'existence d'aucun artefact annexe (captures, JSON d'observation) car hors périmètre de lecture autorisé pour cette tâche.

## Changements et vérification

| Exigence | Preuve examinée | Commande / environnement exacte | Effet observé | Résultat | Évidence |
| --- | --- | --- | --- | --- | --- |
| req-13 (vue changements/exigences restantes/réutilisable vs périmée) | `recovery_evidence_test.go` (TestRecoveryEvidenceChangedUnknownAndUnchanged, TestRecoveryPreviewRefusalDoesNotUseVisitOrOtherTasks) + `tests/recovery_evidence_test.cjs` | `go test ./... -run "TestRecoveryEvidence\|TestRecoveryPreview\|TestManagedPublicationReservesWriterBeforeEvidenceChecks\|TestValidationPolicy" -count=1 -timeout 120s -v` puis `node tests/recovery_evidence_test.cjs` | Go : états `unchanged`/`changed`/`unknown` correctement distingués, `Remaining` recalculé après correction d'un seul fichier (1 critère restant au lieu de 2), `SinceRefusal` borné à la tâche concernée (pas de fuite depuis une autre tâche t2). JS : rendu FR/EN affiche "Depuis le refus"/"Since the refusal", les 3 fichiers (same/changed/missing), les libellés "jamais comme validation"/"never as acceptance", "périmée"/"stale", "non démontrée"/"not demonstrated", et liste exactement le critère restant (1 élément : "criterion 2"). | **PASS** | Sortie exacte ci-dessous (§ Commandes) |
| req-14 (pas de réinventaire complet, pas de hausse de budget automatique) | `recovery_evidence_test.go` (TestRecoveryEvidenceBoundedAndInvalidInputUnknown) + assertions de lecture seule dans TestRecoveryEvidenceChangedUnknownAndUnchanged + JS (0 bouton dans l'aperçu) | même commande `go test ...` + `node tests/recovery_evidence_test.cjs` | Bornes respectées : fichier >8 Mio → `unknown` ; chemin `../outside` → `unknown` ; au-delà de 64 preuves, l'évidence est tronquée à 65 entrées (64 + un marqueur `unknown` final), pas d'extension silencieuse. Après appel de `recoveryPreview`, la révision et le statut de la tâche restent inchangés (`after.Revision != w.Revision \|\| after.Tasks[0].Status != "blocked"` échouerait sinon). Le rendu web ne contient aucun `<button>` ("preview must never execute recovery"). | **PASS** | Sortie exacte ci-dessous (§ Commandes) |
| req-15 (parcours réel refus → correction bornée → reprise, web + CLI) | `docs/plans/clear-launch-recovery/T4-supervisor-recipe.md` (§ Parcours réel, § Confirmation sur la mission active) | Non ré-exécuté par cette tentative — lecture du document uniquement, hors exécution browser/CLI (réservée au superviseur selon consigne de tâche) | Le document déclare : observation CLI publique `T4-recovery-observation.json` (refus r131 → contrôles r135 → avis r137 → acceptation r139), captures navigateur FR/EN × clair/sombre sur port 18792 avec focus clavier (Entrée/Échap) vérifié, cas "aucune revue disponible" après interruption sans revue (r155). | **PARTIAL / NOT INDEPENDENTLY VERIFIED** — le contenu du document n'a pas été corroboré par cette tentative (aucun accès aux fichiers de capture ni au JSON d'observation, hors périmètre de lecture autorisé) | Déclaratif uniquement, voir document cité |

## Commandes exécutées — sortie exacte

**1) Tests Go ciblés**
```
$ go test ./... -run "TestRecoveryEvidence|TestRecoveryPreview|TestManagedPublicationReservesWriterBeforeEvidenceChecks|TestValidationPolicy" -count=1 -timeout 120s -v
--- PASS: TestValidationPolicyRejectsShellAndUnboundedControls (0.00s)
--- PASS: TestManagedPublicationReservesWriterBeforeEvidenceChecks (0.46s)
--- PASS: TestRecoveryPreviewScopesAttemptAndPreservesState (0.09s)
--- PASS: TestRecoveryEvidenceChangedUnknownAndUnchanged (0.06s)
--- PASS: TestRecoveryPreviewRefusalDoesNotUseVisitOrOtherTasks (0.07s)
--- PASS: TestRecoveryEvidenceBoundedAndInvalidInputUnknown (0.05s)
--- PASS: TestValidationPolicyGuidedPreviewApplyModifyAndRemove (0.08s)
--- PASS: TestValidationPolicyPauseHumanReviewAndStalePreview (0.08s)
--- PASS: TestValidationPolicyCLIUsesSamePreviewToken (0.09s)
--- PASS: TestValidationPolicyWebActionUsesSharedContractAndAssets (0.07s)
PASS
ok  	swarm.local/companion	1.170s
```
Exit code : 0. Note : `TestRecoveryPreviewScopesAttemptAndPreservesState` est apparu dans le run (test additionnel couvrant le même module, non listé dans la consigne mais inclus par le filtre `-run`) — aucun test n'a échoué ni n'a été ignoré.

**2) Test Node (rendu FR/EN)**
```
$ node tests/recovery_evidence_test.cjs
PASS FR/EN recovery delta, changed/unknown inputs and remaining criteria without approval
```
Exit code : 0.

**3) Vérification diff**
```
$ git diff --check
```
Sortie vide, exit code : 0 (aucune erreur d'espace en fin de ligne ni marqueur de conflit).

## Points de comparaison et limites explicites

- **Preuves réutilisables vs périmées** : la comparaison testée ici porte sur des empreintes de contenu de fichiers (`hash([]byte(...))`), pas sur un jugement de qualité — une preuve "inchangée" est réutilisable comme entrée, jamais comme validation (assertion explicite dans le test JS et dans le texte du document superviseur). Cette distinction est vérifiée textuellement, pas évaluée sémantiquement par cette tentative.
- **Lecture seule / pas de hausse de budget** : vérifiée uniquement au niveau unitaire Go (révision/statut inchangés après `recoveryPreview`, bornes de taille/nombre respectées) et au niveau rendu JS (absence de bouton). Aucun test d'intégration moteur réel (via CLI/web en conditions live) n'a été rejoué par cette tentative pour confirmer l'absence de hausse de budget côté moteur — cette garantie reste basée sur les tests unitaires cités et sur la déclaration du document superviseur (§ "Confirmation sur la mission active").
- **Parcours réel web/CLI (req-15)** : non observé par cette tentative. Le document superviseur affirme des résultats précis (captures, focus clavier, absence d'erreur console, r131/r135/r137/r139, r155) mais cette vérification n'a pas eu accès aux artefacts cités (`T4-recovery-observation.json`, captures PNG) — consigne de tâche explicite : lecture bornée aux 3 fichiers listés, captures réservées au superviseur. Ce critère reste donc **attesté par une source non vérifiée indépendamment dans cette tentative**, pas démontré de première main.
- **Historique borné** : le document superviseur signale lui-même une fenêtre d'historique de 500 événements ; au-delà, une absence reste explicitement inconnue (non re-vérifié ici).
- **État du dépôt** : l'arbre de travail contient de nombreuses modifications hors scope T4 (listées dans `git status --short`, ex. `console.go`, `dispatcher.go`, `web/cockpit.js`…), présentes avant cette tentative et non touchées par elle. Aucun commit, aucune modification de base Swarm, aucune hausse de budget ou de plafond par cette tentative.

## APEX / PDCA checkpoint

- Analyse / PLAN : vérifier req-13/req-14/req-15 du complément T4 déjà annoncé terminé, sans ré-ouvrir d'inventaire ni de correction.
- Exécution / DO : lecture des 3 fichiers prescrits ; exécution des 3 commandes prescrites ; aucune autre action.
- Vérification / CHECK : 10 tests Go nommés PASS (exit 0), 1 test Node PASS (exit 0), `git diff --check` propre (exit 0). Sorties exactes capturées ci-dessus.
- Ajustement / ACT : aucune correction nécessaire côté code (hors périmètre de cette tâche). req-15 reste à faire corroborer par une revue indépendante disposant d'accès aux artefacts cités, ou par une tentative future autorisée à les lire.
- Limites de reprise : budget outils utilisé ≈ 11/25 sur cette tentative ; aucune limite ni plafond modifié.

## Next action et limites

Responsable du plan : transmettre ce handoff et `docs/plan-115c11e8f8-T4.md` pour revue indépendante sur le même SHA candidat (`1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, dirty). La revue indépendante doit, si possible, corroborer req-15 en accédant aux artefacts cités par le superviseur (captures, `T4-recovery-observation.json`) ou en rejouant elle-même le parcours réel web+CLI, ce que cette tentative n'était pas autorisée à faire. Aucune acceptation, aucune fusion, aucun changement de base Swarm effectué ou proposé par cette tentative.


---
# Annexe du superviseur IA Codex — preuve réelle complémentaire du critère 3

Le texte précédent est le rapport original du worker, conservé intégralement ; il décrit correctement les limites de CE worker. La recette suivante a été réalisée séparément par le superviseur IA Codex, pas par un humain et pas par le producteur. La référence vers un autre document ne suffisait pas : le moteur ne transmet au vérificateur que le rapport et le Markdown déclaré en livrable, ainsi que les reçus moteur et images. Les fichiers de contrôle supplémentaires sont liés par empreinte mais ne sont pas lus comme documents par ce vérificateur. Cette annexe rend la preuve effectivement accessible sans prétendre que le worker l’a observée.

# T4 — reprise ciblée : recette du superviseur

Base Git `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, candidat non commité.
Le premier agent `auto-f6aa5085c5f63a1e7fad`, tentative `a-4060d356d9f554c29087fad8`, a commencé le prototype moteur puis a été interrompu après 25 outils (7 lectures, 3 écritures, 15 non classés). Le superviseur a terminé le moteur, CLI, interface, traductions et tests. Les captures sont celles du superviseur ; aucune attribution à l’exécutant de vérification.

## Contrat

Le CLI `mission recovery TRAVAIL TACHE` et le bouton « Avant une relance » affichent les changements durables depuis le refus de cette tâche, les empreintes des seules preuves liées à son avis et les critères restant à vérifier. Maximum 64 fichiers, 8 Mio par fichier et 32 Mio au total ; hors limites, absence ou chemin non autorisé = inconnu. Aucun inventaire global. Une preuve inchangée est réutilisable comme entrée, jamais comme acceptation. Une preuve modifiée périme l’ancien contrôle. L’aperçu reste en lecture seule et n’augmente aucune limite.

## Vérifications exécutées

- Tests Go ciblés récupération/politique de validation/concurrence : PASS 2,535 s.
- Même périmètre avec détecteur de courses : PASS 15,991 s.
- `npm test` : PASS, y compris rendus FR/EN et parité des traductions.
- `go vet ./...` et `git diff --check` : PASS.
- Test isolé de réservation SQLite : échec reproduit avant correctif (écriture concurrente entrée pendant contrôle), PASS après ajout de `validation-policy.change` aux opérations réservant le rédacteur avant leur garde. Aucun changement de base de mission par accès SQLite direct.
- Le premier agent masquait l’échec de compilation par `go build ... | head`. Le superviseur a reproduit puis corrigé l’argument Task/*Task ; compilation directe actuelle réussie.
- Suite Go complète antérieure : PASS 1483,907 s. Suite candidat T4 : première commande malformée `-timeout40m`, échec immédiat ; corrigée en `-timeout 40m`, en cours, aucun PASS prétendu.

## Parcours réel

Observation CLI publique jointe `T4-recovery-observation.json` : refus T1 ancré r131, puis contrôles r135, avis r137 et acceptation r139. Les modifications courantes de `locales/en.json` et `web/i18n-en.js` sont signalées comme périmées, avec les trois critères restant à vérifier. Les autres preuves inchangées sont regroupées et repliées. Ce n’est pas une fixture ni une approbation.

Navigateur réel sur port 18792 construit depuis ce candidat : FR et EN, thèmes État et sombre. Captures `t4-recovery-{fr,en}-{light,dark}.png` ; ouverture par Entrée, fermeture Échap, retour du focus au déclencheur confirmés dans les quatre variantes ; aucune erreur console relevée. Les contenus rédigés par l’utilisateur restent français en interface anglaise. Le texte et les composants de l’application sont traduits. Capture complémentaire `t4-recovery-changed-fr-light.png`, relevé DOM des preuves modifiées. Les décisions de revalidation se font ensuite par les actions normales, sur les mêmes producteurs terminés ; aucun nouveau producteur T1/T2/T3.

## Limites

L’historique consulté est borné à 500 événements ; une absence au-delà de cette fenêtre est explicitement inconnue. La comparaison d’empreinte ne juge pas la qualité d’un fichier. Les résultats de revue indépendants et les contrôles restent obligatoires. La fin de mission reste à obtenir après T4 et T5.

## Confirmation sur la mission active

Deux applications publiques de politique de validation T4 réussies pendant que le serveur et son suivi tournaient : pas d’arrêt requis après le correctif SQLite. Sur T4 interrompue sans revue, le CLI/web indiquent explicitement « Aucune preuve liée à un avis disponible ; réutilisation non démontrée », conservent les trois critères et montrent la consigne corrigée r155 depuis son arrêt. Capture complémentaire `t4-recovery-no-review-fr-light.png` ; Entrée ouvre, Échap ferme, focus retrouvé sur recovery-preview-plan-115c11e8f8-T4. Le contenu de l’aperçu ne lance aucun agent.

## Reprise effectivement obtenue après le complément T4

Après le refus historique T1 et les modifications bornées, bouton web « Reprendre la vérification » sur le même producteur, contrôles moteur réellement exécutés, revue indépendante favorable, nouvelle gate de six contrôles, puis « Confirmer la validation » dans le cockpit, sans dérogation. T1 acceptée r165, T2 r170 et T3 r175. La lecture CLI publique `work show` confirme à r180 quatre tâches actuellement validées, avec les preuves courantes suivantes :

```json
[
  {
    "task": "plan-115c11e8f8-T1",
    "status": "accepted",
    "fresh": true,
    "review": "review-b97c5870b8ade635d4df372d",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T1/a-62b5c700c364e7236430a9ca-receipt-7b0dcd5b01e19fb7b0407205.json"
  },
  {
    "task": "plan-115c11e8f8-T2",
    "status": "accepted",
    "fresh": true,
    "review": "review-44c35bed9cfee9044da62bd0",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T2/a-4fea4cb374a0cf60b5cc2680-receipt-50e742fbecb2419db9115501.json"
  },
  {
    "task": "plan-115c11e8f8-T3",
    "status": "accepted",
    "fresh": true,
    "review": "review-70477a6401487b971a1116bd",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T3/a-9c26df876eb8691a342f2e00-receipt-f9de2da8be77d4d4aee83139.json"
  }
]
```

Puis bouton « Relancer cette tâche » T4 : consigne de vérification bornée contrôlée dans le formulaire, fournisseur et workspace conservés, deuxième et dernière tentative lancée `3500cb7c-3823-48cf-a454-295a032b309e` / `a-9b4502ef490c0a9b2227676b`. État running confirmé par CLI public et cockpit. Tests ciblés PASS, rapport en rédaction. Cette nouvelle tentative ne change pas l’application ; sa propre revue et acceptation restent à obtenir. La recette réelle du superviseur prouve ce parcours web/CLI, elle n’est pas attribuée à l’exécutant.

## Complément exigé par le refus indépendant — séquence, pas écran statique

L’avis T4 a reconnu les critères 1/2 et demandé la preuve du parcours réel pour le critère 3. Événement de refus r185, observé dans le cockpit à r187 (la consigne abrège « refus r187 » : il s’agit de la révision d’observation, pas de l’ancre du moteur). Aucun défaut de code établi par cet avis.

Séquence de captures réelles, prises pendant les actions du superviseur :
1. `t4-recovery-no-review-fr-light.png` : première tentative interrompue sans revue, nouvelle consigne bornée r155 affichée, preuves inconnues conservées.
2. `t4-verification-running.png` : après clic réel « Relancer cette tâche » puis « Confirmer », quatre acceptations courantes et un agent T4 démarré. CLI agent show confirme 3500cb7c-3823-48cf-a454-295a032b309e, tentative a-9b4502ef490c0a9b2227676b, terminée ensuite en 12 outils.
3. `t4-refusal-current.png` : avis indépendant réel, tests réellement exécutés code 0, deux critères avec preuve présente et troisième preuve insuffisante, pas d’acceptation.
4. `t4-correction-after-refusal.png` : après task update public r188, bouton « Avant une relance » ouvert par Entrée. L’interface montre r188 depuis le refus, les preuves inchangées repliées et UN critère restant : le parcours réel. Échap rend le focus au déclencheur. Aucune action de production dans l’aperçu.

JSON CLI publics AVANT/APRÈS : `T4-recovery-before-correction.json`, `T4-recovery-after-correction.json`. Dans le second : since_refusal.from_revision=185, to_revision=188, événement decision r188 de cette seule tâche, un seul critère restant et preuves inchangées. Cette observation n’est pas un JSON inventé ni une fixture.

Correction bornée de la preuve, sans toucher au code, sans nouvelle production et sans relever les budgets : le présent complément, les étapes capturées et un troisième contrôle moteur qui appelle réellement les commandes CLI publiques mission recovery et work show sur l’état persistant. Ce contrôle examine l’ancre du refus, la correction r188, l’état du producteur terminé et les deux tentatives déjà consommées ; il ne substitue pas les JSON archivés à une lecture live. Les catégories du bilan final seront relues séparément. La nouvelle revue apprécie ces preuves supplémentaires sans demander une troisième tentative de production ni effacer le refus.

## Résultat de l’observation web directe

Après task update public r188, le superviseur a réellement ouvert le bouton Avant une relance par Entrée dans le navigateur Swarm. Le DOM rendu montrait « r188 · Consigne de reprise modifiée », « Voir les preuves inchangées » et exactement un critère restant, « Parcours réel refus puis correction bornée puis reprise vérifié en web et en CLI ». Échap a fermé la modale et document.activeElement portait data-mission-action=recovery-preview-plan-115c11e8f8-T4. Cette observation accompagne la capture t4-correction-after-refusal.png, pas un résultat simulé. Le contrôle moteur live-refusal-correction-recovery exécute les lectures CLI de cette même mission persistante ; captures et lecture CLI se complètent. L’ancien avis/refus et le rapport PARTIAL du worker restent conservés.

## Observation web conservée et contrôle de son enregistrement

`docs/screenshots/clear-launch-recovery/t4-browser-journey.json` provient de l’interaction CUA réelle du superviseur, et non d’une fixture. Ouverture par Entrée à 2026-10-03T16:51:53.482Z, DOM de la modale ouverte montrant r194 et T4-handoff.md comme preuve modifiée. Fermeture Échap à 2026-10-03T16:51:53.955Z, dialogOpen=false, focusVisible=true et focusedAction=recovery-preview-plan-115c11e8f8-T4. Le contrôle moteur vérifie cet enregistrement et ses empreintes, sans prétendre rejouer le navigateur ; les pixels correspondants t4-declared-correction-web.png et la séquence antérieure sont joints. Le contrôle CLI est une exécution live distincte et corroborante.

## Nouveau contrôle navigateur exécuté pendant la validation moteur

Le contrôle live-browser-cua-recovery exécute tests/live_recovery_browser_control.py. Il crée un défi aléatoire valable 45 secondes, demande l’interaction Entrée → DOM rendu de la reprise → Échap → retour du focus, et attend la réponse d’un pilote externe CUA sur le vrai navigateur. Le superviseur exécute cette interaction après la demande du moteur ; il ne substitue ni ancienne capture ni JSON enregistré. La réponse doit correspondre au nonce et à la fenêtre du contrôle. Le contrôle vérifie l’URL de cette mission, la modale ouverte, la correction affichée depuis le refus, T4-handoff.md signalé comme preuve modifiée, les critères restants, la fermeture et le focus_visible sur le déclencheur de cette même tâche. Il exige la capture réelle créée pendant l’interaction. Un contrôle sans pilote CUA frais échoue : aucune réussite simulée ou silencieuse.

C’est une recette supervisée avec pilote navigateur externe, pas un test autonome ni une assertion de recette par le producteur. Elle démontre une exécution actuelle demandée par le moteur, en complément des lectures CLI live. L’essai préalable sans nouvelle correction après le dernier refus a justement été rejeté : l’affichage ne contenait aucune correction nouvelle. La correction documentaire actuelle est enregistrée par task update public, sans modifier le code de production, le nombre de tentatives ou les plafonds. Le contrôle moteur attestera réellement le script et son code de sortie ; cette annexe décrit sa couverture, elle ne déclare pas PASS avant cette exécution.

Code exact du contrôle, lié par empreinte :
```python
#!/usr/bin/env python3
"""Live acceptance bridge. Requires an external CUA browser driver, never a fixture.

The engine starts a fresh, expiring challenge. The supervisor must perform the
read-only UI recipe through CUA while this process is alive and return its DOM
observations. No archived observation, API shortcut or extra agent is accepted.
This supervised recipe is deliberately not in the unattended unit-test suite.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import time

p = argparse.ArgumentParser()
p.add_argument('--work', required=True)
p.add_argument('--task', required=True)
p.add_argument('--directory', required=True)
a = p.parse_args()
directory = Path(a.directory)
directory.mkdir(parents=True, exist_ok=True)
nonce = secrets.token_hex(16)
started = time.time()
request = directory / 'request.json'
response = directory / ('response-' + nonce + '.json')
challenge = {'nonce': nonce, 'started_unix': started, 'expires_unix': started + 45,
             'work': a.work, 'task': a.task, 'operation': 'CUA Enter -> rendered recovery preview -> Escape -> focus',
             'response': str(response)}
tmp = directory / ('request-' + nonce + '.tmp')
tmp.write_text(json.dumps(challenge), encoding='utf-8')
os.replace(tmp, request)
while not response.exists():
    if time.time() > challenge['expires_unix']:
        raise RuntimeError('Fresh CUA browser response missing: no recorded JSON substituted')
    time.sleep(0.1)
r = json.loads(response.read_text(encoding='utf-8'))
assert r['nonce'] == nonce and r['driver'] == 'CUA actual browser interaction'
assert started <= r['observed_unix'] <= time.time() <= challenge['expires_unix']
assert a.work in r['url'] and '127.0.0.1:18792/' in r['url']
assert r['opened']['dialog_open'] and r['opened']['theme'] in ('etat', 'sombre')
text = r['opened']['text']
assert 'Depuis le refus' in text and 'Consigne de reprise modifiée' in text
assert 'Critères restant à vérifier' in text and 'T4-handoff.md' in text
assert 'Contenu modifié depuis l’avis' in text
assert not r['closed']['dialog_open'] and r['closed']['focus_visible']
assert r['closed']['focused_action'] == 'recovery-preview-' + a.task
assert Path(r['screenshot']).is_file()
record = {'result': 'PASS', 'execution': 'Fresh CUA UI interaction requested during this engine control, not an archived DOM fixture',
          'challenge': challenge, 'observation': r}
(directory / ('receipt-' + nonce + '.json')).write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(record, ensure_ascii=False, indent=2))

```
