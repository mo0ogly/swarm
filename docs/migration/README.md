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

Les contrats actuels sont décrits dans [PREPARATION-UX.md](../../PREPARATION-UX.md)
et la [référence technique](../../REFERENCE.md). Les preuves opérationnelles et
les données des missions du projet source ne sont pas distribuées.

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

## Utilisation depuis un projet existant

Construire Swarm ici, puis lancer `bin/swarm --root /chemin/du/projet web`.
Choisir un port libre si un cockpit existe déjà pour ce projet ; ne pas démarrer
un deuxième conducteur uniquement pour changer de dépôt source. La bascule du
serveur courant est une opération distincte, après vérification des agents actifs.
