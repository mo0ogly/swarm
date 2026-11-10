# RETEX-QW2 — revue indépendante du correctif de complétude

Base b61694264cbeaf0d72cea8e55201c1fd1f7aaed1, delta non commité lié par empreintes ci-dessous. Lecture seule ; aucun test/build/provider, agent supplémentaire, modification candidat ou accès à une base privée.

**Verdict : approve. P2 du rapport initial corrigé et vérifié ; aucun finding ouvert identifié dans ce delta.** Le rapport initial request changes reste intact.

## Correction du registre complet

agents_store.go extrait la lecture commune agentHistory(work,recent), requête paramétrée par Work. agents(work) conserve recent=true et LIMIT200 ; spendingAgents(work) demande recent=false sans limite. Même decodeAgentRow, fermeture de rows et propagation d’erreurs. missionStatus utilise la lecture complète seulement pour missionSpending. Aucun élargissement de la vue récente, lecture inter-Work, mutation, modèle inféré ou politique cachée ajouté.

Le compteur de reprises distingue les tentatives non vides par TaskID et prend max(nombre distinct -1, liens Previous explicites legacy), borne inférieure0. Les agents étrangers sont filtrés avant comptage. Les agrégats et attemptLedgers reçoivent l’historique complet ; le compteur ne dépend plus uniquement de Previous, absent des reprises ordinaires du superviseur. Les reservations planning/reviewer ne sont pas comptées dans cette boucle de reprises. Pas de double addition du minimum legacy et du nombre distinct.

## Régressions reçues examinées, non réexécutées

- retex-qw2-history-red.jsonl SHA2ef93f6a25247275d48ca62b2ae864b32a5a35a5b8448c784501dd471d501f9f : TestMissionSpendingPreservesCompleteWorkerHistoryAndRetries FAIL valide, silent worker history truncation200 instead of201.
- retex-qw2-history-green.jsonl SHA6b1ac876c616cd2278d7293917bbf5eab56a8f9827510d48fc2f06efdfeec95f : cinq tests spending top-level PASS, dont unités/absence coût, reviews/retries par tâche, controls managed/automatic, ledger usage/cache et histoire complète/reprises. -race rapporté par propriétaire, aucun FAIL/race observé au journal.
- retex-qw2-history-isolation-green.jsonl SHA14b336d0f0c2e39689d44a75bb791121a8a07319938d7f90c14caa79b04ee65c : variante finale historique PASS. Source test inspectée :201 appels, première ledger-000 présente, foreign-worker absente, vue récente toujours200, agrégats201 input/402 output,200 reprises.
- retex-qw1-spending-recheck.json SHA9b8227c39e7d3b61c93924ad748818dbc3b158028fd7545e97ee95a5c63ea10f : relecture publique du scénario réel, deux tentatives même recover-number, worker_retries1/task_retries1 ; double initial usage null, worker correctif modèle demandé persisté, caches bruts et coût non rapporté préservés. Pas de nouvel appel pour répéter QW1.

## Harnais et navigateur

Le harnais interdit combinaison setup-only/run-prepared/resume-mission via groupe argparse et exige resume-setup avec setup-only. La vérification QW1 lie une décision native à un ID de revue changes_requested réellement vu, via input_events, avec operation.kind retry, même task.id et instruction next non vide. Ceci remplace la simple recherche textuelle retry dans l’événement ; aucun verdict natif n’est fabriqué.

La recette browser courante ouvre l’accordéon actions seulement si son état DOM est fermé, puis utilise le vrai trigger spending et toggle details ; aucune source UX contournée. Les premières erreurs de harnais d’accordéon/état mémorisé restent distinctes d’un défaut produit. retex-qw2-browser-final/result.json SHA32bc2a7deb3c08ec4f878cbe5a841063e85883c36d0bea6ea47ad045778556bb : quatre combinaisons FR/EN × etat/sombre PASS, sept appels, clavier/Enter, Escape/focus-restoration, largeur390 sans overflow, aucune exception/request failure/HTTP error ; le script compare Work avant/après par parcours public. Ce sont les assertions du navigateur réellement exécuté par propriétaire, pas un nouvel exercice reviewer.

Le reviewer a inspecté les captures mobileFR/etat et desktopEN/sombre : modal lisible, actions de fermeture et registre visibles selon scroll ; information coût/usage manquant explicite, aucun prix fictif. La capture mobile ne montre qu’une partie du contenu défilant, donc aucune couverture visuelle exhaustive de chaque carte n’est inférée. Les tests navigateur portent l’interaction effective et les diagnostics ; la capture seule n’est pas traitée comme PASS.

## Contrats inchangés reconduits

decodeSpendingUsage reste identique à la première revue : null whitespace absent, nombres nécessaires non négatifs, optional cache/cost conservés et erreurs propagées. Affichage textuel sûr et coût null distinct de zéro explicite. Aucun fournisseur/modèle historique planner/reviewer déduit des réglages courants. QW1 conserve son premier producteur double clairement étiqueté, la revue réelle refusant son code, retry reçu, worker/révision revue neufs et accepted/fresh/end. Aucun coût ou acceptation historique inventé.

Le compteur operator_interventions_after_start du harnais seul reste une déclaration du harness et non un inventaire indépendant de tout changement extérieur ; la portée contrôlée QW1 et ses opérations de setup/fin restent explicites. Cette revue ne qualifie ni QW3 encore active ni campagne d’échelle, endurance, facture, déploiement final ou identité d’un prochain commit.

## Empreintes du delta lu

internal/engine/agents_store.go : 26dbb4d6bbddc016e1efbd7af85db57042418bc618c795b33cc07b6cc155629f

internal/engine/mission_status.go : 552552072eb73e5ca1d479d833256af86876033330292de77eca256147e62f9c

internal/engine/mission_insights.go : 1e98afcf0fc899e719165eae0b480d43d3e469ab7fd858d4d85bc96ed1b788d2

internal/engine/spending_calls.go : 17d274b7795388da08bed63add537b6ad6ca2a8fe4e7c47c8796cbdc25ea97cf

internal/engine/spending_calls_test.go : ab4dfed3493a805108695a53b4c55852ee222cfa3de5ea751ec6a9ba44ce0aa9

web/mission-insights.js : d291a341b41a7292efab5b93457282bf8ee282f07fc978f39c99ae5ea7cf306f

locales/en.json : d0b8e7d544846a8dd98dff539b0813b2a75f2b1f7479baddcda6e20d14d8fa17

web/i18n-en.js : be047dcb55a7c7397a536970b2ce42c2ea5d76a099c10de2c5a3f1882d7a935f

tests/call_ledger_ui.cjs : 75585aa1fd3caaf167a615dfd311805f138bd82cc66bca4cb83fac630294b6b8

tests/retex_real_campaign.py : aee4768e34322565ab7e451d56a7cb977ab5732a0a5a4a7151dc28268a82fba5

Diff suivi SHA-256 : 0bb34e91f242230a2d6c2b7733ff4989282f356c55465202da613a5d43354603

Sources libérées. Aucun check reviewer actif. Tout delta pertinent ultérieur doit être lié à son source/input ; aucune répétition provider QW1 requise pour ce correctif de lecture.
