# Revue indépendante du delta request-local des gardes de revue

Base 6bfd1005b663319021be60745a38c66c4d5e20c5, delta non commité examiné le 2026-10-13. SHA-256 du diff des quatre fichiers production : 4b46294342f6d2f11a67fc7c469e94ed58b5b94b8ac13a7dd127f4a409480e2c. Nouveau test review_read_scope_test.go SHA-256 150ae915f15820bcc24dce1ad69e1be0c7babd6b28f1774666c8c78063eb037e. Aucun changement source effectué par le reviewer, aucun test/build concurrent, appel provider ou accès à une base privée.

**Verdict final du delta 3f121f759200b5f8d0a0d9c5d0170208b29b07ba : approve, sans finding concret identifié dans ce périmètre. Gain de latence natif qualifié sur la mission affectée, avec Work/validation/avis inchangés. La mesure ne prouve pas un plafond universel et le CLI reste à 10.42 s.**

## Contrats contrôlés

- Extraction du garde : comparaison exacte de son ancien corps depuis git HEAD et inspectIndependentReviewGuard courant, identité stricte. Schéma, critères, droits, candidat, tentative, preuves, garde de profil et mode historique inchangés.
- Store.readReviews est nil dans le Store vivant. Seul readScope construit sa carte dans une clone locale, au même endroit que readDigests. La closure du handler /api/v1/snapshot crée sa clone à chaque requête, avant cockpit/decisions/missionStatus ; missionStatus établit sa frontière top-level et réutilise la clone si déjà dans le snapshot. Aucune affectation de la clone au Store vivant et aucun retour de cette clone hors des projections examinées.
- Clé JSON [historical, Work, Task] puis SHA-256 : lie toutes les données sérialisées des gardes, dont contrat/tentative/état/candidat/configuration/modèle. json.Marshal est déterministe pour les maps Go et toute erreur est propagée. Le lookup avec known distingue succès nil, refus non nil et absence. Aucun refus perdu ni succès vide inventé.
- Les appels de projections examinés sont séquentiels ; une requête concurrente obtient une autre carte. Aucun goroutine ne reçoit cette clone. La goroutine de console.go située hors cockpitSnapshot n’est pas un partage de carte. Le cache ne prétend pas être sûr pour un futur partage entre goroutines ; tout futur parallélisme devra établir cette propriété avant usage.
- Les chemins vivants d’acceptation/apply ne demandent pas readScope et inspectent donc encore les fichiers. Les writes annexes de decisions (cartes opérateur) et assistCurrentTurns (réconciliation d’anciens tours) existaient déjà ; ils ne sont pas des mutations d’acceptation réutilisant les gardes en cache. Le commentaire « nil on live mutation paths » doit être compris comme propriété du Store vivant, pas comme affirmation que toutes les méthodes de projection sont sans aucun write SQL.
- Une projection partage intentionnellement l’observation physique de ses premières lectures. Aucun succès n’est retenu entre requêtes. Si Work/Task changent au cours de la projection, la clé change ; changement externe de preuve sans changement de Work est détecté à la projection suivante, comme pour les empreintes des gates existantes.

## Régression inspectée sans exécution reviewer

TestIndependentReviewReadScopeReusesOnlyItsOwnSnapshot : garde original favorable, disparition du rapport, réemploi dans même projection, refus dans nouvelle projection et Store vivant ; refus stable dans même projection après restauration et succès dans projection suivante. Variantes contrat, tentative, candidat, état et historical imposent des clés distinctes et les refus attendus. La variante historical démontre succès historique avec ancien ProjectContext puis refus courant. Ces assertions portent sur le comportement, pas seulement présence du cache.

Le propriétaire conserve le rouge valide t7-review-read-scope-valid-red.jsonl et exécute ses quatre tests ciblés -race. À l’heure de cette revue, leurs résultats ne sont pas inventés ni déclarés PASS par le reviewer. Pas de nouvelle suite entière, tests ou build lancés ici.

## Qualification de latence à compléter

Baseline publique rapportée par propriétaire : snapshot natif 41.66258s, CLI work show 20.95156s. Work lent à cinq tâches acceptées, deux lots de revue et contexte partagé. Le cache évite la répétition d’un même garde ; les gardes de tâches distinctes gardent leur première validation complète. Un succès de tests n’établit donc pas le temps native ou l’effet sur cette mission.

Comparer une seule lecture publique CLI et une seule lecture HTTP sur le nouveau binaire propre, mêmes données/racine/révision, sans provider et sans superposition avec une requête précédente encore active ; contrôler non seulement durée/exit mais les cinq avis, acceptations et fraîcheur. Préserver les timeouts originaux et les résultats exacts. Aucun verdict de déploiement ou clôture métier n’est déduit de cette revue source.

## Empreintes de source lues

store.go : 31fea4b7f69f932b28354a9605cfe05e11b4335e918e504f4dad361435317502
independent_review.go : ab786c3d8d6e51d1d98cbf099fe776d3ec21c04b98453292d7f7503c95e1c2bb
mission_status.go : 0eaec20b48ba59fa2357c6fa7a6490368714a53fef0931524ceb11bd2479a084
web_server.go : 9ba70f5c3f045adeca6668753ca624d1ed7765b5ed4af808a2149ba1dcc23166

Sources libérées ; aucune exécution reviewer active. Une modification pertinente ultérieure doit être liée à son nouveau diff/candidat.


## Qualification finale additive sur candidat propre 3f121f7

Candidat 3f121f759200b5f8d0a0d9c5d0170208b29b07ba confirmé par git rev-parse et checkout propre lors de cette lecture. Les cinq empreintes des sources/test ci-dessus sont inchangées depuis la revue non commitée. Le garde extrait reste identique et le verdict source est reconduit.

Preuves originales inspectées : rouge valide 7f7c72f19f09eb67e7f8aa940f3801350ff56c5617641f7329cad6e304dab06e (same projection repeats physical guard après suppression du rapport), puis testlog green e16c07c28a813ad54cbc14390c96186e2e4ee5c27a64f6354ea3e1166b15a83b. Le journal montre quatre tests top-level exécutés et PASS, neuf avec sous-cas : IndependentReviewProcessPersistsAndBlocksStaleEvidence, ReviewerReplacementHistoricalBaselineIsNotFreshAcceptance, IndependentReviewReadScopeReusesOnlyItsOwnSnapshot et ValidationDriftBlocksAckAndDependencies. Le propriétaire atteste commande -race ; pas de FAIL/race dans le journal. Le reviewer n’a pas relancé ces tests ou compilé.

Mesure réelle publique native originale conservée : t7-native-latency-original.json SHA 6bd18e21400518a4f7331ffde7846ca4b1c4ab6cbe7300b0e5ea7a1c39870f2d, 41.662579617s, r43. Mesure corrigée t7-native-latency-corrected.json SHA b29233c67f26b984862aaf4b530c7f2bdf233e081eb7a5ad4c0ea7df8cd7dea3 : snapshot 1.767678870s, r43, cinq validated, zéro stale, zéro agent actif et cinq avis passed. CLI work show exit0 en 10.417999247s contre 20.95156s. Ce sont des mesures ponctuelles du propriétaire, pas un benchmark contrôlé avec répétitions ; gain effectif observé, CLI encore relativement coûteux.

Comparaisons faites par le reviewer sur les buffers exacts :

- Snapshot original SHA 91832e044607511fbdec7a80fb70c7649fd56f9e710207757a6911f90cad09dd et corrigé SHA 6400cddc038a190b294b9480c7a8a708c5626a56e4bee19cf2260a950855f4a5 : work, validation, independent_reviews strictement égaux comme objets JSON.
- CLI original SHA 03794bce2edaf5d363d1ea01fe924f92397a7f2e13f8429eb7d007b3778c5a0d et corrigé SHA 35d7aaf3101a1586963fd94746824f3aaa5d724dc0ac8bab271cc261cda773da : work, validation et current_gates strictement égaux.
- maintenance-paused versus maintenance-upgraded : les quatre mêmes Works conservent révision, planning_activations, planning_decisions, review_calls, task_statuses, candidat, slots et autonomie. Postérieur tous paused, zéro agent/agent incertain, aucune revue/exécution active. Ce reçu qualifie la phase de maintenance ; la restauration d’un autre Work initialement non paused reste une opération distincte du propriétaire.

Déploiement vérifié par observations locales en lecture seule : reçu t7-native-deployment-code-receipt.json SHA 22a4719de69c4c4910218228cc6091bfe063e1b9c6a23b0fe5fda1add194343b. Les cinq assets effectivement servis ont des empreintes égales aux fichiers web sources. /proc/2599704/exe vise /home/fpizzi/Documents/ChatGPT/swarm/bin/swarm ; empreinte des octets de cet exécutable égale au reçu 35526c31c35712fa0b52ee45aa7fac9230c0e0ef7f9bcfde85a7033627654fc9. L’identité publique injectée du serveur rapporte commit 3f121f7, modified=false, ready, correspondance source/binaire. go version -m confirme Go 1.24.3 et le module Swarm ; cet exécutable trimpath ne fournit pas un champ VCS supplémentaire, donc identité de commit établie par reçu de construction + diagnostic public, pas par un champ absent inventé.

La recette livrée isolée a aussi été relue par le propriétaire via nouveau binaire, sans appel payant : t7-final-reader-freshness.json SHA a1a7c343a196db019f1b9a950e3482bfa8866b65305cd7addde49a8c6195f7db, revision127, deux validated, zéro stale. SHA stdout 440367a576f89c7991442a5db63c4b42e922ae6dfaee0f646d0b71a16d92f09f identique à la lecture qualifiée précédente sur 6bfd100. Ce contrôle prolonge la preuve de fraîcheur du candidat livré b569 sans refaire la recette provider.

Aucun finding nouveau. La correction est qualifiée dans son périmètre : cache par lecture, gardes préservés, tests de régression, service actif et gain réel sans changement métier. Limites conservées : mesures ponctuelles, annulation du travail serveur après timeout non corrigée par ce delta, coût résiduel CLI 10.42s, historique d’interventions et recette traversant plusieurs versions. Une évolution future du contexte de requête/parallelisme doit suivre sa propre revue ; aucune nouvelle politique de temps, droit ou acceptation n’est créée. Documents ultérieurs seuls ne changent pas ces modules ; tout autre delta engine devra être réexaminé.

Sources libérées et aucun check actif.
