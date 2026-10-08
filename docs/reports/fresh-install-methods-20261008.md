# Recette d’installation neuve et méthodes embarquées

## Défaut reproduit et correction

Une installation native neuve du commit distant `7cbec49` dans un projet vide
rendait les quatre méthodes de préparation indisponibles. APEX affichait
« Méthode hors périmètre : .claude/skills/apex/SKILL.md ». Le catalogue et le
contexte IA cherchaient les fichiers dans le projet piloté, alors que les sources
étaient déjà présentes dans le dépôt de Swarm.

Le lecteur commun utilise désormais les ressources embarquées lorsqu'un fichier
local est absent. Une personnalisation locale reste prioritaire. Un lien
symbolique, un fichier non régulier, binaire, vide, trop grand ou inaccessible
est refusé explicitement ; aucun repli ne cache une personnalisation dangereuse.
La méthode et le contrat complets sont transmis au fournisseur de préparation.

Le lanceur conserve également le binaire indiqué dans `configure` : `start` et
`restart` sans argument reprennent bien le binaire installé. La vérification de
disponibilité utilise la session privée du projet. Les profils de consignes du
projet restent distincts des méthodes livrées avec Swarm.

## Vérification du candidat

| Contrôle | Résultat | Observation et limite |
| --- | --- | --- |
| Test absent/présent, override et fraîcheur | PASS | Régression reproduite avant correction ; les quatre méthodes transmettent leurs sources canoniques |
| Fichiers dangereux | PASS | Liens au niveau fichier et parent, lien pendant, dossier, FIFO, taille et contenu invalides refusés |
| Cycle du lanceur natif | PASS | Huit tests, dont binaire explicite conservé, redémarrage et processus étranger |
| Installation native et Compose | PASS | Installation, HTTP authentifié, quatre méthodes, recréation, persistance, propriété des fichiers et intégrité SQLite |
| Navigateur sur projet vide | PASS | Besoin enregistré, méthodes sélectionnables au clavier, échange puis adoption explicite du brief |
| Contexte réellement envoyé | PASS | Sources APEX et contrat complets retrouvés dans la requête reçue par le serveur simulé local |
| Clé après redémarrage | PASS | Nouveau test réussi sans ressaisie ; clé non affichée et absente du diagnostic copié |
| Cadre vert avec réponse JSON | PASS | Défaut reproduit ; JSON décodé, vrais retours à la ligne ; texte ordinaire préservé |
| Langues, thèmes, clavier et copie | PASS | Français clair et anglais sombre, focus au clavier, copie égale au diagnostic, aucune erreur JS observée |
| Suite Go exhaustive | PASS CI | 1 071 cas découverts : 1 059 passés et 12 skips optionnels explicites ; 19 groupes, tous sortis avec le code 0 |

La recette navigateur utilise un **fournisseur simulé HTTP local**, nommé
« Local installation test double ». Aucun appel IA externe ni départ d'agent
n'est réalisé. Elle vérifie le transport, les fichiers embarqués, la persistance
et l'adoption ; elle ne démontre pas la qualité d'un modèle, l'authentification
d'un fournisseur réel ou l'exécution autonome d'une application.

Après publication, le second clone propre a récupéré `45b18eb` depuis GitHub.
Installation native et recette complète native/Compose : code de sortie 0.
Le besoin, l'échange, le brief adopté et la clé restent disponibles après
réinstallation. Un changement d'adresse refuse de transmettre la clé conservée ;
un test sans clé sur un port fermé affiche une erreur TCP, puis le test réussit
avec la destination d'origine. Aucun de ces essais ne modifie la connexion
stockée.

La [CI du code `45b18eb`](https://github.com/mo0ogly/swarm/actions/runs/37836580717)
est verte : contrôles moteur, interface, lanceur et installation.
La [PR #10](https://github.com/mo0ogly/swarm/pull/10) est fusionnée dans `main`
au commit `c1f2b92`. Le second clone a ensuite récupéré **main**, reconstruit et
installé ce binaire (`modified: false`), puis relancé avec `swarm.sh` sans
répéter les paramètres du projet ou du binaire. Une nouvelle racine temporaire
vide expose les quatre méthodes, sans créer `.claude`.

Les scripts de navigateur de CI vérifient les méthodes des agents et l'i18n ;
la recette manuelle décrite ici utilise le navigateur intégré. Les recettes
optionnelles ignorées par Go ne sont pas présentées comme exécutées : voir le
[rapport de réorganisation](repository-organization-20261008.md).

## Captures de la recette

Projet vide : méthodes de préparation disponibles sans dossier `.claude` local.

![APEX dans le projet vide](../screenshots/fresh-install-20261008/fr-apex-empty-project.jpg)

Test simulé local : réponse lisible et clé conservée, en français.

![Connexion avec vrais retours à la ligne](../screenshots/fresh-install-20261008/fr-connection-test-double.jpg)

Même contrôle en anglais et thème sombre, avec diagnostic copié.

![Diagnostic anglais sombre](../screenshots/fresh-install-20261008/en-connection-test-double-dark.jpg)

Brief relu puis adopté explicitement ; aucun agent lancé.

![Brief adopté](../screenshots/fresh-install-20261008/fr-apex-brief-adopted.jpg)

## Répéter l'installation

Suivre [la recette française](../FRESH-INSTALL.md) ou
[the English checklist](../en/FRESH-INSTALL.md). Les méthodes canoniques sont
versionnées dans [`.claude/skills`](../../.claude/skills), les commandes dans
[`.claude/commands`](../../.claude/commands). Les documents privés sur site,
certificats, overrides, clés et bases locales restent exclus du dépôt.

Auto-revue dans la session de l'auteur : ce rapport n'est pas une revue
indépendante ni une acceptation enregistrée par le moteur.
