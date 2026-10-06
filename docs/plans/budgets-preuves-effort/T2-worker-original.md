# T2 — QW5 : bouton « Régler cette limite » — suivi

Mission w-01567e073c1ed2f3d4c71c9e ; tâche plan-01567e073c-T2 ; agent auto-ed82e20e57f3200af263 ; tentative a-b45d8905fcf816f645fa0ec2 ; départ 1/2.
Racine de travail : /home/fpizzi/workspace/swarm-action-skills (HEAD au départ cc3069d).

## Critères (req-4, req-5, req-6) — état

- req-4 (bonne mission/bonne limite, consommé/plafond/restant, sans mélange outils/revues) : EN COURS
- req-5 (confirmation explicite avant hausse, historique conservé, reprise expliquée) : EN COURS — mécanisme déjà existant (previewBudget/configureBudget, revision_conflict) ; à vérifier bout en bout.
- req-6 (FR/EN, deux thèmes, clavier/focus, CLI racine isolée) : EN COURS

## Constat initial (inspection, pas encore une preuve de correction)

- Existant confirmé par lecture directe (pas seulement rapport T1) :
  - `budget_commands.go` : `previewBudget`/`configureBudget`/`budgetCLI` — mutation du budget USD (mission entière, une seule ligne `budgets` par `work_id`), contrôle de révision optimiste, `CommandError{Code:"revision_conflict", Retryable:true}`.
  - `budgets.go` : `writeBudget` journalise un événement `cockpit_events(kind="budget")` à chaque changement → historique conservé nativement.
  - `escalations.go` : `budgetEscalation` produit une carte `Kind:"budget"` **sans TaskID** (c'est le travail entier qui est concerné, jamais une tâche) quand `Remaining<=0` (épuisé) ou `Warning` (seuil 80%).
  - `web/conduite.js` `inboxCard(d)` : avant correction, les cartes `gate`/`handoff` avaient un bouton dédié ; les cartes `budget` (et `cout`) n'avaient que « Examiner et décider » (acquittement, n'ouvre pas le réglage réel).
  - `web/cockpit.js` : un réglage budget existe déjà (`$('budget-edit')`, onglet « Budget ») avec aperçu (`budget-preview`) puis confirmation (`budget`), montrant plafond actuel/proposé, engagé conservé, disponible après modification — mais atteignable seulement via l'onglet, pas depuis le blocage.
  - CLI déjà présente : `swarm budget show|preview|apply <travail> [--input budget.json]` (main.go) — même garde de révision, même historique.
  - Tests Go déjà présents (`budget_commands_test.go`) : chemin normal, conflit de révision (erreur), écriture concurrente (1 succès/1 conflit), remise à niveau après baisse (reprise). Non modifiés ici.
  - Test navigateur déjà présent `tests/budget_ui.cjs` : aperçu sans écriture, parité web/CLI, modale périmée rejetée, Échap restaure le focus, FR/EN, deux thèmes.

Donc le manquant réel pour QW5 n'est pas le réglage budget (il existe et est déjà testé), mais **le chemin direct depuis la carte de blocage** vers ce réglage, avec la bonne cible (le travail courant, jamais une tâche).

## Correction appliquée (bornée à ce manquant)

- `web/cockpit.js` : extraction de la fermeture anonyme `$('budget-edit').onclick` en fonction nommée `openBudgetEdit()`, réutilisable, sans changer son comportement (même modale, même aperçu/confirmation, même ciblage du travail courant via `openModal`'s `workID:work`).
- `web/conduite.js` `inboxCard(d)` : nouveau bouton « Régler cette limite » affiché uniquement pour `d.kind === 'budget'`, appelant `openBudgetEdit()`. Les cartes `gate`/`handoff` et le bouton générique « Examiner et décider » restent inchangés. `d.kind === 'cout'` (dépassement par tâche, levier différent) volontairement laissé hors périmètre : ce n'est pas le blocage budget visé par QW5, et le mélanger aurait élargi le périmètre au-delà de la tâche confiée.
- `web/i18n-en.js` : ajout de la traduction « Régler cette limite » → « Set this limit ».

Aucun fichier moteur Go modifié : le mécanisme de révision/historique/reprise existait déjà et répond aux critères req-5. Changement minimal, cohérent avec la règle « plus petit changement cohérent ».

## Vérifications prévues

| ID | Contrôle | Environnement | Résultat attendu |
| --- | --- | --- | --- |
| v1 | `npm run test:i18n` | racine dépôt | Traduction ajoutée reconnue, aucune clé orpheline |
| v2 | `go build ./...` puis `go vet ./...` | racine dépôt (aucun fichier Go modifié, contrôle de sécurité) | Build et vet propres |
| v3 | `swarm budget show/preview/apply` sur racine isolée (`--root` temporaire) | CLI réelle | Normal (aperçu sans écriture, application avec révision incrémentée), erreur (révision périmée → `revision_conflict`), reprise (relecture puis nouvelle application avec la révision à jour) |
| v4 | Test navigateur dédié (nouveau fichier `tests/qw5_resolve_blocking_limit_ui.cjs`) | Puppeteer headless, racine isolée | Carte de décision `budget` affiche « Régler cette limite » ; clic ouvre la même modale que l'onglet Budget, ciblée sur le travail courant, avec plafond/consommé/restant visibles avant confirmation ; FR/EN, thèmes clair/sombre, focus clavier (Échap restaure le focus) |

## Limites connues avant exécution des contrôles

- Déclencher une carte `budget` réellement par réservation moteur (lancement d'agent réel) demande un fournisseur IA configuré, absent de ce bac à sable isolé et hors périmètre réseau des tests existants (`quick_wins_ui.cjs` vérifie explicitement l'absence de requête hors boucle locale). Le test v4 injecte donc une décision `budget` synthétique dans `snapshot.decisions` côté page (technique déjà utilisée par `tests/pilotage_contracts_ui.cjs` et `tests/planning_review_retry_ui.cjs` pour des cartes `silence`/`gate` difficiles à provoquer en direct) puis appelle `renderConduite()`. C'est un contrôle DOM/état fourni par fixture, pas une preuve de réservation réelle par un fournisseur — distinction consignée ici pour ne pas la confondre avec une autonomie réelle démontrée.
- Aucun doublon de la revue indépendante : ces contrôles sont personnels au worker, pas une revue indépendante.

(Section mise à jour après exécution des contrôles ci-dessous.)
