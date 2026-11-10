# M28 — lecteurs généraux et mutations communes de Work

17 octobre 2026, reprise « suivant », Codex seul, replan/APEX. Suite de la
découverte M27 : les lecteurs `Store.get/list` décodent seulement le corps SQL.
Le chantier identité a vérifié son coordinateur/témoin privé ; son service et
ses opérations publiques restent à réaliser. Sources de ce chantier préservées,
sans message inter-chat, modèle réel ou mission native.

## Observation et choix

Les appelants des deux méthodes couvrent CLI/web, supervision, plans, budgets,
assistants et coordination. Les mutations communes `mutateWithHookOnce` ont
aussi deux lectures directes : reprise exacte d'un événement et mutation
ordinaire. Renforcer get/list seuls laisserait cette voie commune hors contrat.
L'inventaire source doit distinguer ces appels des autres lectures SQL directes.

Deux voies : appeler les helpers de mémoire depuis Store, ou extraire le
lecteur de ligne dans un module général. Retenir l'extraction par responsabilité :
validation clé SQL/révision/corps, lecture unitaire et liste sans résultat partiel.
Les wrappers mémoire gardent leur contexte d'erreur. Le module ne dépend pas de
la mémoire ou du noyau identité. Création : le Work nouveau est validé par son
producteur et reçoit sa première révision lors de l'écriture ; aucune exception
n'est créée pour une ligne persistée à révision nulle.

Périmètre de réalisation : `Store.get/list`, lectures commune de mutation et de
rejeu, nouveau module général et tests ; wrappers mémoire, docs FR/EN et suivi.
Pas de changement de migration, politique, droits, données utilisateur ou
lanceur. Les autres lectures SQL explicites (export, automation, préparation,
organisation, etc.) sont inventoriées avec leur propriétaire et restent une
suite distincte ; ne pas déclarer toutes les lectures applicatives corrigées.

## Critères observables

| ID | Effet / preuve |
| --- | --- |
| M28-01 | Contre-exemples avant correction : get/list et rejouage acceptent une ligne incohérente ; entrées et sorties originales conservées. |
| M28-02 | Validation commune id/révision/corps, UTF-8/JSON/schéma 1 ; Work/collection absents sur refus, SQL/JSON causes conservées, aucun identifiant invalide recopié. |
| M28-03 | CLI/web de lecture refusent les lignes incohérentes sans résultat partiel ou migration ; retour au cas valide par fixture isolée, sans outil public de réparation prétendu. |
| M28-04 | Mutation/rejeu refusent avant callback/hook/enregistrement métier ; création, idempotence, CAS/concurrence et rollback restent valides. La réservation SQL existante peut déjà avoir exécuté une instruction sans changement. |
| M28-05 | Contrats mémoire M27, lancement/contexte, export/import, plans et inspection/migration vérifiés sur leurs frontières affectées ; race ciblé pour les opérations communes concurrentes. |
| M28-06 | Inventaire et limites précis, entrées stables, statique/compilation, revue locale, docs FR/EN et RETEX ; aucune authenticité ou acceptation globale déduite. |

Tests avant correction, fixtures SQL minimales pour les données que les triggers
migrés refusent déjà. Garder ces triggers ; ne pas les supprimer pour fabriquer
un cas. Les contrôles couvrent un contrat transversal, donc un ensemble plus
large que M27 est justifié ; pas de suite complète après chaque correction.
Aucun watchdog global justifié : `go test -timeout=0`, subprocess sans timeout,
progression suivie. Les délais fonctionnels du produit ne changent pas.

## M28-I01 — compilation du chantier parallèle et entrées mobiles

La première série rouge ne collecte aucun test : un nouveau test du service
identité référence des types pas encore écrits. Neuf entrées inventoriées
changent pendant ce contrôle ; échec de compilation et sources mouvantes, pas
contre-exemple comportemental. Originaux `red-*` conservés. Lecture du statut
public de la session identité (sans message) : adaptateur désormais écrit et
son propre lot en cours. Les types manquants sont maintenant présents :
précondition modifiée pour une nouvelle série `red-current`, sans modifier
leurs sources ou cacher les tests incomplets.

Comparer l'exclusion par overlay de fichiers d'identité en cours à l'attente
de leur compilation actuelle : retenir les sources communes maintenant écrites
et le contrôle d'empreintes avant/après. Aucun fichier n'est désactivé. Les
sources Go restent figées pendant nos contrôles et ceux observés du chantier
partagé ; inventorier/documenter en attendant. Une demande asynchrone propose
l'autorisation explicite de coordonner les phases par message inter-chat ;
aucun message n'est envoyé avant une réponse qui l'autorise.

L'inventaire AST des 369 fichiers Go de production sélectionnés par `go list`
recense 168 sélecteurs get/list et 40 SELECT littéraux sur works, avec chemins,
fonctions et lignes. Ce n'est pas un graphe dynamique complet ni une preuve
de propagation d'erreur dans chacun des appelants ; limites dans
`callers-summary.json`. Le helper exact est conservé ; pas de copie complète
du dépôt. Les lectures SQL directes hors périmètre restent une suite explicite.

Une recherche a supposé `transaction_reservation_test.go`, absent. Reprise
par recherche de symboles et glob dans le répertoire réel ; aucune mutation
n'a suivi cette hypothèse. Les tests de réservation existants seront sélectionnés
par leurs noms réellement recensés, pas par un chemin inventé.

## M28-I02 — contre-exemples stables, contrat commun extrait

La seconde série rouge exécute onze noms FAIL (quatre parents et sept sous-cas),
plus le paquet ; zéro PASS, entrées stables. get accepte cinq formes incohérentes,
list publie une collection, le rejeu rend le corps incompatible et une mutation
atteint son callback avant le refus de mise à jour SQL. La CLI affiche aussi la
ligne incohérente. Source et sorties `red-current-*` conservés.

Le noyau général est extrait sans dépendre de la mémoire ou de l'identité. Il
contrôle uniquement l'enveloppe persistée, pas toute l'histoire métier. Les
wrappers mémoire conservent leur contexte. Mutation et rejeu lisent ce noyau
sous leur réservation SQL existante ; la création garde son parcours producteur
normal. Un rejeu valide doit rendre le Work courant, pas la révision ancienne
de l'événement ; un test dédié le spécifie. Les champs optionnels historiques
restent lisibles et les lectures ne réécrivent pas les champs inconnus stockés.

## M28-I03 — décompte et noms réellement exécutés

Le lot normal passe 56 noms (36 parents et 20 sous-cas), le lot race 32 noms
(17 parents et 15 sous-cas), sans FAIL/SKIP et avec entrées stables. La garde
`TestReviewRetryConnectedWriterRejectsChangedStorageBeforeMutation` passe aussi
séparément : un rédacteur déjà connecté ne peut réécrire le stockage après un
changement de version. Vet et compilation réussissent sur les mêmes entrées
Go. Ces occurrences se recouvrent : elles ne s'additionnent pas en tests uniques.

Une branche de sélection `TestLaunchWriter` ne correspondait à aucun nom réel.
Le cas effectivement recensé `TestLaunchWaitsForConcurrentWriterWithoutDuplicate`
est sélectionné et passe sous race ; l'audit final exige explicitement ce nom
et les autres parents attendus dans les reçus. Une réussite de paquet n'est
pas une preuve de chaque branche de regex. Une recherche de documentation a
aussi supposé `docs/REFERENCE.md`, absent : le fichier français est `REFERENCE.md`
à la racine, confirmé par l'inventaire ; ses originaux FR/EN sont conservés dans
`documentation-baseline.json` avant modification, sans écraser la baseline initiale.

## M28-I04 — faux positifs de l'inventaire, correction vérifiée

La revue identifie un défaut du filtre de notre méthode : `FROM works` par
sous-chaîne accepte également `FROM workspace_*`. Les 40 candidats SQL
initiaux contiennent neuf faux positifs et 31 références exactes à `works`.
Les originaux restent intacts. Le helper AST corrigé conserve la syntaxe mais
exige une limite de nom de table, nue ou citée, après FROM. Réappliqué aux
candidats originaux, il exclut les neuf requêtes de workspace ; exécuté sur
les 371 fichiers de production courants, il recense 168 sélecteurs et
27 requêtes littérales sur la table exacte. Les quatre sites en moins résultent
de l'extraction des six requêtes Store/mémoire vers les deux requêtes communes.
Les deux fichiers de production supplémentaires depuis l'inventaire initial
sont le module Work et le service identité désormais écrit.

Preuves : `inventory-current.go.txt`, `callers-inventory-correction.json`,
`callers-current.json` et `callers-current-summary.json`, avec empreintes
stables pendant l'inventaire. Le filtre n'est pas un parseur SQL complet :
requêtes construites, noms qualifiés, appels indirects et propagation d'erreur
restent hors preuve. Prévention : vérifier les limites lexicales et les noms
réellement exécutés, pas seulement un volume ou un succès global.

## Livraison locale et reprise

Contrats FR/EN, références et suivi actualisés dans le même incrément. Pas de
modification des contextes embarqués d'agents, d'asset web ou de consigne commune
dans M28 ; pas de génération nécessaire pour ces pages de documentation. La
revue est locale par l'auteur. Les schémas Mermaid sont relus, pas rendus par
un navigateur dans cet incrément. Ni serveur en cours ni mission native n'est
redémarré. Le rapport `RESULTATS-MEMOIRE-24.md` conserve la matrice et le RETEX.
Les lecteurs SQL directs restants et la propagation d'erreur sont une suite
distincte ; les dix critères publics M20 restent ouverts avec leur dépendance
identité protégée. Aucune autorité nouvelle ne vient du lecteur de ligne.
