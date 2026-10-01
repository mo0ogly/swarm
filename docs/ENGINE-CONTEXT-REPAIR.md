# Correctif ciblé : contrat courant et reprise des workers

## Défaut observé

Mission Administration : le worker a traité un rapport ancien comme le résultat
courant. Le contexte hiérarchique omettait le brief commun. Le contrat de tâche
était stocké dans `Task.Next`, qui sert aussi au statut opérationnel et est modifié
au lancement. Un profil conservé ne constitue pas une source fiable du contrat.

## Choix

Reconstituer le contexte depuis le dernier plan approuvé contenant la tâche,
plutôt que recopier l'ancien prompt ou ajouter une deuxième copie persistée du
contrat. Aucune migration de base nécessaire. Les contraintes du profil restent
présentes, mais les consignes historiques ne priment pas sur le contrat courant.

- Inclure périmètre, preuves, conditions et limites de la seule tâche concernée.
- Indiquer mission, tâche, agent, tentative, compteur et révision courants.
- Donner le brief partagé aux workers du périmètre racine. Les workers délégués
  conservent leur objectif local et leurs exigences assignées, sans le brief racine
  ni les contrats des autres tâches.
- Demander un rapport actualisé et attribuable lors d'une reprise ; les rapports
  antérieurs restent des éléments historiques, jamais une acceptation implicite.
- Conserver la limite globale de contexte existante : refus explicite au-delà de
  128000 octets, sans troncature silencieuse.

## Vérification

Tests ciblés : dernière révision du contrat, identité courante, isolation des
périmètres et transmission réelle au fournisseur factice. Le test d'intégration
échoue lorsque le raccordement du correctif est retiré, puis passe avec lui.
Le fournisseur est local et factice : aucun appel payant, aucune mission réelle
acceptée, aucune preuve d'autonomie complète déduite de ces tests.

## Limites

Ce correctif n'évalue pas automatiquement la qualité sémantique d'un rapport et
ne garantit pas qu'un modèle respectera ses consignes. Il ne relève aucun budget,
ne remet aucun compteur à zéro et ne contourne aucune revue indépendante.
La mission Administration reste en pause et ses 3 tentatives restent conservées.

## Résultats du 29 septembre 2026

- `go test -run 'TestWorkerContract|TestAgentWorkflowReachesWorker' .` : PASS.
- Test de retrait du raccordement : échec attendu sur l'identité courante absente
  du prompt réellement transmis au fournisseur factice ; raccordement restauré.
- `go test -timeout 25m ./...` : PASS, version finale 355,477 s (première passe
  347,926 s avant ajustements mineurs du libellé et des décisions déléguées).
- `go vet ./...` : PASS ; `git diff --check` : PASS.
- Build installé : SHA-256
  `e9d022cb524935131351ba7c4b6f9143a6a14c5e108c9ec9cccd7742db6a419e`.
- Cockpit rouvert après remplacement du serveur : mission conservée, bouton
  « Reprendre la mission », aucune relance effectuée. État métier toujours 0/8.

Le binaire inclut également les modifications de texte bilingue déjà présentes
avant cette intervention. Les changements ne sont pas encore commités. Une
ancienne copie du binaire et les métadonnées privées sont conservées hors dépôt
pour retour arrière. Aucun résultat de mission n’a été promu pour cette livraison.

## Reprise d’un rapport absent après épuisement des tentatives

La mission Administration a révélé une impasse distincte : le quatrième essai
exceptionnel exigeait un refus indépendant courant, alors que l’absence de rapport
empêchait cette revue. Le plan conserve son maximum ordinaire de trois tentatives.
Le parcours opérateur `authorize-recovery` accepte désormais `missing_report: true`
comme cause exclusive, avec confirmation, motif, nouvelle consigne et identité de
la dernière tentative. Il exige un refus de relais enregistré par le conducteur
sur un agent terminé de cette tentative. Les copies Git gérées gardent leur parcours
lié au commit. Aucun avis indépendant n’est inventé.

Le contrôle est répété dans la transaction SQLite avant validation. Une absence
non démontrée, une autre tentative, un agent actif, une revue active ou une seconde
autorisation sont refusés. L’octroi conserve les critères, les tentatives, les
budgets d’appels et la pause ; seule cette tâche reçoit un quatrième essai. Le
contexte du worker explicite cette autorisation et sa correction, pour éviter
qu’un plafond historique du plan ne soit pris pour le plafond opérationnel courant.

Tests ciblés et deux recettes navigateur isolées (reprise après avis, reprise sans
rapport) réussis : FR/EN, deux thèmes, fermeture clavier, refus de révision périmée,
persistance, historique conservé, aucun appel fournisseur dans les recettes.
Ces tests ne constituent pas une validation du contenu de T1. Le résultat réel
reste à observer après déploiement et relance depuis le cockpit.

### Vérifications finales et déploiement

- `go test ./...` : PASS 355,832 s, puis PASS 359,796 s sur la version finale
  incluant le contexte d’autorisation corrective.
- Tests ciblés `-race` de reprise et contexte : PASS ; `go vet ./...` : PASS.
- `npm test` : PASS ; recettes de reprise avec et sans avis : PASS.
- Extension web du délai de revue : recette FR/EN, clair/sombre, refus de 901,
  sauvegarde de 300 secondes sans modification des appels ni tâches : PASS.
- Binaire courant SHA-256 :
  `c5b345d2ce60eacebe7c51f125ca0447170eab7854b163d7749ac17b84d68186`.

Le parcours réel et ses limites sont consignés dans `RETEX-ADMIN-PREPARATION.md`.

### Citations de la revue documentaire

La revue à 300 secondes a rendu une réponse, rejetée au critère 2 pour citation
introuvable. Le schéma des revues non gérées ne précisait pas les contraintes de
citation déjà présentes dans les revues Git. Alignement : un court extrait contigu,
aucune paraphrase ni concaténation, analyse séparée. La présence de la citation
utilise désormais le même contrôle Markdown que les fragments : retours physiques
et délimiteurs inline tolérés, frontières de paragraphe et blocs de code conservés.
Les tests couvrent citations pliées, code inline, invention, paraphrase, collage de
paragraphes et altération de chaîne dans un bloc de code. Aucun avis rejeté n’est
converti rétrospectivement en succès. L’extrait refusé est désormais affiché de
façon bornée dans le diagnostic pour permettre un examen précis au prochain échec.

Tests ciblés et `go vet ./...` : PASS. Binaire déployé :
`d1d4ad546e032e9b3d75a45647972e5fc4eb5575b0d055e32df5ab9bf0b2884d`.
La cause exacte de la citation précédente (mise en forme ou paraphrase) n’est pas
démontrée : l’ancien diagnostic ne conservait pas l’extrait rejeté. Le correctif
ne doit donc pas être présenté comme une preuve que cette citation était correcte.

### Livrable documentaire distinct du rapport de suivi

Le verdict réel suivant a reconnu la citation du critère 2, mais demandé la preuve
ADM-01 à 04 : le moteur n’avait transmis que le rapport de suivi, pas le livrable
`docs/T1-inventaire-existant.md`. Le dossier de revue inclut maintenant le livrable
Markdown déclaré sous `docs/`, s’il est distinct et présent, limité à 48 Ko sans
troncature. Aucun lien du rapport n’est suivi, aucun balayage de fichiers n’est fait.
Son empreinte est persistée dans `report_artifacts`, contrôlée à l’enregistrement
et avant acceptation. Le test fournisseur vérifie la présence du livrable dans le
prompt réel ; changer ce fichier invalide l’avis. Un document absent reste absent,
sans contenu inventé. Les anciens avis sans `report_artifacts` gardent leur ancien
périmètre : ce correctif ne leur attribue pas rétroactivement une lecture.

La revue a aussi relevé la contradiction du critère 3 : un seul document autorisé
alors que le moteur exige un livrable et son rapport de suivi. Une clarification
est enregistrée dans le brouillon web du plan, limitée aux deux chemins précis,
sans autoriser de modification applicative. Elle n’est pas appliquée à la mission
et ne transforme pas l’avis négatif en succès. Les départs sont en pause.

## Révision du plan et soumission du rapport — 29 septembre, suite

Deux ruptures distinctes du parcours web ont été reproduites et corrigées :

1. La révision du plan conservait le refus indépendant courant, bloquant une
   revue sur le nouveau contrat. Elle écrasait aussi une reprise exceptionnelle
   inchangée. Les avis impactés sont désormais conservés dans l'historique,
   les compteurs ne sont pas remboursés et les revues actives interdisent l'édition.
2. Une soumission opérateur créait une tentative synthétique, sans producteur.
   Le rapport d'un agent terminé conserve désormais son identité. Une réparation
   explicite par la même action web reconnaît l'ancien événement fautif, conserve
   son entrée dans `legacy_report_submissions` et ne retire aucune exécution réelle.

Tests ciblés : révision + soumission web + véritable processus de revue simulé
par adaptateur déterministe, refus de réparation avec agent réel, événement absent,
rapport différent, et conservation du parcours manuel historique sans planificateur.
La première suite complète après le correctif de révision réussit en 350.299 s.
La suite suivante a identifié une incompatibilité avec la soumission manuelle
historique d'un rapport reconstitué par l'opérateur ; cette compatibilité est
rétablie uniquement hors mission hiérarchique. Nouvelle suite complète en cours.

La revue réelle T1 (appel 5/40) confirme les critères 1 et 2 et demande la preuve
brute pour le critère 3. Le journal natif a été retrouvé : 14 outils, 9 commandes
Bash, 3 lectures et 2 écritures documentaires. Les commandes et sorties Git sont
ajoutées au rapport avec attribution explicite au superviseur. Aucun nouveau
worker, aucun effacement de tentative ni augmentation de plafond. Les critères
restent inchangés ; la révision précise la provenance des preuves attendues.

Validation finale de ces correctifs : `go test ./...` PASS (351.302 s),
`go vet ./...` PASS et `git diff --check` PASS. Le parcours web réel a abouti
à une acceptation normale de T1 et affiche 1/8, sans dérogation. Les tests
isolés ne sont pas confondus avec ce résultat réel. Voir le RETEX pour la
préparation encore technique de l'évaluation documentaire par le superviseur.
