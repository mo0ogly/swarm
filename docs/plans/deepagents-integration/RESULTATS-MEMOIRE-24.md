# M28 — enveloppe Work commune, lectures et mutations

17 octobre 2026. Livraison locale par Codex seul ; aucun commit, déploiement,
redémarrage, agent natif ou appel de modèle réel. Branche
`codex/clear-product-method-names`, HEAD de reprise
`4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, checkout partagé déjà modifié.
Le périmètre propre se lit dans `evidence/m28-20261017/owned.diff`, séparément
de l'ensemble du travail partagé. [Plan de l'incrément](INCREMENT-MEMOIRE-24.md).

## Résultat et portée

`work_row.go` extrait le contrat SQL de M27 dans un module général. Une lecture
obtient clé SQL, révision et corps ensemble ; UTF-8/JSON, schéma 1, clé valide
et concordance identité/révision positive sont exigés. `Store.get/list`, les
deux lectures de mutation/rejeu communes et les wrappers mémoire le partagent.
Un refus de ligne rend un Work vide ou une collection absente avec erreur,
sans résultat partiel ni callback métier/hook. La réservation SQL existante
peut avoir exécuté une instruction sans changement avant le refus ; ce dernier
préserve l'état métier et les événements par rollback.

Les causes SQL/JSON restent enveloppées ; `sql.ErrNoRows` reste reconnaissable.
Un rejeu exact retourne l'état courant validé sans répéter l'action. Les
champs historiques optionnels restent lisibles ; les octets inconnus stockés
ne sont pas réécrits par une lecture. Cela ne signifie pas que le type Go
restitue les champs JSON inconnus ni qu'une mutation ultérieure les préserve.
La création conserve son producteur et sa première révision persistée positive.
Les gardes de version du stockage, de réservation, d'événement et de CAS ne
sont pas remplacées.

Contrat opérationnel : [FR](../../architecture/WORK-ROW-CONTRACT.md) /
[EN](../../en/WORK-ROW-CONTRACT.md). Aucune concordance SQL/JSON n'atteste
propriétaire, auteur, droit d'usage ou authenticité contre une réécriture
conjointe des données. Les dix conformités publiques M20 et l'engagement
atomique restent ouverts.

## Échecs originaux et reprises

Les fichiers de preuve ci-dessous sont dans `evidence/m28-20261017/`.

1. `red-run.json` : compilation impossible, zéro test exécuté. Un test du
   chantier identité référençait un service pas encore écrit ; neuf entrées
   inventoriées changent pendant le contrôle. Ce résultat n'est pas un
   contre-exemple comportemental. Le statut public du chantier est consulté,
   sans message ; les définitions sont ensuite présentes et leur compilation
   observée terminée. Aucun fichier n'est exclu ni désactivé pour tester.
2. `red-current-run.json` : onze noms FAIL (quatre parents et sept sous-cas),
   plus l'échec du paquet, sur entrées stables. get accepte cinq formes de
   ligne incohérente, list publie une collection, la mutation atteint le
   callback avant le refus SQL, le rejeu retourne le corps incompatible et la
   CLI le publie. `red-current-test.go.txt` conserve le test exact avant correction,
   avec empreinte identique à celle du manifeste de cette exécution.
3. Après correction, les gardes positives explicitent le rejeu d'un événement
   ancien vers l'état courant, les champs historiques et les causes d'erreur.
   Une branche de regex `TestLaunchWriter` ne désignait aucun test ; le nom
   réel `TestLaunchWaitsForConcurrentWriterWithoutDuplicate` est exécuté sous
   race. L'audit final exige des noms réellement présents dans les reçus.
4. La revue de l'inventaire détecte neuf faux positifs `workspace_*` dans les
   40 candidats SQL initiaux. Le filtre exige maintenant la limite exacte du
   nom `works`, appliquée aussi aux données originales. Les originaux sont
   conservés ; voir `callers-inventory-correction.json`.

Les recherches de chemins supposés `transaction_reservation_test.go` et
`docs/REFERENCE.md` n'avaient aucun résultat. Reprise par inventaire du
répertoire et des symboles réels ; la référence française est à la racine.
Ces défauts de notre méthode n'ont pas déclenché de modification sur un chemin
inventé. Leurs corrections et limites sont conservées dans l'incrément.

## Vérifications exécutées

Les recettes n'imposent aucun watchdog global : `go test -timeout=0`, subprocess
sans timeout, progression observée. Les délais fonctionnels du produit restent
inchangés. Go `go1.24.3 linux/amd64`, dépendances figées `-mod=readonly`.

| Lot | Résultat réel | Preuve |
| --- | --- | --- |
| Normal ciblé transversal | 36 tests parents et 20 sous-cas PASS ; zéro FAIL/SKIP | `candidate-run.json`, `candidate-tests.jsonl` |
| Concurrence ciblée `-race` | 17 parents et 15 sous-cas PASS ; zéro FAIL/SKIP | `race-run.json`, `race-tests.jsonl` |
| Version de stockage sur connexion existante | Un parent PASS ; zéro FAIL/SKIP | `storage-guard-run.json`, `storage-guard-tests.jsonl` |
| Analyse statique et compilation isolée | `go vet ./internal/engine`, `go build ./cmd/swarm` PASS | `candidate-run.json`, logs vet/build et SHA du binaire |
| Audit final | Empreintes, noms exécutés, diff propre, liens locaux, gofmt et `git diff --check` | `audit-final.json`, `audit-final.py`, `owned.diff` |

Les trois lots se recouvrent : 89 occurrences PASS, **58 noms distincts**
(38 parents et 20 sous-cas), pas 89 tests indépendants. Les inventaires locaux
avant/après chaque lot sont identiques. Les entrées Go des trois lots sont
identiques après exclusion du seul nom/empreinte de leur recette propre ;
l'audit vérifie aussi leur état courant. L'inventaire des dépendances couvre
1 099 fichiers locaux dans le lot normal, y compris des fichiers de tests de
dépendances qui ne sont pas tous exécutés par cette commande.

Le binaire isolé de preuve est `/tmp/swarm-m28-bin-ll38teiz/swarm`, SHA-256
`8bf97ad824a4533333ca6b65b1bdd0856bed3362b6dfe62632f0c7b2f7c7655c`.
Il ne remplace aucun serveur déjà démarré. Les fixtures CLI utilisent le
parcours `run`, l'API son handler HTTP réel avec session de fixture ; aucune
vérification navigateur, serveur de production ou fournisseur réel n'est
déduite. Les tests de contexte/provider emploient leurs doubles locaux.

### Matrice des critères de M28

| Critère | Preuve observée | État local |
| --- | --- | --- |
| M28-01 | Les onze échecs nommés de `red-current` reproduisent get/list, mutation/rejeu et CLI ; source originale liée au manifeste | Vérifié |
| M28-02 | `TestWorkRowStoreGetRejectsInvalidRows`, les tests `TestMemoryWork*` réexécutés sur le noyau partagé et `TestWorkRowReadEnvelopePreservesLegacyAndCauses` | Vérifié pour l'enveloppe, pas tous les champs métier |
| M28-03 | `TestWorkRowPublicReadersRefuseIncoherentWork` : CLI show/list code 2 sans stdout, HTTP 400, stockage inchangé et retour à une fixture valide | Vérifié CLI/handler HTTP, pas navigateur |
| M28-04 | `TestWorkRowCommonMutationRejectsBeforeCallbacks`, rejeu courant dédié, idempotence, concurrence, rollback, attente du rédacteur et garde de version connectée | Vérifié, dont variantes sous race |
| M28-05 | Mémoire versionnée et reçus extractifs, réouverture, lancement, archives/import hostile, plans/agents, inspection/migration dans les noms exécutés | Vérifié sur frontières ciblées ; aucune suite complète prétendue |
| M28-06 | Inventaire corrigé, revues des appelants touchés, entrées stables, vet/build, docs FR/EN, audit et RETEX ci-dessous | Vérifié localement par l'auteur ; aucune revue indépendante |

Les sources Go propres sont `internal/engine/work_row.go`, `work_row_test.go`,
`store.go` et `memory_work.go`. Les corps de get/list et les deux lectures de
mutation/rejeu changent ; le reste du protocole de mutation reste identique
aux préimages. Les sept sources marquées `read_only` dans `baseline.json`
(lancement, identité, archives et mémoire publique) restent inchangées par ce
lot et liées aux entrées vérifiées. Pas de nouvelle politique ni de plafond.

## Inventaire corrigé et suite concrète

L'inventaire AST initial portait sur 369 fichiers de production Go sélectionnés.
Il détectait 168 sélecteurs syntaxiques get/list et 40 candidats SQL, dont neuf
sur `workspace_*` par erreur de sous-chaîne : **31 références exactes à works**.
L'inventaire corrigé courant examine 371 fichiers et conserve 168 sélecteurs,
avec **27 requêtes littérales sur la table exacte**. Les deux nouveaux fichiers
de production sont le noyau Work et le service identité ; l'extraction des six
requêtes Store/mémoire vers deux requêtes communes explique quatre sites en moins.
Le helper exact et ses empreintes stables sont conservés dans
`inventory-current.go.txt` et `callers-current-summary.json`.

Ce filtre lexical couvre les littéraux SELECT avec FROM works, nu ou cité.
Ce n'est pas un parseur SQL complet, un graphe d'exécution, un inventaire des
noms qualifiés/requêtes construites, ni un audit de propagation d'erreur dans
les 168 sites. Les nombres ne constituent aucun pourcentage de couverture.
Certaines requêtes ne lisent que révision/existence et certains lecteurs
valident déjà leur propre contrat. Ne pas déclarer ces 27 sites vulnérables.

| Suite conservée | Origine et valeur attendue | Propriétaire et reprise | État |
| --- | --- | --- | --- |
| Lecteurs SQL directs des sources mémoire, puis avant effet | `memorySummarySource`, `prepareLaunch`, `export` et autres fonctions dans `callers-current.json` ; éviter qu'une voie distincte contourne le contrat d'enveloppe | Chantier mémoire/Store (Codex) ; examiner producteur/validation/consommateurs puis reproduire avant extension. Critère : refus avant publication/effet, historique inchangé, reprise valide | Inventorié, audit spécifique restant |
| SQL explicite automation, préparation, graphes, organisation, lifecycle, règlement et récupération | Inventaire AST, périmètres et transactions propres ; cohérence générale sans altérer droits/réservations | Mainteneur de chaque module, coordination par le plan mémoire/Store ; établir le contrat actuel et un cas discriminant avant correction | À instruire, défaut global non démontré |
| Lecteurs du plan/registre identité | `readAgentIdentityRecordedPlan` et publication dans l'inventaire ; cohérence sans contourner le service protégé | Chantier identité ; relier à ses contrôles déjà réalisés et à ses preuves actuelles | Hors modification M28, pas de message envoyé |
| Propagation d'erreur des get/list | Les sélecteurs syntaxiques ne prouvent pas la gestion des refus par tous les appelants | Chantier mémoire/Store ; prioriser publication et effets, vérifier absence de succès vide et contexte exploitable | À auditer par parcours |
| Droits M20 publics et autorité protégée | M20-U01–U10 du contrat ; propriété/auteur, identité affectée et engagement jusqu'à l'effet | Chantiers mémoire et identité, opérations protégées et principal humain légitime requis ; conserver la frontière locale sous compte distinct déjà choisie | Toujours ouverts ; aucun octroi implicite |

L'extension exhaustive immédiate a été comparée au noyau commun borné : elle
croiserait plusieurs contrats transactionnels et le chantier identité en cours.
Le noyau commun est réalisé et vérifié ici ; les autres chemins restent visibles
avec critères et propriétaires. Une coordination inter-chat a été proposée via
une question d'autorisation ; sans réponse humaine explicite, aucun message
n'est envoyé. Le service identité désormais présent est compilé dans nos entrées,
ce qui ne prouve ni son déploiement ni son raccord public complet.

## Revue locale, transmission et RETEX

Auto-revue avec la méthode `code-reviewer` : diff propre et contexte des quatre
sources, callbacks/hook après validation, rejeu avant callback, réservation,
version de stockage, event conflict, CAS, rollback, fermeture du curseur,
causes et sorties publiques. Aucun défaut introduit reproductible restant
dans ce périmètre après les contrôles. Ce verdict local ne satisfait pas une
revue indépendante du moteur. Le diagramme a été relu mais son rendu Mermaid
n'est pas testé dans cet incrément ; l'UI n'est pas modifiée.

L'apprentissage réutilisable est reporté dans l'incrément et les contrats
FR/EN : extraire au contrat partagé sans dépendance au consommateur mémoire,
lier colonnes et corps avant callback/rejeu, préserver les causes et l'histoire,
vérifier les noms effectivement exécutés et les limites lexicales de l'inventaire.
La première compilation en cours d'écriture ne justifiait pas une relance inchangée :
la présence des définitions et l'arrêt observé du lot voisin ont changé la
précondition avant le vrai reproducer. Les preuves initiales restent distinctes.

Les pages persistantes et le point de reprise du plan transmettent ce résultat
aux prochaines sessions. Les consignes communes imposent déjà ces vérifications ;
aucune consigne n'est dupliquée ni changée ici. Ces pages ne sont pas embarquées
dans les contextes d'agents ; aucun processus démarré n'est rafraîchi implicitement.
Le besoin initial de mémoire gouvernée reste ouvert pour l'autorité, les
attributions et l'effet courant ; M28 clôt uniquement cet incrément local.
