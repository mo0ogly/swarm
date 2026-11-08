# Qualification des anciennes preuves Go — 8 novembre 2026

Demande : corriger la qualification elle-même, refus/reprise en fixture, aucune
relance d’agent ni réécriture de l’historique ; docs FR/EN, preuves et installation.
Replan/debug/verify-fix, Codex seul. Suivi :
`.cache/historical-go-qualification-20261108/PLAN.md`.

## Contrat et cause établie

361 : `work show` refuse globalement managed-review-fragment-protocol-not-ported.
La tâche T1 a un avis passed simple Go ; T2 a un avis unknown avec fragments v4,
jamais favorable. Le garde de projection refusait tous les protocoles non portés
avant toute qualification par tâche. Le remplacement de contrôles pour affichage
exigeait ensuite le même protocole sur des avis qui ne peuvent justifier de PASS.
Sources lues : work_projection.rs, managed_receipt.rs, review_evidence.rs et les
producteurs Go f32b9f8fa managed_git/preparation/review_fragment_store. Originaux
conservés dans go-source ; schémas observés séparés des preuves d’exécution.

Correction : les avis non favorables en protocole non porté restent explicitement
non qualifiés sur leur tâche, code conservé dans les limites des preuves. Leur
reçu de remplacement n’est pas consommé. Ils ne suppriment pas l’examen des avis
simples voisins. Les avis favorables non portés refusent toujours ; les contrôles
de contexte/contrat/candidat/fichiers/Git des avis simples restent inchangés.
Aucun état inconnu n’est converti en succès, aucune opération d’exécution ou
d’acceptation ne reçoit un nouveau droit. Les principes de revue indépendante
et de fraîcheur ne changent pas. Ceci n’est pas le port complet des fragments
favorables ni une authentification des opinions Go.

Consommateur réel réutilisé : CLI work show et Reprise/OODA, Résumé calculé
(`/api/v1/work-resume`), via inspect_work_view et inspect_work_validation. Ce
parcours calcule depuis les preuves actuelles ; le résumé enregistré n’est pas
utilisé comme secours. Le statut opérationnel global mission status possède
d’autres frontières de migration, conservées sans prétendre les corriger ici.

## Témoins, refus et incidents de méthode

Deux cas reproduisent le refus après témoin valide, avant correction. Le cas fort
réutilise le candidat Git réel et les gardes de revue/receipt/contexte existants ;
il constitue une concordance de données, pas une revue indépendante ni une
acceptation moteur. La fixture est une dérogation contrôlée, pas une tâche
acceptée par une action fabriquée. L’ajout d’un avis unknown provoquait le refus
global. Après correction : le témoin reste courant, unknown reste non qualifié ;
rapport altéré non courant, reçu original manquant refusé, restauration exacte
permettant la reprise. Chaque lecture conserve les octets de la base de fixture.
Les sept régressions gérées existantes passent, y compris les avis favorables
fragments/batches qui doivent toujours refuser. Nouveau cas fort et cas de
projection passent, noms/comptes dans les sorties brutes.

Le premier lancement n’a pas compilé : garde disque 20GiB, espace insuffisant.
18 images régénérables de tests Cargo ont été archivées/reluées sur Lexar, puis
retirées seulement sous le verrou réel Cargo1.96 et après revalidation et contrôle
FD/exe/maps accessibles. Aucun verrou, source, bibliothèque, mission ou résultat
de test retiré. Inventaire, SHA et restauration : cache-relief.json. Un test
secondaire a ensuite atteint la restauration mais réutilisé une révision déjà
présente dans la fixture ; son compteur est corrigé pour ajouter après l’historique
conservé. Tous ces échecs restent retenus ; aucun n’est compté comme PASS.

Recette maintenue : historical-go-qualification.json. Fournir les chemins des
SDK fraîchement construits via SWARM_GQ_REVIEW_SDK et SWARM_GQ_PROJECTION_SDK.
Elle contrôle les cas nommés et archive les sources déclarées ; elle ne qualifie
pas à elle seule le graphe effectif du SDK ou le binaire de production, ni
l’indépendance d’une revue. Résultats publics, installation et RETEX ci-dessous.

## Résultat public et installation

Recette SDK : 9 cas nommés PASS (7 régressions + 2 nouveaux témoins), intégrité
VERIFIED. Les sorties originales FAIL et les SDK de reproduction sont conservés.
Profil normal362 : VERIFIED_DEFAULT_NATIVE_PROFILE, aucune feature/test-fixtures
dans les crates centrales, dépendances stables. SHA256 :
775949d9dd767e28a155dcbe5ff91b4387fdd14cafa4fb2972dabc6207e4b93c.
work show sur la mission réelle passe FR et EN : huit tâches ; zéro validation
actuelle, une acceptation historique, une périmée. T1 reste enregistrée accepted
mais sa qualification actuelle est stale ; T2 reste blocked/unknown. Le refus
de protocole est conservé dans ses limites de preuve, jamais transformé en PASS.

Installation supportée restart --prepared-build du seul serveur géré : aucune
autre entrée compilée installée. /proc/2028641/exe correspond au SHA362. Dans le
navigateur authentifié réel, avant remplacement : résumé calculé refusé avec
command_not_ported et secours enregistré proposé. Après remplacement : résumé
calculé reçu, secours non utilisé et caché, révision881 et huit tâches présentes.
FR/EN, clair/sombre : observations positives ; anglais sombre à320px sans
débordement, texte pre-wrap/anywhere. Notes Version FR/EN, numéro362, lien Git
d084c1487726f0d240de84d521d19d14d7938250 et modifications non commitées vérifiés.
Aucune erreur console observée ; pas de capture exhaustive des requêtes revendiquée.

Après CLI et installation : les 49 tables, Work et manifestes ont des empreintes
identiques au témoin initial. Aucun appel de lancement/reprise d’agent, fournisseur,
review-refresh, acceptation ou mutation des budgets n’a été exécuté sur cette mission.
Le bouton/écran Reprise et OODA est un lecteur de résumé ; ce n’est pas mission resume.

## RETEX

La cause corrigée est la portée du refus : un protocole non consommable sur un avis
non favorable empêchait de calculer les faits qualifiables d’autres tâches. La
correction reste dans la projection partagée, avec code de non-qualification
explicite ; le garde favorable et les lecteurs de preuves ne sont pas assouplis.
La concordance des anciens fichiers n’authentifie pas une revue indépendante et
un avis enregistré favorable n’établit pas sa fraîcheur actuelle. Le témoin Git
et les restaurations montrent la continuité du lecteur, pas une nouvelle acceptation.

Leçon de méthode : identifier le consommateur public de qualification déjà
existant avant de confondre lecture enregistrée, qualification et statut opérationnel.
Le lecteur calculé a été réutilisé ; aucune route ni exception de mission ajoutée.
La documentation FR/EN et les recettes transmettent ce contrat aux prochains
contextes ; aucun contexte d’agent déjà démarré n’est modifié.

Limites conservées : avis favorables batch/fragment non portés toujours refusés ;
statut global mission status et attribution des anciennes copies Go ont leurs
frontières distinctes. Propriétaire : migration Rust. Reprise de ces ports après
contrat complet des producteurs et témoins/refus/restauration portant leur chaîne ;
aucune relance de la mission ni réécriture de ses preuves pour les rendre natives.

Complément ciblé de la même régression : avis changes_requested avec batches
non portés, diagnostic distinct conservé, contrôles inconnus, reprise après
restauration ; PASS. Seul le test est enrichi, aucun octet effectif de production
modifié après362. La recette est rejouée dans recipe-sdk-02 avec ce SDK relié
aux sources courantes ; recipe-sdk reste une preuve historique conservée.

## Compléments de la revue générale avant livraison finale

Le bandeau global pouvait dire « qualification actuelle non évaluée » alors que
le résumé calculé avait réussi. Observation reproduite sur362 ; libellé FR/EN
corrigé en « statut opérationnel non évalué », sans changer le calcul. 363 a été
préparé et conservé, pas installé.

Un second défaut du consommateur Git a été reproduit avec témoin canonique
courant : à fichiers et Git identiques, un alias absolu vers le même stockage
rendait la revue non courante à cause d’un strip_prefix lexical. Le lecteur
réutilise maintenant with_bound_storage : mêmes State/Work privés par descripteurs
retenus, forme standard et lane existante, Git sur le chemin canonique contenu.
Aucun locator ou original n’est réécrit. Le callback Git et les erreurs de
fermeture indépendantes restent conservés via le wrapper Root Cleanup existant ;
l’invariant callback absent refuse le calcul, sans classement silencieux en stale.
Pas d’injection de fermeture noyau revendiquée : cette branche est revue dans le
code, les traversées de wrappers existantes ne changent pas.

Le nouveau témoin passe : alias du même stockage courant ; autre datastore privé
avec même nom de mission et candidat Git cloné refusé par binding-mismatch ;
alias absent, dépôt Git temporairement absent et lane occupée refusés ; chaque
restauration reprend. Octets DB de fixture inchangés et aucun verrou Rust créé
dans le stockage Go. Les SDK et sources originaux de ce refus sont conservés.
La copie Git de cette fixture désactive les configurations globales/système et
les paramètres Git hérités ; le commit cloné est explicitement comparé. Les
autres identités Fixture sont des données de test synthétiques préexistantes,
aucun commit du dépôt n’est attribué à une identité inventée.

La correction ajoute seulement une garde d’association physique à une observation
de données. protocol_verified et authority_verified restent faux. Les branches
favorables non portées ne sont pas autorisées ; le statut opérationnel global
et les droits de lancement restent distincts. Le résultat réel de la mission
reste à contrôler sur le dernier candidat normal avant l’annonce finale.

## Livraison finale364 — qualification rétablie

Recette finale recipe-sdk-03 : 10 cas nommés PASS, intégrité VERIFIED (7 régressions
et 3 nouveaux cas, dont l’alias avec refus/restaurations). Aucun agent/provider de
la mission historique n’est exécuté. Le candidat363 reste une étape non installée ;
364 réunit les deux corrections générales et le bandeau FR/EN précis. Profil
VERIFIED_DEFAULT_NATIVE_PROFILE, graphes centraux sans test ni test-fixtures,
dépendances effectives stables. SHA256 final :
aed1422210080ceae550cdc7133f242134bfa4eb0b7088a44c40b37d3c9414f8.

Résultat réel work show FR/EN : mission blocked, une acceptation historique et
une validation courante, zéro périmée. T1 accepted/fresh/passed ; T2 blocked,
non courante et unknown ; huit tâches présentes. L’attente de mon premier contrôle
final (zéro validation comme362) était incorrecte : le défaut lexical d’alias
expliquait encore le classement stale de362. L’assertion échouée et la sortie CLI
valide sont conservées dans verification-expectation-01.json/final-show-fr.stdout.
Aucune révision, preuve, revue, tentative ou acceptation n’est réécrite pour obtenir
le nouveau résultat ; la garde des mêmes objets physiques est maintenant satisfaite.

Installation exacte par restart --prepared-build, zéro autre entrée compilée
installée. /proc/2171843/exe correspond au SHA364. Navigateur authentifié réel :
Résumé calculé reçu en FR/EN à la révision881, section Travail validé/Validated work,
les huit tâches et l’avis unknown préservés ; aucun secours enregistré utilisé.
Bandeau opérationnel désormais exact dans les deux langues. Clair/sombre et
anglais sombre320px sans débordement ; retour en français laissé à l’utilisateur.
Version364, notes FR/EN, commit d084c1487726f0d240de84d521d19d14d7938250 et
modifications non commitées effectivement lus dans le dialogue protégé. Aucune
erreur console observée sur ce parcours. Pas de capture exhaustive des requêtes
ni de hash intégral du DOM revendiqués.

Contrôle final : 49 tables, Work et manifestes identiques au témoin initial ;
zéro changement des entrées effectives du binaire installé. Les sources/docs
courantes sont revues par empreinte dans source-review-final-364.json. Les
originaux FAIL et les archives intermédiaires restent conservés.

RETEX complémentaire : une correction locale du premier refus ne suffit pas
à qualifier un résultat stale. Un témoin positif et l’examen de tous les lecteurs
ont distingué l’avis réellement inconnu du refus artificiel d’un alias équivalent.
La garde physique existante a été réutilisée, avec les deux causes d’erreur
conservées, plutôt qu’une réattribution de confiance par chemin, UID ou contenu.
Ce raccord restaure la qualification des données Go cohérentes ; il ne crée ni
revue indépendante authentifiée, ni nouvelle acceptation, ni port des protocoles
favorables batch/fragment encore refusés. Les limites du statut opérationnel global
restent séparées, avec le propriétaire/critère de reprise décrits plus haut.
