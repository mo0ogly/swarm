# Vérification — premier lot APEX

## Couverture

| Exigence | Résultat | Preuve / limite |
|---|---|---|
| Refus détaillé au premier appel | PASS | Fournisseur simulé via sous-processus : citation/ligne/explication/reproduction/attendu conservés ; un appel avant refus |
| Citation inventée, ligne incorrecte, justification vide | PASS | TestFragmentV2ActionableRefusal refuse ces entrées |
| Lecture historique | PASS | Schéma v1 inchangé ; nouveaux plans v2 ; suite historique complète |
| Prévol sans dépense, CLI et API cohérents | PASS | TestReviewCostAvailableWithoutBudgetAndWithoutMutation, budget zéro, aucun compteur ni état modifié |
| Deux interruptions identiques | PASS | TestRepeatedFragmentRetryStopsBeforeMutation : refus avant mutation, compteur conservé |
| Réutilisation entre candidats | NON TERMINÉ | Contrat et ordre de mise en œuvre dans DIFFERENTIAL.md ; zéro cache automatique actif |
| Réduction du coût E6 | NON DÉMONTRÉE | Prévol réel en lecture seule :275pièces,256fichiers de diff,10modifiés depuis refus,13+2appels,7disponibles |
| Acceptation E6 | NON | Aucun nouvel appel réel, aucun changement d'acceptation |

## Commandes exécutées

- `go test ./...` : PASS 433.096s, source de production finale. Un premier passage sur une version antérieure a été interrompu pour diagnostic ; il n'est pas compté comme succès.
- `go vet ./...` : PASS.
- `go test -race -run 'TestRepeatedFragmentRetryStopsBeforeMutation|TestReviewCostAvailableWithoutBudgetAndWithoutMutation|TestManagedFragmentStoreConcurrentReservationOnce|TestManagedFragmentRefusalRequiresOriginalProof' -count=1` : PASS11.088s.
- `go test -run 'TestFragmentV2|TestManagedFragmentRuntimeProvider|TestReviewCost|TestRepeatedFragment' -count=1` : PASS31.749s.
- `go test -run 'TestRepeatedFragment|TestManagedFragmentRefusal|TestFragmentDefectSource' -count=1` : PASS3.978s, inclut le test transactionnel supplémentaire.
- `python3 tools/agent-workflows/check.py` : PASS, huit méthodes partagées.
- `git diff --check` : PASS.

Tous les tests fournisseur ci-dessus emploient des doubles déterministes. Ce ne sont pas des avis indépendants sur la mission ni des mesures de qualité d'un vrai modèle. Les logs complets et le prévol live en lecture seule sont conservés hors dépôt dans les artefacts de la session.

## Deuxième lot — RD3 (remplace le statut « non terminé » ci-dessus)

| Exigence | Résultat | Preuve / limite |
|---|---|---|
| Réduction avec couverture complète | PASS | `TestManagedFragmentHistoricalSavingsAndIdentity` : 8 → 4 appels prévus, aucune preuve renommée sous le nouveau SHA |
| Entrée/dépendance modifiée | PASS | Groupe original entier invalidé ; contrat changé entraîne retour à la revue complète |
| Réserves et impacts obligatoires | PASS | Omission, citation historique seule, rapport seul, fichier seul et preuve inventée rejetés ; `unknown` interdit l’acceptation |
| Parcours public et persistance | PASS | `TestManagedFragmentHistoricalPublicRecovery/pass` : 6 → 4 appels réels au sous-processus simulé ; contrôles/revue même SHA, réouverture et rejeu sans dépense |
| Nouveau défaut | PASS | Même parcours `/fail` : une inspection, refus conservé, aucune publication |
| Corruption historique | PASS | Altération de l’ancien journal invalide la nouvelle preuve |
| Taille et budget connus avant appel | PASS | Prévol lecture seule inclut capacité finale et volume restant à inspecter |
| E6 débloquée | NON | 10 appels nécessaires, 7 disponibles, décision finale trop grande ; aucun appel réel lancé |

Tests ciblés : `go test -run 'TestManagedFragmentHistorical|TestReviewCost' -count=1 -v`
PASS 25.536s avant les derniers contrôles de petite modification/renommage et le
calcul exact de taille des observations historiques. Les résultats globaux finaux
seront consignés après leur exécution.

### Pourquoi le dossier E6 contient du bruit

Le contexte du dernier refus comporte 252 fichiers de diff : 107 moteur/autres,
84 tests, 20 interface, 41 documents/preuves. Les deux exports
`docs/e6-trial-originals/agents.json` et `snapshot.json` représentent environ
174 Ko de patch. Le candidat corrigé comporte maintenant 256 fichiers de diff,
mais seuls dix fichiers ont changé depuis ce refus.

Le contrat E6 demande un parcours réel refus → correction → acceptation. Son
dossier embarque également la revue des critères E1–E5 et les évolutions du moteur
accumulées depuis la dernière base acceptée. Ce n’est donc pas seulement une
répétition verbale du modèle : le moteur lui transmet effectivement un périmètre
beaucoup plus large. Supprimer les exports ou exclure arbitrairement des fichiers
ne démontrerait pas la conformité. La suite doit séparer revue des changements,
preuves d’exécution consultables et bilan final, avec une couverture attribuée
aux critères et des avis encore liés au candidat courant. Cette séparation n’est
pas implémentée par le simple cache d’observations RD3.

### Vérification finale du deuxième lot

- `go test ./...` : PASS 296.292s sur le code final. Le passage intermédiaire
  précédent était également réussi (303.268s).
- `go test -race -run 'TestManagedFragmentHistorical|TestReviewCostAvailableWithoutBudgetAndWithoutMutation|TestManagedFragmentStoreConcurrentReservationOnce' -count=1` : PASS 182.519s.
- `go test -run 'TestManagedFragmentHistoricalCapacity|TestManagedFragmentHistoricalTiny' -count=1` : PASS 0.539s ; petites modifications, renommages et conservation des questions dans le calcul de capacité.
- `go vet ./...`, `python3 tools/agent-workflows/check.py`, `git diff --check` : PASS.
- Aucune surface web modifiée ; aucun nouveau contrôle visuel revendiqué.
- Relecture effectuée par l’auteur avec la méthode code-reviewer, pas un avis
  indépendant de la mission. Recette verify-fix via l’entrée publique de reprise,
  transport sous-processus simulé, état relu après réouverture.

## Protocole 4 — recette finale

- Tests ciblés admission/protocole/Store : PASS 17,796 s.
- `go test ./...` : PASS 322,856 s sur le code final.
- `go test -race -run 'TestManagedFragmentToken|TestObservedReviewInput|TestReviewInputTokens' -count=1` : PASS 166,086 s.
- `go vet ./...`, contrôle du contrat agent, `git diff --check` : PASS.
- Prévol CLI du candidat E6 existant avec le client natif configuré : six appels
  requis pour sept disponibles, transport prêt, état de mission identique avant
  et après ; zéro appel IA. Les cinq groupes historiques et 275 pièces restent
  présents. Résultat enregistré dans `v4-native-client-cost.json` des artefacts
  de livraison, mesures/tests dans `v4-checks.json`.
- Relecture par l’auteur : découverte et correction du cache provenant d’un autre
  client, du faux refus sur JSON échappé et de la divergence prévol/exécution.
  Aucun avis indépendant de la mission revendiqué pour cette relecture.
- Pas de changement d’interface ni validation visuelle nouvelle revendiquée.
- E6 n’est pas acceptée par ces tests : la revue indépendante doit encore aboutir.

### Livraison et reprise effective — 2026-09-27T15:06:24.675156+00:00

Commit moteur a0780d62f95e3e7c70e1be8e214d7b9bb214f447 installé, PID1649530,
SHA256 f98c84adb1bd3fb8871d04ab00cbb0ba7ea8e641838698d457bae5859a2de3b2.
Santé prête, état conservé pendant installation, huit assets web identiques.
CLI et HTTP installés donnent le même prévol6/7 sans mutation. Mission reprise.
Action publique retry-review enregistrée sous retry-e6-token-protocol4-20260927.
Revue réelle E6 démarrée : review-60fafd58dc2f40ba6541fc97, état running,
révision461, premier appel réservé, compteur65/71. Aucun nouveau producteur,
même candidat et mêmes contrôles. Ce n’est PAS encore une acceptation.
Le statut global de la tâche et son ancien motif restent blocked pendant cette
revue ; lire independent_review.state pour l’activité réelle, ne pas relancer en
parallèle à cause de cette ancienne étiquette. À suivre comme point de lisibilité.

## Revue réelle du 27 septembre : opérateur inventé dans un défaut

La première inspection de `review-60fafd58dc2f40ba6541fc97` a été interrompue.
Le modèle a cité `Calls + len(batches) >= MaxCalls`, alors que la pièce originale
contient `Calls + len(batches) > MaxCalls`. La citation n’existe nulle part dans
cette pièce et sa ligne annoncée était également erronée. Le moteur a correctement
refusé cette réponse ; modifier le code pour satisfaire ce faux constat aurait
été une régression. Un appel reste consommé : 65/71, sans restitution.

Défaut moteur identifié dans l’aide à la reprise : elle ne décodait que les
anciennes réponses à findings en tableau et ne traitait pas les défauts à citation
invalide. Les réponses compactes actuelles ne recevaient donc aucun retour utile.
Correction : même décodeur que le protocole courant, retour borné contenant la
citation rejetée et les lignes originales voisines, avec obligation de réexaminer
les pièces. Aucune réponse n’est réparée, promue ou acceptée automatiquement.
Le JSON est désigné comme données non fiables et les journaux restent intacts.

Le test de reproduction a aussi détecté que le formateur d’affichage `guardBlock`
normalisait les tabulations. Il est remplacé ici par une coupe en caractères qui
conserve exactement les opérateurs et les espaces d’origine. Tests ciblés :
PASS 0,061 s ; suite Go complète PASS 301,478 s ; vet, contrat et diff réussis.
Aucun appel IA pour cette correction.

### Recovered producer review projection (27 September)
A real independent review identified an early return in resultPresentation: a
failed or interrupted producer hid the review of its externally repaired result.
TestManagedReviewPresentationPreservesInterruptedProducer fails on the previous
implementation and passes with the correction. The projection now checks the
review before rendering the terminal process fallback. The process remains failed
or interrupted; the review is not acceptance. Existing attempt/producer, report
digest and contract checks remain mandatory. A foreign producer review is ignored.
Targeted source tests: PASS (1.405s); attributed candidate: PASS (1.609s).
Go vet, workflow contract and diff checks pass. Full suites are recorded separately
in recovered-presentation-full.log and recovered-presentation-candidate-full.log.
No model call, quota change or production database edit was used for this fix.

Full source suite PASS304.679s; attributed candidate suite PASS145.935s.
Both Go vet invocations pass. No shared-state or synchronization change.

### Repeated correction retains the original baseline (27 September)
A second corrected result previously lost every historical observation because
the prior plan used protocol3/4. Reproduced through the public Store recovery
flow: the second correction returned zero reused observations.
The planner now resolves one original baseline from the prior plan, requires
equality with its durable review event, and rechecks the anchored original
plan/journal/context. It does not promote the intermediate review, combine
multiple baselines or rewrite original replies. Current coverage and the complete
diff from that baseline are recalculated; original unknowns and impact questions
remain mandatory. Unsupported baselines retain the full-review fallback.
Alternative rejected: implicitly trust the intermediate plan or relabel its
observations as current. That would obscure provenance and omit accumulated
changes. Pure quota increase would leave repeated work unfixed.
Public recovery regression: FAIL before, PASS after. Historical tests38.573sPASS;
budget diagnostic test1.757sPASS; vet/contract/diffPASS. A token-plan shortfall
now reports required inspections/final calls/available calls, not a stale-plan
error.
Read-only E6 preflight on the same candidate6bc38636:9calls before,6after,
5original groups retained,275pieces preserved,transport ready.67/71 spent stays
unchanged;4available is still insufficient. No model invocation or quota change.
This does not validate E6 or demonstrate that the next model verdict will pass.

Full Go suite PASS350.178s. Evidence: reuse-origin-checks.json and reuse-origin-full.log.

### Original observations survive an intermediate review error
The public recovery regression now includes a provider exit between the original
review and the next operator correction. Before this fix it lost all historical
observations (FAIL15.093s). The engine may now read a terminal error's anchored
references to an original review, but never reuse that error's own replies.
A differential plan, original references and no reserved call are mandatory.
The original durable record, packet, journal, model/provider/method and current
artifact identity are checked through the existing path. No error is reclassified
as changes_requested. New delta/impact questions and final review remain required.
Read-only current candidate8f1c0d2d:9→6calls,5historicalgroups,276pieces unchanged;
5callsavailable,68/73spent. No model calls or quota increase.
Read-only code review found no bypass; existing source-corruption coverage remains,
while the new exit scenario specifically tests repeated public recovery and budget.

Targeted public recovery tests PASS61.601s; full Go suite PASS364.603s; vet, contract and diff checks PASS. No new synchronization or shared-state mutation.

### Two grounded findings from review-fff0b791b3e895a69b995904
The real review stopped at its first packet with changes_requested (69/74 calls).
1. Publication contention fixture configured busy_timeout via sql.DB, then used
a potentially different pooled connection for UPDATE. Both statements now use
one dedicated sql.Conn with a ten-second context.
2. reportInAddedDiff appended a newline even when present; a full report was
therefore not deduplicated. Regression test failed before correction. A missing
final newline is added only once, preserving the existing trimmed-report contract.
Targeted race tests PASS2.384s, vet/diff PASS. Three matching files applied to the
attributed candidate. Current source-parity metadata recalculated together:
624/636 identical, twelve explicit differences; no new claim of real autonomy.
No model call during development. Full suites recorded separately.

Host full suite PASS372.485s; attributed candidate full suite PASS157.277s.

### Prepared-launch replay identity
Review8c514d4b4c6f4e8790d8f5bd returned changes_requested at70/75calls.
The existing-agent shortcut only checked WorkID. Four isolated cases reproduced
false success: missing SQL attribution, mismatched task, unbound attempt and
missing preparation manifest. All failed before correction (0.737s).
Replay now checks the ready manifest, saved request/schema, nonempty preparation
contract, SQL attribution/base/path, agent workspace, derived attempt ID and
its membership in the task history. It still returns the original agent without
launching a duplicate, including concurrent retry. It does not demand a new work
revision for an already completed idempotent response.
Targeted preparation tests with race PASS14.518s; vet/diff PASS. The two modified
files were byte-identical to the candidate baseline before applying the patch.
Candidate metadata recomputed623/636,13differences; historical trial scope stays
explicit. No model call during development; full suites recorded separately.

Validation finale : suite source `go test ./...` PASS (519.633 s), suite copie PASS (233.115 s), tests ciblés race PASS (14.518 s), `go vet ./...` et `git diff --check` PASS. La validation indépendante E6 reste distincte et non acquise.

## Redémarrage explicite demandé par l'opérateur

Demande : réinitialiser E6 de zéro. Choix : ajouter `planning restart-task`, une
décision atomique liée à la dernière tentative et au candidat de départ, plutôt
qu'effacer les données ou créer une mission de remplacement. Autorisation d'une
seule nouvelle production, copie gérée neuve au prochain lancement ; anciens
avis, tentatives et coûts conservés. Tests : parcours CLI, idempotence, double
attribution, confirmation absente, révision/tentative/candidat périmés, revue et
agent actifs. Aucun appel IA dans ces vérifications.

Validation redémarrage : ok  	swarm.local/companion	542.768s ; tests ciblés avec race PASS8.424s ; vet, contrat et diff PASS.

### Correction du premier lancement après réinitialisation

Le lancement public E6 a refusé avec « reprise : résultat Git absent ou
attribution modifiée ». La remise à faire était effective, mais la préparation
appelait encore le transfert du résultat historique. Pour une décision de
redémarrage visant exactement la dernière tentative et encore non consommée,
ce transfert est désormais absent. HEAD reste construit depuis le candidat
cumulatif. Les reprises ordinaires et celles d'une tentative ultérieure
conservent leur transfert. Le test couvre ces trois cas. L'échec public a été
conservé ; aucune référence Git historique n'a été réécrite pour le masquer.

Validation : ok  	swarm.local/companion	332.231s ; ciblés race3.952sPASS ; vet/diffPASS.

## Requalification explicite d'un avis favorable périmé

Le changement Claude vers Codex invalidait les avis cumulatifs E1–E5, alors que
`requalify` n'acceptait que les tâches dépourvues d'avis. Extension bornée :
confirmation explicite et identité de l'avis `passed` devenu périmé, identité
courante de tentative/résultat/candidat, mêmes contrôles de disponibilité et
transaction. Conservation de l'avis dans les historiques, aucune promotion
implicite. Les résultats réparés restent attribués à leur processus arrêté.
Tests : avis frais refusé, absence de confirmation, mauvais avis, ancien résultat
réparé ; nouvelle revue réellement exécutée avec fournisseur déterministe dans
un Store isolé. Aucune démonstration d'autonomie réelle n'en est déduite.

Validation : ok  	swarm.local/companion	329.795s ; ciblés8.771sPASS ; race17.998sPASS ; vet/diffPASS.

La première demande publique a été refusée avant mutation : l'agent historique
appartenait au démarrage précédent de la même machine. La requalification
utilise désormais `recoveryProcessEnded`, déjà employé par la remise de résultat
réparé : identité de machine et espace PID concordants, identifiant de démarrage
différent, état terminal enregistré. Aucun processus distant inconnu n'est
considéré arrêté. Le test de requalification ajoute un cas de redémarrage de
machine avec conservation du résultat et nouvelle revue.

Validation après redémarrage machine : ok  	swarm.local/companion	331.037s ; ciblésrace19.641sPASS ; vet/diffPASS.

## Debug de citation par lots
Le lot échoué ne recevait aucun diagnostic lors de sa reprise explicite. Le moteur transmet maintenant un diagnostic dérivé de la réponse immuable et du parseur, enregistré dans le nouveau claim compté. Le contexte, les lots acquis, leur empreinte de plan et le validateur exact restent inchangés. Le prompt réel, diagnostic compris, est contrôlé avant réservation. Les tests couvrent correction après réouverture, nouvelle citation fausse refusée et réponse antérieure altérée rejetée sans appel. Aucun test ne sollicite de fournisseur réel.

Validation feedback : ok  	swarm.local/companion	347.719s ; ciblés8.374sPASS ; race27.125sPASS ; vet/diffPASS ; relecture séparée sans défaut identifié.

## Recette réelle du diagnostic et reprise de préparation
Revue E5 réellement reprise après installation04d2102 : deux lots passed, premier avec diagnostic enregistré, second sans ; compteur71→73/75, tâche E5accepted, révision526. La préparation E6 a ensuite refusé le nouveau SHA de requalification. Les deux commits8b1e1e4 et c4143b6a ont le même arbre Git c3887ecc48decbeff3c14e9c03838eebb769da2f. Correction bornée : accepter cette seule dérive de commit si tous les autres paramètres du contrat préparé restent identiques et les deux arbres sont exactement égaux. Manifeste, copie, identité de tentative et limites conservés. Tests de reprise publique, changement de consigne refusé et arbre différent refusé.

Validation préparation à arbre identique : ok  	swarm.local/companion	345.583s ; ciblésrace8.128sPASS ; vet/diffPASS. Relecture séparée sans défaut concret identifié.

## 28 septembre — arrêt différé incompatible avec une tentative gérée
La cinquième production E6 a appelé ScheduleWakeup(delaySeconds=600), puis ScheduleWakeup(stop=true), après avoir démarré une recette en arrière-plan. La fin du producteur a interrompu le serveur et l’exécutant de cette recette. Le moteur a refusé le bilan not_tested sans revue payante. Les traces sont conservées dans agent logs operator-launch-e6-fresh-20260927 et /tmp/swarm-e6-real-out/timeline.jsonl. Cette recette contient aussi une délégation initiale décidée par le script opérateur : elle ne prouve pas l’autonomie du planificateur.
Le lancement synchrone Claude ajoute maintenant ScheduleWakeup aux outils refusés, sans remplacer les interdictions déjà configurées ; les modes interactifs et les autres adaptateurs restent inchangés. La CLI locale expose --disallowedTools. Tests ciblés PASS0.015s ; suite complète en cours. Cette restriction ne garantit pas à elle seule la réussite d’une recette.

Validation finale du refus ScheduleWakeup : ok  	swarm.local/companion	307.781s ; vet/diffPASS.

## 28 septembre — priorité des événements nécessaires à E6

Cause reproduite sur une exportation publique immuable : après l'acquittement
explicite de 47 retours historiques, les événements de la dernière tentative
acceptée d'E5 précédaient encore ceux d'E6 bloquée. Classer seulement les anciennes
identités de tentative ne suffisait donc pas. Le contexte réduit pouvait livrer
E5 sans E6 et consommer une activation supplémentaire sans traiter son blocage.

Correction : événements des tentatives actuelles bloquées en premier, autres
événements actuels ensuite, anciennes tentatives enfin. L'ordre dans chaque groupe
est stable. Rien n'est supprimé ni acquitté par cette projection ; seule une
décision explicite consomme les événements livrés. Le contrôle hors ligne du
contexte réel livre maintenant E6 dans 37 239 octets, avec six événements et les
entrées de découpage. Les tests unitaires couvrent la conservation de l'historique
et le cas d'une tentative acceptée encore actuelle.

Ce contrôle ne constitue pas l'acceptation d'E6. Au moment du diagnostic, le
compteur reste à 5/8, les 40 activations sont consommées et la revue à 73/75.
Le précontrôle du découpage demande 16 inspections et deux appels finaux.
L'autorisation de plafonds 41/91 est demandée avant toute nouvelle exécution.

Validation de cette correction : suite Go complète PASS (454,891 s), tests
ciblés PASS (0,426 s), `go vet`, contrat de configuration et `git diff --check`
PASS. La projection ne modifie pas les états partagés.

## 28 septembre — exécution du découpage et refus au deuxième fragment

Accord utilisateur reçu pour les plafonds 41 activations et 91 appels. Le moteur
adopte les dix lots à la révision 562, traite les trois événements E6 actuels et
confirme 16 inspections plus deux appels finaux, soit 18 disponibles. La revue
`review-abf29fac5a579c074f265971` démarre réellement sur le candidat
`5bf7197cd65dee3eb3868b7712b532f0ce0e09a1` : un fragment reçoit des observations
à compléter ; le suivant demande une correction de `durable_coordinator.go`.
Compteur après arrêt : 75/91. Aucun des seize appels restants n'est dépensé.

Le diagnostic alléguait une accumulation de goroutines derrière le verrou Git.
Ce verrou utilise déjà LOCK_NB : cette explication ne démontre donc pas une file
bloquée à cet endroit. La protection manquait néanmoins avant le démarrage
asynchrone et avant ses lectures en base. Correction bornée : réservation par
mission/agent/tentative via un verrou système non bloquant avant toute goroutine,
libération après traitement ou arrêt du processus. Les journaux durables et
verrous d'intégration restent les autorités pour les effets et la publication.

Le test avec détection de courses vérifie 100 sollicitations via deux Store :
un seul traitement admis, puis reprise après libération. La correction est aussi
préparée dans la copie E6, sans remplacer le candidat refusé ni son verdict.
La nouvelle livraison devra être testée et revue avant acceptation. Les anciens
lots, liés à l'empreinte du candidat refusé, ne sont pas promus automatiquement.

Reprise corrective : l'ancien plan de revue est conservé dans l'événement de
réparation, puis retiré de la tâche au changement explicite de candidat. Le
nouveau découpage exige une adoption et une couverture fraîches. L'API de quotas
accepte désormais des plafonds de périmètre explicitement nommés et audités,
sans remboursement des compteurs. Tests ciblés avec race PASS (8,065 s).
Les premières suites complètes concurrentes ont expiré à 600 s sans assertion
en échec ; elles ne constituent pas une validation et sont remplacées par une
suite isolée avec un délai de 20 minutes.

Validation finale : suite complète PASS (830,807 s), ciblés race PASS (8,065 s),
régression finale de séparation avant/après de l’aperçu PASS (0,145 s), test de
concurrence dans la copie E6 PASS, vet/configuration/diff PASS. Le test de
séparation de l’aperçu a été ajouté après le départ de la suite complète.

## Nouvelle revue coordonnée — réponse invalide au sixième fragment

Plafonds 42/21/93 autorisés et appliqués ; plan adopté à la révision579.
Le candidat b65223f est inspecté par review-5b1ea86d4176df9e5d7d5354 : cinq
fragments enregistrés unknown, sixième interrompu pour citation de défaut absente
à la ligne originale. Compteur81/93. Aucun avis de succès n'est déduit de ces
observations. Le diagnostic public réservé aux refus recevables rejette ce cas
error avant tout appel ; la réponse brute est examinée localement.

Deux constats sont confirmés séparément : limites documentaires contradictoires
(globales/par tâche) et références de preuves E5 régressées dans le candidat E6.
Correction FR/EN du premier point ; restauration byte pour byte des trois pièces
E5 depuis c4143b6a pour le second. Les sources des contrôles et la preuve de
préservation existent déjà et restent inchangées. Cela rétablit les limites
historiques de la preuve, sans annoncer une nouvelle comparaison R5.
La remise corrective publique conserve l'avis invalide et enregistre le résultat
9e572b82 ; périmètre réduit de353 à350 fichiers. Le nouveau découpage doit être
adopté avant revue. Aucun compteur remboursé, aucune acceptation forcée.

## Revue 6ba6394 — neuf inspections conservées avant une citation invalide

Plafonds43/22/99 autorisés ; adoption à la révision603 et reprise publique.
review-733a298c9e5aaa338855184b conserve neuf observations unknown puis refuse
la réponse du dixième fragment : citation absente à la ligne annoncée.
Compteur91/99. La réponse allègue qu'une revue humaine dispense le dépôt géré
 d'un avis indépendant. Vérification locale : independentReviewGuardVersion est
identique dans la copie candidate et le moteur ; il exige l'avis et ne consulte
pas cet ancien indicateur. Tests MissingOrBudgetExhaustedCannotPublish et
LegacyFlagCannotBypassAcceptance PASS (1,521 s). Aucun changement du candidat
sur cette seule allégation ; aucune substitution de verdict.

Reprise préparée sur le même plan et candidat : neuf inspections conservées,
sept restantes plus deux appels finaux, donc neuf appels nécessaires contre
huit disponibles. Aperçu public de quota100 préparé, non appliqué sans accord.
La reprise transmet le diagnostic de citation déjà implémenté ; elle ne
transforme pas une réponse invalide en preuve favorable.
