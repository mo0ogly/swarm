# Protocole de reprise APEX, audit-PDCA et KS

## Démarrage

Depuis la racine **applicative** du projet (Wattson : `flaskProject/`), employer
`./tools/swarm-companion/swarm`. Les commandes de reprise ne démarrent aucun worker et ne publient
rien. Le cockpit peut lancer des agents sur commande explicite ; voir `COCKPIT.md`. Si l’espace manque, l’initialiser lorsque la persistance est demandée.
En mode plan, consulter seulement les travaux déjà présents.

1. `work list` : présenter les travaux et demander le choix. Un identifiant déjà
   fourni vaut sélection. Ne pas reprendre automatiquement le dernier travail.
2. `resume ID` : lire la fiche fraîche. En mode plan utiliser `work show ID`,
   qui ne régénère pas de fichier.
3. Vérifier contexte Git, tâches interrompues et preuves périmées avant l’action.
   Une tentative « recorded » ne prouve pas qu’un processus existe encore.
4. Si un processus de l’ancienne session est inconnu, examiner son état avec les
   outils disponibles ; ne pas relancer aveuglément le même effet externe.

## Pendant le travail

Créer un travail avec objectif, périmètre et critères réels. Créer des tâches
avec livrable, responsable et critères. Les dépendances désignent des tâches
existantes du même travail ; une tâche dépendante attend leur acceptation et
leurs preuves courantes. Chaque tâche porte un identifiant choisi, stable.

- Début d’exécution : `task update`, statut `running`.
- Transmission : `task update`, statut `submitted`, `outcome: completed` et
  prochaine action de validation. `checkpoint` conserve résultats, réserves et
  liens aux documents pertinents ; les sorties volumineuses restent en fichiers.
- Échec/interruption : `task update`, statut `blocked` ou `todo`, avec
  `outcome: failed` ou `interrupted`, motif et prochaine vérification.
- Fait significatif : `ooda` avec observation, orientation, décision,
  responsable, prochaine action et résultat observé s’il existe.
- Résultat de contrôle : `gate` conserve le document méthode 2 et son verdict.
  `scope_id` doit être l’identifiant de la tâche. Revoir la couverture des
  critères : le moteur ne prouve pas l’exhaustivité de l’inventaire fourni.
- Acceptation : passer `submitted` à `accepted` seulement après une gate
  `delivery` permise et des preuves courantes. Une gate `audit` complète peut
  contenir des findings : elle ne clôture pas une tâche comme livrable accepté.
- Fin de session : `checkpoint` avec situation exacte, prochaine action et
  références au Guide de terrain. Ne pas attendre cette étape pour enregistrer
  les changements : chaque commande mutante persiste immédiatement.

Relire la révision courante avant mutation. Une même opération réessayée garde
le même `event_id` et le même contenu. Une nouvelle opération reçoit un nouvel
identifiant. Un conflit exige une relecture ; ne pas remplacer mécaniquement
`expected_revision` pour contourner un changement concurrent.

Le moteur conserve les événements mais ne lit pas les conversations : un fait
non soumis ne peut pas apparaître magiquement dans la reprise. Les profils
métier et autorisations restent dans les workflows existants.

## Connexion aux fournisseurs

Claude : le hook SessionStart existant affiche la liste lorsque le compagnon
est installé et initialisé. Le hook n’attend pas de saisie terminal : c’est
l’agent qui présente le choix à l’utilisateur dans la conversation.

Codex : `AGENTS.md` demande la liste et la sélection au démarrage. C’est une
instruction de workflow, pas une injection automatique garantie par un service.
La commande explicite fonctionne sans hook spécifique au fournisseur.

Hors Wattson : copier le module ou son binaire et cette procédure ; ajouter la
consigne de démarrage aux instructions du projet. Aucun chemin Wattson n’est
requis par l’exécutable. Conserver une racine logique par projet et ne pas
partager une base SQLite via un système de fichiers réseau.
