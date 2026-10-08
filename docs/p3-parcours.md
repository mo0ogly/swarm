# P3 — Du besoin au lancement sans expertise

## Verdict de cette tentative

**Partiel — implémentation et contrôles ciblés validés ; recette navigateur corrigée mais non exécutable jusqu’au bout dans ce bac à sable.**

Le parcours, son contrat serveur et ses régressions ciblées sont implémentés. Le contrôle navigateur reste requis pour un succès global : il exécute ici son test Go, puis le bac à sable refuse le lancement du binaire enfant avec `EPERM`. Cette restriction n’est pas convertie en succès.

## Cause racine de la reprise

Le reçu associé à `check-a41d0743474ba5118f6eeae5` et à l’empreinte `629a15f3df75c035bfcb5708b1d4b10d6f26be55846a6c91bba67472f2324564` identifie la commande `node tests/audit_acceptance.cjs --case journey` (code 1).

Le rejeu sur l’hôte avait atteint Chrome puis expiré sur `#create-form`. La recette ouvrait directement `/prepare.html` à partir de l’URL `/session/…`, sans visiter cette dernière : le cookie de session n’était donc jamais établi. Il s’agissait d’un défaut réel du test, distinct des restrictions locales de sockets ou de sous-processus.

Correction : `tests/journey_ui.cjs` visite d’abord l’URL de session et exige une réponse HTTP réussie, puis ouvre `/prepare.html` et exige également une réponse réussie. Le test ne peut plus attendre le formulaire dans une page non authentifiée.

## Changements livrés

- Le plan vérifié mène à **Relire l’équipe proposée**, puis à une autorisation unique récapitulative.
- Le récapitulatif affiche le responsable, les exécutants, le vérificateur réellement configuré, les fournisseurs/modèles IA et le mode d’acceptation humain ou moteur.
- Un prévol explicite vérifie l’exécutable, son contexte déclaré, le dossier, l’écriture temporaire et les ressources. Modifier fournisseur, niveau ou dossier invalide le prévol et désactive l’autorisation.
- La commande `authorize-plan` crée atomiquement l’organisation, les missions et leur autorisation. Elle n’appelle pas le dispatch et ne crée ni agent ni réservation pendant la transaction.
- Les anciennes commandes `create-missions` puis `release-plan` restent acceptées pour compatibilité et pour les travaux déjà matérialisés.
- La révision d’un plan existant conserve son flux : comparaison, application atomique, pause et nouvelle autorisation. Les documents et historiques ne sont ni remplacés ni supprimés.
- Les libellés, aides et documentation FR/EN sont alignés. Les styles utilisent les jetons de thème existants.
- `tests/journey_ui.cjs` décrit une recette isolée du besoin saisi dans le navigateur jusqu’au prévol et à l’unique autorisation. Elle initialise par la CLI puis utilise le navigateur et les API, sans ouvrir ni modifier directement `.swarm/state.db`.

## Correction prioritaire P1 issue de la revue externe

Provenance : revue externe P1 transmise avec la tâche P3.

`tests/audit_acceptance.cjs --case states` quittait auparavant avec le code 0 après les seules assertions DOM et invitait seulement à lancer `TestStateContract`. Le runner exécute maintenant cumulativement les assertions DOM, `go test -json -run ^TestStateContract`, puis exige au moins un événement Go `run` et un succès de paquet.

`tests/audit_acceptance_runner_test.cjs` couvre Go absent ou refusé par `EPERM`, Go en échec, sortie réussie sans test exécuté, assertion DOM en échec et cas inconnu. Tous produisent un code non nul. Une restriction de sandbox reste donc un échec explicite.

## Tableau exigence / preuve / verdict / limite

| Exigence | Preuve obtenue | Verdict | Limite |
|---|---|---|---|
| Parcours simplifié vers une équipe proposée | Bouton « Relire l’équipe proposée », récapitulatif et documentation FR/EN | Validé par code, syntaxe et i18n | Navigation réelle non achevée ici |
| Autorisation unique | `TestPreparationAuthorizePlanIsOneAtomicExplicitCommand`; événement `preparation.authorize-plan` | Validé côté moteur | Geste navigateur à rejouer sur l’hôte |
| Aucun démarrage silencieux | Même test : 0 agent et 0 réservation ; prévol sans création de travail | Validé côté moteur | Départ navigateur non observé ici |
| Responsable, exécutants, revue/contrôle et IA visibles | Récapitulatif DOM, routes `planning`/`work`, mode humain ou moteur | Implémenté | Rendu visuel non capturé dans ce bac |
| Prévol exploitable | `TestPreparationPreflightRunsBeforeWorkCreation` : `ready/verified`, 0 travail, 0 agent ; fournisseur absent => `intervention` | Validé côté moteur | Le lancement refait un contrôle frais |
| Révision et historique préservés | Chemins `revise-missions` conservés ; suite `TestPreparation*` verte ; assertions navigateur présentes | Validé par régression Go | Assertion navigateur non exécutée ici |
| Recette navigateur isolée sans accès DB direct | `tests/journey_ui.cjs`, cas `journey`, bootstrap de session explicite | Corrigée, non prouvée de bout en bout ici | Lancement enfant refusé par `EPERM` |
| P1 Go/JSON + DOM cumulatif | Cas `states` : DOM PASS, deux tests Go `run/pass`, paquet PASS | Validé | Aucune |
| Runner fermé en cas d’erreur | `tests/audit_acceptance_runner_test.cjs` | Validé | Doubles contrôlés, complétés par le cas réel `states` |
| FR/EN et thèmes | Catalogue généré et `npm test` verts ; CSS sur variables de thème | Validé pour contrat/code | Captures clair/sombre non produites ici |

## Commandes réellement exécutées dans cette reprise

- Consultation du reçu via une copie en lecture de `state.db` et la CLI Swarm — **cause identifiée** : cas `journey`, puis diagnostic hôte `#create-form` sans cookie de session. La base originale n’a pas été modifiée.
- `npm ci` — **PASS**, 98 paquets installés.
- `node --check tests/journey_ui.cjs`, `node --check tests/audit_acceptance.cjs`, `node --check tests/audit_acceptance_runner_test.cjs` — **PASS**.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p3-retry-go-cache node tests/audit_acceptance.cjs --case journey` — **ÉCHEC explicite local** après réussite de `TestPreparationAuthorizePlanIsOneAtomicExplicitCommand` : lancement du binaire enfant refusé avec `spawnSync … EPERM`. Ce résultat ne reproduit pas le précédent timeout navigateur et ne valide pas le parcours.
- `npm test` — **PASS** : graphes, organisation/revue, cockpit, tests négatifs du runner et i18n.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p3-retry-go-cache go test -count=1 -run '^(TestPreparation|TestPreflight)' .` — **PASS** en 3,959 s.
- `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-p3-retry-go-cache node tests/audit_acceptance.cjs --case states` — **PASS** : DOM puis deux tests Go/JSON réellement exécutés.
- `git diff --check` — **PASS** avant le contrôle final.

## Mesures et RETEX

- Écritures directes dans la base par la recette : **0** ; initialisation CLI, gestes navigateur et lectures API uniquement.
- Autorisations attendues : exactement **1** requête `authorize-plan` ; assertion présente, non exécutée jusqu’au bout ici.
- Démarrages pendant prévol/autorisation : **0** prouvé par les tests transactionnels Go.
- Interventions humaines de la recette : **0 prévues** après lancement ; non mesurées en exécution complète locale.
- Coût fournisseur : **0** dans les tests ciblés ; fournisseur déterministe de recette, aucun essai IA réel.
- Le défaut de session corrigé était une régression du test. Le refus `EPERM` observé après correction est une contrainte de ce bac à sable.

## Risques et limites persistantes

- Le contrôle hôte doit rejouer `npm run test:audit:journey`. Un succès exige la sentinelle `journey acceptance: PASS`, les captures clair/sombre et `test-results/audit-journey/result.json`.
- Le rendu réel clair/sombre/mobile du nouveau récapitulatif reste à observer sur un hôte autorisant Chrome, le loopback et les sous-processus.
- Aucun essai fournisseur IA réel, test utilisateur novice, conclusion commerciale ou autonomie IA n’est démontré par ce lot.
- Aucun push, déploiement, remplacement de serveur utilisateur ou modification directe de la base Swarm n’a été effectué.

## Prochaine action

Sur l’hôte de contrôle, exécuter `TMPDIR=/dev/shm GOTMPDIR=/dev/shm npm run test:audit:journey`. Si `#create-form` expire encore, conserver les réponses HTTP de bootstrap et l’URL finale ; sinon examiner les captures et le reçu JSON avant tout verdict P3 global.
