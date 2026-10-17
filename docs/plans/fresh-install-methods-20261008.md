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

## Suivi du 16 octobre — amélioration autorécursive et capacité des méthodes

L'utilisateur demande de conserver toute amélioration découverte comme un
élément de grande valeur, pour cette session et les suivantes. La règle complète
est enregistrée dans les consignes générales Codex et la skill personnelle
`replan` ; le dépôt, le contrat commun et les méthodes FR/EN la relaient.
Sa réception dans les consignes de la session courante est observée. Cette
persistance ne prouve pas une amélioration autonome du moteur installé.

Le premier ajout au contrat et à la skill embarqués a révélé un dépassement du
cadrage : planner 33 173 octets, subplanner 33 176, pour une borne existante de
32 000. Le contrôle refuse sans tronquer. La reprise conserve cet échec, factorise
le nouvel ajout dans la boucle existante et raccourcit la description de la skill,
sans relever la borne ni retirer les consignes antérieures. Les quatre rôles
passent ensuite `TestAgentWorkflowRolesAreBounded` ; le worker scripté reçoit le
contexte complet figé et sa réouverture le conserve dans
`TestAgentWorkflowReachesWorkerAndSurvivesReopen`. Six noms RUN/PASS, dont les
quatre sous-cas de rôle, sont effectivement appariés. Aucun modèle réel appelé.

Les longueurs affichées par le test de rôle incluent l'enveloppe JSON et ne sont
pas la mesure du corps à laquelle s'applique la borne dans `agent_workflow.go`.
Ne pas comparer ces deux représentations comme si elles étaient identiques.

Amélioration à conserver ouverte : le cadrage reste proche de sa borne.
Propriétaire Codex ; bénéfice attendu : éviter que les prochains apprentissages
empêchent l'admission des rôles. Avant un nouvel enrichissement, inventorier les
répétitions par responsabilité et comparer leur factorisation à un chargement
plus ciblé, sans perdre limites de rôle, originaux requis ou preuves. Vérifier
les quatre assemblages et le contenu reçu par un fournisseur scripté avant
toute annonce. Ce suivi n'autorise ni nouvelle politique de limite ni exécution
native. La compression structurelle reste à instruire ; seule la réduction
ciblée de cet ajout est vérifiée.

Originaux et reçus de cette reprise :
`/tmp/swarm-recursive-improvement-xnzq6g5a/` contient
`shared-contexts-validation.json` (échec initial), les deux préimages surdimensionnées,
`replan-context-size.json` (diagnostic) et `compact-validation.json` (contrôle final).
Ce répertoire local temporaire est conservé ; aucun déploiement ni qualification
des identités des agents ne découle de ces contrôles de méthode.
