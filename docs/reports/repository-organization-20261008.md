# Résultat — réorganisation et installation de Swarm

## Résultat

504 fichiers Go de la racine sont regroupés dans `internal/engine`.
`cmd/swarm/main.go` appelle le moteur ; `resources.go` conserve les ressources
canoniques embarquées. Le moteur reste un paquet : aucun découpage métier en
plusieurs modules n'est revendiqué. Le build, les tests, les installateurs et la
documentation suivent cette structure.

Le code `45b18eb` a passé la
[CI complète](https://github.com/mo0ogly/swarm/actions/runs/37836580717), puis la
[PR #10](https://github.com/mo0ogly/swarm/pull/10) a été fusionnée sur `main`
(`c1f2b92`). Le second clone a installé ce main propre en natif, puis vérifié
les méthodes dans une nouvelle racine vide et le redémarrage du serveur existant.
Les corrections de méthodes et les captures sont détaillées dans le
[rapport d'installation](fresh-install-methods-20261008.md).

## Périmètre et conservation

La base initiale de réorganisation est `a09ae41d09c8bd229a7b7b299184ad7c9ac14eaf`.
Le pack public de formation FR/EN et les changements préexistants d'installation,
de session web et de préparation nécessaires à la distribution sont inclus.
Les travaux de benchmark et de publication scientifique restent hors de ces
commits ; 80 fichiers hors périmètre gardent leur empreinte d'origine.
Aucun état de mission utilisateur ni plafond d'exécution du moteur n'est modifié.

Les directives `go:embed` ne remontent pas vers un parent : le paquet de ressources
à la racine évite de dupliquer les méthodes, configurations et fichiers web.
Les tests gardent leur accès aux fonctions privées dans le paquet du moteur.
Leurs lecteurs et sous-commandes utilisent explicitement la racine du dépôt,
sans changement global du répertoire de travail des processus d'agents.

Le manifeste de réorganisation est un snapshot d'intégrité du code actuel,
prioritaire sur les anciens snapshots. Il ne renouvelle aucune acceptation
historique : les exigences antérieures restent marquées à requalifier.

## Vérification

| Contrôle | Résultat | Preuve et limite |
| --- | --- | --- |
| Structure, build CLI et métadonnées | PASS | Trois paquets, un seul Go à la racine ; binaire installé depuis main identifié et propre |
| Inventaire Go | PASS | 1 068 cas conservés, trois régressions ajoutées : 1 071 cas découverts |
| Suite Go en CI | PASS | 19 groupes, six isolés et treize distribués sur quatre processus ; délai inchangé de 240 s par groupe ; tous les codes de sortie 0 |
| Résultats Go en CI | PASS avec skips explicites | 1 059 passés, 12 ignorés ; les skips ne sont pas comptés comme des succès |
| `go vet ./...`, `npm test` | PASS | CI et contrôles locaux ; régression des réponses JSON avec retours à la ligne incluse |
| Lanceur natif | PASS | Huit tests de vrais processus : paramètres conservés, reprise, persistance et serveur étranger |
| Installation native et Compose | PASS | Candidat local, clone GitHub propre et job CI ; recréation, HTTP authentifié, méthodes, propriété et intégrité SQLite |
| CLI, états et campagne Python | PASS | Racines isolées ; campagne : quatre tests passés et un skip opt-in ; fixture DOM explicitement simulée |
| Contrat et distribution | PASS | Neuf méthodes canoniques versionnées, commandes et documentation FR/EN ; aucun état ou secret suivi |
| Confidentialité et conservation | PASS | Archives publiques inspectées, overrides et documents privés ignorés, empreintes des travaux hors périmètre identiques |

Les douze skips Go en CI sont : `TestManagedRecoveryBrowserRecipe`,
`TestEngineContractTruthBrowser`, `TestLiveProviderStreamReplay`,
`TestCorrectiveRecoveryBrowserRecipe`, `TestHistoricalRequalificationBrowser`,
`TestAssistBudgetSharesTheLaunchEnvelope`, `TestRecoveryHealthBrowserRecipe`,
`TestOperatorPreview`, `TestAssistAskRequiresReviewedContextAndStaysSingle`,
`TestAssistTurnsStayWithinTheirWork`, `TestExecutionExplorationDirectives`
(`rg` absent dans le runner) et `TestMissingReportRecoveryBrowserRecipe`.
Les recettes de navigateur effectivement exécutées par CI sont distinctes de
ces skips : la disponibilité des méthodes des agents et l'i18n.

## Échecs locaux conservés et correction de la recette

La première suite Go séquentielle a dépassé 25 minutes. Des partitions à seize
processus ont également atteint 240 secondes dans trois puis quatre groupes.
Avec quatre processus, les deux protocoles les plus lourds ont encore dépassé
ce délai et le lecteur de manifeste sélectionnait un ancien snapshot après
l'intégration de main. Les protocoles longs sont désormais isolés, le snapshot
courant est sélectionné en dernier, et son entrée de régression JS porte le
bon type. Le test d'intégrité refuse toujours les entrées absentes, modifiées
ou incomplètes ; les contrôles ciblés passent dans le clone propre.

Dans la dernière recette locale, les trois protocoles longs passent en 169,2 s,
75,5 s et 154,2 s. Un test sensible au démarrage a échoué à attendre son
sous-processus ; le contrôle ciblé suivant passe en 1,95 s sans changement de
son assertion. La fin de cette recette locale a été arrêtée lorsque la suite
CI exhaustive du même code a terminé avec succès. Ce run local n'est donc pas
présenté comme une suite verte. Aucun délai ou quota du moteur n'a été augmenté.

Une ancienne fixture DOM optionnelle échouait aussi sur les fichiers exacts de
la base : elle est adaptée aux scopes et boutons actuels, sans supprimer les
assertions. La recette passe et exécute deux tests Go ; le DOM simulé ne constitue
pas une vérification dans un navigateur.

## Revue et limites

Auto-revue dans la session de l'auteur ; aucune revue indépendante ou acceptation
par le moteur n'est revendiquée. Les sauvegardes, inventaires et logs locaux sont
conservés dans le répertoire temporaire de recette. Restaurer uniquement les
fichiers de ce périmètre, sans écraser les travaux apparus depuis ni modifier
les bases `.swarm`.

Le navigateur vérifie l'installation avec un fournisseur HTTP simulé local.
Aucun fournisseur IA externe, départ d'agent, paiement ou livraison réelle n'est
revendiqué. Le serveur utilisateur et le site pizza restent distincts de la
seconde installation de recette ; leurs données sont conservées.
