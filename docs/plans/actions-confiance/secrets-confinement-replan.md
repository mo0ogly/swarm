# T4 — secrets et confinement : reprise après admission des données

Propriétaire Codex. R14 demeure partiel ; aucune activation de compte externe,
aucun paiement, aucun privilège Linux supplémentaire ni orchestration native.
Les tranches pilote/identités/admission restent vérifiées à leurs entrées.

## État observé le 15 octobre 2026

`AIConnection.Key` dans `internal/engine/ai_connections.go` est une clé de
configuration, pas un octroi de secret éphémère lié à une action. La session web
est aussi conservée dans un fichier privé sous `.swarm`. Déplacer les secrets
dans ce répertoire ne prouve donc pas leur isolement face à un exécutable du même
utilisateur. `agents_process.go` lance actuellement le programme fournisseur avec
son environnement et un groupe de processus ; cette lecture ne démontre pas une
frontière OS pour les données sensibles. Les providers existants restent inchangés.

Options comparées : stockage référencé seul, simple à relier mais insuffisant
contre une lecture directe ; courtier en mémoire avec reprise fermée et exécution
confinée, sans secret brut persistant ; coffre externe, dépendant d'une activation
et d'une exploitation tierces absentes. Retenir le courtier en mémoire comme
premier périmètre local, après preuve de la frontière d'exécution. La reprise
après arrêt doit refuser un octroi dont la matière secrète n'est plus disponible,
sans transformer cet échec en succès ni rejouer un effet déjà engagé.

## Précondition nouvelle vérifiée

La requête de capacité Landlock retourne ABI7, noyau6.17.0-41-generic, UID1001,
sans privilège supplémentaire. La sonde isolée applique réellement les règles
dans son seul enfant : lecture/écriture de travail autorisées ; lecture externe,
traversée, lien symbolique sortant, exécution de `/bin/cat` et connexion TCP vers
un écouteur local de fixture refusées. Le parent reste hors confinement.

Preuves : `verification-donnees-20261015/containment-capability-probe.json` et
`containment-isolated-landlock.json`, avec programme conservé à côté. Ces sept
assertions établissent une faisabilité OS, pas une intégration Swarm, une
qualification fournisseur ou un courtier de secrets livré.

La [documentation primaire Linux6.17](https://docs.kernel.org/6.17/userspace-api/landlock.html)
décrit les restrictions par thread/héritage, les droits associés aux descripteurs
et le contrôle réseau par port TCP. Ne pas assimiler ce dernier à un filtrage de
destination IP ou de tout protocole. UDP et les descripteurs hérités ne sont pas
qualifiés par cette sonde.

## Prochaine action autorisée et critères

1. Définir le processus de connecteur minimal et sa fermeture de descripteurs,
   son environnement et ses sorties. Comparer confinement par couche OS et
   connecteur interne de confiance. Ne pas activer un chemin sensible tant que
   lecture directe, UDP/IPC, sockets héritées et binaires ne sont pas maîtrisés.
2. Contrat Admin/API/CLI/Go du courtier : références sans chemin arbitraire,
   principal/digest/révision/destination, durée persistée explicite, consommation
   unique, révocation, concurrence et indisponibilité après arrêt. Toute donnée
   secrète ne doit traverser qu'un canal dédié, jamais requête métier, preuve,
   export, copie d'agent, arguments ou environnement du fournisseur.
3. Contre-exemples et recettes isolées avant code : refus sans matière conservée,
   bon octroi vers le seul consommateur, autre action/sujet/destination refusés,
   réemploi refusé, panne avant/après effet et réconciliation unique.
4. Implémenter la plus petite frontière vérifiable, puis relier le courtier.
   Politiques persistées et éditables Admin avec validation commune ; aucun
   plafond ou délai métier caché. Documenter FR/EN et vérifier le parcours public.
5. Revenir ensuite à la révocation distante des services R13. Les processus
   métier, preuves avancées, connecteurs, eBPF, recette globale et RETEX restent
   ouverts. Une sonde réussie ne retire aucun de ces critères.

## Tranche suivante retenue — exécution locale scellée

Livrer une primitive de processus séparé, sans code arbitraire ni réseau, et sa
vérification publique `trust confinement-check` / API homonyme / Admin. Ce n'est
pas un confinement rétroactif des fournisseurs existants. Le parent choisit sa
fixture, passe uniquement des pipes et un environnement minimal, borne l'enfant
par une politique persistée `sealed_execution:{enabled,timeout_seconds}`. Défaut
inactif avec15 secondes affichées ; durée strictement positive et représentable,
0 refusé, omission conserve. Aperçu sans lancement ni mutation.

Le seul enfant applique Landlock sur son thread verrouillé et seccomp synchronisé
sur tous ses threads : pas de création/usage de sockets, io_uring, exécution de
binaire, changement de groupe/session ou accès mémoire d'autres processus.
Descripteurs hérités étrangers fermés, standard uniquement en pipes ; processus
non dumpable. Les opérations du helper restent fixes et de confiance ; aucune
goroutine d'extension ni commande utilisateur. Autre architecture ou protection
indisponible refuse, aucun mode dégradé sensible.

Recette attendue : lecture/écriture autorisées ; refus hors racine, traversée,
lien sortant, /proc, exécution directe, TCP/UDP/Unix et accès par FD hérité ;
environnement minimal et no_new_privs/dumpable vérifiés. Annulation/expiration
ferme le groupe de l'enfant. Révision changée pendant la vérification refuse un
résultat périmé. L'état métier et la configuration ne sont jamais modifiés par
le contrôle. La réponse décrit `scope:sealed_helper_only` et conserve
`provider_containment:false`, `secret_grants_supported:false`.

Cette primitive prépare le courtier en mémoire ; sa réussite seule ne permet pas
encore une action confidentielle. Go/API/CLI/Admin et docs FR/EN doivent fournir
le même contrat ; une compilation ne prouve pas le résultat de l'enfant.

## Résultat de la tranche scellée et replan d'attente

Le contrat est implémenté dans `trust_sealed.go`, le backend Linux x86-64 et son
refus sur autre architecture, les entrées CLI/API et Admin FR/EN. Le processus
séparé est réellement exécuté par le binaire. Les14 assertions passent, y compris
UDP/Unix, descripteur hérité fermé, environnement minimal, no_new_privs et absence
de dump. L'opération ne confine pas rétroactivement les fournisseurs et ne charge
aucun secret. Sa portée reste le helper fixe, sur thread d'opération verrouillé,
avec seccomp synchronisé ; pas d'extension ni de code arbitraire.

Un refus d'attente a été ajouté après contre-exemple public : la première version
attendait le verrou de l'état avant de lire la politique, observée dans
`locks_lock_inode_wait`, puis rendait un succès après libération. Une lecture
immédiate dédiée utilise maintenant `LOCK_NB` et retourne `workspace_wait`, sans
enfant ni mutation. Les autres opérations conservent leur sérialisation. Le CLI
observe le refus en84 ms dans la fixture ; ce temps n'est pas une garantie pour
toute machine. Preuves `sealed-lock-before.json` et `sealed-lock-after.json` sous
`verification-confinement-20261015`.

Les contrôles couvrent aussi l'expiration avec descendant réellement démarré,
l'annulation, la sauvegarde/réouverture, les entrées invalides et la révision
modifiée pendant le contrôle. La recette navigateur compte13 contrôles FR/EN,
clair/sombre, mobile390/320, avec absence de mutation et limites rendues visibles.
La recette publique accepte désormais `--binary` pour vérifier exactement
l'exécutable construit, et en consigne l'empreinte ; aucun rebuild implicite dans
cette variante. Les anciennes variantes avec compilation restent disponibles.

Le garde de nettoyage a rencontré `ESRCH` lors de la disparition d'un `/proc/pid`
pendant lecture ; `ENOENT` seul ne suffisait pas. Les deux disparitions normales
sont maintenant reconnues, permission refusée toujours propagée. Le garde teste
ces branches et vérifie un enfant réellement arrêté. Un zombie est distingué
d'un processus actif, sans prétendre qu'il a déjà été réabsorbé. Les premiers
échecs et la sonde originale restent conservés. Les captures Admin d'origine
sont privées ; les nouvelles capturent le seul réglage concerné et n'exposent
pas les identifiants de fixture.

Une insertion documentaire a échoué sur une mauvaise ancre ; aucun fichier
n'avait changé. La lecture des deux pages a précédé la correction. Ce défaut de
méthode n'est pas attribué au produit. Le contrôle du manifeste reste une preuve
d'intégrité de sources, jamais une acceptation indépendante.

Prochaine action : courtier en mémoire avec canal dédié et référence éphémère,
lié à l'identité, l'action, la révision et la destination. Définir son interface
CLI vers le serveur persistant avant code : le CLI local est un processus court
et ne peut pas promettre la conservation d'une valeur dans sa mémoire après
sortie. Vérifier la reprise fermée, la consommation unique et les pannes avant
de permettre la moindre action sensible. R13/R14 complets et le RETEX restent
ouverts.

## Replan — destination du CLI avant le courtier

Contre-exemple `verification-confinement-20261015/trust-remote-before.json` :
`trust show --server http://127.0.0.1:1` retourne code0 et la vue locale malgré
le serveur inaccessible. Le parseur accepte l'option mais seule `version` la
consomme. Propriétaire Codex ; aucune donnée réelle utilisée dans la fixture.

Deux voies : refuser entièrement le mode serveur pour `trust`, ou fournir le
client local authentifié nécessaire au futur courtier persistant. Retenir la
seconde sans introduire le courtier dans cette correction. `--server` explicite
sélectionne exclusivement le serveur ; aucune ouverture ni migration locale,
aucun repli local sur erreur. Sans cette option, le fonctionnement local reste.

Contrat de recette avant code : origine HTTP avec adresse IP de boucle locale,
port explicite et chemin vide ou `/` uniquement ; aucun identifiant dans l'URL,
proxy, résolution DNS ni redirection. `SWARM_WEB_SESSION` fournit la session
opérateur dans Cookie/CSRF, `SWARM_TRUST_TOKEN` le Bearer métier existant.
Les commandes passent par l'API existante avec validation et politiques serveur.
Les erreurs conservent leur code ; une réponse non JSON ou vide est un refus.
Les corps d'erreur non structurés et URL sensibles ne sont pas réimprimés.
L'annulation coupe le transport, sans prétendre annuler un effet déjà engagé.
Pas de relance automatique ni de nouveau délai métier côté client ; les durées
serveur restent autoritatives. Cette limite d'attente sera explicitement décrite.

Vérifier GET et POST réels avec session/CSRF/Bearer, mutation seulement dans la
fixture serveur, racine locale absente, mauvais jetons, mauvais schémas d'URL,
redirection sans fuite, refus serveur, contenu invalide, annulation et parcours
local historique. Aucun secret de courtier n'est chargé par cette tranche.

Les premiers contrôles gardent leurs échecs. Le test initial contenait des imports
anticipés inutilisés : compilation refusée, aucun comportement testé. Après leur
retrait, le défaut de destination est réellement reproduit. Un témoin d'annulation
ne libérait pas son serveur après échec ; groupe propriétaire arrêté et original
conservé. Le client envoyait aussi un corps inutile sur GET : retrait, garde
explicite et nettoyage borné du témoin. L'annulation du client était déjà observée,
pas celle du serveur ; ne pas les confondre.

Le premier branchement serveur précédait l'ouverture du Store mais suivait encore
`EvalSymlinks(root)` : les tests à racine absente et la recette publique échouaient.
La décision de destination est déplacée avant toute résolution locale. Les erreurs
originales restent dans `trust-remote-final2-race.out` et le diagnostic privé de
recette. Prévention : couvrir le point d'entrée complet, avec racine absente, et
non le seul client HTTP. Une première phrase de progression laissait entendre des
contrôles passés avant leur résultat ; elle a été rectifiée immédiatement.

Résultat vérifié : `trust_remote.go` relie les opérations existantes à l'API
locale authentifiée ; la sélection explicite précède toute résolution de racine.
32 cas Go race (8 principaux,24 sous-cas), sept contrôles de recette publique,
build, vet, i18n,12 contextes et contrôle nommé du manifeste1398 sources passent.
La recette vérifie notamment le helper scellé réel, l'accord exact, un seul effet
simulé puis le refus après arrêt du serveur. Le nouveau CLI lit aussi l'aperçu
existant : deux demandes/un effet, SHA métier inchangé, sans redémarrage serveur.
Preuve `verification-confinement-20261015/trust-remote-verification.json`.
La revue reste une auto-revue ; ni courtier livré, ni acceptation globale.

Le binaire client est `bin/swarm-actions-confiance-remote` ; le serveur d'aperçu
reste le binaire scellé antérieur. Ce partage est volontaire : protocole API
inchangé, aucune activation ni nouvelle politique. Le client ne résout pas les
noms DNS, ne passe pas par un proxy et ne suit pas de redirection. Aucun plafond
ou délai métier supplémentaire. La prochaine tranche peut désormais définir le
chargement éphémère dans un serveur réellement persistant ; les commandes locales
de chargement devront refuser plutôt que promettre une mémoire après leur sortie.

## Tranche retenue — octroi éphémère au helper fixe

Le client persistant est vérifié ; le courtier reste absent. Comparaison :
intégrer des secrets aux fournisseurs existants laisse des accès non confinés ;
un helper fixe sans réseau permet de vérifier le canal avant cette extension.
Retenir ce dernier, avec destination `local-attestation` et connecteur
`sealed-attestation` exposés comme capacité limitée. Aucun règlement n'utilisera
ce secret et les classifications sensibles resteront refusées.

Contrat avant code : `secret_broker` persiste enabled, material_ttl_seconds300,
grant_ttl_seconds60, max_material_bytes4096 et max_entries16, défaut inactif.
Tous les champs sont explicites, strictement positifs même inactifs ; octets
bornés par le transport HTTP1Mio existant, durées représentables et octroi pas
plus long que la matière. Bornes exposées et édition Admin FR/EN. Omission
conserve ; aperçu sans chargement ni lancement. Consommation mémoire distincte.

`secret-load` exige serveur, opérateur/CSRF et adminI3. Octets bruts sur endpoint
dédié uniquement, jamais JSON métier, arguments, environnement fournisseur ou
original/preuve. Serveur non dumpable avant lecture, aucun fichier de matière.
Une référence opaque ne peut alimenter qu'un octroi. `secret-grant` exige rôle
effect/I2, action approuvée/digest exact, mandat actif, principal/session/révision,
connecteur et destination fixes. Expiration bornée aussi par session et mandat.
Métadonnées durables sans matière ; arrêt perd la matière et refuse l'ancien
octroi. Pas de rechargement sous le même identifiant.

`secret-consume` réserve durablement l'usage avant tout envoi, puis lance le seul
helper hors verrou métier. Celui-ci atteste les14 gardes avant réception sur pipe
dédié et calcule uniquement une attestation HMAC du contexte exact. Aucun chemin,
code, URL ou message arbitraire. Secret retiré/effacé après tentative, même refus
ou panne ; pas de rejeu automatique. Résultat refusé si révision/identité/mandat
ont changé. Un état consuming après panne reste fermé ; nouvelle matière et
nouvel octroi nécessaires, aucun effet externe à rejouer. `secret-revoke` adminI3
retire la matière ; état durable révoqué. `secret-status` ne révèle que métadonnées
et consommation. Perte de réponse après succès se lit par statut, sans nouvel usage.

Recette : configuration stricte/omission/persistance ; refus sans mutation ni
marqueur brut ; mauvais rôle/sujet/session/digest/destination/révision, expiration,
révocation, arrêt et concurrence ; helper réel avec marqueur absent des fichiers,
preuves/export/arguments/environnement ; un seul succès et zéro effet métier.
Une attestation n'est ni un coffre général ni l'activation d'un fournisseur.
Les limites swap et administrateur système restent hors qualification : le
[noyau](https://docs.kernel.org/admin-guide/LSM/Yama.html) distingue accès même UID,
non-dumpable et privilèges de traçage ; aucune modification des sysctls prévue.

La préparation est un échange distinct, avant toute matière : contrôle réel du
helper, protection mémoire du serveur, puis confirmation positive. Les clients
CLI et web doivent contrôler schema_version1, ready:true, protection mémoire vraie,
révision positive et capacité représentable avant lecture/transfert. Le CLI brut
exige stdin ; aucune valeur dans les arguments ou l'environnement fournisseur.
La restriction mémoire du serveur reste jusqu'à l'arrêt, même politique désactivée.

Originaux nouveaux : `secret-broker-before-race.out` confirme le bloc de politique
absent. `secret-broker-policy-first-race.out` refuse une fixture admin seule qui
violait la séparation existante ; fixture corrigée sans changer cette règle.
La première recette navigateur passait l'argument de waitForFunction comme options
Puppeteer : délai12s, aucun défaut produit déduit ; argument transmis à sa vraie
position puis quinze contrôles passés. Diagnostics privés et premiers reçus gardés.

La revue a ensuite reproduit un défaut réel du nouveau canal : les clients se
contentaient de HTTP200/JSON pour la préparation. `secret-ready-before.json` et
`secret-broker-ready-browser-before/ready-refusal-fr.json` montrent des octets de
fixture envoyés malgré ready:false. Les gardes positives sont ajoutées en Go/CLI
et JS avant transfert. Aucun secret réel n'a été chargé. Ce critère reste ouvert
tant que les reproductions après correction et la recette finale n'ont pas passé.
Ne pas confondre les35 puis48 cas Go de tranches antérieures avec la qualification
du binaire final. Le statut expose désormais la disponibilité de matière par
octroi, distincte de ses métadonnées durables ; arrêt/reprise réels déjà observés.

## Résultat vérifié — 15 octobre, 21:54 UTC

Courtier et politiques implémentés en Go/API/CLI/Admin FR/EN, consommateur fixe
séparé et canal dédié. Références éphémères, réservation durable avant transfert,
consommation unique, expiration/révocation/session/révision et reprise fermée.
Les métadonnées et attestations sont durables, la matière n'est jamais dans
l'état, un original, une preuve ou l'export. Le parent du même UID ne peut ouvrir
la mémoire du serveur après préparation. Ces observations sont des fixtures
isolées ; aucun compte externe ou paiement, aucun privilège Linux ajouté.

`secret-ready-after.json` observe un seul appel de préparation, aucun corps brut
après refus et code4. Le navigateur vérifie la même garde en FR/EN. La recette
publique vérifie le HMAC natif attendu, une seule consommation, perte de matière
après redémarrage, reprise par nouvelle matière et révocation sans nouvel usage.
Les529 cas Go significatifs couvrent138 principaux et391 sous-cas en groupes
(252 et278 PASS bruts, helper vide exclu) ; les49 du courtier se recouvrent avec
ces groupes. Ce n'est pas une exécution unique de529. Les154 contrôles navigateur,
sept scénarios historiques et16 contrôles publics du courtier passent.

Les recettes OIDC/step-up/logout ouvraient le premier `details` de la page :
l'ajout du panneau de secrets changeait ce choix, sans ouvrir les champs OIDC.
Elles ciblent maintenant leur champ et son accordéon propriétaire. La capture
Admin a aussi révélé un défaut de preuve : des coordonnées de fenêtre étaient
traitées comme coordonnées de document par Puppeteer, avec hauteur nulle après
intersection. Source installée `cdp/Page.js` lue ; conversion avec scrollX/Y,
capture du périmètre visible et inspection des images effectuées. Captures
incorrectes et diagnostics restent conservés, sans qualification rétroactive.

Prévention commune à ce contrat : confirmation positive avant transfert sensible,
sélecteurs appartenant au composant, géométrie dans le repère attendu et preuves
liées aux sources exactes. Les manifestes intermédiaires ne qualifient pas les
recettes modifiées après leur production ; le manifeste final suit leur gel.
Auto-revue seulement, aucune acceptation indépendante ni diffusion globale.

Reçu consolidé : `verification-confinement-20261015/secret-broker-verification.json`.
Binaire `bin/swarm-actions-confiance-secrets`, SHA256
`8b093f628b5cac89db6efe13700d95ecade94de56e815f2230aadf54a0f5d7c6`.
Aperçu actualisé sans activation du courtier ni modification des deux demandes
et de l'effet simulé ; rechargement IAB non observé. R14 reste partiel hors helper
fixe ; revenir maintenant à l'introspection authentifiée des services R13, sans
réutiliser ce helper sans réseau comme un connecteur réseau qualifié.

## Reprise R14 — substitution de l’image du helper,16octobre04:49UTC

Résultat attendu : le helper fixe consomme la matière seulement sous les contrôles OS réels. `verification-sensitive-20261016/image-before/image-replacement.json` reproduit via CLI distant/API, serveur copié en racine isolée puis chemin remplacé atomiquement : image substituée exécutée, matière factice capturée, attestation fausse, état consumed. Aucun secret réel/compte externe ni binaire du projet modifié. Cause : os.Executable puis exec par chemin, readiness JSON forgeable par image substituée. Propriétaire Codex.

Comparer hash du chemin juste avant départ (reste une course et confiance au chemin) et exécution par descripteur de l’image serveur en cours. Choix : image courante /proc/self/exe ouverte, FD transmis après les FD de fixture, exec via ce FD ; fermeture parent et enfant contrôlée. Corriger le contrat commun des helpers scellé/secret/laboratoire/diagnostic, sans exception de mission. Le collecteur observe utilise déjà ce mécanisme. Rejouer la reproduction exacte et régressions de ces consommateurs, annulation/reprise et recette publique. Ne pas prétendre à intégrité contre root, écriture en place du binaire ou fournisseur général.

### Reproduction après correction et régressions

image-after/image-replacement.json observe substituted_image_executed:false, fixture_material_captured:false, real_attestation_matches:true, puis quatorze contrôles scellés passent encore après remplacement du chemin. Le cas exact qui échouait passe. Image-public/remote.json couvre les16contrôles publics du courtier, reprise/perte matière/révocation incluses.

88cas Go significatifs sous race passent ; les deux fonctions de fixture parent vides du laboratoire et du scellé sont exclues des90 PASS bruts. TestKernelCheckParentDeathStopsHelper reste bien compté : le suffixe Helper seul ne permet pas d’identifier une fixture vide. Aucune source Go/ressource compilée modifiée pendant exécution ; le snapshot large avait inclus la future recette publique, enrichie avant son lancement. Ce delta unique apparaît explicitement dans les reçus, sans prétendre au gel complet de cette liste. La reproduction historique utilise image-recipe-before.py conservé ; le contrôle supplémentaire après substitution ne change pas son assertion initiale.

La matriceR19 refuse l’ancien reçu sur trust_sealed.go ; v4 lie image-secret-controls-receipt.json aux sources présentes et garde les autres références. Compilation/vet passés ; vérifications navigateur et manifeste final à terminer avant bilan. Deux recherches ont encore utilisé des noms supposés de fichier : erreurs de méthode conservées dans le fil, reprise par chemins retournés, aucun défaut produit déduit.

## Reprise R14 — politique de données au transfert secret

secret-data-before.out reproduit une jonction manquante : action métier admise sous politique internal/local-ledger/swarm/simulated-settlement, puis octroi vers local-attestation/sealed-attestation accepté alors que ces valeurs ne sont pas autorisées. La politique propre au courtier bornait le helper mais ne relisait pas les listes de données. Originaux/test avant conservés, propriétaire Codex.

Comparer deux politiques indépendantes avec priorité implicite et admission commune des flux contrôlés. Choix : quand data_policy est active, la matière secrète a intrinsèquement classification restricted et exige restricted/local-attestation/swarm/sealed-attestation dans ses listes. Les règles de ressources de l’action métier restent relues séparément ; aucun client ne classe/déclasse la matière. Les valeurs décrivent une capacité technique fermée, pas une autorisation implicite ; Admin/API/CLI utilisent les listes persistées existantes, sans nouveau champ/limite. Quand data_policy est inactive, le courtier historique reste explicitement dans sa portée fixe.

Garde à la préparation, avant lecture des octets HTTP, au chargement, à l’octroi et à la consommation sous verrou. Nouveau data_context calculé sur l’octroi et lié à un domaine HMAC versionné ; nil garde exactement la préimage historique. Les métadonnées de flux ne contiennent aucune matière. Refus sans original/octet métier/octroi/matière créée, listes indépendantes/claim client/expiration/révocation/reprise testés. Le cas avant peut désormais refuser plus tôt au chargement ; son assertion d’absence de mutation reste présente. Ce raccord n’est ni un nouveau fournisseur général ni l’activation d’une destination externe.

La construction de la future recette publique a échoué sur l’ancre `state=root` alors que le producteur écrit `state = root` ; aucune écriture partielle n’a eu lieu. Lecture ciblée puis remplacement sur l’ancre réelle. Pas de cause produit. La sonde du processus possédé a aussi montré que les tests tournaient déjà alors que le journal était encore vide ; ne pas inférer compilation/blocage de la seule absence de sortie.

### Raccord vérifié en cours de consolidation

secret-data-race.out passe290cas significatifs (75principaux/215sous-cas), une seule série race, fonction de fixture scellée vide exclue. Entrées Go/embarquées inchangées, puis seuls textes UI/contrats/matrice actualisés. Recette publique active17contrôles et legacy16 passent sur le même binaire ; le HMAC natif lie data_context sous domaine versionné. Retrait de destination refuse préparation/chargement sans mutation.17contrôles navigateur vérifient aussi l’attestation native ; une recette complémentaire2contrôles relit les cinq listes exactes dans Admin FR/EN/mobile après redémarrage, sans changement d’état. L’interface n’avait pas changé ses contrôles, seuls les textes explicitaient le nouveau tuple.

La réouverture explicite des listes étendues a été ajoutée comme preuve complémentaire après le premier gel final ; elle ne modifie ni Go ni ressources compilées. Conserver les reçus initiaux et intégrer ce seul harness au manifeste final, sans relancer les campagnes inchangées. Le refus HTTP porte sur la lecture applicative du corps ; il ne prétend pas que le transport TCP n’a reçu aucun octet. L’absence de transfert brut après refus est assurée par la préparation positive obligatoire des clients et sa garde déjà vérifiée.
