# M32 — MEM-P1, fraîcheur des appels structurés

17 octobre 2026, prochaine tranche, Codex seul. Replan/audit-pdca en correction.
Préimages et reçus : `evidence/m32-20261017`. Conserver les incréments précédents
et les chantiers concurrents identité/UI ; aucun appel fournisseur réel, budget
nouveau, délégation, déploiement, commit ou redémarrage du serveur existant.

## État, observation et décision

Six callbacks de revue/planification convertissent une erreur get en false. Le
lanceur booléen ne reçoit donc pas sa cause. Il vérifie avant appel et au poll
de 200 ms existant, mais pas après une réponse rapide, avant son retour. Le
consommateur final recontrôle certaines preuves ; cela ne justifie pas un résultat
structuré annoncé valable si le contrôle de fraîcheur n’a pas été relu.

Deux options : reporter la cause dans une variable partagée entre callback et
consommateur, ou introduire un contrôle typé `func() error` dans le lanceur commun.
Retenir le second : pas d’état partagé ni comparaison du texte d’une erreur. Les
entrées booléennes historiques restent compatibles par un adaptateur, avec refus
explicite si le contrôle est absent. Les six producteurs passent le contrôle
avec cause ; une fonction commune lit le Work avant leur contrat spécifique.

## Critères et séquence

| Critère | Observable |
| --- | --- |
| M32-01 | Réponse rapide invalidée reproduite avant correction ; originaux et nouveaux tests sauvegardés dès leur premier reçu. |
| M32-02 | Contrôle avec cause avant départ, au poll et après fin ; erreurs Is/As conservées, aucune réponse exploitable sous refus, aucun lancement sans contrôle. |
| M32-03 | Six callbacks raccordés sans false sur erreur Work, fichiers ou fournisseur ; pause/remplacement refusés ; reprise valide et consommation conservée. |
| M32-04 | Contrôles ciblés/race, chemin producteur réel avec double local, pas de publication passée/acceptée en échec ; persistance du diagnostic ou refus de lecture explicite. |
| M32-05 | Entrées exactes stables et inventoriées, vet/build, docs FR/EN et schéma concerné, auto-revue et RETEX, limites explicites. |

Boucle bornée : reproduire → contrat commun → six consommateurs → erreur/reprise
et non-publication → documentation/RETEX. Go `-timeout=0`, subprocess sans
watchdog global faute de borne contractuelle ; progression observée. Les délais
produit existants restent intacts. Capturer aussi les entrées web embarquées :
une dérive concurrente doit être conservée et requalifiée, jamais écrasée.

P1 garde les replis/vues, enveloppes liées et injection de préflight déjà ouverts
dans l’audit M31. P2–P5 et droits M20 ne sont pas livrés par ce lot.

## M32-I01 — fixture externe refusée

`red-batch` n’est pas une reproduction du panic : le fournisseur Python de la
fixture tente une écriture SQLite sans la fonction `swarm_identity_plan_digest_v1`
requise par les triggers. L’écriture est refusée, le lot n’est pas remplacé ;
l’assertion du test échoue ensuite. Preuve originale et source conservées, aucun
trigger supprimé. Reprise : barrière ready/release du fournisseur local, mutation
par la connexion du Store avec ses fonctions d’intégrité, puis retour observé
avec capture du panic dans la goroutine propriétaire. Cela conserve les protections
et ne modifie aucune base de mission réelle.

## M32-I02 — frontière de résultat et lecture de pause

`red-batch-native` reproduit le panic exact après annulation : index 0 d’une liste
vide. La correction recontrôle longueur et identité sous la mutation de résultat,
sans rétablir l’ancien lot ni publier le candidat. La consommation du premier
appel commencé reste conservée, sans deuxième appel silencieux.

La revue remarque aussi `paused`, qui absorbe toute erreur SQL et retourne false.
Hypothèse à reproduire : un contrôle paused non booléen peut autoriser l’appel
malgré cette lecture invalide. Les cinq callbacks qui utilisent cette information
peuvent adopter une lecture avec erreur sans modifier le fichier identité partagé.
Le diagnostic explicatif ne consultait pas ce flag : ne pas lui inventer une
nouvelle politique de pause. Une erreur de conversion doit conserver sa cause
sans recopier la valeur non fiable dans son message public. Cette extension reste
limitée aux appels structurés ; les autres replis booléens demeurent à qualifier.

## M32-I03 — mesure de consommation de la fixture

`candidate` exécute 140 noms PASS mais échoue sur l’assertion de consommation de
la nouvelle fixture. Le helper managedReviewCalls compte les lignes du fichier
écrit par l’ancien faux fournisseur ; le nouveau fournisseur à barrière n’écrit
pas ce fichier. Ce n’est pas une preuve de remboursement. Reprendre l’assertion
sur le compteur persistant Reviewer.Calls du Work avant/après, qui est bien le
contrat visé, et vérifier les surfaces CLI/HTTP du refus. Reçu initial conservé.
Aucun test produit existant n’est modifié et aucune consommation n’est remise à zéro.

## M32-I04 — résultat exact du cas de pause

`red-pause` ne prouve pas une publication non autorisée : la réservation de
fragment rejette déjà la valeur invalide, avant un nouvel appel structuré. Le
défaut reproduit est la copie de sa valeur dans le diagnostic de conversion.
Le lecteur partagé readMissionPaused garde la cause via Unwrap et un message
public fixe. Il sert aux cinq callbacks déjà sensibles à la pause, aux mutations
de début/journal/finalisation de fragments et aux contrôles claim/decide/judge/
renew de planification. Absence historique = non pausé ; booléen lisible requis.
Le diagnostic explicatif ne reçoit aucune nouvelle politique de pause.

Les variantes DB/transaction, absent, false, true, texte et entier invalide
vérifient refus sans réparation implicite, message sûr et cause conservée. Les
lecteurs booléens historiques hors de ce périmètre restent une suite explicite,
en particulier paused/prepareLaunch (agents_store), role_model et le contrat de
reprise dans store.go. Les fichiers identité partagés ne sont pas modifiés.

Le contrôle final est élargi aux tests du contrat planning_test.go et des trois
modules de stockage de fragments, car les mêmes gardes transactionnelles de pause
ont changé ; les précédents résultats ne qualifient pas cette nouvelle source.

## M32-I05 — tests concurrents et preuve exacte

La première série `final` exécute 187 noms PASS, vet/build réussis, mais son reçu
reste FAIL : internal/enrollmentbootstrap/bootstrap_linux_test.go et
internal/engine/agent_identity_bootstrap_linux_test.go ont changé entre les deux
inventaires. Les sources de production M32 n’ont pas changé. Les deux versions
et empreintes restent liées aux reçus, aucune modification concurrente écrasée.
La série race partie après ce changement fournit un témoin de stabilité avant
requalification des entrées courantes. Ne pas transformer un exit 0 en preuve
stable, ni modifier un test identité pour faire passer notre reçu.

## M32-I06 — stabilisation coordonnée et ressources web de fixture

Les séries refresh (187 noms PASS, vet/build réussis) et race (34 noms PASS)
restent FAIL au contrôle de version : plusieurs sources Go du chantier identité
ont changé pendant leur exécution. Leurs reçus restent intacts. L’utilisateur
a autorisé la coordination des deux sessions ; le propriétaire identité annonce
maintenant ses sources Go stables et poursuit seulement ses docs/preuves.

Attendre aussi la fin du chantier UI ou figer ses seuls assets embarqués sont
deux reprises possibles. Retenir un overlay Go explicite des onze fichiers web
mouvants : octets de fixture et empreintes conservés, aucun fichier vivant modifié,
aucune source Go substituée, aucun contrôle désactivé. Le même overlay est fourni
à list/test/vet/build. Les autres entrées et l’inventaire avant/après restent
contrôlés. La lecture de ces assets par le système de fichiers à l’exécution
n’est pas virtualisée par Go ; les tests mémoire sélectionnés n’en dépendent pas.
Cette preuve ne qualifie pas l’interface UI courante.

La sélection normale complète est conservée. Sous race, couvrir les neuf parents
nouveaux, renouvellement, disponibilité du conducteur, réservations concurrentes
et rollback de pause ; les variantes de protocole ne créent pas d’état partagé
nouveau et restent toutes exécutées en normal. La série historique race de
34 noms n’est pas rebaptisée stable. Recettes, choix et précondition modifiée :
coordination-and-overlay.json et overlay-selection.json dans les preuves M32.

## M32-I07 — couverture incomplète de la fixture UI

Les reçus stable-race (23 noms PASS) et stable-normal (187 noms PASS,
vet/build réussis) restent FAIL au contrôle de version : web/i18n-en.js et
locales/en.json changent pendant son exécution. Aucune source Go n’a changé.
La première fixture reprenait les changements déjà observés ; elle ne couvrait
pas le contrat UI embarqué complet. Conserver cet échec et son overlay initial.

Reprise générale : figer les 138 ressources web/locales inventoriées, sans
substitution Go. Octets archivés en tar.gz dans les preuves, backing temporaire
et chemins exacts enregistrés ; même overlay pour list/test/vet/build. Toute
autre entrée ou ajout continue d’invalider le reçu. L’ancienne recette n’a pas été éditée pendant sa série normale ; les deux
reçus invalidés sont conservés. La sélection des tests ne change
pas ; nouvelle recette, nouveaux reçus et précondition de fixture complète.

## M32-I08 — espace temporaire saturé avant les tests

qualified-normal échoue en compilation, avant tout cas exécuté : écriture de
_pkg_.a impossible, no space left on device. Les entrées effectives sont stables.
/tmp est un tmpfs de 7,2 Gio saturé ; le disque du checkout garde environ 12 Gio.
Aucun ancien reçu, cache partagé, lock ou fichier d’un tiers n’est supprimé.

Entre nettoyer des données dont l’appartenance n’est pas garantie et choisir
un répertoire temporaire privé sur disque, retenir TMPDIR local au processus
de vérification. Même code, même overlay complet et mêmes tests ; emplacement
des seuls temporaires différent, espace mesuré dans disk-resume.json. Aucun
plafond ou réglage produit modifié. La série race déjà partie est observée
jusqu’à sa fin ; aucun processus tiers interrompu.

## Résultat local et clôture de l’incrément

M32-01 à M32-05 vérifiés localement : quatre contre-exemples produit conservés,
187 noms normaux (108 parents, 79 sous-cas), 23 sous race (15 parents, 8 sous-cas),
avec recouvrement ; vet/build réussis et entrées effectives stables dans les deux
reçus finaux. CLI via sa fonction Go et handler HTTP authentifié sur fixtures,
aucun serveur existant ou fournisseur réel. Rapport, FR/EN, schémas et RETEX
actualisés. Auto-revue seulement, aucune acceptation globale.

Le replan n’efface aucun reçu FAIL. Les erreurs produit, de fixture, de mesure,
de version et d’espace temporaire restent séparées ; la nouvelle précondition
est prouvée par la coordination reçue et les manifestes stables. La qualification
UI courante reste hors de ces fixtures. MEM-P1 et P2–P5 restent ouverts ; suite
prioritaire : autres lecteurs de pause et replis, selon AUDIT-REPRISE.md.

## M32-I09 — lien de synthèse avant sa génération

verification-1 échoue au contrôle des liens : le rapport lie la synthèse que
le vérificateur doit lui-même écrire après réussite. Produit et reçus Go ne
changent pas ; l’échec documentaire est conservé. Reprise : lien vers le dossier
de preuves déjà existant et nom de synthèse explicite dans le texte, sans fichier
PASS provisoire ni assouplissement du vérificateur. Refaire seulement ce contrôle
de sources/documents dans verification-2 ; réutiliser les reçus Go stables.
