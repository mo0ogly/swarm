# T2 — Blocages expliqués : résultat courant du 3 octobre 2026, 17 h

Auteur de cette révision : superviseur Codex. Elle complète le travail des deux tentatives Claude ; aucune troisième tentative ni hausse de budget. Les anciennes rédactions, erreurs de capture et conclusions sont conservées intégralement dans `docs/plans/clear-launch-recovery/history/T2-*-before-*.md`, ainsi que dans l'historique moteur des revues 12 et 13. Ce document décrit exclusivement la recette actuelle, sans attribuer les interventions du superviseur au worker.

## Résultat observé par critère

| Critère inchangé | Observation réelle actuelle | Résultat de recette |
| --- | --- | --- |
| Cause, acteur et action directe réellement autorisée pour chaque cas testé | Organisation absente : cause explicite, acteur Vous/You, bouton Préparer l'organisation/Prepare organization ouvrant la modale explicative. Tentatives épuisées : cause 2/2, acteur Responsable de la mission, bouton Examiner les tentatives et les refus ; les actions lancer/relancer sont déclarées indisponibles. | PASS pour ces deux cas |
| Aucun contournement de limite proposé ou appliqué | Pas de démarrage supplémentaire, pas de modification du plafond 2/2, pas de remboursement de tentative, pas de levée d'un délai fournisseur. L'action d'examen ne démarre aucun agent. | PASS |
| Recette sur des cas de blocage réels, sans doublure remplaçant le comportement vérifié | Organisation réellement non configurée dans une racine créée via CLI public ; vrai serveur, vrai navigateur et vrai stockage. Plafond réellement consommé par les deux tentatives Claude de T2 dans la mission actuelle ; état observé en CLI et navigateur. Aucun objet de test n'est injecté dans la mission. | PASS pour ces deux cas |

Ces résultats de recette ne constituent pas une acceptation moteur : contrôles actuels, avis indépendant et gate restent requis. Le quota fournisseur réel n'a pas été provoqué ; il n'est pas revendiqué comme cas testé. La suite de tests de cooldown reste une preuve comportementale distincte. Les critères demandent une recette réelle des cas testés, pas de provoquer un dépassement de quota externe.

## Cas A — Organisation absente, CLI et navigateur réels

Racine isolée conservée `/tmp/swarm-t2-qw2-v2-1046661`, travail `w-9812b5ebba819a4e3828d127`, révision 1. Le worker avait créé ce travail via CLI public ; aucun responsable n'y est configuré.

Commande effectivement exécutée : `bin/swarm --root /tmp/swarm-t2-qw2-v2-1046661 --json mission status w-9812b5ebba819a4e3828d127` (sortie 0). Relevé conservé : `docs/plans/clear-launch-recovery/T2-organization-observation.json`.

Le même binaire courant sert cette racine sur le port 18794. Dans le vrai navigateur, le bandeau dit **Organisation autonome non configurée** et **Vous — Préparer une mission hiérarchique avec un responsable et des contrôles explicites**. Entrée sur **Préparer l’organisation** ouvre réellement la modale **Organisation de la mission**, qui explique le responsable absent et la marche à suivre. En anglais : **Autonomous organization not configured**, **You**, **Prepare organization**, **Mission organization**. Pas de création de mission ni d'appel IA par cette action d'explication.

Captures actuelles (pixels réels, inspectés, aucune composition) :
- `docs/screenshots/clear-launch-recovery/t2-organization-fr-light.png`
- `docs/screenshots/clear-launch-recovery/t2-organization-fr-dark.png`
- `docs/screenshots/clear-launch-recovery/t2-organization-en-light.png`
- `docs/screenshots/clear-launch-recovery/t2-organization-en-dark.png`

FR/EN et clair/sombre ont été rendus et inspectés. Entrée ouvre ; Échap ferme ; le focus revient au déclencheur avec `:focus-visible=true`. La modale est lisible, en-tête/pied séparés et détails sur surface secondaire. Les captures ne contiennent aucun avis de vérificateur ni verdict pré-écrit. Quelques libellés secondaires historiques (fil d'activité/coordination) ne sont pas intégralement traduits ; aucune affirmation de traduction générale.

## Cas B — Plafond réel de deux tentatives, mission en cours

Mission `w-115c11e8f802a4f98c3def32`, T2, révision 93. Tentative 1 interrompue à 25 appels d'outils ; tentative 2 terminée après 12. Le plafond est effectivement 2/2 : observation actuelle `docs/plans/clear-launch-recovery/T2-observation.json`, obtenue depuis `bin/swarm --json mission status w-115c11e8f802a4f98c3def32`.

Le bandeau réel du navigateur dit **Mission bloquée — tentatives épuisées** ; **1 tâche(s) ont épuisé leurs tentatives autorisées. Le conducteur ne peut pas les relancer** ; **Responsable de la mission — Examinez les tentatives et les refus pour décider de la suite**. L'action **Examiner les tentatives et les refus** ouvre le dossier de reprise, sans démarrage. Le détail de T2, ouvert par double-clic sur sa carte, affiche exactement **Lancer un agent : Plafond de tentatives du plan atteint** et **Relancer une tentative : Plafond de tentatives du plan atteint**. Une action d'autorisation supplémentaire existe mais n'a pas été utilisée. Les anciennes revues, rapports et tentatives restent enregistrés.

## Correction moteur et attribution

Le superviseur a reproduit puis corrigé un défaut dans `result_presentation.go` : une revue défavorable actuelle d'une mission ordinaire était ignorée, donnant un faux « rapport détecté, à soumettre ». `currentReviewPresentation` projette désormais l'avis réel, son motif et la reprise autorisée pour le même producteur et la même tentative. Une revue d'une tentative précédente n'est pas réattribuée ; des preuves modifiées deviennent périmées ; un avis favorable ne remplace pas une décision d'acceptation.

Tests ciblés réellement exécutés : `go test ./... -run 'TestMissionGuidance|TestPlanningFailureDoesNotHide|TestMissionStatusKeepsExistingAttempt|TestTaskUnderstanding|TestAttemptDiagnostic|TestAcceptedReviewRevalidation|TestOrdinaryRefusedReview|TestManagedReview.*Presentation|TestResultPresentation' -count=1 -timeout 120s` : sortie 0. Nouveau test `ordinary_review_presentation_test.go` : refus, attente de revue, erreur, preuve périmée et attribution de tentative. Les contrôles du moteur sont enregistrés séparément ; les assertions sur les JSON de recette vérifient le relevé conservé, elles ne rejouent pas le navigateur.

La suite complète a atteint son délai de test par défaut de 10 minutes dans un test d'intégration fragmentée préexistant. Elle est relancée avec `go test ./... -timeout 20m`, sans modification de limite des agents. Son résultat sera consigné dans le RETEX global ; ne pas la déclarer réussie avant sa fin.

## Preuves complémentaires lisibles — après revue 14

Les quatre modales précédentes masquent leur arrière-plan : elles prouvent l'action mais pas le libellé d'acteur dans le bandeau. Les vues **non floutées** suivantes complètent les preuves (aucun avis de vérificateur dans les pixels) :

- `docs/screenshots/clear-launch-recovery/t2-organization-banner-fr.png` et `t2-organization-banner-en.png` : cause, Vous/You, bouton Préparer l’organisation/Prepare organization.
- `docs/screenshots/clear-launch-recovery/t2-limit-banner-fr.png` et `t2-limit-banner-en.png` : Mission bloquée — tentatives épuisées, Responsable de la mission/Mission owner, bouton Examiner/Inspect, aucune reprise immédiate.

Les captures de la mission actuelle portent sa sélection et son lien permanent ; celles de la recette isolée portent « Recette QW2 v2 » et son lien propre. Leur seule lecture ne reconstitue pas l'exécution : la recette manuelle attribuée au superviseur et les relevés publics ci-dessous complètent les pixels. La recette CLI, la recette navigateur et les tests comportementaux sont trois preuves différentes. Aucune automatisation navigateur n'est revendiquée par le contrôle de cohérence des JSON.

### Extraits des sorties CLI réellement obtenues (sélection de champs, pas une simulation)

Cas organisation absente, commande publique indiquée plus haut :
```json
{
  "organization": {
    "ready": false,
    "label": "Organisation autonome non configurée",
    "issues": [
      "Responsable de mission absent : planification hiérarchique non configurée."
    ],
    "next": "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.",
    "verification": "Politique incomplète ; aucune vérification de mission démontrée."
  },
  "guidance": {
    "what": "Organisation autonome non configurée",
    "next": "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.",
    "actor": "Vous",
    "primary": {
      "kind": "organization",
      "label": "Préparer l’organisation",
      "effect": "Affiche les rôles et les conditions manquantes.",
      "tone": "attention"
    }
  },
  "understanding": {
    "what": "Organisation autonome non configurée",
    "next_step": "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.",
    "actor": "Vous",
    "actor_kind": "user",
    "situation": "decision_humaine"
  }
}
```
Cas plafond consommé, commande mission status sur la vraie mission :
```json
{
  "source": "swarm --json mission status, public CLI, revision 93",
  "revision": 93,
  "state": "intervention",
  "attempts_used": 2,
  "attempts_allowed": 2,
  "attempt_limit_reached": true,
  "mission_guidance": {
    "what": "1 tâche(s) ont épuisé leurs tentatives autorisées. Le conducteur ne peut pas les relancer.",
    "next": "Examinez les tentatives et les refus pour décider de la suite. Attendre ou actualiser ne lève pas cette limite.",
    "actor": "Responsable de la mission",
    "primary": {
      "kind": "task",
      "label": "Examiner les tentatives et les refus",
      "effect": "Ouvre les preuves et les actions autorisées ; aucune reprise immédiate.",
      "task": "plan-115c11e8f8-T2",
      "tone": "attention"
    }
  },
  "browser_launch_actions": [
    "Lancer un agent : Plafond de tentatives du plan atteint.",
    "Relancer une tentative : Plafond de tentatives du plan atteint."
  ]
}
```

## Origine et reproduction du cas A — contrôle produit direct

La racine isolée contient un vrai travail enregistré par les commandes publiques `swarm init` puis `swarm work create`, sans configuration de planificateur : c'est le cas normal d'une nouvelle mission non encore préparée. Elle n'injecte aucun objet Go, aucun faux fournisseur, aucune réponse HTTP, aucun texte de blocage ni valeur dans SQLite. La configuration absente est la précondition exercée ; le comportement observé est calculé par le vrai moteur. L'isolation évite d'altérer la mission active et ne remplace pas son comportement par une doublure. Ce cas de recette n'est pas revendiqué comme incident organique de production ni comme autonomie démontrée.

Un **contrôle enregistré exécute maintenant directement le vrai CLI**, plutôt que de lire uniquement le JSON écrit par le superviseur : `bin/swarm --root /tmp/swarm-t2-qw2-v2-1046661 --lang fr mission status w-9812b5ebba819a4e3828d127`. Sa sortie est conservée dans le reçu du moteur et transmise à la revue indépendante. Commande rejouée personnellement avant enregistrement, code 0. Les premières lignes produites sont :

```text
Mission
La mission en bref
Organisation autonome non configurée
Vous — Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.
Action principale : Préparer l’organisation
Effet : Affiche les rôles et les conditions manquantes.
```

Le cas B reste l'incident organique de la mission active avec deux tentatives Claude consommées. La recette ne se réduit donc pas à l'environnement isolé. Les captures et les relevés distinguent explicitement les deux environnements. Aucun contrôle sur JSON enregistré ne prétend rejouer une interaction navigateur.

Le contrôle utilise `python3 -c` et `subprocess.run` pour appeler exactement ce CLI, programme direct non reconnu par le catalogue de contrôles. Il imprime stdout/stderr, exige le code 0 et les libellés réels cause/acteur/action ; aucune réponse de substitution. Le wrapper complet figure dans le reçu.

## Relecture directe du cas B par le contrôleur

Le contrôle `real-cli-consumed-attempts` ré-exécute deux commandes publiques sur la vraie mission active : `bin/swarm --json planning show w-115c11e8f802a4f98c3def32`, puis `bin/swarm --json mission status w-115c11e8f802a4f98c3def32`. Il lit et imprime la révision courante, l'identité de tâche, le plafond et les deux tentatives réellement persistées (interrupted puis completed), et vérifie deux tentatives utilisées sur deux autorisées. Il ne lit ni T2-observation.json ni les anciennes captures pour cette assertion. Le contrôle est en lecture seule ; pendant la vérification, le statut submitted/review peut remplacer le bandeau de blocage antérieur, mais le nombre de tentatives et leur identité ne changent pas. La sortie directe sera transmise au vérificateur dans le reçu moteur, comme pour le cas A. Les deux cas disposent ainsi d'une relecture indépendante directe du produit courant, distincte du contrôle documentaire et de la recette navigateur manuelle.
