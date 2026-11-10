# Correction globale de transmission des preuves — 10 octobre 2026

Statut : correctif d'architecture et essais de protocole après observation.
Cette note prolonge le [cas des deux refus](20261010-refus-contexte-revue.md).
Elle ne modifie aucune campagne historique, aucun verdict ni aucune donnée de
facturation. La correction manuelle de T1 et cette évolution globale restent
identifiées séparément.

## Choix et portée

Une consigne seule aurait encore laissé chaque IA découvrir puis ajouter un
manifeste. La correction retenue traite la déclaration déjà structurée : les
fichiers `delivery.criteria[].evidence` sont joints automatiquement, depuis le
candidat Git contrôlé, avec octets complets, blob et SHA256. Le manifeste existant
ajoute les pièces complémentaires. Le rapport déjà fourni n'est pas dupliqué.
Aucun identifiant de mission, de tâche ou de fournisseur n'oriente ce comportement.

Les responsables racine et enfants reçoivent le contrat commun de transmission
et un inventaire vérifié des sources reçues par la revue courante. La copie de
reprise reçoit le même inventaire. Une source citée uniquement en prose peut
encore manquer : le moteur n'infère pas une liste de fichiers depuis le discours.
Une source déclarée mais illisible bloque avant un appel de revue payant.

Le [contrat d'architecture FR](../../../../architecture/REVIEW-EVIDENCE.md) et sa
[version EN](../../../../en/REVIEW-EVIDENCE.md) précisent les composants, limites,
compatibilité et flux. Trois schémas Mermaid expliquent le défaut, la transmission
corrigée et la décision de reprise. Le contrat partagé, replan et le modèle de
rapport documentent le comportement pour tous les rôles concernés.

## Vérification effectuée

| Contrôle | Résultat et portée |
| --- | --- |
| Régression avant correction : `go test ./internal/engine -run '^TestManagedDeliveryEvidenceJoinsUnchangedOriginalWithoutManifest$' -count=1 -timeout 45s` | FAIL attendu : `unchanged declared evidence not supplied completely: []` |
| Régression et flux associés : `go test ./internal/engine -run 'TestManagedDeliveryEvidenceJoinsUnchangedOriginalWithoutManifest|TestManagedDelivery|TestManagedReviewSources|TestManagedRecoveryHandoff|TestPlanningDelivery|TestPlanningContext' -count=1 -timeout 90s` | PASS, 10,203 s |
| Cas globaux : `go test ./internal/engine -run 'TestManagedDeliveryEvidence|TestReviewEvidenceInventory|TestManagedRecoveryHandoff|TestAgentWorkflow' -count=1 -timeout 90s` | PASS, 6,919 s |
| Parcours associés, revues cumulatives et méthodes : `go test ./internal/engine -run 'Workflow|ManagedDeliveryEvidence|ReviewEvidenceInventory|ManagedCompleteDelivery|ManagedReviewSources|ManagedRecoveryHandoff|PlanningDelivery|PlanningContext|ManagedReviewPacket|ManagedReviewBatch|IncrementalReview|AddedReviewSource' -count=1 -timeout 120s` | PASS, 18,365 s |
| Concurrence : `go test -race ./internal/engine -run 'TestManagedDeliveryEvidence|TestReviewEvidenceInventory|TestManagedReviewSources|TestManagedRecoveryHandoff|TestManagedCompleteDelivery|TestPlanningDelivery|TestAgentWorkflow' -count=1 -timeout 120s` | PASS, 26,128 s |
| `go vet ./...` | PASS |
| `python3 tools/agent-workflows/check.py` | PASS, 12 méthodes communes |
| `git diff --check` | PASS |
| `go test ./... -timeout 5m` | NON QUALIFIÉ : manifeste D03 historique périmé sur `PREPARATION-UX.md`, puis délai global de 5 min atteint dans `TestManagedFragmentHistoricalCapacityPreservesKnownInputs` ; limites déjà observées avant ce correctif |

Les essais de protocole utilisent de vrais processus enfants pour le fournisseur
simulé. Ils vérifient octets reçus, résistance à une modification de la copie,
déduplication, attribution par tâche, refus indépendant maintenu, absence d'appel
payant devant une preuve trop grande ou non textuelle, remise au responsable
racine/enfant et refus d'attester un contexte altéré. Une erreur de décodage du
banc (réutilisation d'une map JSON) et une assertion trop précise sur le diagnostic
binaire ont été corrigées ; les essais correspondants ont ensuite passé.
Ces observations ne mesurent pas la qualité de décision d'un modèle réel.

## Installation et continuité des preuves

Le correctif a été construit par `sh build.sh /tmp/swarm-global-evidence-fixed`
depuis le commit `1169df70bec73d50fd23f640245b8ffd1317b9de` avec modifications.
Le binaire porte une date de construction `2026-10-10T18:04:16Z`.
Son SHA256 est `0be752032093ce5da2c017cb4a6bb4bf07e4f42d6f366f22156945f33c03fe83`.

La liste publique des agents a été vérifiée sur les trois missions servies par
l'instance : aucun agent actif. Le binaire a été remplacé atomiquement et le
serveur local redémarré. La commande publique `version --server` confirme des
identités CLI/serveur identiques. Le candidat testé reste distinct du résultat
figé de la mission Cursor : la correction commune devra être intégrée au candidat
par T5, avec revue indépendante du lot.

Après redémarrage, T1 reste `accepted`, revue
`review-bc4a136c4713357aed3c56b9` passée, deux tentatives consommées sur deux,
80 appels d'outil autorisés par tentative et 4/30 appels de revue. Les avis
antérieurs et leurs empreintes restent conservés. Installation ne signifie pas
acceptation du nouveau correctif par la mission gérée.

## Limites et suite de recherche

Joindre les sources réduit le manque établi dans ce cas ; cela ne prouve pas
qu'une IA comprend ses entrées ou décide toujours une correction suffisante.
L'inventaire est un reçu de transmission, pas un verdict. Les limites techniques
existantes ne sont pas relevées, les preuves ne sont pas tronquées et les budgets
ne sont pas remboursés.

Une nouvelle comparaison doit pré-enregistrer fournisseur, modèle, tâches et
critères : consigne seule contre transmission automatique, avec originaux exacts,
citations fausses, originaux omis et contraintes de capacité. Compter séparément
inconnus justifiés, faux avis favorables, diagnostic de manque, reprises et coût.
Le résultat réel de T1 après réparation manuelle n'est pas une mesure de cette
évolution globale.

## Vérification des schémas et transmission au plan géré

Les cinq blocs Mermaid FR/EN ont été parsés et rendus en SVG avec Mermaid
11.12.0 et Chrome dans un répertoire temporaire, sans dépendance ajoutée au projet.
Les cinq rendus contiennent des éléments graphiques et aucun diagnostic JavaScript
n'a été observé. Les SVG de vérification restent hors du dépôt.

Le [patch du correctif global](../../../../plans/engine-cursor-contract/recovery/review-evidence-20261010.patch)
et son [manifeste de reproduction](../../../../plans/engine-cursor-contract/recovery/review-evidence-20261010.json)
permettent à T5 de l'intégrer au candidat géré, après le correctif de vivacité déjà
transmis. Les adaptations aux lots intermédiaires doivent conserver leurs
changements et faire l'objet de nouveaux contrôles et d'une revue indépendante.
