# Vérification de la publication Casa Pizza

Vérification du 8 octobre 2026, réalisée dans la session du formateur.
Base du dépôt Swarm : `7cbec49`. Périmètre : ce dossier de formation et les
liens ajoutés aux deux README principaux. Aucun changement du moteur Swarm.

| Contrôle | Exécution ou inspection | Résultat |
| --- | --- | --- |
| Guides français et anglais | Rendu avec le compilateur LibreOffice fourni par l'environnement, inspection des 36 pages de chaque version | PASS |
| Application contenue dans le kit français | ZIP extrait dans un dossier temporaire, `python3 -W error::ResourceWarning -m unittest -v` | PASS, 11 tests, 4,691 s |
| Application contenue dans le kit anglais | Même commande dans un autre dossier temporaire | PASS, 11 tests, 4,447 s |
| Création du projet et plan optionnel | Deux scripts exécutés dans un projet temporaire via le binaire Swarm installé | PASS, Git et besoin créés, aucun agent lancé |
| Confidentialité du paquet | Liste explicite, intégrité ZIP, inspection des textes et XML Word | PASS, aucune clé ou configuration interne publiée |
| Liens de documentation | Résolution des liens locaux des trois README | PASS |
| Contrat des méthodes | `python3 tools/agent-workflows/check.py` | PASS |
| Propreté du diff | `git diff --check` | PASS |

La recette navigateur de la référence courante est décrite dans
[REFONTE.md](projet-pizza/docs/REFONTE.md). L'ancienne recette et la remise de
l'exécutant restent conservées et sont explicitement historiques.

Cette vérification n'est pas une revue indépendante. Les formulaires de
validation automatique du guide ont été prévisualisés puis annulés. Le plan
à six tâches n'a pas été exécuté. Aucun résultat autonome, paiement réel,
livraison réelle ou déploiement du site n'est revendiqué.
