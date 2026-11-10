# Actions de confiance — repartir du code existant

Le code est présent et réutilisable. La recette publique vient d’être relancée
le 15 octobre 2026 sur les sources existantes : les sept scénarios passent,
avec de véritables commandes et interactions navigateur en environnement isolé.
Le pilote simule les effets métier ; aucun paiement réel n’est exécuté.

## Ce qui fonctionne dans le pilote vérifié

- Définir et révoquer un mandat, vérifier l’identité et les contraintes Biba.
- Présenter une demande et approuver son contenu exact.
- Refuser une identité invalide, une approbation incorrecte ou une autorité périmée.
- Engager un effet simulé une seule fois et retrouver le résultat après coupure.
- Prévisualiser et enregistrer la configuration, conserver les preuves et vérifier leur intégrité.
- Utiliser l’interface en français et en anglais, dans les deux thèmes et sur mobile.

Ce résultat porte sur le parcours testé, pas sur toutes les extensions promises.
Les 126 tests Go et les contrôles antérieurs sont conservés séparément.

## État actuel des extensions

| Fonction | Situation pour ce programme |
| --- | --- |
| Identités d’entreprise | Cycle de vie, OIDC, services, délégations, step-up, déconnexion, contexte, groupes et introspection implémentés et vérifiés localement. Révocation distante bornée par la fenêtre configurée. |
| Données et secrets | Admission exacte et courtier éphémère au helper fixe vérifiés. Fournisseurs sensibles, swap et traçage privilégié hors qualification. |
| Processus métier | Définitions et dossiers versionnés, validations/exception, API/CLI/UX et build séparé vérifiés localement. |
| Preuves avancées | Registre/signatures publiques, témoin séparé et archives non destructives vérifiés. Ni temps tiers ni stockage immuable contre le même UID. |
| Épreuves et connecteurs | Laboratoire scellé vérifié. SPI et accès simulé distinct implémentés et vérifiés localement. R19 fournit une matrice contextualisée vérifiée localement, sans conformité globale. |
| eBPF | Réception et backend fixes, lanceur séparé sur fixture possédée et diagnostic Admin/API/CLI vérifiés. Le parcours public observe available:false/code3 après EPERM, sans événement synthétique. Raccord aux tentatives métier et qualification noyau/surcoût restent ouverts. |
| Livraison complète | Recette de l’ensemble et RETEX restent à réaliser ; aucune acceptation globale. |

## Reprise

Conserver le pilote existant comme base. Vérifier ses parcours réels, traiter ses
écarts avérés puis avancer fonction par fonction sur les extensions. Le code ne
doit pas être réécrit seulement parce que l’ancien lancement n’a pas abouti.
Aucune preuve, source ni consommation n’est effacée.

Les sources du pilote sont désormais dans le projet principal :
`internal/engine/trust_actions*.go`, `web/trust-actions.*` et les recettes
`tests/actions_confiance_t2*`. La copie de travail précédente reste conservée.
L’ancien lancement est suspendu pendant cette reprise. Son historique de validation
est distinct de l’existence et du fonctionnement du code ; il ne pilote plus
l’explication de l’état produit.

[Recette publique exécutée sur le code existant](verification-launch-20261014/rebaseline-current-code-public-20261015.out).
[État des lieux technique conservé](verification-launch-20261014/rebaseline-initial-inventory-20261015.md).

## Raccordement au projet principal

Le pilote existant est repris dans les sources principales, avec ses tests et
sa documentation FR/EN. Les points d’entrée CLI et API sont raccordés sans
modifier les trois fichiers déjà identiques. L’ancienne mission reste suspendue.

Plan borné : vérifier les 126 cas du pilote avec race dans ces sources, exécuter
la recette publique CLI/navigateur puis les vérifications statiques et de
traduction. Les sorties doivent contenir les cas nommés, les refus sans mutation,
la reprise exacte et les interactions FR/EN/thèmes/mobile. Aucun de ces résultats
ne sera déclaré obtenu avant son exécution sur les sources principales.
Les extensions restantes conservent leur périmètre ; pas de nouvelle politique
ni de changement de l’autorisation native pendant ce raccordement.

### Résultat du raccordement

Dans le projet principal, les 126 cas Go avec race (33 principaux, 93 sous-cas),
les sept scénarios publics, les 21 contrôles navigateur et les traductions
passent ; aucun diagnostic navigateur inattendu. L’analyse statique et les
douze contextes de méthodes passent. Le binaire dédié est
`bin/swarm-actions-confiance-pilot`. Un aperçu local isolé affiche deux demandes
dont un règlement simulé confirmé. L’onglet a été chargé pendant la tranche de cycle de vie ; son ancien état « en attente » est dépassé. Les autres serveurs ne sont pas
redémarrés et l’ancienne mission reste suspendue.

[Preuves du raccordement](verification-launch-20261014/canonical-pilot-integration-20261015/verification.json).
Prochaine tranche : identités d’entreprise, en reprenant les contrats existants
et avec une recette isolée avant toute activation externe.

### Cycle de vie des identités

Une première partie supplémentaire est vérifiée : suspension/réactivation
locale via Admin, persistée et contrôlée avant les rejeux. Les 139 cas Go avec
race et 23 vérifications navigateur passent ; la demande est refusée pendant
la suspension, puis admise après réactivation avec un nouveau mandat.
La connexion OIDC a ensuite été implémentée et vérifiée sur un émetteur isolé ; elle ne qualifie pas les autres fonctions d’identité d’entreprise.
[Résultat détaillé](identites-entreprise.md).

### Connexion humaine OIDC

Authorization Code + PKCE S256 aboutit à une session métier opaque avec mapping
local explicite, suspension et révocation. Admin expose l’émetteur, le client,
l’adresse de retour, les correspondances et trois durées persistantes. L’aperçu
ne contacte pas le fournisseur. L’API et le CLI partagent la même validation.

37 cas OIDC avec race passent, en complément des 139 cas du pilote conservés et
des sept recettes publiques. Les 14 contrôles navigateur OIDC FR/EN, clair/sombre
sur mobile vérifient aussi le refus traduit, la demande créée, son affichage après
actualisation et la déconnexion. Aucun diagnostic inattendu. Le binaire dédié
est `bin/swarm-actions-confiance-oidc` ; le serveur d’aperçu à la même adresse
conserve exactement son état métier (deux demandes et un effet simulé). OIDC reste
désactivé dans cette démonstration sans fournisseur réel. L’API et le HTML de ce
serveur ont été observés après remplacement ; le rechargement de l’onglet IAB
par l’utilisateur n’est pas présumé.

[Preuves OIDC](verification-identites-20261015/oidc-verification.json).
Suite après la connexion humaine : services `private_key_jwt` (désormais vérifiés ci-dessous), délégations et step-up,
puis les autres fonctions du programme et le RETEX. Aucun lot historique n’est
accepté automatiquement par ces résultats locaux.

### Identités de services

La connexion `private_key_jwt` est maintenant implémentée et vérifiée avec un
émetteur isolé. Le service signe son assertion avec sa propre clé ; Swarm ne
configure que les clés publiques, contrôle les deux JWT distincts et retourne
une session métier opaque. Les assertions rejouées, les ID tokens humains, les
clés privées même en configuration désactivée et les autorités périmées sont
refusés. Les trois durées et les mappings sont persistants et éditables dans
Admin FR/EN ; l’aperçu ne contacte aucun émetteur.

230 cas Go avec race passent (56 principaux, 174 sous-cas), dont 54 pour les
services. Les sept scénarios publics/23 contrôles navigateur du pilote, les
14 contrôles OIDC humain et 12 contrôles services passent. Ces derniers vont
jusqu’à l’effet métier simulé via CLI/API puis son affichage, la réouverture des
clés publiques et durées et la déconnexion. Aucun diagnostic inattendu.

L’aperçu utilise `bin/swarm-actions-confiance-workloads` et conserve ses deux
demandes, son effet et ses octets métier. Ses connexions OIDC/services restent
désactivées sans fournisseur réel. API et HTML observés après remplacement ;
rechargement de l’onglet IAB non présumé.
[Preuves services](verification-identites-20261015/workload-verification.json).
Les délégations, le step-up, les notifications de révocation fournisseur et les
autres extensions du programme restent ouverts, puis la recette globale/RETEX.


## Délégations administrées — résultat du 15 octobre

La chaîne parent-enfant est maintenant persistée et vérifiée à la demande,
à l’accord et avant l’effet. Chaque mandat enfant est émis par Admin I3,
lié au digest parent et limité à son autorité. La révocation d’un ancêtre
bloque ses descendants ; l’effet déjà engagé reste conservé et réconciliable.
Admin persiste la profondeur ; 0 désactive la fonction. L’expiration omise est
plafonnée par celle du parent. Les anciens digests racines sont conservés.

264 cas Go race (66 principaux, 198 sous-cas) passent, dont 34 délégations.
Sept scénarios publics et 23 contrôles navigateur du pilote, 14 OIDC humain,
12 services et dix délégations passent sans diagnostic inattendu. La dernière
recette vérifie le parcours CLI/API/UX, FR/EN, thèmes et mobile, le plafond
exact au-delà de 2^53, le refus d’amplification et la révocation ascendante.
I18n, vet, compilation, douze contextes et manifeste passent.

L’aperçu sert le binaire dédié aux délégations et conserve ses deux demandes
et son effet simulé, avec état métier identique. OIDC, services et délégations
restent désactivés dans cette démonstration sans fournisseur réel. API/HTML
vérifiés ; rechargement IAB non observé à cause d’un délai CDP. Les recettes
navigateur isolées sont vérifiées séparément.
[Preuves délégations](verification-identites-20261015/delegation-verification.json).
Le step-up, la révocation fournisseur, les autres lots et le RETEX restent
ouverts ; cette tranche ne qualifie pas la diffusion globale.


## Approbations avec authentification renforcée — résultat du 15 octobre

Une politique Admin active l’exigence OIDC pour le contenu exact de l’action.
Elle persiste les ACR convenus, l’âge maximal et la validité de la preuve.
L’âge 0 demande une nouvelle authentification ; un champ vide est refusé.
L’identité, l’action, le mandat et la révision sont liés avant la reconnexion.
Le moteur vérifie auth_time/ACR signés, PKCE/nonce et sujet inchangé. L’accord
consomme la preuve une fois sous verrou. Session faible, jeton local et service
mappé à l’approbateur ne contournent pas la règle. Les listes nulles sont
refusées en Go comme en JSON avant écriture ; la configuration reste lisible.

Les 37 gardes Go race finales passent. Avec 264 résultats pertinents réutilisés,
301 cas distincts sont couverts ; la série commune précédente de 298 cas et les
37 gardes finales sont des exécutions séparées. Sept scénarios publics et
79 contrôles navigateur passent : 23 pilote,14 OIDC humain,12 services,
10 délégations et20 step-up. Aucun diagnostic inattendu ; refus volontaires
attribués. La recette renforce une connexion pour une action, refuse une autre
action, approuve le contenu exact et observe un seul effet puis la déconnexion.
Les captures mobile FR clair/EN sombre ont été inspectées.

Documentation FR/EN, i18n, vet, compilation canonique et douze contextes passent.
L’aperçu garde deux demandes et un effet simulé, avec état métier identique ;
les connexions fédérées, délégations et le step-up y restent désactivés sans
fournisseur réel. API/HTML vérifiés ; rechargement de l’onglet IAB non observé.
[Preuves step-up](verification-identites-20261015/step-up-verification.json).
La révocation initiée par le fournisseur, la qualification globale R13, les autres
lots et le RETEX restent ouverts. Les changements Admin ne bénéficient pas de
ce step-up d’approbation ; leurs règles opérateur existantes restent applicables.


## Déconnexion fournisseur humaine — résultat local du 15 octobre

L’événement OIDC signé retire les sessions correspondant à issuer/sub/sid,
y compris les sessions d’authentification renforcée. Autres personnes, services,
actions et effets restent conservés. Une connexion déjà en cours est bloquée
si elle correspond à une cible révoquée ; une nouvelle connexion reste permise.
L’anti-rejeu persiste après redémarrage et reconfiguration, au moins jusqu’à
l’expiration du jeton signé. Admin règle fraîcheur et conservation, affiche
l’adresse de retour et sépare la consommation. Zéro conserve sans purge.
L’actualisation de l’interface retire l’identité inactive avec un message traduit.

Les contre-exemples sont conservés : valeurs sid/sub/azp vides, connexion en vol
recréant la session et type MIME valide refusé. Les gardes passent après
correction. Les trois séries Go race passent343,123 et9 cas ; leur union couvre
353 cas distincts, sans prétendre à une exécution unique de353 cas. Les parcours
publics du binaire final couvrent sept scénarios et97 contrôles navigateur :
23 pilote,14 OIDC,12 services,10 délégations,20 step-up et18 déconnexion.
Aucun diagnostic inattendu ; les refus volontaires sont attribués. Les captures
Admin EN sombre320px et résultat FR clair390px ont été inspectées.

L’aperçu reste une démonstration sans fournisseur configuré ni connexion fédérée
activée ; deux demandes et un effet sont conservés. Les recettes utilisent des
émetteurs isolés, sans paiement, compte externe ou déploiement. Cette déconnexion
humaine ne révoque pas les access tokens des services chez leur émetteur.
[Preuves déconnexion fournisseur](verification-identites-20261015/oidc-logout-verification.json).
Avant de qualifier R13, confronter les sources à identity-context-v1 et aux
critères de mapping groupes/rôles. Les autres lots et le RETEX restent ouverts.

### Contexte commun d’identité — vérification locale du 15 octobre

Le contexte prévu par R01/R13 est produit en Go/API/CLI, utilisé par les nouvelles
preuves et consultable après connexion dans l’interface. Il distingue les
identités locales, humaines et de service ; les rôles et l’intégrité restent
locaux. Les anciennes preuves et sessions sont conservées, sans métadonnées
inventées. Le mapping configurable des groupes reste à réaliser.

La sonde d’expiration après attente d’un verrou est négative après correction :
refus sans mutation en CLI et API. Les trois groupes communs369 puis29 cas
ciblés finaux, avec recouvrement, passent ; sept scénarios publics et99 contrôles
navigateur passent sur le binaire final. La première série interrompue et les
échecs de fixtures restent conservés et attribués dans
[la reprise du contexte](identity-context-replan.md). Ce résultat ne clôt pas R13.
Preuve : [identity-context-verification.json](verification-identites-20261015/identity-context-verification.json).

### Mapping local des groupes — résultat courant

Le mapping configurable est maintenant implémenté et vérifié pour personnes et
services. Les groupes signés restreignent les rôles locaux ; ils ne créent ni
identité, ni intégrité, ni permission hors plafond. Les nouvelles preuves lient
le digest de règles aux rôles effectifs, y compris pour l’effet d’un service.
Les32 gardes actives et les régressions126/246 passent, puis32 gardes après index.
Sept scénarios publics et111 contrôles navigateur passent sur le binaire final.
Voir [group-mapping-verification.json](verification-identites-20261015/group-mapping-verification.json)
et [la reprise](group-mapping-replan.md). R13 reste à qualifier ; les lots suivants
et le RETEX ne sont pas livrés par cette tranche.

### Admission des données — première tranche T4 vérifiée

Le pilote peut maintenant exiger classification, destination, fournisseur et
outil. La classification provient d'une règle de ressource administrée ; le
demandeur ne peut pas la diminuer. Le contexte fait partie du digest exactement
approuvé. Une destination non maîtrisée ou une donnée sensible refuse avant
conservation de l'original. Admin FR/EN persiste les listes et règles explicites.
Le registre simulé local est le seul chemin pris en charge ; pas de secret ni de
confinement système déclaré disponible.

Les36 cas Go T4 finaux passent. Les régressions126 pilote et278 identités sont
réutilisées pour leurs branches sans politique de données, inchangées par l'ajout
des règles de ressources :440 cas distincts en groupes, pas une exécution unique.
Sept scénarios publics et124 contrôles navigateur passent ; la recette publique
ajoute son contrôle de parité i18n. Le dépassement initial de25 secondes reste
un échec conservé. Le nettoyage de son groupe de processus a été corrigé et
vérifié. Cette valeur est un paramètre de recette, aucune limite métier ajoutée.

L'aperçu contient toujours deux demandes et un effet, avec les mêmes octets
métier ; aucun rechargement IAB n'est affirmé. Voir le
[reçu courant](verification-donnees-20261015/data-policy-verification.json),
[la reprise](data-policy-replan.md) et la
[documentation produit](../../actions-confiance-T4.md).

R14 demeure partiel. La prochaine tranche est le courtier de secrets et son
confinement, avant la révocation distante des services R13. Les connexions IA
existantes portent des clés de configuration ; elles ne sont pas un octroi
éphémère par action. Ne pas en importer les clés dans l'état métier ni les
preuves. Processus métier, preuves avancées, connecteurs, eBPF, recette globale
et RETEX restent ouverts.

### Processus local scellé — résultat courant

Le contrôle `trust confinement-check` est maintenant intégré en CLI/API/Admin
FR/EN et réellement exécuté dans un enfant interne. Sa politique persistée
`sealed_execution` reste inactive par défaut, délai affiché15 secondes, 0 refusé.
Les14 assertions couvrent les accès fichiers, exécutable, TCP/UDP/Unix,
descripteur hérité, environnement, no_new_privs et absence de dump. La portée
reste `sealed_helper_only` : les fournisseurs et les secrets sont non qualifiés.

Les45 cas Go significatifs, répartis en groupes avec une fixture de sous-processus
exclue du décompte, passent. Les137 contrôles navigateur comprennent13 nouveaux
contrôles scellés et124 régressions ; sept scénarios publics utilisent le même
binaire et consignent son empreinte. Build, vet, i18n, douze contextes et manifeste
1395 sources passent. La lecture de contrôle refuse maintenant immédiatement un
état occupé, après contre-exemple conservé. Expiration avec descendant réellement
démarré et refus de résultat périmé sont vérifiés. Les disparitions normales de
`/proc` sont distinguées d'une erreur de permission dans le garde de nettoyage.

Voir le [reçu courant](verification-confinement-20261015/sealed-verification.json)
et le [replan secrets/confinement](secrets-confinement-replan.md). L'aperçu conserve
deux demandes, un effet et les mêmes octets métier ; son contrôle reste désactivé.
Le courtier en mémoire est la prochaine tranche, pas une fonction déjà livrée.
R13/R14 complets, extensions suivantes, recette globale et RETEX restent ouverts.

## Accès CLI au serveur persistant — 15 octobre, 21:20 UTC

Le besoin de courtier en mémoire a révélé un prérequis incorrect : `--server`
était ignoré pour `trust`, avec réponse locale possible malgré un serveur absent.
Le CLI choisit maintenant le serveur avant toute résolution de racine, utilise
session/CSRF/Bearer existants et refuse erreur, redirection ou contenu invalide
sans repli local. L'accès sans `--server` reste local. Aucune nouvelle politique
ou limite métier ; annulation du transport distincte de l'annulation d'un effet.

Résultat :32 cas Go race (8 principaux,24 sous-cas), sept contrôles publics,
quatorze assertions OS du helper via le nouveau client, et les contrôles statiques
passent. Le manifeste1398 sources et son test nommé passent. Le client lit aussi
l'aperçu actuel sans le redémarrer : deux demandes/un effet, état métier inchangé.
Preuves et originaux : `verification-confinement-20261015/trust-remote-verification.json`.
Les premiers échecs de fixture, d'annulation et de placement du branchement sont
conservés dans le replan. Auto-revue seulement ; aucun renouvellement fictif des
anciennes recettes navigateur. Binaire client `bin/swarm-actions-confiance-remote`,
SHA256 `8aa54660104bf3eeee22157262c9a3b57e0668db98f46ee7806de488e645674d`.
Le serveur d'aperçu reste la version scellée vérifiée ; aucun rechargement IAB.

La suite reste le courtier éphémère et son consommateur fixe, avec reprise fermée
et absence de secret brut persistant, puis la révocation distante des services.
Ce prérequis ne livre pas les octrois de secrets et ne qualifie pas R14 global.

## Courtier éphémère et consommateur fixe — 15 octobre, 21:54 UTC

Le courtier est implémenté dans le serveur persistant, avec Admin FR/EN, API et
CLI distant. Matière uniquement en mémoire, références opaques, octrois liés à
session/principal/action/digest/révision/mandat, usage unique et reprise fermée.
Le seul consommateur est `sealed-attestation` / `local-attestation` : préparation
positive et protections OS avant transmission sur pipe, HMAC du contexte exact,
aucun secret dans les preuves ou export. Le règlement simulé reste indépendant.
Les fournisseurs et effets sensibles restent non qualifiés.

Preuve `verification-confinement-20261015/secret-broker-verification.json` :529 cas
Go significatifs en groupes (dont49 du courtier, inclus),154 contrôles navigateur,
sept scénarios publics historiques et16 contrôles publics du courtier passent.
Refus de préparation sans envoi, rôle/portée incorrects sans mutation, concurrence,
révocation, redémarrage et perte de matière sont observés. Auto-revue seulement.
Les défauts de préparation détectés avant qualification et ceux des recettes
restent conservés avec leur correction vérifiée dans secrets-confinement-replan.md.

L'aperçu utilise `bin/swarm-actions-confiance-secrets`, SHA256
`8b093f628b5cac89db6efe13700d95ecade94de56e815f2230aadf54a0f5d7c6` ; courtier
inactif par défaut, deux demandes/un effet et octets métier inchangés. Pas de
rechargement IAB affirmé. La prochaine tranche est la révocation distante des
services R13 : authentification/introspection hors verrou et sans jeton brut
persistant. Les autres lots et le RETEX restent ouverts ; Swarm n'est pas terminé.

## 15 octobre, 22:32 UTC — révocation distante des services vérifiée

R13 : introspection RFC7662 authentifiée private_key_jwt propre au serveur, token
fournisseur uniquement en mémoire référencée depuis la connexion validée. Endpoint
exact sur l’origine administrée, sans proxy/redirection/cache ; réseau hors verrou,
revalidation par requête sous verrou pour actions, contexte et octrois. Rôles locaux
conservés. Inactif, erreur, expiration, suspension, révision ou perte mémoire refusent ;
les effets déjà engagés restent intacts. Une révocation reçue invalide les preuves
positives en vol. Statique local des identités mappées refusé comme repli.

Politique persistée/Admin Go/API/CLI, durées/capacité/taille strictement positives,
consommation séparée. Nouvelle clé publique et login après chaque révision ou arrêt.
Docs T2 FR/EN et recette service `--introspection --token-response-bound` livrées.
Le reçu `verification-confinement-20261015/introspection-verification.json` lie
570 cas Go distincts en groupes252/319 (145 principaux,425 sous-cas, helper vide
exclu), dont41 dédiés ;172 contrôles navigateur et sept scénarios publics passent
sur le binaire final. Build/vet/i18n/12 contextes, manifeste1 411 sources et garde
d’horloge passent. Ce bilan n’est ni une exécution unique570 ni une revue indépendante.

Échecs conservés : contrat absent ; preuve positive survivant à une révocation ;
témoin JWT audience chaîne/liste ; témoin timeout restant actif ; régression du
LOCK_NB ajouté aux anciens parcours. Le dernier est corrigé par sérialisation
historique, refus après attente effectivement rejoué. L’hypothèse de lecture OAuth
non bornée est réfutée : `trustOIDCBody` refusait déjà au-delà de1Mio. Pas de correctif
inutile ajouté. Le replan R13 conserve causes, propriétaires et reçus avant/après.

Aperçu isolé actualisé, deux demandes/un effet et octets métier inchangés ; politiques
fédérées et introspection désactivées. Rechargement de l’onglet IAB non observé. R14
reste limité aux chemins contrôlés, aux données public/internal et au helper fixe ;
aucun fournisseur sensible qualifié. Prochaine étape : arrêter la matrice de couverture
R14 puis cadrer T5/processus avec les contrats d’identité et d’admission disponibles,
sans fabriquer l’acceptation des lots historiques. Autres lots et RETEX restent ouverts.

## 16 octobre Paris — première tranche Processus vérifiée

Le code suit maintenant des dossiers distincts des missions de développement.
Définitions FR/EN publiées par I3, versions immuables, création sur version exacte,
participants locaux, transitions et exceptions attribuées, révision/digests exacts,
séparation et admission T4 obligatoires. Les nouvelles preuves portent le contexte
d’identité commun. Rejeu, concurrence et redémarrage sont vérifiés ; aucun paiement
n’est approuvé ni engagé par une transition de dossier.

API `/api/v1/processes`, CLI `process`, Admin avec aperçu/sauvegarde/réouverture et
artefact web construit séparément sont présents. Portée locale public/internal,
chemin exact `local-process-state/swarm/process-workflow`. Les sources et recettes
courantes sont décrites dans `docs/actions-confiance-T5.md` et sa version anglaise.

277 cas Go distincts (71 principaux,206 sous-cas) passent en quatre groupes sous
race, pas une exécution277. Sept scénarios publics du pilote et61 contrôles
navigateur sur le binaire final (23 pilote,38 Processus) passent, sans diagnostic
inattendu ; les deux auto-validations volontairement refusées sont attribuées.
Les preuves originales et les incidents de méthode sont conservés dans
`verification-processus-20261015` et `processus-replan.md`. Auto-revue uniquement.

L’aperçu isolé est actualisé à partir du binaire Processus ; les deux demandes,
l’unique règlement simulé et les octets métier restent inchangés. Le nouveau
suivi est désactivé par défaut sur cette démonstration ; ses essais utilisent des
racines distinctes. Rechargement IAB non observé. Les effets liés T2, les extensions
de version, les connecteurs/preuves durables, eBPF, recette globale et RETEX restent
à poursuivre ; cette tranche ne termine ni le programme ni sa qualification.

## 16 octobre Paris — signature publique et cycle de clés R16

La première tranche R16 ajoute un registre public, clés Ed25519 éphémères en
mémoire protégée, activation exacte par une autre identité administrateur I3,
rotation/retrait et révocation d’urgence attribuée. La révocation reste possible
quand la signature est désactivée. L’ancre publique schéma3 lie clé/révisions,
séquence/empreinte et export v2 complet ; vérification publique avec pin acquis
séparément, possible sans ouvrir de racine locale. Historique HMAC conservé.

Go/API/CLI et Admin FR/EN partagent la politique enabled/key_ttl_seconds, défaut
inactif86400secondes, valeurs positives et bornes affichées. Arrêt, expiration ou
nouvelle configuration font perdre la capacité de signer ; nouvelle rotation
nécessaire. Statut public et disponibilité privée sont distincts et visibles.
Aucune matière privée dans état/originaux/export/argument/environnement. La
protection mémoire ne qualifie ni swap, traçage privilégié ni fournisseur arbitraire.

Vérifications conservées dans `verification-preuves-20261016` :22 cas R16 sous
race, plus un cas final ciblé de huit préparations concurrentes, et326 régressions
pertinentes réutilisées sous préconditions et diff exact vérifiés.349 cas distincts,
88 principaux/261 sous-cas, pas une exécution349. Onze contrôles publics CLI/API,
jonction réelle avec les trois preuves d’un dossier T5, sept scénarios historiques
pilote et55 contrôles navigateur sur le binaire final (32 preuves,23 pilote),
sans diagnostic inattendu. Build/vet/i18n/12contextes/diff et manifeste1436 sources
avec contrôle nommé passent ; dernier contrôle après ajout du cas de concurrence.
Auto-revue seulement, aucune acceptation indépendante ou globale.

Aperçu actualisé, deux demandes/un effet et octets métier inchangés, signature
publique inactive. L’erreur d’import du script de contrôle est traitée en retrouvant
le processus existant, sans lancement doublon ; identité et effet HTTP vérifiés.
IAB non rechargé. Les défauts du pin contradictoire et du retrait sous politique
inactive, les erreurs de lecteurs de recette et les captures précédentes restent
conservés avec leurs gardes. Aucun refus n’est assimilé à un succès.

La signature publique seule ne termine pas R16 : ancrage témoin hors racine éditable
et archivage/rétention restent ouverts. Un ancien pin hors ligne n’apprend pas la
révocation ; ne pas présenter le pin du fichier à vérifier comme sa propre autorité.
Puis laboratoire isolé R17, second connecteur R18 et matrice d’applicabilité R19,
eBPF, recette globale et RETEX. Swarm reste non qualifié pour diffusion.

## 16 octobre — témoin et archivage non destructif R16

Le vérificateur accepte un témoin indépendant explicite (ancre + pin séparés),
refuse export authentique plus ancien ou préfixe divergent, et indique sans
ambiguïté quand aucun témoin n’est fourni. Acquisition/relecture dans un répertoire
de fixture distinct vérifiées ; ceci ne crée ni service tiers ni immutabilité OS.

Archives Go/API/CLI/UX FR/EN : aperçu sans mutation, copie de préfixe signé et
originaux exacts, registre/reçus attribués et fichiers adressés par empreinte sans
remplacement. La durée minimale vient de evidence_retention_days Admin existant ;
instant/durée figés à création, réduction ultérieure sans raccourcissement. Une
levée I3 explicite refuse avant échéance, passe à frontière puis conserve copie,
originaux et preuves. Ni suppression physique ni purge automatique ajoutées.
Fichier absent, altéré ou lien symbolique refuse même au rejeu ; reprise d’une
publication orpheline seulement sur octets exacts. Réouverture et concurrence
vérifiées. Pin d’archive historique et état de révocation courant sont distincts.

La série native finale passe47 cas R16 ; les9 cas initiaux d’archive et33 témoins/
signatures antérieurs se recouvrent et ne sont pas additionnés. Les326 régressions
Go précédentes restent historiques aux entrées documentées ; aucune exécution
globale de373 cas n’est annoncée. Recettes finales conservées dans
verification-preuves-20261016/archive-verification.json :18 contrôles CLI/API,
42 navigateur propres et sept scénarios pilote avec leurs contrôles navigateur.
Les captures intermédiaires et défauts de titre/bouton anglais sont conservés,
gardes visibles ajoutées. Auto-revue seulement ; aucune acceptation indépendante.

Ces capacités locales ne qualifient pas la diffusion globale ni l’isolement des
providers sensibles. Suite autorisée : laboratoire adversarial isolé versionné
R17, SPI/second connecteur simulé R18, applicabilité R19, observation eBPF, recette
et RETEX. Les limites R13/R14 et la mission ancienne suspendue restent conservées.

## 16 octobre — laboratoire R17 fixe et isolé

Le corpus versionné contient cinq scénarios Trust : Biba, accord exact, unicité
d’effet, réponse perdue/réconciliation et original altéré. API/CLI laboratory et
Admin FR/EN, paramètres de durée/concurrence persistés (défaut inactif30s/1),
graine uint64 en chaîne et entrée limitée au corpus/digest/connecteur simulé.
Aucune racine, URL ou commande du demandeur. Tentative durable réservée avant
lancement ; helper fixe par pipes et environnement minimal,14 contrôles réels
Landlock/seccomp, opération confinée sur thread verrouillé, arrêt sur annulation
ou mort du parent. Résultat accepté seulement après revalidation courante.

Les entrées, corpus, politiques, observations et limites persistent ; tentative
identique sans doublon, rejeu explicite lié et déterministe, historique lisible si
corpus ancien, consommation jamais remise à zéro. Un arrêt brutal peut laisser
une fixture temporaire orpheline ; aucun scan/destruction globale ajouté.
Ces simulations ne qualifient ni fournisseur sensible ni observation eBPF.

Vérification :18 cas Go significatifs (19 PASS bruts, helper vide exclu), plus126
T2 et47 signature/témoin/archive R16, soit191 cas distincts dans trois groupes.
Huit contrôles publics, sept scénarios pilote et55 contrôles navigateur sur même
binaire (32laboratoire/23pilote,20 contextes FR/EN×thèmes×5largeurs), sans
diagnostic inattendu ; deux refus de droits attribués. Build/vet/i18n/12contextes
et diff passent ; manifeste final et preuves consolidées doivent être lus dans
verification-laboratoire-20261016/laboratory-verification.json. Pas d’acceptation
indépendante ou globale. Les anciens résultats ne sont pas additionnés.

FAIL original policy-replay conservé : comparaison inadéquate d’ordre de champs
struct/map, CLI confirme JSON égal et octets inchangés, témoin corrigé sans
assouplir le moteur. Erreur de fixture ad hoc (jeton trop court) conservée dans
le diagnostic ; ne pas l’attribuer à une panne produit. Captures finales inspectées.
Aperçu actualisé en gardant deux demandes/un effet et octets métier inchangés,
laboratoire inactif ; IAB non rechargé. Suite R18/R19, T7 et recette globale/RETEX.


## Connecteurs R18 — résultat du 16 octobre, 01:38 UTC

Deux effets simulés distincts partagent maintenant intention protégée, accord
exact, admission, idempotence et lecture de confirmation : règlement et octroi
d'accès. Aucun droit réel n'est changé. Le contrat legacy conserve ses digests.
Le formulaire conserve aussi les montants supérieurs à2^53 exactement ; le refus
d'auto-approbation est vérifié dans les deux langues.

327 cas Go significatifs distincts en quatre groupes (91 principaux/236 sous-cas),
11 contrôles publics connecteurs, sept scénarios pilote et56 contrôles navigateur
passent sur leurs entrées attribuées. Aucun total d'exécution unique327. Le binaire
final ne diffère des entrées Go testées que par les correctifs web vérifiés.
Build/vet/i18n/12 contextes/diff et manifeste1455 sources passent. Reçu courant :
[connector-verification.json](verification-connecteurs-20261016/connector-verification.json).

Aperçu actualisé, deux demandes/un règlement simulé et mêmes octets métier ;
rechargement IAB non observé. Auto-revue, pas d'acceptation globale. Suite : matrice
R19, observation eBPF, recette globale et RETEX ; limites fournisseurs R14 ouvertes.


## R19 — matrice de contrôles et limites, 16 octobre 02:01 UTC

Douze contrôles sélectionnés du catalogue NIST SP800-53 5.2.0, source primaire
figée : neuf partiels, deux non évalués, un hors périmètre. Cinq reçus historiques
et36 entrées pertinentes explicitent portée et limites. CLI sans écriture, carte
FR/EN à lecture explicite, aucun claim de certification/conformité globale.

Dix tests nommés du validateur, treize contrôles publics, sept scénarios pilote
et96 contrôles navigateur (40 matrice/33 connecteurs/23 pilote) passent. Go métier
inchangé depuis R18 ; aucune nouvelle campagne globale annoncée. Build/vet/i18n/
12 contextes/diff et manifeste1465 sources passent. Reçu :
[controls-verification.json](verification-referentiels-20261016/controls-verification.json).

Aperçu actualisé et octets métier préservés ; IAB non rechargé observé. R20 reste
ouvert : la sonde de map vide reçoit EPERM, sans attachement/collecte. Préparation
du collecteur borné selon ebpf-replan.md puis recette globale/RETEX, sans droit
Linux implicite ni déclaration de fin globale.


## R20 — diagnostic public fermé, 16 octobre vers03:43UTC

Diagnostic Admin/API/CLI sur son propre helper, politique persistée et refus actuel available:false/code3 vérifiés. Aucun PID ou commande client, aucun événement synthétique annoncé noyau. Annulation/mort parent, autorité/révision fraîches et refus sans mutation contrôlés. Une perte de focus Admin pendant saisie a été reproduite puis corrigée avec garde ; refus403 antérieur conforme, sans assouplissement moteur.

223 cas Go distincts en trois groupes, six contrôles publics kernel, sept scénarios pilote et55 contrôles navigateur passent dans leurs périmètres, sans qualification globale. Preuve : [check-verification.json](verification-ebpf-20261016/check-verification.json). Manifeste1483 sources et contrôle nommé passent ; aperçu actualisé, octets métier préservés et politique noyau inactive. R20 noyau positif/attribution métier/surcoût reste ouvert ; préparation T8 dans [qualification-globale-replan.md](qualification-globale-replan.md).


## R20 / T8 — source enregistrée et campagne commune, 16 octobre vers04:02UTC

La source interne dérive son empreinte du registre agent/travail/tâche/tentative, refuse pause/arrêt/remplacement et relit après I/O. Elle est vérifiée avec le producteur réel et un processus de fixture réellement lancé/arrêté. Aucun collecteur métier automatiquement attaché : R20 reste partiel. [Preuve](verification-ebpf-20261016/attempt-source-verification.json) :75 cas Go en une série sous race, dont38 nouveaux et37 réceptions rejouées ; manifeste1485 sources et contrôle nommé passent.

[Campagne fonctionnelle commune](verification-globale-20261016/public-campaign.json) : même binaire pour connecteurs11 contrôles, preuves18, laboratoire8, courtier16, pilote7 scénarios dont23 navigateur, Processus4 scénarios dont38 navigateur et matrice13 contrôles. Ces unités ne sont pas additionnées en un score. Une erreur du conducteur après succès connecteur a été reprise sans nouveau lancement ; les autres étapes ont été exécutées. Le conducteur durci relit désormais tous les résultats sans les rejouer, dans un rapport de reprise distinct.

Statut partiel : collecte métier/noyau, mesure de surcoût, revue indépendante applicable et RETEX final ouverts. L'aperçu reste identifié sur le binaire diagnostic précédent ; aucun redémarrage ni rechargement IAB effectué pour la factory interne.

## R20 — collecteur explicite raccordé, 16 octobre vers04:36UTC

L’observation explicite d’une session enregistrée est implémentée : I3, source du registre, helper fixe par image ouverte, contrôle Unix/fraîcheur et arrêt possédé. Politique kernel_observation persistée/éditable Admin et API/CLI, paramètres visibles, aucune cible PID/commande/digest client. Pertes noyau et événements omis restent distincts ; aucune couverture complète ou journal durable ajouté.

[Reçu courant](verification-ebpf-20261016/observe-verification.json) :297 cas Go distincts sous race en groupes141/155/1 (80 principaux/217 sous-cas), sept scénarios publics pilote et55 contrôles navigateur (33 observation/22 pilote) sur le binaire final. Zéro diagnostic inattendu. Le parcours distant du collecteur observe réellement available:false/code3 et l’attribution à une fixture de registre possédée, sans événement synthétique ni mutation métier. Réponses navigateur de collecte explicitement interceptées comme fixtures ; elles ne prouvent aucun événement noyau.

Build/vet/i18n/douze contextes/diff et manifeste1500 sources, contrôle nommé sous race, passent. Matrice R19v3 lie le nouveau reçu des155 régressions partagées ; ancien reçu périmé refusé et conservé. Le binaire final ne change que la matrice/références après les contrôles Go ; limites d’attribution du hash de l’ancien binaire public explicitées dans le reçu, sans capture préalable inventée.

Aperçu actualisé, PID3812517/démarrage5974314, bin/swarm-actions-confiance-observe SHA25636b59418b5e17a6f99823e1510f8966c4017a6fa74dd869bedde1cb43fd11302. Deux demandes/un règlement, octets métier historiques préservés ; observation inactive. API/HTML/JS/matrice observés, aucun rechargement IAB affirmé. R20 noyau positif/exclusion réelle/pertes/reprise/surcoût, R14 cible sensible, revue globale applicable et RETEX final restent ouverts.

## R14 — identité des helpers fixes,16octobre vers04:57UTC

L’extension sensible a révélé un défaut réel de la primitive existante : un serveur de fixture lancé depuis une copie puis son chemin remplacé exécutait un faux helper, qui capturait la matière factice et faisait accepter une fausse attestation. Aucun secret réel ni binaire canonique touché. Originaux image-before et recette avant conservés dans verification-sensitive-20261016.

Correction commune : l’image en cours est ouverte par /proc/self/exe, transmise après les FD de fixture et exécutée par descripteur. Quatre consommateurs corrigés (scellé, secret, laboratoire, diagnostic), sans changement de politique/droit/interface. Les FD ajoutés sont fermés dans l’enfant avant readiness/transfert et côté parent. Le collecteur observe utilisait déjà une image épinglée et reste inchangé.

[Reçu](verification-sensitive-20261016/image-verification.json) :88 cas Go significatifs sous race (36 principaux/52 sous-cas), deux fonctions de fixture vides exclues des90 PASS bruts ;16 contrôles publics et30 navigateur (17 secrets/13 scellé) passent. La reproduction exacte et sa reprise sur le binaire final observent aucune image substituée, aucune capture et le vrai HMAC ;14gardes scellées passent encore après remplacement du chemin. Zéro diagnostic inattendu ; image EN du résultat inspectée.

Build/vet/12contextes/i18n/diff et manifeste1503sources avec contrôle nommé passent. MatriceR19v4 relie un nouveau reçu secrets aux entrées actuelles ; historiques conservés. Le snapshot initial signalait le seul enrichissement de la future recette publique, pas une modification Go/embarquée ; le gel final est intégralement vérifié. Aucun résultat historique modifié.

Aperçu PID3927519/démarrage6103154, bin/swarm-actions-confiance-fixed-image-qualified SHA2568ee6463f481cd1264fd008c137e985a336b014db54572022d9bfffeb3dbb992f. Ancien PID3812517 identifié puis arrêté ; deux demandes/un règlement et octets métier inchangés. Politiques avancées inactives ; API/assets/matrice observés, aucun rechargement IAB affirmé. Cible sensible fournisseur R14, noyau positif R20, revue applicable et RETEX final restent ouverts.

## R14 — remise de matière restreinte sous politique,16octobre05:16UTC

Le courtier relit désormais les listes de données avant tout transfert applicatif et lie un data_context calculé à l’attestation : restricted/local-attestation/swarm/sealed-attestation. Les ressources de l’action restent classées séparément ; aucun déclassement client. Sans politique active, la préimage historique est conservée. Un cas avant reproduit l’octroi non autorisé, puis le refus intervient désormais dès chargement/préparation sans mutation.

[Reçu](verification-sensitive-20261016/secret-data-verification.json) :290cas Go significatifs sous race (75principaux/215sous-cas), une série, fonction de fixture scellée vide exclue.17contrôles publics actifs/16legacy et19navigateur (17parcours+2réouverture/absence débordement) passent, zéro diagnostic inattendu. HMAC natif exact, retrait de destination, reprise sans matière/rejeu, listes Admin FR/EN/mobile persistées et relues. Build/vet/i18n/12contextes/diff et manifeste1507sources/test nommé passent. R19v5 relie les nouveaux reçus effets/secrets ; les historiques ne sont pas réécrits.

Aperçu PID4018272/démarrage6213956, bin/swarm-actions-confiance-secret-data-qualified SHA2567de055bcfff8b030828aab87b7ed8744a1a1cffeee2d32f458098e1105da0e79. Ancien PID3927519 identifié puis arrêté ; deux demandes/un règlement et octets métier préservés, politiques avancées inactives. API/assets/matrice vérifiés, IAB non rechargé observé. Couverture R14 réexaminée selon critères initiaux : contrôle local vérifié, providers non maîtrisés refusés, aucune acceptation globale ou qualification générale ajoutée. R20 positif/surcoût, revue applicable et RETEX restent ouverts.

###16octobre05:34UTC — campagne actuelle et décisions restantes

Même binaire secret-data-qualified SHA2567de055bcfff8b030828aab87b7ed8744a1a1cffeee2d32f458098e1105da0e79 : neuf étapes publiques,104contrôles navigateur d’identité et407cas Go utiles sous race passent, résultats séparés et unités non additionnées. Voir verification-globale-20261016/candidate-current/review-packet.json et docs FR/EN actions-confiance-qualification.md.1510sources du manifeste/test nommé conformes ; aucune modification Go/ressource embarquée. Aperçu inchangé, aucun serveur interrompu ou mission relancée.

Fixture eBPF positive préparée sans collecte, politique relue exacte, garde environnement inchangé code3 vérifiée avant lancement. Demandes utilisateur en attente : environnement Linux autorisé et revue séparée explicitement autorisée ; aucune élévation/délégation implicite. R20 positif, revue R11 et RETEX final R12 restent ouverts. La revue du même contexte n’est pas indépendante. Originaux du consolidateur de chemins conservés puis code0 vérifié.

### 16octobre2026 à07:30UTC — périmètre convenu livré, revue et RETEX

Décision utilisateur conservée : revue Codex en contexte séparé autorisée ; essais eBPF natifs reportés. Le même réviseur rend approve sur le candidat corrigé, après reproduction indépendante F1/F2/F3,14contrôles et contrôle1073entrées/204preuves. Binaire bin/swarm-actions-confiance-proof-exact SHA2565b9ce6f43d0498624d120c70ab72b8c6b3a66785f0d856df8a357115fedfd2c6. Reçus sous verification-globale-20261016/review-corrections : review-packet.json, independent-final-review.json et delivery-verification.json. Neuf étapes publiques sur ce binaire,15contrôles CLI preuve,46navigateur preuve et11gardes client passent.722cas Go utiles dans deux groupes structurés PASS ; second exit Go inconnu après conducteur143, limite explicitement conservée. Rien n’est annoncé comme exécution globale unique ni acceptation métier globale.

RETEX-actions-confiance-20261016.md est final pour ce périmètre. Le contrôle de livraison relit toutes les empreintes du paquet, intactes après revue. AperçuPID505619/démarrage7013049, reçu qualified-preview-process.json écrit dès spawn puis vérifié HTTP ; deux demandes/un règlement simulé, SHA métier d3b01d34c5c750ca91cbe1591ccd1c978ecb3a52a6416ffbbb0d610cfddd48c1 inchangé. Politiques avancées désactivées. Aucun rechargement IAB observé ; ne pas prétendre que l’utilisateur voit déjà la nouvelle page.

R20 reste DEFERRED_BY_USER_NOT_QUALIFIED. Propriétaire de la décision d’environnement : utilisateur ; recette et mise en œuvre après autorité disponible : Codex. Aucun sudo/capabilité/sysctl/VM, sonde inchangée ou compte/paiement implicite. Le conducteur natif et la mission Cursor arrêtés restent préservés ; aucune consommation effacée. Le suivi automatique de cette livraison est clos, aucune reprise privilégiée automatique organisée.
