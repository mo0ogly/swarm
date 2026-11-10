# PAEA — incrément60 : création atomique du contrôleur

Reprise59 vérifiée dev208 ; Codex seul sans GLM/agent/commit/push. PA00..13 et
RETEX restent objectifs complets,0sur17 globaux. APEX/replan : supprimer la fenêtre
try_wait/pidfd_open, pas l’accepter comme frontière opérationnelle.

Choix : Rust std create_pidfd indisponible stable1.96 (sonde E0658), GNU natif
pidfd_spawn disponible2.43. Utiliser le second avec résolution de capacité avant
appel et aucun fallback PID/PATH/shell. ABI installé bits/spawn_ext.h et spawn.h,
libc0.2.186 types POSIX existants mais wrapper absent ; FFI uniquement kernel.
Référence primaire : https://sourceware.org/pipermail/glibc-cvs/2024q1/084106.html.

Entrées explicites : chemin exécutable/répertoire absolus, argv nonvide et env
exacts sans héritage, trois flux empruntés, masques/défauts signaux fournis par
appelant. Ne pas inventer politique de lancement : approbation/identités/budget
et image restent responsabilités du consommateur à raccorder. Dupliquer flux
hors0..2 puis fermer tous autres FD dans enfant ;3 est frontière technique des
flux POSIX, pas plafond opérateur. Pas d’ouverture/troncature de fichiers cible.
Précontrôler pidfs64 avant exécution ; symbole absent refuse avant invocation.

Résultat possède handle atomique même si enfant déjà terminé. Après appel réussi,
tout refus de validation ou cleanup conserve le processus dans Failure.child,
avec invocation connue ; aucun Err=pas d’effet. Attente uniquement sur pidfd,
observation sans attente et attente explicite, jamais PID numérique. Fermer ne
signale/récolte pas implicitement. Drop dernier recours ferme FD seulement.
Nouveau parcours Service reçoit le type Spawned nonforgeable puis conserve UID,
processus, namespace58 et tous contrôles/Reply/erreurs existants.

Témoins/refus/reprise : vrai socket attribué, cwd/env/signaux/FD effectifs, sortie
rapide, exec absent, attente et réouverture de contrôle ; panne post-spawn avec
propriétaire gardé ; intégration stricte qui ne contourne aucun namespace/UID.
Recettes/docstrings/FR-EN/revue politiques-fonctions-erreurs/source exacts, profil
normal et livraison locale. UIDmap admin/ptrace réel/humain/confinement/évaluateur
indépendant restent ouverts ; aucune activation globale ou délégation implicite.

## État de reprise : vérifié en bibliothèque, livraison en attente

35tests Rust distincts PASS (neuf nouveaux,26antérieurs), Rustdoc strict six
sources/trois pages avec neuf HTML originaux, trace finale11 CLONE_PIDFD et
waitid(P_PIDFD) sans capture du PID enfant. complete07/regressions08/trace11
ont des intégrités vérifiées. La première recette06 refusait une assertion
observable absente ; corrigée avant07. Trace05 reste historique après ajout
des étapes documentaires à la recette. Refus initial verify --output corrigé
en --evidence ; aucune preuve fabriquée.

Les builds09,10,11 ont compilé mais leurs profils ont refusé des entrées partagées
modifiées pendant compilation :09CLI/session_visit ;10nouvel asset
web/native-session-visit.js non prélié ;11notes de version et fichiers moteur.
Aucun de ces candidats installé. Dev208 reste la dernière instance qualifiée
par PAEA, ELF06592796d06752f478e03a82ac17d6c95c6ad177a364f321afa74dc79d1aaf90.
Ne pas relancer inchangé : reprise après phase source de l'autre chat stabilisée
ou candidat pris en charge contenant67entrées PAEA actuelles et notes propres.
Détails/empreintes dans replan-delivery-concurrency.json sous
`evidence/20261027-atomic60-zzc42alw/`. Continuer les travaux indépendants.

Bibliothèque GNU seulement ; aucun daemon, séparation réelle des comptes,
confinement, humain ou acceptation indépendante. PA02partiel/0sur17/E2/E3/PA13/
RETEX ouverts. uidmap administrateur et permission ptrace inter-UID restent
ouverts ; pas de fallback UID partagée.


## Livraison locale de la bibliothèque atomique

v0.1.1-dev.209, ELF `ebe65e0d253b012d2a6fb988ea7c3de03a857d750d7aa631c150ceb2021b2e74`, PID1713359 au contrôle final. Profil normal
645 entrées effectives/67 PAEA ; sans test-fixtures,
les sources cfg(test) exclues. 35tests Rust distincts PASS : neuf nouveaux et
26antérieurs ; les deux tests privés atomiques ne sont pas comptés deux fois.
Rustdoc strict : six sources exactes/trois pages de contrat, neuf HTML archivés.
Trace native finale11 : CLONE_PIDFD, waitid(P_PIDFD), aucun pidfd_open de PID enfant.
Trace05 reste historique après ajout des étapes documentaires à la recette ;
11 est liée à la recette finale.

Même ELF installé pour cinq appels/56commandes, sept personnels+trois gardes,
trois paires peer/six commandes et56contrôles UI FR/EN+garde. Le parcours natif
existant ne prétend pas appeler un daemon atomique inexistant. Version/notes,
commit réel et état modifié vérifiés. Recettes et originaux sous
`evidence/20261027-atomic60-zzc42alw/` ; revue liée aux fichiers liée aux empreintes et archive.

Build09 compilé puis refusé : deux entrées partagées modifiées pendant compilation,
sans installation. Build10 reprend leurs nouvelles empreintes, sans modifier les
sources étrangères. Refus recette06 conservé : assertion observable Rustdoc absente,
ajout avant07 PASS. Appel verify initial avec --output refusé, corrigé en --evidence ;
aucune preuve transformée. Neuf nouveaux tests et preuve atomique sont locaux ;
post-erreur injectée est synthétique, pas panne OS destroy réelle.

PA-02partiel,0sur17/E2/E3/PA13/RETEX ouverts. Aucun daemon, approbation/image,
identité humaine, séparation de comptes réelle ou confinement acquis. uidmap admin
et ptrace inter-UID toujours ouverts. Suite : consommateur protégé, admission et
PA03. L'entrée documentaire actuelle FR/EN est réconciliée avec209 et ses limites ;
le suivi documentation-followup.json conserve la découverte. Aucun commit/push/GLM/agent.
