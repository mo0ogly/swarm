# Mémoire de reprise : reçu du contexte complet corrigé

16 octobre 2026, après « next ». Le lot M9 revient au moteur Swarm après les
recettes de comparaison. Les quatre tâches de
[l’itération 5](INCREMENT-MEMOIRE-05.md) sont réalisées et vérifiées localement.
Aucun appel de modèle, aucune mission réelle, aucun déploiement. Les changements
préexistants du moteur sont conservés par préimages avant modification.

## Défaut reproduit et correction

Deux cas échouaient avant correction :

- dans un aperçu de livraison gérée, le manifeste décrivait **38 347 octets**,
  alors que le prompt complet en contenait **43 416** ;
- après ajout de la mémoire de reprise au départ, le fournisseur local recevait
  **40 196 octets**, sans manifeste correspondant à ce texte complet.

`agents_store.go` calculait l’empreinte avant les consignes de livraison gérée ;
`agents_process.go` ajoutait la reprise après ce calcul. `saveAgent` conservait
les valeurs sans les réconcilier. Les deux échecs nommés sont conservés dans
`evidence/m9-20261016/context-receipt-red-executed.json/.log`.

Le calcul commun dans [agent_context.go](../../../internal/engine/agent_context.go)
lie désormais `context.sha256` et `context.bytes` au `prompt` complet. La
préparation le fait après les consignes de livraison. Au départ d’un exécutant
non interactif, le manifeste initial est conservé dans `prepared_context`, la
projection de reprise est ajoutée, puis le contexte final est contrôlé et
enregistré avant de démarrer le fournisseur. Les références mémoire et leurs
décisions sont conservées. La copie du manifeste ne modifie pas celui de l’étape
précédente.

La capacité existante de 128 000 octets s’applique aussi à cet ajout, sans nouvelle
politique de budget ou rétention. Une entrée trop grande ou un échec d’écriture
empêche l’invocation ; l’erreur et la tentative en échec restent consultables.
Un reçu enregistré décrit le texte destiné au fournisseur, pas une preuve de
réception ou de réussite. Le test d’entrée réellement reçue apporte cette preuve
uniquement pour son adaptateur local.

## Parcours et preuves

Le test prépare deux tentatives isolées. La première conserve deux appels d’outil
observés et un usage de 17 tokens d’entrée et 3 de sortie, puis échoue. La seconde
reçoit une projection attribuée de cette histoire. Un script de fournisseur local
enregistre exactement son entrée standard et son nombre d’invocations ; aucun
modèle n’est appelé et aucun raisonnement autonome n’est simulé.

Après réouverture, le texte reçu est identique au `prompt` conservé et à son
manifeste. Dans cet original de test, le contexte préparé fait **39 069 octets**
et le contexte reçu **40 194 octets**. Les deux empreintes et tailles sont
vérifiées séparément. L’enregistrement complet de la première tentative est
strictement identique avant/après, ses compteurs non nuls restent présents, les
deux identités de tentative sont conservées et la tâche n’est pas acceptée.
Le fournisseur local n’a été invoqué qu’une fois.

Les différences de quelques octets entre exécutions viennent des identités et
racines de fixture ; chaque reçu reste lié aux octets de sa propre exécution.
Les originaux utiles sont sous `evidence/m9-20261016/received-context/` :
`prepared-prompt.txt`, `provider-input.txt`, `agent-after-reopen.json`,
`prior-before.json` et `prior-after.json`.

| Contrôle | Résultat | Preuve dans `evidence/m9-20261016/` |
| --- | --- | --- |
| Livraison gérée et reprise hors empreinte avant correction | Deux FAIL effectivement exécutés | `context-receipt-red-executed.json/.log` |
| Correction, réouverture, garde de capacité et refus d’écriture avant invocation | PASS | `context-receipt-green.json/.log`, reçus exportés |
| Aperçu figé, mémoire versionnée, assemblage des méthodes, identité d’observation et histoire de reprise | PASS, lot de 20 tests nommés et 13 sous-tests | Même journal ; liste exacte des tests dans le reçu |
| Supervision, refus sans fournisseur, copies de manifeste et lecture publique CLI/API | PASS, 6 tests nommés avec `-race` | `context-receipt-race.json/.log` |
| API visant une autre mission | 403, sans divulgation du prompt | Test public du même lot `-race` |
| Capacité exacte et dépassement, compte en octets UTF-8, ancien manifeste absent | PASS | Test `TestMemoryContextBindingPreservesMetadataAndRefusesOverflow` |
| Analyse statique et compilation du candidat | PASS | `vet.receipt.json/.log`, `build.receipt.json/.log`, `built-binary.json` |
| Version du binaire construit | PASS du parcours `--json version`, sans serveur remplacé | `built-version.json` |
| Reprise d’une conversation fournisseur interrompue, effet d’un LLM réel ou fonctionnement du serveur installé | NOT TESTED | Ce lot ne lance pas ces parcours. |

Les lots comprennent **21 noms de test distincts** : les 20 tests initiaux et le
test public supplémentaire ; cinq des six tests `-race` recoupent le premier
lot. Ils ne sont pas additionnés comme 26 scénarios indépendants. Après ajout
du seul test public et de ses imports, les octets exacts du fichier de test vert
initial ont été retrouvés par empreinte et conservés dans `preimages/`.
`green-test-source-assessment.json` explique cette liaison ; aucune source de
production n’a changé entre les deux lots.

La CLI `agent show` et l’API authentifiée `/api/v1/agent-detail` sont vérifiées
par leurs vrais points d’entrée compilés dans le test. Le binaire de candidature
est construit séparément. Aucun résultat n’est attribué au serveur déjà en cours,
et aucune recette navigateur n’est revendiquée : la présentation web n’a pas été
modifiée. Les documents [FR](../../PROJECT-MEMORY.md) et
[EN](../../en/PROJECT-MEMORY.md) décrivent les mêmes champs et refus.

## Incidents de méthode et revue locale

Le premier reproducer ne compilait pas : `finishAgent` attend un code de sortie
`*int`, pas un objet d’usage. `context-receipt-red.json/.log` conserve cet essai
sans aucun test exécuté. La signature réelle a été lue, l’usage placé dans
`Agent.Usage`, puis les deux échecs de comportement ont été obtenus avant toute
correction du moteur. Cet essai sans collecte ne compte pas comme régression
reproduite ou test réussi.

Plusieurs recherches initiales utilisaient encore des noms supposés de fichiers
absents. Les producteurs et consommateurs ont ensuite été localisés par la liste
réelle des chemins et des symboles : `agents_store.go`, `agents_process.go`,
`worker_recovery_context.go`, `console_cli.go` et `web_server.go`. Le défaut vient
de cette exploration, pas d’une consigne manquante du dépôt. L’ancre trop précise
d’un premier patch documentaire a également été refusée ; le patch n’a été
appliqué qu’après correction de l’ancre, sans écraser les documents.

La revue locale compare les modifications aux préimages, pas à un checkout
supposé propre. Elle suit le manifeste de préparation, l’ajout au départ,
l’écriture durable, l’entrée réelle du fournisseur de test et les lecteurs
CLI/API. Le consommateur d’identité d’observation est couvert par deux tests
ciblés, dont une supervision réelle avec processus local contrôlé. Cette revue
dans le contexte d’implémentation n’est pas une approbation indépendante.

Apprentissage réutilisable : l’empreinte doit être calculée après tous les ajouts
de l’étape concernée ; un aperçu et une entrée de départ enrichie sont deux reçus
distincts ; aucune invocation ne doit précéder l’enregistrement de son contexte.
Ces règles sont portées par le calcul commun, les appels de production, les
tests de panne et la documentation FR/EN, sans réécrire les preuves historiques.

## Limites et suite du besoin mémoire

Le lot qualifie la traçabilité d’une mémoire de reprise pour une **nouvelle
tentative**. Il ne restaure pas une session fournisseur après arrêt du wrapper,
ne prouve pas qu’un LLM vérifie un effet historique et n’autorise aucun rejeu
aveugle. Le refus de redémarrage implicite du dialogue est préservé. Les textes
terminés et les évaluations V1/V2/V3 antérieures restent historiques.

La reprise complète de conversation devra définir la frontière entre un tour
répondu, un effet dont le résultat manque et une nouvelle tentative, en conservant
consommation et autorisation du superviseur. Résumé progressif et suppression
restent également ouverts. Aucune politique nouvelle ni autonomie supplémentaire
n’est créée pour fermer ces écarts par déclaration.
