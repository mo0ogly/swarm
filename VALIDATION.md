# Validation du pilote local — 11 septembre 2026

## Résultats observés

- Go 1.24.3, Linux amd64 ; exécutable statiquement lié, construit avec
  `CGO_ENABLED=0 go build -trimpath -o bin/swarm .`.
- `go test -race ./...` : 14 tests passent, dont interruption avant commit,
  écritures concurrentes, révision périmée, retour dupliqué, reprise persistante,
  transition sans preuve, abandon distinct de la réussite, dépendance rouverte, preuve modifiée, archives
  invalides et lecture seule de la fiche.
- `go vet ./...` : aucune erreur.
- `tests/parity.py` : 16 tests de référence, 155 évaluations différentielles
  Python/Go. Même résultat complet (empreintes incluses) et mêmes codes de sortie
  pour les cas testés : quatre phases, statut NA, preuves absentes/périmées,
  compte fractionnaire, entrées invalides et moyenne 94 bloquante.
- `tests/smoke.py` : nouvelle invocation de processus pour chaque commande,
  reprise après checkpoint, pas de double création, avertissement sur tâche
  active, lignes de 80 colonnes maximum, export/import vers un autre projet.
- Tests Python existants du noyau : 19 passent.
- Syntaxe des scripts shell et contrôle `git diff --check` ciblé : réussis.

## Portée et limites

Ces tests ne prouvent ni l’exhaustivité des critères renseignés ni la vérité
métier des preuves. Aucun benchmark de swarm autonome n’est revendiqué.
L’évaluateur de référence reste `tools/agent-workflows/evaluate.py`, associé au
commit d’intégration 972b90e03 ; le module Go est autonome à l’exécution.

Claude Code 2.1.237 et Codex CLI 0.154.0 sont présents. Le hook Claude et les
instructions Codex sont raccordés, mais aucune nouvelle session de modèle
Claude ou Codex n’a été exécutée pour cette validation. La reprise réelle dans
ces deux fournisseurs reste à exercer et ne doit pas être présentée comme PASS.
Les commandes de reprise dans des processus distincts sont, elles, vérifiées.

La première version est un compagnon local sans ordonnanceur, daemon,
synchronisation réseau ni exécution automatique de workers. Les modèles
continuent à choisir les actions au travers des workflows autorisés.
