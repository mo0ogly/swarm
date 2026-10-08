# T5 — diagnostic du contrôle final

Observation du 4 octobre 2026, 17:47 UTC. Attribution : supervision du contrôle moteur.

Le reçu public `.swarm/validation/w-a03028dc0d3f69d1f52f0ee6/plan-a03028dc0d-T5/a-2c484d614151b6e78cc327d2-receipt-a911d8bba34061d3a7641af5.json` conserve un échec : délai de 300 secondes dépassé, code -1, du 17:41:55.876 UTC au 17:46:55.879 UTC. La seule sortie capturée est le lancement de `go test ./... -count=1`. La recette Python capturait les sorties de Go jusqu’à sa fin ; ce reçu ne permet donc pas d’identifier le test lent ou bloqué. Vet, configuration, diff, build et navigateur ne sont pas démontrés par ce contrôle interrompu.

Le conducteur a démarré automatiquement la seconde tentative T5 à 17:47:02 UTC, avant qu’une correction de précondition soit démontrée. Cette reprise respecte le nombre maximal de deux tentatives, mais le déclenchement ne démontre pas la règle « reprise après correction vérifiée ». Ne pas attribuer ce défaut au fournisseur, ni considérer la seconde production comme un test réussi. Aucun plafond augmenté, aucun reçu remplacé et aucune acceptation forcée.

Prochaine vérification : inspecter le handoff de la seconde tentative et n’autoriser un nouveau contrôle global qu’après une correction concrète vérifiée. Les quatre tâches précédentes restent acceptées ; la mission est 4/5, non clôturée. Le coût réel reste inconnu.

## Second contrôle — 18:05 UTC

Révision publique 131 : T5 bloquée, deux tentatives de production consommées. Second contrôle moteur du 17:54:09.769 UTC au 17:59:09.771 UTC : même code -1, même délai de 300 s et même empreinte de sortie `29ec6ccd3f8dae47dc9b76eda2e43f719a081cfdacac62174fbf0067e0278424`. La seconde production n’avait corrigé que le rapport ; aucune précondition d’exécution n’avait changé. Le conducteur a donc rejoué une suite identique en échec : défaut de reprise attribuable au moteur, pas preuve d’un échec fournisseur. Aucun agent actif ni processus de cette recette observé à 18:05 UTC.

La lecture ciblée des fixtures de silence, processus et terminal ne démontre pas à elle seule la cause interne du dépassement global. Il faut un diagnostic observable borné ; aucune troisième production ni répétition globale sans correction vérifiée n’est autorisée. Les preuves et reçus des deux échecs sont conservés.

## Correction autorisée par l’utilisateur

Le dispatcher refuse désormais une correction automatique par un worker lorsque le contrôle n’a pas été exécuté ou ne dispose pas d’un code de fin (timeout : -1). L’empreinte de cause exclut le chemin unique du reçu ; une cause identique après correction n’est plus rendue nouvelle par un reçu différent. Les échecs objectifs avec code de sortie restent admissibles aux corrections dans les plafonds existants.

Vérification : `go test ./... -run 'TestValidationRecoveryRequiresObjectiveVerdict|TestAutomaticValidationCorrectionReplacesOldEvidenceWithinLaunchBound|TestValidationTimeoutKillsDescendants' -count=1 -timeout=60s` : code 0, 1.165 s. `go vet ./...`, contrôle de configuration et diff : code 0. Build canonique `/tmp/swarm-validation-recovery-candidate` : code 0 ; installation live non effectuée à ce stade.

La recette transmet les sorties progressivement et demande JSON + mode verbeux Go ; un délai Go interne de 240 s fournit une trace avant la limite moteur inchangée de 300 s. Premier diagnostic complet : code 1 après 241.7 s ; trace dans `/tmp/swarm-final-observable.log`, dans la validation de fragments pendant TestManagedFragmentHistoricalPublicRecovery. Contrôle ciblé de ce même test : code 0, 52.041 s (`/tmp/swarm-fragment-diagnostic.log`). La trace globale ne démontre donc pas un blocage du test ; sa durée et le cumul des autres tests restent à traiter. Aucune réussite globale ni clôture revendiquée.

Le candidat corrigé a ensuite été lancé sur 127.0.0.1:18792 après vérification publique de l’absence d’agent actif. La mission reste 4/5 ; aucun budget, tentative ou verdict modifié.

## RETEX de la phase finale et découpage proposé

La phase finale cumule six responsabilités : inventaire des contrôles, exécution moteur, CLI/configuration, web, cohérence des preuves, avis indépendant puis gate de livraison. Le test historique isolé termine en 50–52 s ; la charge mesurée porte notamment sur JSON/empreintes/relectures, pas sur un fournisseur externe. Le regroupement séquentiel dépassait 300 s et masquait sa progression. La recette complète répartie a couvert 973 tests une fois chacun, tous codes 0, puis vet/configuration/diff/build/web, en environ 234 s. Ce résultat local doit encore être confirmé par le reçu moteur sur le candidat courant.

Architecture proposée pour une future évolution : un superviseur de validation possédant un manifeste figé du candidat et des contrôles ; lots moteur, CLI/configuration, web, preuves/RETEX ; sous-agents pour analyse qualitative ou anomalie, processus déterministes pour les contrôles automatiques. Chaque lot livre commandes, codes, couverture, limites et empreintes. Le superviseur vérifie exhaustivité et identité du candidat ; une modification invalide ses lots dépendants et impose les régressions concernées. Le vérificateur indépendant reçoit la consolidation et les originaux nécessaires. Seul le moteur décide de la gate et de la clôture.

Ce découpage n’est pas implémenté par la seule recette à quatre processus : elle répartit la suite de tests, pas la gouvernance entre agents. Il doit conserver plafonds globaux, coûts inconnus, historiques, refus d’omission et distinction entre processus fini, preuve disponible et résultat accepté. Éviter d’ajouter un appel IA pour simplement lancer une commande déterministe. La décomposition doit améliorer diagnostic, responsabilité et reprise, pas créer quatre validations isolées sans contrôle des interactions.

## Clôture publique vérifiée

La reprise après correction a produit un reçu moteur réussi, complet et non tronqué (4 683 octets), couvrant 974 tests puis vet, configuration, diff, build et dix contrôles navigateur. Le vérificateur indépendant a déclaré les trois critères PASS. La gate de livraison fraîche et l’acceptation normale ont été appliquées publiquement ; cinq tâches sur cinq acceptées. Le responsable root a ensuite rendu sa décision et son état public est closed. Le diagnostic public indique : « Résultats validés et responsabilité racine clôturée dans cette mission. » Aucun budget augmenté, aucune troisième tentative, aucun commit ni push.

Les FAIL et timeouts du rapport T5 décrivent les étapes historiques ; le reçu frais et l’avis ultérieur favorables les supplantent pour la décision actuelle. Ce fichier explicite la chronologie sans modifier les artefacts acceptés. La documentation utilisateur et moteur existe en français et en anglais ; l’organisation entre sous-agents reste une proposition, distincte des quatre processus de tests implémentés. L’ancienne mission reste cinq tâches acceptées sur six et n’a pas été clôturée.
