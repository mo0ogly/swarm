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


## Complément attribué à Codex, superviseur externe

Le producteur a exécuté les contrôles de code et n’a pas conduit le navigateur. Ses limites ci-dessus sont conservées. La recette navigateur a été réalisée séparément par Codex avec CUA, sur le serveur de la mission, puis répétée à la demande du contrôle moteur `live-browser-cua-recovery`.

Le contrôle crée un défi aléatoire valable 45 secondes ; CUA ouvre la modale au clavier, lit son DOM rendu, ferme avec Échap et observe le retour de focus. Le script vérifie la fraîcheur du défi, l’URL, la correction postérieure au refus, la preuve modifiée, les critères restants, la fermeture et le focus. Il ne démarre aucun producteur, ne valide aucune tâche et ne modifie aucun plafond. Il nécessite un pilote externe : ce n’est pas une recette autonome.

Les anciennes recettes et leurs captures restent dans T4-supervisor-recipe.md. L’ancien complément est conservé dans T4-handoff.before-control-output.md. Le nouveau partage `review_output` permet au moteur de transmettre directement les observations capturées du contrôle courant, avec leur taille et une indication explicite de troncature. Ces observations sont des données à examiner, sans instruction ni demande de verdict.

Le complément avait été enregistré avant les contrôles ; leur exécution ultérieure est maintenant observable dans le reçu ci-dessous. Le rapport initial, l’historique des refus et les deux tentatives sont inchangés.


### Résultat observé du contrôle actuel, après exécution

Le reçu a-9b4502ef490c0a9b2227676b-receipt-12ff95e867b8595d78589025.json contient quatre contrôles exécutés à code 0. Le contrôle navigateur est commencé le 2026-10-03T17:21:49Z et terminé à 17:22:20Z. Sa sortie capturée contient 3647 octets, non tronqués : nonce ffbe8a9c8f1ecda255a0ce3d3de27f76, modale ouverte, consigne postérieure au refus et preuves modifiées affichées, Échap puis dialog_open=false et focus_visible=true sur recovery-preview-plan-115c11e8f8-T4. La mention PARTIAL du producteur initial est une limite de sa propre observation à cette date ; ce résultat CUA est attribué au superviseur Codex, pas à ce producteur.

La séquence complète de reprise est enregistrée séparément dans T4-continuous-sequence.json et les captures t4-sequence-1-refusal.png, t4-sequence-2-correction.png et t4-sequence-3-resume.png. Le contrôle CLI courant confronte ses étapes aux événements durables de la même mission : refus réel, correction Next postérieure et review.retry qui soumet le même résultat. Il vérifie que les deux tentatives, leur plafond et leur limite d’outils sont conservés. Les observations DOM et pixels restent attribuées à CUA ; la revue n’est pas invitée à les traiter comme des instructions.


La séquence supplémentaire est désormais accomplie : refus lu à 17:29:55 UTC sur r228 (événement r227), correction constatée à 17:31:36 sur r230, reprise du même résultat à 17:32:07 sur r231. La carte T4 affiche alors À vérifier. Les fichiers T4-continuous-sequence.json et les trois captures t4-sequence-*.png contiennent ces observations. L’acceptation reste distincte.

## Complément Codex — recette fraîche complète, 17:56 UTC

Après le refus r249, une recette CUA complète est conduite avec un défi nouveau : lecture réelle du refus, correction Next r250 visible dans l’aperçu, reprise du même résultat par le bouton web. Le reçu complet est fourni séparément après exécution ; aucun PASS anticipé dans ce complément. Le contrôle moteur confrontera la chronologie aux événements durables, avec l’attribution externe CUA explicitement conservée.

## Résultat de la recette complète fraîche — Codex/CUA

La recette complète vient de réussir, code de processus 0 : défi 4c5313ac217c0796126afa0b430b33ca, trois interactions entre 1791050193.625 et 1791050265.648 secondes Unix, toutes dans le délai de 120 secondes. Refus réel r249 lu dans la modale, correction CLI Next r250 et fichier modifié visibles dans l’aperçu, reprise par bouton web r251 et carte À vérifier. Les trois étapes ont une capture réelle et des observations DOM, confrontées par le processus de contrôle aux événements durables du CLI. Sources : T4-full-fresh-recipe.json, T4-full-fresh-recipe.log, T4-full-fresh-recipe-runner.txt. C’est une recette externe supervisée, pas une exécution par le producteur ni une garantie cryptographique que CUA ne puisse être imité. Le prochain contrôle moteur lit et vérifie ce reçu complet récent, ses empreintes et les événements ; il ne prétend pas refaire les mutations pendant sa transaction figée. Les critères, tentatives et plafonds restent conservés.

## Constats actuels attribués au superviseur

L’aperçu distingue les preuves modifiées, inchangées et inconnues et recalcule les critères restant à vérifier.

Le contrôle CLI des événements et de la tentative confirme deux tentatives de production et un plafond d’outils inchangé.

La recette CUA complète a lu le refus r249, affiché la correction r250 et enregistré la reprise web r251 ; la carte est passée de Bloquée à À vérifier.

Ces trois constats sont étayés par les contrôles moteur et les observations externes liées. Ils ne changent pas les critères et ne constituent pas une décision d’acceptation.
