# Plan — plan-115c11e8f8-T4 (REQ-QW4, tentative de vérification)

Base SHA `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, arbre de travail **dirty** (nombreux fichiers modifiés hors scope T4, listés dans le handoff). Aucune écriture applicative par cette tentative : rôle = vérification seule du complément déjà livré par le superviseur.

Identité courante : mission `w-115c11e8f802a4f98c3def32` ; tâche `plan-115c11e8f8-T4` ; agent `3500cb7c-3823-48cf-a454-295a032b309e` ; tentative `a-9b4502ef490c0a9b2227676b` ; départ 2/2 ; révision travail 175 (mémoire de reprise moteur : 176).

Tentative précédente `auto-f6aa5085c5f63a1e7fad` / `a-4060d356d9f554c29087fad8` : **interrupted** après 25 outils (prototype moteur commencé : `recoveryPreview`, `printRecoveryPreview`, édition de `mission_insights.go`, pas de tests rejoués, build non confirmé propre). Le superviseur a ensuite terminé moteur/CLI/web/traductions/tests et produit `docs/plans/clear-launch-recovery/T4-supervisor-recipe.md`. Cette tentative ne reprend pas ce travail : elle vérifie le complément déjà annoncé terminé.

## Critères (depuis le contrat courant)

| ID | Critère | Statut avant vérification | Statut après |
| --- | --- | --- | --- |
| req-13 | Vue affiche changements depuis le refus, exigences restantes, distingue preuves réutilisables vs périmées | Non vérifié par cette tentative | **PASS** (tests Go + JS, voir handoff) |
| req-14 | Aucun réinventaire complet déclenché, aucune hausse de budget automatique | Non vérifié par cette tentative | **PASS** (bornes + lecture seule testées) |
| req-15 | Parcours réel refus → correction bornée → reprise vérifié en web et CLI | Non vérifié par cette tentative | **PARTIAL** — attesté uniquement par le document du superviseur ; non ré-exécuté par cette vérification (hors périmètre autorisé : captures navigateur réservées au superviseur) |

## Commandes exécutées (résultats réels)

1. `go test ./... -run "TestRecoveryEvidence|TestRecoveryPreview|TestManagedPublicationReservesWriterBeforeEvidenceChecks|TestValidationPolicy" -count=1 -timeout 120s -v` → exit 0, `ok swarm.local/companion 1.170s`, 10 tests nommés PASS.
2. `node tests/recovery_evidence_test.cjs` → `PASS FR/EN recovery delta, changed/unknown inputs and remaining criteria without approval`, exit 0.
3. `git diff --check` → exit 0, aucune erreur d'espace/conflit.

Détails complets, limites et attribution : voir `docs/plans/clear-launch-recovery/T4-handoff.md`.

## Prochaine action

Aucune autre tâche engagée. Remettre au responsable pour revue indépendante sur le même SHA candidat (non committé) avant toute déclaration de livraison. Pas d'acceptation, pas de modification de base Swarm par cette tentative.


## Complément de recette attribué à Codex — 3 octobre, 17:32 UTC

La conclusion PARTIAL / NOT INDEPENDENTLY VERIFIED ci-dessus décrit la recette que le producteur n’a pas menée lui-même. Elle reste conservée. Codex, superviseur externe, a depuis exécuté la recette sur le vrai navigateur avec CUA, sans se l’attribuer au producteur. Le reçu moteur 12ff95e867b8595d78589025 a capturé 3647 octets de cette interaction pendant un contrôle exécuté à code 0, sans troncature.

Une séquence complète supplémentaire vient d’être observée dans la même mission : lecture du refus courant à 17:29:55 UTC (écran r228, événement de refus r227), correction bornée du dossier et de la consigne à 17:31:36 (écran r230, task.update r230), puis reprise de la vérification par le bouton de l’interface à 17:32:07 (écran r231, review.retry r231). La carte passe de Bloquée à À vérifier. Les trois captures et le relevé DOM sont joints au contrôle CLI, qui confronte ces étapes aux événements durables. La même tentative a-9b4502ef490c0a9b2227676b et le même producteur terminé sont utilisés ; deux tentatives consommées, leur plafond et la limite d’outils restent identiques. Aucune acceptation ne résulte de cette recette : les contrôles et la revue courants restent à terminer.

Preuves : T4-continuous-sequence.json ; t4-sequence-1-refusal.png ; t4-sequence-2-correction.png ; t4-sequence-3-resume.png. Cette recette est supervisée et ne prouve pas l’autonomie générale.
