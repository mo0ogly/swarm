# PAEA72 — association persistée et dépôt Git détenu

Reprise 28 octobre10:41 UTC : dernier71, HEADcbd6fecd7e40016d6c259296ac3a21373a4bbd60,
version225/PID3323052/ELFbba13470359ddfb0fecc17b958935ccab77dbc74ec40253c853288acab1aa8f5
observés. Aucun Cargo actif ;120Go libres. Codex direct sansGLM/délégation/commit/push.

Replan PA03 : le service ne doit pas interpréter repository_id comme chemin libre.
Deux routes : registre SQL supplémentaire, ou déclaration privée immuable associant
les capacités physiques. Choix : fichier canonique NOREPLACE sous Root du service,
id dérivé des identités Root/dépôt réellement détenues, ouverture sous descripteurs.
Pas de nouvelle politique ou de plafond ; borne de lecture = taille exacte du
frame attendu, dérivée des observations typées. L'opérateur prépare explicitement
un enfant privé bare ; aucune création/adoption par une proposition agent.

L'association lie le projet physique, son pin explicite, le work et son empreinte
de dépôt géré, le Root service, le nom d'enfant et l'inode réellement ouvert.
Elle n'atteste pas le pin humain ni la séparation UID. Le journal70 doit déjà être
provisionné avec ce pin. Aucun accord, transfert d'objets, autorité de dernier
usage, COMMIT Git ou réconciliation terminale n'est ajouté par cette déclaration.

Critères : vraie fixture native retenue71 en lecture seule, service/dépôt jetables,
provision explicite/reprise, mauvais pin/projet/dépôt/candidat refusés, rebinding
et symlink refusés, inode conservé pendant déplacement, fichier altéré/noncanonique
refusé sans remplacement, causes et publication connue/incertaine conservées.
Rustdoc, recette et revue politiques/fonctions/erreurs aux octets ; livraison normale
locale après qualification. PA02/03partiels,0/17/UID/humain/E2/E3/PA13/RETEX ouverts.

core01 FAIL compilation, intégrité VERIFIED : AtomicStorageFault exporté à la
racine seulement avec feature test-fixtures. Import interne local_storage::None
corrigé ; aucune fixture activée et aucun test exécuté dans01. Cause de méthode :
recherche du symbole sans sa garde cfg adjacente ; lire l'export complet, pas la
seule ligne. Reprise sur import normal corrigé, sans contourner le profil.
Garde explicite ajoutée contre un alias physique du répertoire racine du service
lui-même (un simple nom ne suffit pas contre un mount préparé par un opérateur).

core02 PASS : cinq tests Rust nommés (normal1 + SDK ignorés explicitement4),
Rustdoc strict, sources stables et intégrité VERIFIED. ELF effectivement exercé
figé dans test02-artifacts.json. Le projet natif71 est réutilisé en lecture seule ;
le test normalprovision/reprise vérifie son fichier DB strictement inchangé.
Probe Git réel via exécutable résolu/empreinte capturée, pas une maquette.
Visibility seules des helpers git/meta modifiées ; leurs comportements antérieurs
sont inchangés, contrat et erreur réutilisés, pas de featuresfixtures activées.

Suite conservée : vérifier la compatibilité du format d'objets entre source et
dépôt privé avant tout transfert/engagement ; inspect72 n'atteste aucun transfert
ou stockage de candidat dans le bare. Propriétaire Codex/PA03 ; critère : SHA1/
SHA256 incompatible refuse avant intention/effet, compatible permet le transfert
vérifié. Réexaminer aussi les autres arguments Git dépendant d'un /proc/PARENT/fd
après erreur d'enfant non récolté ; le nouveau probe utilise cwd propre + git-dir
point pour éviter cette dépendance. Risque hérité à qualifier par cas contrôlé,
pas une faille globale prouvée. Pas de changement des anciens chemins dans ce lot.

Revue après02 : validation de proposition initialement enveloppée SqlTransaction,
malgré absence de SQL à cette frontière. Classification corrigée avec famille
fermée paea-git-proposal et cause typée conservée ; journal garde sa propre phase
paea-git-journal. Test de référence contenant newline/commit : refus dans la phase
de proposition, sans phase SQL ni mutation, puis témoin valide. Le match utilise
désormais l'OID emprunté dès la première discrimination, sans unreachable.
ELF02 et preuve verte02 restent conservés ;03 requalifie ces changements précis.

core03 PASS/VERIFIED : cinq tests nommés et Rustdoc strict ; erreur de proposition
malformée correctement classée, aucun label de phase SQL, reprise valide. Source
stable et ELF03 figé avant build04. Pas de modification de corps workflow/agent,
UI layout ou nouvelle politique ; pas de mesure de ces corps requise pour ce lot.

## Résultat local vérifié

Version **v0.1.1-dev.229**, PID3583154, installée via candidat préparé exact :
`1e2384be40e0383520204fa86bad0cb56f578316330b44626e4ca2ae4a93285e`.
Le commit effectif du binaire est bcd0a3f3880155722121ec65c2c1b9a096af73ec,
avec modifications non commitées signalées. HEAD avait changé depuis la reprise :
un acteur concurrent a commité le correctif de preuves du cache de livraison
(neuf fichiers ; git show consulté). Ce chat n'a exécuté aucun commit/push ;
ce commit et ses tests ne sont pas attribués à ce lot.

Cinq tests Rust nommés03 PASS : non-adoption sans journal, provision/inspection/
réouverture, contexte/ressource/candidat/Root malformés, substitution/symlink et
corruption/reprise. Le projet réel natif71 est lu sans mutation de son DB ; les
services/bare sont des fixtures jetables. Git réel résolu et empreinte capturée,
aucun provider ni revue positive fabriquée pour72. Source stable, intégrité
VERIFIED, Rustdoc strict PASS ; ELF03 retenu séparément dans test03-artifacts.json.
Core01 reste FAIL de compilation avant tout test, original conservé ;02 PASS
conservé avec son propre ELF,03 couvre la classification affinée de l'erreur.

Build normal : 681 entrées effectives,481 communes aux tests03, seules les notes
version_history diffèrent après qualification. Features cœur vides, test=false,
sansfixtures. Graphe normal225→229 identique ; trois écarts transitifs test/normal
inchangés, justification66/71 réutilisée explicitement. Aucune équivalence
ELFtest/ELFnormal revendiquée. Aucun nouveau consommateur public HTTP/CLI ne
provisionne de dépôt automatiquement : bibliothèque native, pas service activé.

Version06 :56 assertions UX/Version et garde de script PASS/VERIFIED ;25assets
protégés/notes comparés à l'inventaire compilé, FR clair/EN sombre, commit réel,
noncommité, Escape/focus et captures inspectés. Le vrai ELF actif correspond au
candidat. Documentation FR/EN, Mermaid et recette permanente actuelles ; revue
politiques/fonctions/erreurs aux octets. Diff de nos fichiers propre ; diff global
exit2 étranger conservé, sans déclaration de conformité globale.

## Limites et raccord restant

L'association persistée lie maintenant le Root service/projet, le work, le pin
explicite du journal, l'empreinte du dépôt géré et le bare privé réellement détenu.
L'id est dérivé des identités physiques ; la proposition ne choisit aucun chemin.
Un frame manquant/altéré, mauvais contexte ou chemin substitué refuse ; NOREPLACE
interdit la remise à zéro silencieuse d'une association. Une publication incertaine
exige ouverture/lecture explicite, pas suppression ou nouvel essai aveugle.
Le pin opérateur et les protections de stockage ne constituent pas une identité
humaine attestée ni une séparation UID. Ce résultat ne crée aucun mandat.

Reste PA03 : compatibilité de format et transfert contrôlé des objets, admission
courante65/66 et scope/plan/Trust/rôles sous réservations, intention durable70 avant
COMMIT Git69, contrôle OLD/type direct sous verrou, et réconciliation terminale.
Le nouveau inspect fournit une observation, pas une barrière d'engagement. Sous
un writer projet déjà détenu, le lecteur71 qui ouvre son snapshot devra être
composé par un helper interne sur cette même transaction, jamais par une deuxième
transaction ou un booléen de dispense. Ne pas dupliquer une revue ou adopter un
candidat qui ne serait pas le vrai dernier natif.

PA02/03 restent partiels, invariants protégés démontrés0/17 ; humain/UID/confinement/
daemon/E2/E3/PA13/RETEX final ouverts. Prérequis administrateur et approbations
déjà pendants conservés, sans demande répétée ou contournement. Pas de clôture PAEA.
