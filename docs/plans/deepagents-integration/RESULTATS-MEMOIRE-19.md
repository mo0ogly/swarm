# M23 — cachets intégrés à la page mémoire

17 octobre 2026. Codex seul, développement direct, `replan` puis livraison
bornée suivant APEX ; recette publique suivant `verify-fix`. Revue locale de
l'auteur, sans revue indépendante ni acceptation par une mission Swarm.

## Résultat et périmètre réel

Les sources de `project-memory.html` chargent le composant commun. Chaque fiche
se termine par son cachet de cire, son type réel et sa référence. Un badge près
de la sélection distingue version active, historique, retirée et indisponible.
Le passage procédure → expérience est vérifié par remplacement réel. Les types
et états nouveaux restent inconnus ; les cases ne deviennent pas sélectionnables.

L'API actuelle ne fournit pas les attestations M20. Propriétaire et auteur
restent donc non renseignés, identité et droits inconnus, sans succès vert.
`version.actor` reste du texte historique dans les sources. Aucun champ technique,
nom, rôle, hash ou version active ne devient une preuve d'identité ou de droit.

`frontend/embed-signals.mjs` copie seulement les deux assets canoniques vers
`web/signals.*`. `--check` contrôle leur égalité sans modifier les copies. Le
catalogue M22 reste hors application. Documentation et exemple de réutilisation :
[FR](../../SEMANTIC-SIGNALS.md), [EN](../../en/SEMANTIC-SIGNALS.md).

## Preuves et limites d'environnement

Préimages, branche, HEAD et état sale : `evidence/m23-20261017/baseline.json`,
`before/` et `status-before.txt`. Le code identité et les travaux préexistants
sont préservés. Le processus Go identité observé a terminé avant l'application
des assets ; l'instantané est dans `embed-generation.json`. Aucune interruption
de processus tiers ni aucun message envoyé à l'autre session.

La recette utilise l'API/CLI réelle du binaire M19 conservé, SHA-256
`026fcf377d64cd43951e5c7fc3a6d20ee5ea42c9323e3ffeaf65b1566895f7e2`,
et un bundle isolé produit par la fixture Go M3, jamais une base de mission
réelle. Les cinq fichiers frontend candidats sont servis par interception
bornée dans le navigateur ; leurs octets et empreintes sont conservés et
contrôlés après exécution. Les opérations mémoire réelles ne sont pas simulées.
Un contrôle de rendu distinct injecte les seuls états indisponible/inconnu et
un acteur malveillant. Il ne constitue pas une décision du moteur.

Le fournisseur est un sous-processus Python local recevant le contexte réel,
sans appel de modèle. Cette recette ne compile pas le moteur identité actuel.
La sonde Go de ressources vérifie séparément l'embarquement des cinq assets.
Le serveur utilisateur n'a pas été redémarré ; aucun déploiement ni validation
du moteur courant n'est déclaré.

## Matrice de vérification

| Critère | Résultat | Preuve |
| --- | --- | --- |
| M23-01 Cachet et types réels | PASS frontend/API M19 | Admission procédure, remplacement expérience et cachet en pied de chaque fiche. |
| M23-02 États distincts | PASS dans le périmètre testé | Active/historique/retrait par opérations réelles ; indisponible/type et état inconnus par projection synthétique déclarée. |
| M23-03 Aucune attestation inventée | PASS frontend | Droits/identité inconnus, attribution exacte non renseignée FR/EN, aucun badge de succès ni HTML issu de `actor`. |
| M23-04 Parcours conservé | PASS API/CLI/navigateur M19 | Aperçu, Échap/focus, sélection, contexte reçu, historique après retrait ; aperçu périmé refusé sans écriture ni perte de saisie. |
| M23-05 Langues et rendu | PASS | 20 configurations FR/EN × deux thèmes × 1440/1024/760/390/320 px ; contraste texte minimal 5,75:1 clair et 8,02:1 sombre. Captures inspectées en clair bureau et sombre anglais mobile. |
| M23-06 Génération et documentation | Vérification finale dans `audit-final.json` | `--check`, sonde d'embarquement, égalité des octets, docs FR/EN, liens, diff propre. |
| Moteur courant complet, serveur utilisateur, droits M20 | NOT TESTED dans ce lot | Pas de nouvelle compilation moteur ou de déploiement ; le chantier identité conserve la responsabilité des attestations. |

Onze groupes d'assertions passent dans `browser-final/checks.json`, avec
commandes/dates/codes dans `browser-final-run.json` et sortie originale dans
`browser-final.log`. Aucune exception de page ni requête échouée. Le seul
HTTP 409 et son message console sont le refus périmé intentionnel, observé après
vérification d'une console vide. Les entrées pertinentes sont inchangées.

## Contre-exemple, incidents et récupération

- `browser-red/` reproduit l'absence de cachet sur l'ancienne page : attendu 1,
  observé 0. Les hashes des assets consommés figurent dans son environnement ;
  leurs originaux restent dans `before/web/` et les sources canoniques M22.
- `browser-candidate/` vérifie le premier candidat ; la couverture est ensuite
  élargie aux deux types persistants et à l'état inconnu. `browser-verified/`
  puis `browser-final/` sont liés aux fichiers `web/` effectivement livrés ;
  `browser-final/` porte la source finale de la recette.
- Un patch documentaire groupé est refusé parce qu'une phrase anglaise attendue
  n'existait pas. Aucun fichier de ce patch n'est appliqué. Lecture de la fin
  réelle, correction du contexte puis application réussie ; une coquille locale
  `transitions,+contexte` est retirée avant contrôle final. Ce défaut de méthode
  ne modifie aucun code ni preuve de test.
- Le premier audit documentaire refuse cette même chaîne dans le RETEX, où elle
  décrit volontairement l'incident historique. Le garde est restreint aux pages
  opérationnelles concernées ; la preuve originale n'est pas effacée. La revue
  locale clarifie aussi un commentaire de recette : les actions utilisent l'API
  réelle, tandis que les variantes de lecture sont explicitement synthétiques.
  La recette est relancée sur cette source finale.

## RETEX et reprise

La séparation type/version/identité/droits évite d'assimiler disponibilité et
autorisation. La documentation commune transmet les mappings, les valeurs
inconnues et le chemin de génération à la session identité, sans prétendre
qu'elle a déjà adopté le composant. Les prochains consommateurs doivent relier
leurs états à des reçus vérifiés ; le cachet ne peut ni fabriquer ni remplacer
ces reçus. Le présent lot ne ferme pas M20/IA3.

La compilation partagée rend nécessaire une vérification bornée et traçable :
préparer hors ressources communes, attendre la fin observée du consommateur,
vérifier les octets livrés et distinguer frontend sur API conservée, embarquement
Go et binaire moteur réellement servi. Le build courant et son éventuel
redémarrage restent un incrément distinct à coordonner avec le chantier identité.
La reprise fonctionnelle suivante demeure M20.2 : persistance de la propriété,
auteur attesté et droits de lecture/résumé/transmission au niveau des producteurs
et consommateurs, selon `CONTRAT-USAGES-MEMOIRE.md`, après disponibilité des
opérations publiques d'autorité nécessaires, notamment avant adoption d'un plan.
