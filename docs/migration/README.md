# Extraction autonome — 19 septembre 2026

## Provenance et conservation

Source : sous-dossier `flaskProject/tools/swarm-companion/` du dépôt parent, HEAD
`43e0e0867bca618fc5faf3947868c485be468518`. Le module de sécurité navigateur
`flaskProject/static/js/security.js` a été extrait avec son historique vers
`frontend/preparation-security.js`.

52 commits ont été exportés pour ces deux chemins et filtrés dans une copie
indépendante. Les identifiants sont réécrits par l’extraction ; `commit-map.txt`
conserve leur correspondance. L’historique a été fusionné sans force avec le
commit initial du dépôt GitHub, qui porte sa licence Apache 2.0.

Les 387 fichiers courants du composant ont ensuite été repris, y compris les
85 fichiers modifiés et 77 fichiers non suivis observés. Leur contenu initial
est décrit par `source-manifest.json` et conservé dans un commit distinct,
antérieur aux adaptations de portabilité. Aucun fichier original n’a été
supprimé ou déplacé. Les données de missions, clés et binaires ne sont pas copiés.

Les documents d’architecture Markdown et les sources Mermaid ont été repris
avec leur contenu historique. Ils peuvent décrire des états antérieurs ; les
garanties actuelles doivent être croisées avec PREPARATION-UX.md et les tests.
Les Word, captures et preuves opérationnelles restent dans le projet source :
ils n’ont pas été publiés automatiquement avec les données d’exploitation.

## Adaptations réalisées ici

- Build frontend : module de sécurité local, aucun accès au répertoire parent.
- Coordination des anciens plans : chemin de l’exécutable réellement lancé.
- Lanceur et démarrage documentés depuis la nouvelle racine.
- Exclusion des données `.swarm/`, secrets `.env`, dépendances et résultats locaux.
- Harnais npm verrouillé pour Puppeteer ; navigateur configurable par CHROME_BIN.
- Cibles `make build`, `test`, `frontend`, `smoke` et recette API sans clé réelle.

## Limites explicites

La préparation APEX/KS/PDCA recherche encore les méthodes et
`tools/agent-workflows/CONTRACT.md` dans le projet piloté. Ces ressources ne
sont pas copiées depuis une installation personnelle ni activées implicitement.
Sur un nouveau projet sans méthodes, leur indisponibilité reste affichée.

Le test différentiel `tests/parity.py` attend toujours l’oracle du projet
agent-workflows. Plusieurs anciennes recettes navigateur attendent encore la
structure de fixtures du monorepo ; elles ne sont pas incluses dans `make test`.
Les recettes avec Claude/Codex réels exigent une installation et une autorisation
explicites. Les tests déterministes et la recette API utilisent des fixtures.

Linux est le périmètre validé ; aucun binaire macOS/Windows ni publication de
release n’est revendiqué. Aucun transfert de la base de missions existante.
Le serveur déjà ouvert continue d’utiliser le binaire installé avant extraction.

## Utilisation depuis un projet existant

Construire Swarm ici, puis lancer `bin/swarm --root /chemin/du/projet web`.
Choisir un port libre si un cockpit existe déjà pour ce projet ; ne pas démarrer
un deuxième conducteur uniquement pour changer de dépôt source. La bascule du
serveur courant est une opération distincte, après vérification des agents actifs.

## Vérifications avant envoi

Gitleaks 8.30.1 : contrôle de l’historique extrait et des fichiers courants, sans
secret détecté. Ce résultat ne garantit pas l’absence de toute donnée privée.
Les licences des bibliothèques embarquées sont conservées.

### Résultats du 19 septembre 2026

- `make build test smoke` : PASS ; suite Go 38,179 s, trois suites Node et
  recette CLI de reprise/export/import réussies.
- `make frontend` puis `make build` : PASS depuis ce dépôt autonome.
- `npm run test:connections` : PASS avec serveur API simulé, sans clé réelle ;
  création, test, réponse structurée du sous-processus, désactivation et deux thèmes.
- Les essais sont exécutés sur la même machine Linux, avec cache Go disponible.
  Une installation sur une machine vierge et les campagnes fournisseurs réels
  restent à vérifier.
- Un lien de session local a été retiré d’un document d’architecture avant commit.
  Aucune occurrence de jeton `session-` suivi de 24 à 64 caractères hexadécimaux
  n’a été trouvée dans l’historique importé lors du contrôle complémentaire.
