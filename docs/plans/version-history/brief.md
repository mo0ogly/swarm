# Version et nouveautés de Swarm
Gabarit : Améliorer une application ; méthode Analyse et planification (APEX).
R1 : version explicite unique, commit du binaire, état modifié et date de build ; distinction version installée / sources locales. Aucun faux numéro de release historique.
R2 : swarm --version et swarm version ; JSON exploitable sans initialiser SQLite ; cohérence build local, install.sh et Docker. Pas de dépendance Git au lancement d’un binaire distribué.
R3 : bouton Version et nouveautés dans le cockpit et la préparation ; modale accessible FR/EN, thèmes sombre/État, fermeture Échap et retour focus.
R4 : historique embarqué de versions avec dates, résumés utiles, commits et liens GitHub ; données bornées, échappées, sans commande Git arbitraire ni requête réseau périodique. Historique indisponible explicitement signalé.
R5 : tests comportementaux CLI, API et navigateur ; docs utilisation/installation bilingues ; preuves liées au candidat exact ; aucun vert sans revue.
Comparer historique embarqué vs Git à la volée ; préférer un historique embarqué compatible avec Docker, conserver un lien GitHub pour l’historique complet. Identifier la convention de version avant de la fixer. Ne pas inventer des releases ou dates.
Équipe : responsable de planification, exécutants séquentiels dans cette copie de travail, vérificateur indépendant sans écriture. Les revues et acceptations ne sont jamais celles du producteur.
Exclusions : pas de refonte de l’UX, pas de suppression de missions ou fichiers utilisateur, pas de modification des modèles/budgets existants, aucune publication ou fusion automatique. Préserver output/ et tmp/ du dépôt d’origine.
Rapports courts avec critères PASS/FAIL/NOT TESTED, commandes et résultats, SHA et diff sale, récupération concrète. Ne pas parcourir tout le dépôt : lire seulement les fichiers du périmètre. Si blocage, diagnostiquer avant toute répétition.

## Parcours du gabarit
frame → research → plan → implement → review → deliver
