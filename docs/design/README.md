# Maquettes de conception Swarm

Maquettes produites le 8 novembre 2026 avec le canvas Design de claude.ai.
Ce sont des **propositions visuelles** : elles ne sont pas branchées au moteur,
ni à l’API, ni à `web/`. Les données affichées sont des exemples ou des
emplacements entre crochets. Aucune inspection visuelle ni recette n’a été faite.

| Dossier | Canvas claude.ai (privé) | Planches |
| --- | --- | --- |
| `swarm/` | https://claude.ai/artifact/P4U3fe9gRnrWAx7Kh1177h | `Main.dc.html` cockpit de conduite, vue Graphe ; `Couloirs.dc.html` même cockpit, vue Couloirs (file « À vous d’agir », rôles × phases) ; `Preparation.dc.html` dialogue IA ; `Securite.dc.html` Configuration › Administration › Sécurité (Biba, Bell-LaPadula, conflits de rôles, identités, clés) ; `Audit.dc.html` Suivi › Audit et preuves |
| `admin/` | https://claude.ai/artifact/8QxgcH2oP1xv7JR9UH9cAM | `Main.dc.html` écran Administration actuel ; `Proposition.dc.html` modale repensée |

Sources lues pour les composer : `docs/UI-DESIGN.md` (palette, rail, navigation
01–05), `web/index.html` (vues `#admin`, `#brainstorm`), `web/admin.js`
(`runLimitsFieldList`). Les points de la proposition non vérifiés côté moteur :
valeur effective par limite avec son origine (Admin) et sections structurées du
brief (Préparation).

Les vues Graphe et Couloirs partagent un sélecteur « Graphe | Couloirs » ; dans
la maquette il relie les deux planches. Comportement visé dans le produit : une
seule vue Conduite, choix mémorisé localement sur l’appareil comme les
préférences `swarm-nav:*`, sans effet sur la mission (non implémenté).

Sécurité et audit : proposition de ranger les écrans aujourd’hui isolés
(`web/trust-native.html`, `web/security-proofs.html`, `web/trust-actions.html`,
`web/paea*.html`, non liés depuis le menu du cockpit) par activité : politiques
dans Administration › Sécurité, audit et vérification dans Suivi › Audit et
preuves. Champs et avertissements repris de `web/trust-native.html` et
`web/trust-native.js` ; sujets I1/I2, niveaux de classification, compartiments,
paires de rôles et transactions sont des exemples. Les gestes quotidiens
(demander, approuver, engager) restent à placer dans Conduite › Décisions.

Format : fichiers `.dc.html` du canvas Design (balises `<x-dc>`, `<sc-for>`,
`<sc-if>`, script `DCLogic`) et index `canvas.json`. Ils ne s’ouvrent pas
directement dans un navigateur ; le rendu se fait dans le canvas claude.ai.
Le logo référencé (`/_blob/…`) est une copie de `web/swarm-logo.png` stockée
dans chaque canvas. Le canvas claude.ai reste la version de travail ; ce dossier
en est une copie versionnée à la date du commit.
