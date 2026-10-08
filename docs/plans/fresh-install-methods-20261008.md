# Installation neuve et méthodes de préparation

Le clone GitHub `main` à `7cbec49` a été installé en mode natif dans un second
dossier, avec un projet vide séparé. `prepare methods` retourne quatre méthodes
indisponibles ; APEX annonce « Méthode hors périmètre ». Le chargement dans
`prephase_methods.go` et `prephase_dialogue.go` exige des fichiers dans le projet.
Les méthodes des agents sont pourtant déjà intégrées au binaire.

Copier les méthodes dans chaque projet à l'installation imposerait des écritures
et une politique de remplacement lors des mises à jour. Le choix retenu est une
lecture des méthodes embarquées pour chaque fichier absent. Un fichier local
existant reste prioritaire ; un lien, un accès refusé ou un fichier invalide
reste une erreur explicite. La préparation conserve ses limites et ses empreintes.

| ID | Résultat attendu | Vérification |
| --- | --- | --- |
| INS-01 | Les quatre méthodes fonctionnent dans un projet vide | CLI, API, contexte complet, aucun fichier de méthode créé |
| INS-02 | Les personnalisations restent détectées | Modification, retrait et contexte périmé |
| INS-03 | Aucun accès hors projet ou remplacement silencieux d'un fichier invalide | Liens feuille/parent, FIFO, dossier, fichier trop long |
| INS-04 | Installation reproductible depuis GitHub | Installation native, Compose, redémarrage et persistance |
| INS-05 | Guide public et corrections publiés | Documentation FR/EN, commit, push et recette du candidat distant |

La recette complète utilise des projets jetables. Aucun fournisseur IA réel ni
donnée de mission existante n'est nécessaire. La suite Go est rééquilibrée autour
des trois tests longs qui ont dépassé les groupes de 240 s ; aucun cas ne doit
disparaître de l'inventaire. Aucun plafond d'exécution du moteur n'est modifié.
