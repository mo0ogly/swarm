# Équipe initiale des préparations Rust — 26 octobre 2026

## Parcours et décision

Owner Codex, reprise APEX/replan après release-plan. Le service initial dev121
est distinct de la recette ; les fichiers partagés et les travaux d’autrui sont
préservés. Aucune autorisation de commit/push n’est déduite de « next ».

planning.enable est porté mais refuse un Work contenant des tâches : le chaîner
avec des écritures de tâches aurait une publication partielle et ne satisferait
pas le contrat Go. Le raccord configure-team assemble donc l’organisation dans
le writer transactionnel de préparation. Premier périmètre : conversion entière
intacte, non libérée, fournisseurs CLI configurés, revue humaine et aucun dépôt
géré. La création/reconfiguration automatique, les contrôles automatiques et
l’adoption d’historique sont refusés explicitement, pas remplacés par un succès vide.

Le formulaire existant est repris avec les champs réellement portés. Ses plafonds
restent vides jusqu’au choix opérateur ; aucun20/40 ou max100 historique. Le
minimum de tâches vient des tâches affichées, sans plafond frontend supplémentaire.
La revue lie deux révisions et le plan converti ; l’autorisation de départ reste
séparée. Les IDs fournisseurs viennent du catalogue du projet, pas d’une liste
codée dans le formulaire. Leur saisie manuelle ne prouve pas une découverte UX.

## Contrats, documentation des fonctions et erreurs

- **Politiques** : max_tasks et max_calls sont des choix opérateur persistés dans
  Work.planning et reviewer, consommés par les gardes existantes. max_calls fournit
  les trois plafonds décisions/activations/revue, comme l’assemblage Go ; ce n’est
  pas une réserve monétaire. Les defaults reviewer et gates sont lus dans les
  réglages réservés courants. Aucun nouveau réglage, timer, fallback ou retry.
  max_calls>0 et max_tasks≥nombre existant sont des préconditions du protocole.
  root, worker, req-N, human et reviewer_required=true sont le contrat de cette
  capacité initiale déclarée ; les valeurs des recettes sont des entrées de test.
- **Fonctions** : configuration décode les champs exacts, worker observe les
  sources sous réservation, configure assemble et persiste sous double CAS.
  populate est extrait de création et réutilisé sur un candidat Work privé neuf
  pour comparer les tâches d’origine, y compris leurs histoires/politiques. Ses
  erreurs font abandonner ce candidat, sans garantie d’atomicité sur une mémoire
  arbitraire fournie par un autre appelant. Witness final couvre les trois types
  d’événements ; adaptateurs CLI/API, contrôles, formulaire/revue/confirmation sont
  documentés dans leurs fonctions et les guides FR/EN. Aucun avis GLM ne remplace
  la lecture des vrais producteurs et consommateurs.
- **Erreurs** : rejeu canonique avant IO fournisseur ; plan frais, CAS et contrat
  exact avant sources ; provider/worker/workspace et capacité avant publication.
  Work, événement, préparation et reçu sont relus avant commit. Les échecs
  rollback/commit/close/output gardent leurs causes et leur issue connue/inconnue.
  Le client garde l’événement après réponse perdue ; seules les erreurs connues
  avant publication autorisent l’édition. Un lock de coordination providers.lock
  peut être créé lors de l’observation et subsister après un refus : ce n’est pas
  une configuration fournisseur publiée ni un processus exécuté. La réservation
  est non bloquante ; aucune boucle d’attente ajoutée. Les sources ne sont pas
  louées jusqu’au lancement et les vérifications de lancement restent requises.

La revue actuelle des octets et ses limites sont conservées dans source-review.json.
Le compte-rendu ne constitue ni conformité globale, ni acceptation native indépendante.

## Replans et preuves originales

1. Baseline121 : configure-team refuse sans mutation SQL.
2. Premier build : profile n’est pas le champ Rust/JSON, launch_profile l’est.
   Notre contexte GLM comportait aussi cette mauvaise hypothèse. Le code et la
   recette consomment désormais le vrai champ ; erreurs compilateur conservées.
3. La proposition GLM donne le mapping critères/exigences mais invente Prepared.id,
   FNV et plusieurs types/champs. Ces parties ne sont pas intégrées ; populate
   réutilise le SHA et les contrats existants. Aucun code modèle exécuté tel quel.
4. Régression aide : une assertion supposait create-missions et release-plan
   adjacents. L’ajout de commande ne doit pas casser ce contrat d’aide ; l’assertion
   exige désormais la présence réelle de release-plan dans la liste.
5. Régression focus : l’ancienne recette conversion échantillonnait avant close.
   Le témoin stable et les autres thèmes fonctionnaient, puis en-sombre révélait
   la course. Elle attend maintenant l’événement natif, comme les deux recettes
   suivantes ; pas de délai arbitraire ni suppression de l’assertion de focus.
6. Contre-exemple HTTP122 : fournisseur inconnu donnait500. Les seules sélections
   refusées typées sont désormais400 (provider/workspace) ou409 (policy stale),
   sous l’opération préparation. Les causes IO, fermeture et rollback ne sont pas
   dégradées par une classification générale. La recette reproduit500 puis
   exige les statuts corrigés avec SQL inchangé.

## Vérification et livraison

Le profil normal dev124 est sans fixtures. Le SHA testé et exécuté est
cee2ab96e0d3f09e93d17bfc9aca712ae89333056589a37fa85c1af7c325fe0d,
commit déclaré cbd6fecd7, modified=true. L’instance partagée utilisait déjà ce
candidat : aucune interruption supplémentaire pour simuler une installation.
38 assertions CLI/API +54 navigateur FR/EN × etat/sombre passent sur ce binaire :
source canonique, requêtes invalides, sources périmées, workspace extérieur,
contrat édité, budgets/defaults, CAS, CSRF, reçu perdu et rejeu, champs de budget
accessibles et navigation clavier à390px. Aucun programme fixture fournisseur
n’est exécuté, aucune API réelle ni aucun compte sollicité par la recette.

Conversion :50 CLI/API +51 navigateur. Libération :52+42. Ces régressions sont
sur122 avant le seul delta de classification HTTP concerné et les entrées
concurrentes non possédées ; elles ne deviennent pas des tests globaux sur124.
La recette équipe actuelle vérifie création/conversion, configuration puis
libération séparée sur124. Le test Witness actuel passe1 test nommé, trois types
d’événements et dommages tardifs Work/lock/event, avec témoins, rollback et reprise.
Les traductions passent. Les recettes finales sont vérifiées par l’outil permanent.

Livraison réelle :16 contrôles HTTP/processus/ressources +14 Version FR/EN +6
préparation en lecture seule. SHA du processus et du reçu, ressources servies et
nouvelles notes concordent. Réglages conservés et révision/pause/autonomie/slots/
consentement/archive des deux missions identiques à121. Aucun POST de recette sur
l’instance locale. L’activation de l’écriture Admin et les budgets réels restent
une décision opérateur, pas une modification de cette livraison.

GLM demandé/déclaré glm-5.2 : proposition107,823s,16000 tokens ; revue1018,167s,
29891 tokens. Le modèle n’a exécuté aucun test. Son verdict ne démontre aucun
défaut reproductible, sous faits appelants. Les limites citées ont été recoupées :
bootstrap lit/coordonne et peut créer providers.lock mais ne publie pas de config ;
preparation_project n’expurge pas les documents ; actor système vient de host/uid,
pas d’une identité humaine. Le terme « rejeté » pour le reçu identique dans l’avis
est inexact : le code et les tests prouvent un rejeu retourné sans écriture.
Aucune économie de tokens ni accélération n’est déduite de ces deux appels.

## RETEX et suite conservée

Fournir les vrais types Rust au modèle, sans transposer les noms Go ; refuser tout
hash placeholder. Propager les apprentissages d’événements navigateur à leurs
anciennes recettes, sans réécrire les résultats historiques. Classer les refus au
niveau de leurs types/consommateurs, sans exception de mission ni simple chaîne
transformant toutes les pannes en400. Les recettes permanentes et les sources de
cette livraison transmettent ces corrections aux prochains contextes ; les
contextes natifs déjà lancés ne sont pas supposés mis à jour.

Découvertes utiles hors scope : améliorer la sélection de fournisseur avec une
liste autoritative de métadonnées non secrètes (owner UX/CLI, critère IDs du même
catalogue, reprise prochain parcours équipe) ; consolider le critère global Go
actuellement répété entre création et vérification (owner moteur, critère une
source commune avec sorties inchangées, reprise lors de son évolution). Ce sont
des améliorations conservées, pas des effets déjà vérifiés.

Restent hors lot : organisation automatique/API, repository, modifications d’une
équipe existante, conversion/révision partielle et dialogue IA. La qualification
Go différentielle complète, le lancement réel de fournisseurs et le RETEX final
de migration restent ouverts. Les preuves privées sont conservées hors cache
sous evidence/20261026-preparation-team/private et ignorées Git.

Les valeurs reviewer Admin sont modifiées puis restaurées par les opérations publiques de la fixture : les nouvelles équipes consomment la nouvelle valeur et les équipes existantes conservent leur snapshot. Le diff propre au lot passe ; le diff global conserve les deux alertes EOF étrangères déjà observées.
