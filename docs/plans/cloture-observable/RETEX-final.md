# RETEX — clôture observable et reprise fiable

## Résultat vérifié — 4 octobre 2026

Mission `w-58060a818f1fad5e9a3286b7` : cinq tâches acceptées avec gates et revues indépendantes, responsabilité racine `closed` à la révision 96. Le reçu final r89 `a-a7289c486eacd9880ed58709-receipt-65a590f2ae74522196d29687.json` atteste le contrôle réel `python3 tests/supervision_final_acceptance.py`, code 0, du 20:18:46.826751905Z au 20:22:06.507759619Z : environ 199,7 secondes murales. Il comprend suite Go répartie sans omission, vet, configuration, diff, build et navigateur isolé, puis les contrôles ciblés d'observabilité et recontrôle FR/EN clair/sombre. Le contrôle local n'est pas une preuve d'autonomie d'un fournisseur dans tous les environnements.

Le serveur 18792 a été remplacé après les validations et sans agent actif par `/tmp/swarm-cloture-observable`, build canonique du 20:26:49Z, base `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`, checkout modifié. Le diagnostic public de version confirme le serveur installé. Aucun commit ou push. Les documents et preuves acceptés des missions précédentes ont été conservés ; l'ancienne mission `w-01567e073c1ed2f3d4c71c9e` n'est pas clôturée par ce travail.

## Incidents, responsabilités et corrections

| Observation | Attribution | Correction vérifiée | Limite restante |
| --- | --- | --- | --- |
| Bouton de recontrôle invisible : lecture `auto_validation` au lieu de `automatic_validation` | Interface ; contrôle antérieur insuffisant | Parcours navigateur réel, lecture du champ public corrigée | Le résultat d'une fixture ne prouve pas tous les parcours métier |
| Deux textes de recontrôle restés en français en mode anglais | Catalogue d'interface | Traductions, bundle régénéré, quatre combinaisons langue/thème | Vérification ciblée, pas audit de tout le catalogue |
| Test héritait d'un focus imposé et d'un champ `revision` inexistant | Recette de test | Focus dans la modale ; assertion `expected_revision` | Un test écrit ne signifie pas un test réellement passé |
| Le checker documentaire triait par chemin, le manifeste par lignes sha256sum | **Supervision**, pas producteur | Tri exact corrigé ; 23 entrées vérifiées | Reprise prématurée par la supervision a déclenché une seconde production inutile |
| Documents FR/EN référencés mais absents du dossier présenté | Constitution du dossier de revue | Pièces jointes réelles, nouveau contrôle et revue sur résultat existant | Le reviewer ne peut vérifier une pièce simplement annoncée |
| Nouveaux alias CLI modifient la clé d'aide multiligne ; traduction ne correspond plus | Couplage catalogue / modification CLI | Nouvelle clé exacte, test bilingue ciblé vert | Une traduction par clé multiligne reste fragile |
| Sockets, PTY et Chrome interdits au producteur mais disponibles au conducteur hôte | Environnement fournisseur | Contrôles réels exécutés par le conducteur hôte | Ne pas déduire les capacités d'un fournisseur de celles de l'hôte |
| Recette longue réussie mais publication refusée après mutation de révision globale | Moteur et **supervision concurrente** | Stabilisation des décisions de planification pendant les contrôles ; reçu frais et revue normale | Pas de correction structurelle livrée de ce conflit : publication sur révision pertinente et réutilisation sûre des contrôles restent à concevoir |

## Enseignements et améliorations suivantes

1. Préparer et vérifier les contrôles avant d'autoriser leur exécution. Une défaillance de recette doit être distinguée d'un défaut du produit avant toute reprise de producteur.
2. Lier les preuves au candidat et au contrat pertinent ; une mutation administrative ne devrait pas imposer une nouvelle suite identique si toutes les entrées pertinentes sont inchangées. Toute réutilisation exige une vérification de fraîcheur, pas un contournement de gate.
3. Garder verdict courant et historique distincts. Les échecs antérieurs restent consultables, sans masquer le reçu actuel ni être présentés comme résolus avant une preuve réelle.
4. Mesurer séparément outils, requêtes modèle, jetons, coût, durée murale, CPU et attente. Les quelque 199,7 secondes du contrôle final ne sont pas un coût en jetons. Ne pas remplacer les valeurs inconnues par des estimations à partir des appels d'outils.
5. Les quatre processus de tests sont déterministes ; ce ne sont pas quatre sous-agents. Les analyses par domaine doivent avoir un responsable, une couverture explicite et une synthèse indépendante, sans déclaration de capacité non démontrée.
6. La combinaison moteur déterministe / agents probabilistes requiert aussi une supervision disciplinée. Un moteur ne compense pas une mauvaise recette ni des mutations intempestives de son superviseur.

Les mécanismes proposés ne sont pas annoncés comme implémentés. Ce RETEX est un bilan séparé : il ne remplace ni ne modifie les rapports acceptés et leurs empreintes.
