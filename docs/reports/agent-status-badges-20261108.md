# Badges de statut des agents — 8 novembre 2026

Demande : même code couleur que les tâches dans les deux thèmes. Codex seul,
replan/verify-fix ; aucune opération d’agent, fournisseur ou mission autorisée
par la présentation. Avant : le lecteur natif rendait tous les états sous forme
de notice info avec leur code brut ; dix tentatives conservées observées.

Changement : SwarmWorkbench.agentStatusBadge utilise le style partagé mais un
vocabulaire d’agent distinct : bleu queued/starting/running, vert completed,
rouge failed, ambre stopping/interrupted et états explicitement non confirmés,
gris inconnu. Texte uniquement, état conservé dans data-agent-state ; la mention
« État enregistré » et les observations d’autonomie restent présentes. Une
tentative terminée n’accepte pas sa tâche et ne constitue pas une preuve d’activité.
Aucune politique, limite, temporisation ou droit ajouté.

Régression conservée avant correction : missing agent status badge, avec sources
antérieures. Après : quatre contrôles agents nommés PASS, dont code inconnu sûr,
réponse invalide refusée avant remplacement et reprise vide ; cinq contrôles de
non-régression tâches PASS. Recette permanente agent-status-badges.json, PASS et
intégrité VERIFIED, sans watchdog. Double DOM, distinct des preuves navigateur.

Livraison normale v0.1.1-dev.358, SHA-256
`cbc3307978b1e3c67d23936909eca6ef145db31500061151f2243f13e6d33ac3`,
build puis restart --prepared-build. Profil sans feature de fixture, 782 entrées
effectives archivées ; 3 073 empreintes du manifeste avant build inchangées.
Processus réel relu, dialogue Version FR/EN vérifié avec notes courantes, commit
de base b4e07cb26066cffdb20b36ac2fc9c60f758cb23e et modifications non commitées.
Aucun commit/push réalisé ici, travaux préexistants préservés.

Navigateur CUA, application installée authentifiée : dix cartes réellement
conservées (interrupted/completed), FR/EN × clair/sombre, largeur 1265 px, pas de
débordement. Labels Interrompu/Terminé et Interrupted/Finished ; contraste minimal
6,31:1 clair et 8,21:1 sombre. Aucun avertissement/erreur dans la console capturée.
Le parcours actuel est une observation historique ; aucune tentative lancée.

Fixture navigateur isolée : dix métadonnées explicites couvrent attente, démarrage,
activité, arrêt, fin, échec, interruption, non-confirmation, autre poste et inconnu.
Les dix ressources produit sont extraites de l’archive effective du même build,
chaque hash comparé au record ; contrôleur HTML/JS séparé, versionné sous tests,
sans serveur moteur ni fournisseur. FR/EN × deux thèmes, dix badges par combinaison
d’affichage, contraste minimal 6,31:1 clair et 8,06:1 sombre, pas de débordement.
Cette fixture ne prouve pas l’activité réelle de ces variantes. Le serveur HTTP
statique possédé est arrêté après lecture, avec sortie normale ; la fixture reste
conservée. Deux CSS de la vraie page protégée sont aussi exportés par CUA et
comparés au checkout et à l’archive compilée.

Reprise de méthode : ancien onglet1 absent et inventaire vide ; nouvel onglet du
même cockpit créé, session existante disponible, aucun secret manipulé. Catalogue
FR/EN existant réutilisé. Nouveau helper de fixture documente inputs/outputs,
préconditions, limites et refus ; les erreurs du badge remontent au chargeur
existant qui affiche son refus et vide la projection concernée.

Docs UI-DESIGN.md, en/WINDOW-IDENTITY.md et version_history.json actualisées dans
le même incrément. Pas de matrice de toutes les largeurs ni de capture exhaustive
des requêtes CUA ; aucune acceptation indépendante. Preuves locales :
.cache/agent-status-badges-20261108, LOCATION.json pour archive Lexar et restauration.


Diagnostic de la mention « Lecture partielle » conservée : le statut complet de
la mission historique refuse mission-root-unavailable, cause
managed-storage-binding-mismatch. Même lecture publique refusée avec l’image
gelée357 puis358 ; ce n’est pas une régression du badge. Les dix métadonnées
d’agents sont lues indépendamment et restent des états archivés. Propriétaire :
liaison de stockage de la mission historique en pause ; reprise : identifier le
Root/inode attendu et préparer une restauration ou réassociation publique autorisée
avec preuves, sans modifier la base active ou fabriquer une identité. Cette demande
de présentation n’autorise pas à réparer la liaison ni relancer la mission.

Conservation achevée : 75 fichiers comparés aux sources, copie USB
relue ; SHA-256 `69a3fcd96652b1f11cd12c485eb1e0af4c16d560c1fbb58511fd62235709f78e`.
Recette restaurée et revérifiée VERIFIED/PASS. Aucun original supprimé.
Diagnostic complémentaire et rapport final postérieurs à l’archive copiés séparément
avec leurs empreintes ; archive initiale immuable.
