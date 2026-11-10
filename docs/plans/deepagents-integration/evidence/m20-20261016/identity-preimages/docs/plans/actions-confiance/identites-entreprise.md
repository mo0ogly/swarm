# Identités d’entreprise — R13

Le pilote fonctionne avec des identités locales. La cible complète conserve
Authorization Code avec PKCE pour les personnes et private_key_jwt pour les
services, puis le mapping explicite des droits, la délégation et la révocation.
Le protocole ne donne aucun droit métier par lui-même.

Tranches vérifiables :
1. Cycle de vie : suspension et réactivation explicites, persistées, aperçu sans
   mutation, ancienne révision refusée ; toute opération d’une identité suspendue
   échoue avant mutation, y compris les rejeux et après redémarrage.
2. Fédération OIDC : issuer autorisé, signature, audience, nonce, PKCE, dates et
   rotation des clés vérifiés sur un émetteur isolé ; aucune connexion réelle
   ni création de compte externe pour les recettes.
3. Services, mapping, délégation, séparation et interface FR/EN : droits issus de
   la configuration administrée, révocation et reprise observées via CLI/API/UI.

Les nouvelles règles passent par la configuration Admin/API/CLI/Go commune.
Conserver la compatibilité des identités existantes ; la suspension ne supprime
ni identité, ni action, ni preuve. Toute configuration appliquée périme les mandats
selon le contrat actuel ; une réactivation ne restaure pas une ancienne autorité.
La première tranche seule ne constitue pas la livraison OIDC ou la fin de R13.

Sources primaires examinées :
- [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html).
- [PKCE — RFC 7636](https://datatracker.ietf.org/doc/html/rfc7636).
- [Client Go OpenID Connect](https://github.com/coreos/go-oidc).

Propriétaire : Codex. Les tentatives de l’ancienne mission restent conservées et
la mission demeure suspendue pendant le développement direct.

## Premier résultat vérifié

La suspension/réactivation locale est implémentée dans le contrôle d’identité
commun et éditable par cases dans Admin. L’aperçu montre les identités sans
jetons. La suspension bloque aussi les rejeux, persiste après redémarrage et ne
modifie ni les actions ni les preuves. Les types invalides sont refusés.

139 cas Go avec race (37 principaux,102 sous-cas), sept scénarios publics et
23 contrôles navigateur passent. Le parcours navigateur FR/EN suspend, refuse
une demande, réactive puis crée une nouvelle demande avec mandat renouvelé.
I18n, vet et compilation passent. L’aperçu à l’adresse déjà utilisée est actualisé
en conservant ses deux demandes et son effet simulé ; l’ancien processus a été
identifié avant arrêt. À cette première étape, OIDC et les workloads restaient non implémentés ; leurs résultats ultérieurs figurent ci-dessous.

[Preuves](verification-identites-20261015/verification.json).

## Reprise OIDC — contrat de la tranche suivante

Deux voies examinées : accepter directement un jeton fourni par le navigateur,
ou terminer côté moteur Authorization Code + PKCE. La seconde est retenue :
elle lie nonce, état, vérificateur PKCE et révision administrée. La première ne
prouve pas cette liaison et n’est pas exposée.

Configuration optionnelle `oidc` commune à Admin, CLI et API : activation, issuer,
client_id, redirect_uri, mapping explicite `external_subject` vers `subject` local,
`request_timeout_seconds`, `login_ttl_seconds`, `session_ttl_seconds`. Les trois
durées sont strictement positives (0 est refusé), exprimées en secondes. Omission
de `oidc` conserve la configuration ; `enabled:false` désactive et révoque les
sessions. Aucun secret client n’est requis pour ce client public avec PKCE.

Parcours public `oidc-start` puis `oidc-finish` : début lié à un secret de navigateur,
retour avec état/code, échange serveur hors verrou, contrôle issuer/signature/
audience/azp/nonce/iat/expiration, puis session opaque limitée par la date du jeton
et la politique locale. Toute configuration appliquée invalide les connexions et
sessions précédentes. Les rôles et l’intégrité viennent de l’identité locale ;
un compte externe absent du mapping ou suspendu est refusé. Aucun jeton OIDC,
code ou secret de session n’entre dans les preuves métier. Les données temporaires
PKCE restent dans le stockage privé et sont consommées avant l’échange ; un échec
exige une nouvelle connexion. Les refus ne modifient aucune action/mandat/preuve.

Recette isolée avant qualification : connexion réelle au faux issuer avec PKCE,
autorisation métier par session, reprise après réouverture, rejet mauvais issuer,
signature, audience, nonce, dates et mapping ; rejeu/concurrence, révocation par
configuration/suspension, rotation de clé et erreur réseau. Admin FR/EN/thèmes
permet l’édition, l’aperçu sans échange réseau, la sauvegarde et la réouverture.
Propriétaire Codex. Aucune connexion d’entreprise externe n’est revendiquée.

Dépendance retenue : go-oidc v3.17.0 (dernière branche compatible Go 1.24), avec
go-jose v4.1.4 ; aucune mise à niveau implicite de Go. Les versions plus récentes
exigent Go 1.25 ; ce choix reste une limite de compatibilité à réexaminer.

## Replan de la recette du pilote

La première recette OIDC navigateur passe (8 contrôles, deux connexions et demandes
métier FR/EN, aucun diagnostic inattendu). La recette historique échoue ensuite
sur la taille des boutons OIDC masqués, mesurés à 0 × 0. Cause : son sélecteur
incluait les contrôles invisibles, alors que l’assertion porte sur les cibles
visibles. Le correctif conserve la mesure de 44 px et filtre les éléments sans
rectangle de rendu ; les boutons OIDC visibles sont exercés par la nouvelle
recette. Propriétaire Codex. Original : `oidc-public-delivery.out`. Reprise :
recette publique complète, puis observation des captures OIDC. Aucun contrôle
d’autorisation ou d’accessibilité visible n’est supprimé.

La mesure mobile supplémentaire du parcours OIDC révèle ensuite une cible de
bouton inférieure à 44 px (`oidc-browser-final.out`). Les nouveaux boutons ne
reprennent pas encore les sélecteurs de dimension du pilote. Correction attendue
dans le style commun de cette page, puis mesure et capture FR/EN, clair/sombre.
Cet écart bloque la qualification visuelle OIDC malgré la connexion fonctionnelle.

La dépendance cryptographique est fixée à go-jose v4.1.4, qui corrige
[GO-2026-4945](https://pkg.go.dev/vuln/GO-2026-4945) selon la base officielle Go.
La branche v3 du client OIDC est couverte par sa
[politique de sécurité](https://github.com/coreos/go-oidc/blob/v3/SECURITY.md).
Cela ne remplace pas les contrôles de nonce, azp et PKCE de l’application.
La tranche conserve ouverts les workloads, délégations, step-up et les événements
de révocation émis par un fournisseur externe ; aucune qualification globale R13.

Le contrôle de type supplémentaire reproduit l’acceptation de `enabled:null`
comme désactivation implicite par le décodeur JSON Go (`oidc-enabled-before.out`).
Le booléen d’activation devient strict : seuls true/false sont admis ; les cas
null, texte, nombre, objet et tableau restent comme garde de régression. Ce
correctif porte sur le contrat de configuration commun, sans exception de mission.

Le premier build du booléen strict échoue sur deux expressions Go mêlant bool et
son type nommé (`oidc-build-final.out`, `oidc-race-final.out`). Cause liée : les
conversions explicites manquaient dans les expressions composées. Elles sont
ajoutées au contrôle commun ; les prochains résultats sont enregistrés sous
`oidc-race-verified.out` et `oidc-build-verified.out`, sans remplacer ces échecs.

La première correction CSS reste insuffisante : la recette mesure encore 38 px
(`oidc-browser-verified.out`), car `swarm-design.css`, chargé ensuite, réaffecte
la hauteur avec une règle de même spécificité. Le sélecteur du pilote couvre
désormais tous ses boutons et liens d’action sous `body`, avec une spécificité
supérieure. La feuille partagée et les autres interfaces restent inchangées.
La nouvelle recette est conservée sous `oidc-browser-rendered.out`.

## Résultat courant de la tranche OIDC

37 contrôles Go OIDC avec race (8 principaux, 29 sous-cas) passent, dont
`enabled:null` refusé et délai réseau d’une seconde observé sans mutation. Les
139 contrôles du pilote sont conservés à leurs entrées ; la recette publique
complète repasse sept scénarios et 23 contrôles navigateur historiques.

La recette OIDC finale passe 14 contrôles : Admin, validation traduite sans
application, aperçu sans réseau, sauvegarde/réouverture, connexion entre deux
origines, privilèges locaux, demande métier affichée après actualisation et
déconnexion, en FR/EN, clair/sombre à 390/320 px. Les deux réponses de rejet
volontaires sont attribuées ; aucun diagnostic inattendu. Les captures corrigées
sont conservées avec les mesures de cibles à 44 px minimum.

I18n, vet, compilation et douze contextes de méthodes passent. Le serveur de
démonstration a été remplacé après identification du seul processus propriétaire,
sans modifier ses octets métier. OIDC y reste désactivé ; aucune connexion réelle
n’est revendiquée. [Preuve et limites](verification-identites-20261015/oidc-verification.json).

Apprentissage réutilisable : couvrir null parmi les booléens JSON ; observer les
dimensions après toute la cascade CSS ; filtrer les contrôles masqués dans les
mesures de cibles visibles tout en exerçant leurs états visibles ; vérifier la
connexion jusqu’à l’opération métier et son affichage, sans déduire ce résultat
d’un seul jeton ou d’un HTTP 200. Ces gardes sont dans les recettes communes du
pilote et d’OIDC ; elles ne modifient aucun contexte de rôle natif ou droit.

## Tranche services — décision et assertions de recette

Le code ne fournit encore aucune connexion de workload. Deux voies ont été
comparées : assimiler l’access token à un ID token humain, ou vérifier deux JWT
distincts. La seconde est retenue : assertion client privée signée hors de Swarm,
puis access token signé par l’émetteur. La clé privée ne transite jamais dans la
configuration Admin ; seules les clés publiques sont enregistrées.

Contrat `workloads` optionnel commun Go/API/CLI/Admin : enabled strict, issuer,
audience de la ressource, request_timeout_seconds, assertion_ttl_seconds et
session_ttl_seconds strictement positifs (0 refusé), clients avec client_id,
external_subject, subject local et clés publiques JWK identifiées. Les valeurs
proposées des trois durées sont 15/300/900 secondes. Omettre la configuration la
conserve ; désactiver révoque les sessions. Toute configuration appliquée impose
une nouvelle connexion, sans effacer les identifiants d’assertion déjà consommés.

`workload-login` reçoit client_id et client_assertion. Avant le réseau, Swarm
vérifie la signature par une clé publique configurée, iss/sub=client_id, audience
du point token découvert, iat/exp/nbf, durée maximale et jti obligatoire. La
réservation anti-rejeu persiste avant l’échange ; toute reprise après échec
demande une nouvelle assertion. Client credentials utilise exclusivement
private_key_jwt et l’émetteur doit annoncer cette méthode. Le JWT d’accès est
contrôlé par les clés de l’émetteur : typ at+jwt (ou application/at+jwt), issuer,
audience ressource, client_id/sub configurés, iat/exp/nbf et jti. Les rôles/groupes
externes ne donnent aucun droit. Une session opaque expire au plus tard avec
l’access token ; ses droits viennent de l’identité locale courante.

Propriétaire Codex, sans compte externe. Recette isolée préalable : requête
private_key_jwt réellement vérifiée par un faux serveur d’autorisation, effet
métier du service, CLI/API, persistance, rejeu concurrent, mauvais JWT/signature/
clé/issuer/audience/type/client/sub/dates, refus d’un ID token humain, rotation,
suspension, configuration concurrente, indisponibilité et délai. Les refus ne
modifient aucun mandat/action/effet/preuve ; seule une assertion valide arrivée à
l’échange peut consommer sa réservation d’authentification. Admin FR/EN permet
aperçu sans réseau, sauvegarde et réouverture ; aucune clé privée n’est acceptée.
Les délégations et le step-up restent des tranches suivantes.

Sources primaires : [RFC 7523](https://www.rfc-editor.org/rfc/rfc7523.html),
[RFC 9068](https://www.rfc-editor.org/rfc/rfc9068.html) et
[authentification client OIDC](https://openid.net/specs/openid-connect-core-1_0.html#ClientAuthentication).

La première compilation révèle un champ mal supposé dans la bibliothèque :
`jwt.Expected` utilise `AnyAudience`, pas `Audience` (`workload-config-first.out`).
Le producteur et sa validation ont été lus puis la structure corrigée. La
signature et les claims d’assertion sont contrôlés avant la découverte ;
l’audience est comparée ensuite au point token découvert, avant l’échange.
Cette précision remplace « avant le réseau » pour l’audience seulement.

La recette directe Go reproduit une panique sur clé publique typée nil : la
méthode Valid() de la bibliothèque présuppose un pointeur non nil
(`workload-key-before.out`). Le contrôle d’entrée vérifie désormais le pointeur
avant l’appel, ainsi que le point P-256 et les paramètres RSA. Les cas nil et
incomplets restent des gardes. Le premier parcours service/effet/reprise est
passé (`workload-first-flow.out`), distinct des recettes adversariales à venir.

Le test de confidentialité confirme que la configuration désactivée acceptait une
clé privée dans son aperçu (`workload-private-key-before.out`) ; la recette
s’arrête avant application. Cause : le retour anticipé sur enabled=false
court-circuitait le contrôle des clés. Le refus des clés privées/malformées est
déplacé avant ce retour pour tous les clients, indépendamment de l’activation.
Cette correction est soumise à la recette Go/API/Admin et ne qualifie pas encore
la livraison services. Aucun état de démonstration n’a reçu cette configuration.

La première recette navigateur services atteint l’authentification mais refuse
l’effet : son approbation omettait mandat, montant, devise et bénéficiaire, donc
« accord non exact ». Le refus moteur est conforme. La recette vérifie désormais
les champs exacts et l’état approved avant la connexion de service ; les trois
événements originaux sont attribués dans `workload-browser-first/replan.json`.
Les sept scénarios publics du pilote et la recette OIDC humaine repassent sur
ce binaire ; ils ne remplacent pas la preuve d’effet du nouveau service.

## Résultat courant de la tranche services

La suite complète concernée passe 230 cas Go race : 56 principaux et 174
sous-cas, dont 54 services (11 principaux, 43 sous-cas). Elle couvre les anciens
contrats du pilote et d’OIDC, la signature client indépendante, l’access token
distinct, l’effet approuvé, la persistance, la concurrence, la révocation, les
configurations en cours d’échange, l’anti-rejeu, la rotation et le délai réseau.

Les recettes publiques passent sept scénarios et 23 contrôles navigateur du
pilote, 14 OIDC humain et 12 services. Admin FR/EN clair/sombre mobile vérifie
le refus traduit, l’aperçu sans réseau, les valeurs exactes persistées et leur
réouverture, puis le CLI s’authentifie et l’API engage l’effet affiché dans l’UX.
Aucun diagnostic inattendu ; deux refus volontaires attribués dans chaque
recette d’identité. Les captures ont été inspectées.

I18n, vet et compilation passent. L’aperçu a été actualisé après identification
du seul processus propriétaire, sans modifier son état métier ; les connexions
fédérées restent désactivées dans cette démonstration. Le serveur d’émetteur
factice est fermé après chaque recette.
[Preuves et limites](verification-identites-20261015/workload-verification.json).

Apprentissage : contrôler les secrets même dans une configuration inactive ;
valider les pointeurs Go avant les helpers cryptographiques ; maintenir
l’anti-rejeu à travers les changements de configuration ; vérifier chaque état
métier intermédiaire avant d’attribuer un refus d’effet à l’authentification.
Ces gardes sont exécutées dans les tests et recettes versionnés, sans modifier
les contextes de rôle ni accorder de droits natifs.

Suite R13 : délégations et step-up, puis révocation fournisseur et qualification
globale. Les autres tranches du programme et le RETEX restent à réaliser.

## Tranche délégations — mandat enfant administré

Le pilote conserve un délégant authentifié mais aucune chaîne contrôlée. Deux
voies examinées : laisser un demandeur I1 produire directement une autorité I2,
ou conserver l’endorsement par l’administrateur I3 pour chaque mandat enfant.
La seconde est retenue pour préserver le contrat Biba. Cette tranche porte sur
les délégations administrées ; elle ne crée pas d’auto-délégation des demandeurs.
Le champ delegator reste l’administrateur effectivement authentifié ; le lien
parent explicite évite de prétendre qu’un autre titulaire a signé.

`mandate-create` accepte `parent_mandate_id`. Le moteur lie l’enfant au digest
exact du parent et refuse l’amplification : même ressource/devise/bénéficiaire,
actions incluses dans celles du parent, plafond inférieur ou égal, intervalle
de validité inclus. Le sujet de l’enfant est une identité locale active.
Le parent et sa chaîne doivent être actifs, à la révision courante, sans cycle
ni empreinte périmée. Les sujets des ancêtres doivent rester actifs.

La configuration optionnelle `delegation:{max_depth:N}` est commune Go/API/CLI/
Admin. N compte les liens parent-enfant ; 0 désactive les délégations, défaut
compatible avec les configurations antérieures. Une valeur négative ou hors
représentation entière commune est refusée. Omission conserve la valeur. L’aperçu
ne modifie rien et l’application conserve l’histoire et périme les mandats
comme toute configuration existante. Aucun plafond métier caché supplémentaire.

Avant une nouvelle demande, une approbation ou un premier engagement, chaque
lien est relu dans le même verrou que l’effet. Révoquer un parent bloque donc
les descendants sans réécrire leurs données ni leurs preuves. Un effet déjà
engagé reste conservé et réconciliable. Les anciens mandats sans parent gardent
leur digest et leur parcours. Propriétaire Codex, fixtures isolées uniquement.

Assertions de recette avant qualification : chaîne à plusieurs niveaux puis
effet exact, champs d’amplification et dates refusés, sujet inexistant/suspendu,
profondeur, révocation du parent à chaque frontière, identité d’ancêtre suspendue,
chaîne altérée/cyclique refusée sans boucle, persistance, concurrence et rejets
sans mandat/effet créé. CLI/API/UX montrent le lien et la cause ; Admin FR/EN,
thèmes et mobile vérifient preview, sauvegarde et réouverture. Les preuves de
refus restent conservées. Step-up et révocation fournisseur restent suivants.

Replan de recette : `delegation-depth-before.out` prouve que `null` devenait 0 ;
le décodage exige maintenant un entier explicite. `delegation-expiry-before.out`
prouve que la durée par défaut refusait un parent plus court ; une durée omise
est maintenant plafonnée par le parent, tandis qu’une extension explicite reste
refusée. La première série complète (`delegation-targeted.out`) a aussi révélé
un défaut de fixture : même identifiant d’événement réutilisé par deux acteurs.
Le refus de rejeu moteur était conforme ; la seconde commande de recette reçoit
son propre événement, sans modifier le contrat de sécurité. Ces preuves restent
conservées ; vérification suivante dans une nouvelle sortie.

La première tentative navigateur n’a exécuté aucun scénario (`EACCES`) : une
compilation Go à la racine avait produit une archive, pas le CLI. Correction de
méthode : utiliser exclusivement `sh build.sh OUTPUT`, puis vérifier le binaire
exécutable avant la recette. La sortie d’origine est conservée dans
`delegation-browser-first.out` ; elle ne vaut aucun résultat produit.

## Conception initiale de la réauthentification renforcée (implémentation ci-dessous)

Constat de source : la session OIDC actuelle ne conserve ni `auth_time` ni
`acr` ; elle ne peut donc pas prouver une réauthentification récente ou un
niveau convenu pour une approbation sensible. Le jeton statique local ne doit
pas contourner une politique activée. Source primaire :
[OpenID Connect Core §2, §3.1.2.1, §3.1.3.7 et §5.5.1.1](https://openid.net/specs/openid-connect-core-1_0.html).
`max_age` impose la présence d’`auth_time` ; `acr_values` seul est volontaire.
Décision prévue : demander ces paramètres et contrôler les claims signés côté
Swarm ; aucune valeur ACR ne crée un rôle ou un niveau Biba local.

Comparer deux voies avant modification : exiger une reconnexion forte pour
chaque connexion, ou une preuve renforcée liée au contenu exact d’une action
avec durée et ACR configurés. La seconde répond mieux au step-up sensible mais
requiert une liaison d’action et une protection contre le changement de sujet.
Définir la surface Go/API/CLI/Admin et les refus sans mutation avant de coder :
claim absent/ancien/futur, ACR non convenu, session faible/statique/service,
mauvais sujet/action/digest, révision changée, expiration, rejeu, suspension,
révocation et concurrence. Démontrer ensuite refus → réauthentification réelle
sur l’émetteur isolé → opération exacte autorisée → expiration/révocation.
Les durées et déclencheurs seront persistés et éditables dans Admin, sans
seconde limite cachée. Les valeurs proposées restent à arrêter par la recette.
La notification de révocation fournisseur demeure également non implémentée.

Actualisation de l’aperçu : identité du PID 540889 et empreinte vérifiées avant
arrêt ; nouveau serveur dédié et API/HTML contrôlés avec deux demandes et un
effet, octets métier inchangés, délégation désactivée par défaut. L’attachement
à l’onglet IAB a échoué sur un délai CDP `Emulation.setFocusEmulationEnabled`.
Cela ne démontre pas une panne du produit : la recette navigateur et l’API
passent. Ne pas affirmer un rechargement observé dans cet onglet. Propriétaire
de ce contrôle : intégration navigateur Codex ; reprise après accès CDP rétabli.

## Qualification locale des délégations — 15 octobre

La nouvelle série `delegation-all-race.out` passe 264 cas nommés : 66 principaux
et 198 sous-cas, dont 34 délégations (10 principaux, 24 sous-cas). Les contrôles
existants du pilote, des connexions humaines et des services sont donc relus
sur le contrat d’autorisation modifié. Aucun détecteur de concurrence ne signale
de défaut. Les sept scénarios publics et 23 contrôles navigateur du pilote,
14 OIDC humain, 12 services et dix délégations passent sur cette version.
Aucun diagnostic inattendu ; les refus volontaires historiques sont attribués.
Les captures Admin anglais sombre mobile et chaîne française claire ont été
inspectées. La page conserve les montants exacts, le lien et le motif traduit.

I18n, Go vet, compilation canonique, douze contextes de méthodes et manifeste
passent. Le nouveau serveur d’aperçu garde les octets métier intacts et les
connexions fédérées/délégations désactivées. L’onglet IAB reste non rechargé
selon la limite CDP documentée ; API/HTML et navigateur isolé sont des preuves
distinctes. [Synthèse et entrées](verification-identites-20261015/delegation-verification.json).

Apprentissage : conserver les empreintes racines avant d’ajouter une chaîne ;
contrôler tous les ancêtres sous le verrou d’effet ; ne pas faire dépendre un
mandat enfant d’une durée par défaut plus longue que son autorité parente ;
distinguer la nouvelle opération du rejeu d’un événement par un autre acteur.
Ces gardes sont dans les tests et recettes nommés, sans modifier l’autorité des
agents. Suite autorisée : step-up puis révocation fournisseur, qualification
R13 et autres lots jusqu’au RETEX. R13 reste partiel et non accepté globalement.

### Contrat retenu pour le step-up

La voie retenue est une preuve OIDC renforcée liée à l’action, plutôt qu’un
niveau élevé attribué à toute connexion. Première opération sensible :
`approve`, déclencheur visible et persisté. Cette étape ne prétendra pas protéger
les changements Admin, qui restent soumis à leur contrat opérateur existant.

Configuration proposée `step_up` : `enabled` booléen strict, `operations` contenant
`approve` lorsqu’activée, `acr_values` explicites convenus avec l’émetteur,
`max_age_seconds` entier (0 demande une nouvelle authentification) et
`proof_ttl_seconds` entier positif (valeur Admin proposée 300 secondes).
La fonction reste désactivée par défaut et requiert OIDC activé avant activation.
Omission conserve la valeur, preview sans réseau/écriture, application persistée
à la même révision que les autres politiques. Aucun ACR prédéfini ne signifiera
arbitrairement MFA ; le fournisseur et l’opérateur doivent convenir de son sens.

Réutiliser `oidc-start` avec `step_up_action_id` et le Bearer actuel : authentifier
un approbateur local I2, contrôler sa séparation du demandeur et le mandat actif,
puis lier le pending au sujet, à l’identifiant/digest de l’action, au digest du
mandat et à la révision. Envoyer max_age et une demande essentielle des ACR
convenus, avec les gardes PKCE/nonce existantes. `oidc-finish` doit refuser un
sujet changé, les claims manquants, un ACR non convenu et un auth_time trop ancien
ou futur. Pour max_age=0, contrôler la fraîcheur à la précision en secondes du
claim depuis le début de cette nouvelle demande. Ne jamais persister le jeton
fournisseur. La session opaque porte seulement les attributs vérifiés et la
liaison d’action ; son expiration ne dépasse ni la durée de preuve ni celle de
la session/du jeton fournisseur. Aucun nouveau rôle ou niveau Biba n’est créé.

Avant un accord, le verrou métier vérifie la politique actuelle et cette preuve
contre le contenu exact : jeton statique, service, session faible, autre action,
autre mandat/digest, preuve expirée ou révision ancienne refusés sans accord ni
effet. Une preuve valide n’est consommée que par un accord valide ; le rejeu du
même événement conserve sa réponse historique selon l’autorité existante.
L’UI doit permettre la reconnexion renforcée de l’action sélectionnée, afficher
la cause et conserver la navigation FR/EN/thèmes/mobile et l’accessibilité.

Recette à écrire avant le moteur : échec du contrat absent, refus faible/statique/
service et sans mutation ; assertions de requête auprès de l’émetteur isolé ;
matrice auth_time/ACR/sujet/nonce/signature ; accord exact après reconnexion ;
expiration, modification de configuration, suspension, révocation de mandat et
concurrence. Persistance et limites Admin seront vérifiées par CLI/API et
sauvegarde/réouverture navigateur. Les valeurs et interfaces ci-dessus sont une
spécification de développement, pas une fonction livrée. Propriétaire Codex.


## Reprise du step-up — implémentation du 15 octobre

Le contre-exemple `step-up-before.out` constate le contrat de configuration
absent, sans prétendre que la connexion OIDC précédente était défectueuse.
Le premier moteur ciblé passe 34 cas (11 principaux,23 sous-cas), dont le service
réellement authentifié et associé à bob, la persistance, les claims et la
consommation concurrente. `step-up-config-first.out` prouve l’aperçu sans mutation.
La série complète est relancée car le contrat commun d’approbation a changé.

Replan de recette navigateur : `step-up-browser-first.out` atteint la connexion
faible refusée, la reconnexion liée à l’action, le refus sur une autre action
et l’accord exact. L’assertion de déconnexion lisait l’état immédiatement après
un clic dont la requête est asynchrone. La recette attend désormais l’état nul
et le bouton masqué. Ce premier échec ne prouve pas une panne de déconnexion ;
le résultat effectif reste à vérifier dans la nouvelle recette. L’assertion a
aussi imprimé un jeton opaque de fixture locale : original conservé privé,
révocation par opération publique et synthèse sans cette valeur. Aucun compte
externe ni jeton fournisseur n’a été utilisé. Propriétaire Codex.

La seconde recette navigateur passe 18 contrôles FR/EN et mobile, dont le CLI
sans identité refusé avant découverte réseau, le compte ordinaire refusé et
l’accord exact après reconnexion. La déconnexion est observée. Un contrôle
supplémentaire (`step-up-blank-before/step-up-ui-failure.json`, diagnostic privé)
reproduit cependant un âge vide converti en zéro par Number(''). Le formulaire
sera rendu obligatoire et gardera ce cas comme garde avant qualification.
La revue du contrat Go/JSON vérifie en parallèle les listes nulles d’une
politique désactivée : elles ne doivent jamais produire un état que le décodeur
strict refuse ensuite. Les résultats positifs précédents ne clôturent pas ces
critères ; propriétaires Codex, fixtures et opérations publiques uniquement.

`step-up-typed-policy-before.out` reproduit les deux cas : la validation Go
acceptait une liste nil dans la politique inactive, l’écriture produisait null,
puis la relecture stricte refusait cet état. La frontière autoritaire refuse
maintenant ces listes avant écriture, comme JSON. Le formulaire exige aussi les
deux durées et refuse explicitement une chaîne vide avant conversion numérique.
Les originaux restent conservés ; aucune réparation de fixture privée ou d’état
vivant n’est utilisée pour fabriquer un résultat.

La série complète antérieure passe 298 cas (77 principaux,221 sous-cas). La
correction suivante ne change que les entrées non null de la nouvelle politique ;
les 264 cas historiques sans step_up utilisent la branche nil inchangée. Décision
avant vérification : réexécuter tous les cas step-up avec détection de concurrence,
y compris la nouvelle garde de round-trip, puis tous les parcours publics sur le
binaire final. Réutiliser les 264 autres résultats plutôt que relancer leur série
identique. Propriétaire Codex ; résultat final encore attendu.

Les gardes finales `step-up-race-final.out` passent 37 cas nommés (12 principaux,
25 sous-cas), y compris les deux listes nil sans aucune mutation. Avec les
264 cas historiques pertinents de la série complète, 301 cas distincts sont
couverts ; il ne s’agit pas d’une nouvelle exécution unique de 301 cas.
La compilation finale ne change ensuite qu’un espace de fin de ligne JavaScript.
La validation des données de politique, les méthodes et les entrées Go restent
identiques aux gardes passées. Les parcours navigateur sont à relire sur ce
binaire final avant de qualifier l’interface.

Prévention réutilisable : un décodeur strict ne suffit pas si un appel Go peut
construire un objet que la relecture refusera. Valider la représentation persistée
à la même frontière que JSON, avant écriture, même fonction désactivée. Distinguer
un champ vide d’une valeur zéro à sémantique explicite. Attendre le résultat d’une
interaction asynchrone et garder les diagnostics complets de fixture privés ;
révoquer tout jeton accidentellement imprimé par les opérations publiques.


## Qualification locale du step-up — résultat courant

Le binaire final passe sept scénarios publics et79 contrôles navigateur :
20 step-up,14 OIDC humain,12 services,10 délégations et23 pilote. Les vingt
contrôles incluent le champ vide refusé, l’aperçu sans réseau, la persistance,
la réouverture, la reconnexion pour l’action sélectionnée, l’accord exact,
le refus pour une autre action, un effet unique et la déconnexion observée.
Aucun diagnostic inattendu ; deux refus de configuration volontaires attribués.
Les captures Admin EN sombre320px et résultat FR clair390px ont été inspectées.

37 cas Go race finaux passent ; 264 résultats sans nouvelle politique sont
réutilisés de la série commune de298 cas. Le total de301 cas distincts couverts
ne signifie pas une exécution unique de301 cas. I18n, vet, compilation canonique,
douze contextes et contrôles de syntaxe passent. Le manifeste de source final
est vérifié séparément après les derniers documents. Réutiliser ces résultats
aux sources et entrées indiquées, sans campagne répétée en l’absence de changement.

Aperçu actualisé après identification du seul PID659041 : nouveau PID801731,
démarrage2297580 ; bin/swarm-actions-confiance-step-up SHA-256
9d720b3619fb14b3704b0ee97edfb791be165f0024ddd3391221761cd53ca2c2.
Deux demandes et un effet simulé, octets métier inchangés ; OIDC/services/
délégations/step-up désactivés dans cette démonstration. API/HTML contrôlés,
rechargement IAB non observé. Aucun autre serveur interrompu.
[Preuves, sources et limites](verification-identites-20261015/step-up-verification.json).

Le step-up d’approbation est implémenté et vérifié localement. La révocation
initiée par le fournisseur reste à livrer, puis la qualification R13 et les
autres lots jusqu’au RETEX. Aucune activation externe, acceptation globale
ou qualification de diffusion n’est déduite de ces tests. Les changements Admin
restent soumis à leur autorité opérateur et ne reçoivent pas ce step-up.

## Révocation fournisseur — conception initiale conservée

Source primaire vérifiée : [OpenID Back-Channel Logout 1.0, errata1, §2.4–2.8](https://openid.net/specs/openid-connect-backchannel-1_0.html).
Cette version exige aussi exp : ne pas reprendre l’ancienne hypothèse d’expiration
optionnelle. Préparer un POST form-urlencoded signé, distinct de la session
opérateur ; valider signature/algorithme, issuer/audience/iat/exp, événement de
déconnexion, absence de nonce et sub ou sid. Préserver issuer/sub/sid issus du
jeton humain vérifié pour retrouver les bonnes sessions. Le modèle actuel ne
les conserve pas encore : évolution de données à vérifier en compatibilité.

Comparer sondage externe à chaque action et notification signée. Retenir la
notification avec anti-rejeu persistant, sans réseau périodique ni compte tiers.
Définir les politiques Admin/API/CLI de fraîcheur et de conservation anti-rejeu,
avec validation commune et condition empêchant l’expiration prématurée d’une
garde. Préserver les mandats, actions, effets, preuves et consommation existants.

Recette isolée attendue avant code : déconnexion d’une session par sid puis de
toutes celles d’un sujet, mauvais issuer/audience/signature, jeton expiré/futur,
exp absent, nonce présent, événement incorrect, mauvais sid/sub, rejeu et reprise
après redémarrage/configuration. Les autres personnes et les services doivent
rester utilisables. Vérifier que l’ancien jeton humain et sa preuve step-up
refusent ensuite une opération, tandis que l’effet déjà engagé reste conservé.
L’endpoint ne vaut pas activation chez un fournisseur réel. Propriétaire Codex.

Replan de méthode : entre18:26:01 et18:27:28 UTC, la rédaction d’un gros appel
mise à jour d’automatisation a dépassé le point d’information de60secondes ;
l’outil lui-même a ensuite répondu en moins d’une seconde. Ce délai observé
vient de la taille du bloc rédigé, sans établir une cause des anciennes latences
ou reconnexions. Prévention concrète : blocs d’entrée d’environ1000tokens au plus,
lecture d’horloge après chacun, assemblage en mémoire et point utile avant40s.
Aucun appel lent inchangé à répéter. Les résultats produit restent distincts.

### Contrat d’implémentation de la déconnexion fournisseur

Politique `oidc_logout` optionnelle : enabled strict, max_age_seconds positif
(proposition300), replay_retention_seconds positif ou nul (proposition600 ;
0 conserve les empreintes sans purge). Si non nulle, la conservation minimale
doit couvrir l’âge maximal. Chaque empreinte demeure au moins jusqu’à exp du
jeton signé, même après reconfiguration : augmenter ensuite l’âge accepté ne
doit jamais rendre réutilisable un ancien événement. Défaut désactivé,
omission conserve, champs internes explicites non null. Go/API/CLI/Admin
emploient la même validation. Une activation exige OIDC déjà configuré.

Ajouter issuer/external_subject/sid optionnels aux sessions humaines après
vérification de l’ID token ; les anciennes sessions restent lisibles, et toute
reconfiguration conserve sa révocation existante par révision. Ne pas attribuer
ces champs aux sessions de services. Les informations fournisseur ne confèrent
aucun rôle et ne sont pas exposées par le catalogue public des identités.

Surface : CLI `oidc-backchannel-logout --input -` et API opérateur JSON reçoivent
logout_token. Un endpoint dédié `/api/v1/trust-actions/oidc-backchannel-logout`
reçoit le formulaire standard sans cookie opérateur, sous contrôle d’hôte et
signature JWT. Aucun autre parcours ne perd cookie/CSRF. POST seulement, corps
borné techniquement comme l’API, paramètres inconnus ignorés, doublon de
logout_token refusé. Le jeton est typé logout+jwt dans ce profil afin d’éviter
la confusion avec un jeton de connexion/service ; clés publiques de l’émetteur,
algorithmes existants, issuer exact, audience/azp, iat/nbf/exp stricts, jti,
événement requis, absence totale de nonce et sub ou sid contrôlés.

Sous verrou : vérifier la révision, refuser le rejeu, enregistrer l’empreinte
issuer+jti puis retirer uniquement les sessions OIDC correspondantes. Avec
sid, correspondance sur issuer/sid et aussi sub s’il est fourni ; sans sid,
toutes les sessions issuer/sub. Zéro session trouvée est un succès normatif.
Les requêtes déjà engagées se sérialisent avec cette révocation ; les nouveaux
accords sous session révoquée échouent. Ne modifier ni sessions de service,
mandats, actions, effets, preuves métier, ni consommation. Aucun jeton signé
brut dans l’état ou les diagnostics. Recette complète avant qualification,
avec conservation de chaque échec et refus sans mutation. Propriétaire Codex.


Précision avant recette : typ reste recommandé par la norme, non obligatoire.
Le profil accepte son absence, JWT générique et logout+jwt ; il refuse un type
invalide ou un jeton explicitement de service. Les claims événement/absence de
nonce restent obligatoires. Cela remplace l’option de typage obligatoire
ci-dessus pour préserver les fournisseurs conformes qui omettent ce champ.
Ajouter `oidc-session` : interrogation privée opérateur/CLI sur un jeton opaque,
réponse active booléenne explicite, sans écho du jeton. Le navigateur l’utilise
à l’actualisation pour retirer une identité révoquée ; un champ malformé reste
refusé. Ce statut ne donne aucun accès et ne remplace pas l’authentification des
opérations métier. Recette attendue actif avant puis inactif après déconnexion.

### Replan : connexion en vol après révocation

`oidc-logout-browser-before/oidc-logout-ui-failure.json` reproduit par CLI et
callback HTTP publics : démarrage OIDC, autorisation du code, déconnexion signée
du sid, puis finish accepte encore une session. Le code ne retirait que les
sessions déjà créées. La fonction reste non qualifiée malgré les contrôles
positifs précédents. Propriétaire Codex ; garde navigateur conservée.

Comparer annulation de toutes les connexions en cours et barrière ciblée.
Retenir une barrière issuer/sid/sub appliquée au finish sous verrou, pour garder
les connexions et sessions des autres personnes. Chaque pending conserve son
instant de création ; un événement reçu après cet instant bloque les fins de
connexion correspondant à sa cible. Une connexion démarrée après cet événement
reste permise. Avec sid et sub fournis, la cible combine les deux ; sans sid,
elle couvre le sujet ; sans sub, elle couvre le sid. Aucune élévation de rôle.

La barrière demeure jusqu’à réception + validité de connexion + délai de requête
OIDC, dérivés des politiques existantes éditables Admin. Cette durée couvre aussi
l’échange déjà commencé après consommation du pending. Les additions se font
successivement sur time.Time pour éviter le débordement d’une somme de durées.
Les empreintes anti-rejeu restent distinctes et ne périment jamais avant exp du
jeton signé ; une reconfiguration ne les efface pas. La barrière ajoute une
métadonnée de contrôle ; aucune action, preuve ou consommation n’est modifiée.

Vérification requise : le contre-exemple échoue avant correction et refuse
après, une nouvelle connexion de la même cible réussit, autres sujets/sid restent
utilisables, et échange réseau en cours se sérialise avec la révocation. Puis
reprendre la recette initiale complète, sans clore sur ce seul correctif.


Contrôle de transport supplémentaire : `oidc-logout-media-before` reproduit
HTTP400 au lieu de200 pour un formulaire signé valide avec Content-Type de
casse différente et charset. La comparaison de chaîne imposait une écriture
unique ; ParseMediaType de la bibliothèque Go normalise le type. Correction
prévue après le contrôle actif, puis garde HTTP ciblée et reprise de la recette.
La première série de connexion en vol attend le résultat natif avant toute
modification de ses sources. Sa compilation a pris plus de deux minutes avec
CPU actif, sans tests encore démarrés ; RAM disponible observée environ4,8Go.
Cela n’établit aucune cause des anciennes latences ou reconnexions du produit.


Le finish vérifie également son contexte et l’expiration du pending au moment
de l’écriture, pour qu’une attente de verrou ne dépasse pas la période couverte
par la barrière. Les cinq variantes de ciblage et l’échange réseau déjà commencé
sont des gardes distinctes du seul retrait de sessions existantes. Les refus
MIME et de connexion en vol restent conservés avant toute qualification finale.


### Déconnexion fournisseur — qualification locale du 15 octobre

La correction MIME et la barrière des connexions en cours sont maintenant
vérifiées. Les originaux ci-dessus restent conservés. Trois séries Go race
passent : 343 cas communs, 123 ciblés après la barrière puis neuf ciblés après
la correction MIME. Leur union contient 353 cas distincts (92 principaux,
261 sous-cas) ; ce n’est pas une exécution unique de 353 cas. Les résultats
antérieurs sont réutilisés seulement pour les contrats et entrées inchangés.

Les sept scénarios publics et 97 contrôles navigateur passent sur le binaire
final : pilote 23, OIDC humain 14, services 12, délégations 10, step-up 20,
déconnexion fournisseur 18. Aucun diagnostic inattendu. Les recettes démontrent
le retrait sélectif, le refus des anciens Bearer et preuves renforcées, la
barrière des connexions en vol, une nouvelle connexion permise et la
conservation de l’effet engagé. L’émetteur est une fixture locale.

L’aperçu conserve deux demandes et un effet ; les octets métier sont inchangés.
La fédération y reste désactivée. Le reçu `oidc-logout-preview-process.json`
attribue le serveur au binaire final ; API et HTML sont observés, pas un nouveau
rechargement de l’onglet IAB. La preuve consolidée est
`verification-identites-20261015/oidc-logout-verification.json`.

### Prochaine tranche — couverture du contrat R13

Avant toute qualification globale, confronter les producteurs réels au contrat
`identity-context-v1` et vérifier le mapping versionné des groupes/rôles.
Le mapping explicite sujet/client vers une identité locale existe ; ignorer les
rôles externes n’établit pas un mapping de groupes configurable. Examiner les
métadonnées des sessions de service et les consommateurs T2/T4/T5/T6 avant de
choisir la plus petite évolution testable. La déconnexion humaine ne révoque
pas un access token de service chez son émetteur. R13, les lots suivants et le
RETEX restent ouverts ; aucune activation externe n’est effectuée.

### Contexte commun R01/R13 — résultat local du 15 octobre

Le producteur `identity-context-v1` et les lectures CLI/API sont maintenant
implémentés. Les nouvelles preuves lient leur contexte authentifié à leur
empreinte ; les preuves historiques restent vérifiables. La recette humaine
montre au clavier l’identité, l’émetteur et les droits locaux, puis les efface à
la déconnexion. Les métadonnées inconnues des anciennes sessions restent null.
Aucun rôle externe n’est importé automatiquement.

Une sonde publique a révélé une capture d’heure avant attente du verrou.
`identity-context-clock-before.json` conserve l’acceptation tardive ; après
correction des entrées publiques, `identity-context-clock-after.json` observe
le refus code4 sans mutation. Les gardes HTTP de lecture et demande métier
refusent également. Les horloges internes explicites restent déterministes.

Trois groupes communs terminent avec code0 : contexte16, pilote126, identités227,
soit369 cas distincts. Après la correction d’horloge,29 cas ciblés finaux passent ;
les séries se recouvrent et ne constituent pas une campagne unique. Les entrées et la
réutilisation sont attribuées par les reçus. La première série interrompue143
et sa fixture invalide restent conservées, sans statut PASS. La cause du signal
reste inconnue ; les groupes bornés détachés ont tous terminé avec leurs reçus.

Sur le binaire final : sept scénarios publics et99 contrôles navigateur,
sans diagnostic inattendu : pilote23, humain16, services12, délégations10,
step-up20 et déconnexion18. L’aperçu conserve deux demandes et un effet, octets
métier inchangés ; API/HTML observés, rechargement IAB non observé. Preuve :
`verification-identites-20261015/identity-context-verification.json`.

Prochaine tranche : mapping versionné des groupes/rôles, selon
`identity-context-replan.md`. Ne pas reconstruire les tranches vérifiées.
La révocation fournisseur des services reste distincte de la déconnexion humaine.
R13, les autres lots et le RETEX restent ouverts.

### Mapping des groupes — résultat local du 15 octobre

`group_mapping` est implémenté en Go/API/CLI/Admin FR/EN pour personnes et
services. Un claim signé configuré sélectionne des rôles via des règles locales ;
l’intersection avec le plafond de l’identité empêche toute promotion externe.
L’intégrité reste locale. Le step-up et l’accord utilisent les rôles dérivés.
Sessions/contexte/preuves lient ces rôles au digest de politique et à la révision,
sans conserver les groupes bruts dans les sessions. Les règles sont indexées
avant rapprochement, sans plafond de groupes caché. Le claim personnalisé et les
refus sans session utilisable sont vérifiés.

Les32 cas ciblés initiaux,126 pilote et246 identités passent ; après l’index,
les32 cas actifs passent à nouveau. L’union contient404 cas nommés (106 principaux,
298 sous-cas), pas une exécution unique404. Les branches inactives des126/246
restent inchangées par l’index. Sept scénarios publics et111 contrôles navigateur
passent sur le binaire final :23 pilote,26 humain/mapping,14 services/mapping,
10 délégations,20 step-up,18 logout. Aucun diagnostic inattendu. Les erreurs
volontaires de configuration sont attribuées. Les recettes montrent sauvegarde,
réouverture, clavier, thèmes/mobile, rôle admin restreint et effet de service
exactement approuvé dont la preuve lie les rôles dérivés.

L’aperçu conserve deux demandes et un effet avec les mêmes octets métier ;
OIDC/services/mapping restent inactifs sans fournisseur réel. API/HTML observés,
rechargement IAB non observé. Preuve consolidée :
`verification-identites-20261015/group-mapping-verification.json`.

Prochaine étape : couverture R13 confrontée au contrat et à ses preuves, y compris
les limites de révocation des services. Ne pas confondre déconnexion fournisseur
humaine et révocation d’un access token de service à son émetteur. R13 et la
qualification globale restent ouverts ; poursuivre ensuite les dépendances réelles
(données/secrets, processus, preuves, connecteurs, eBPF, recette et RETEX).

La [matrice R01/R13 courante](identites-r13-coverage.md) distingue les critères
locaux prouvés et le critère distant de révocation service non implémenté. La
progression autorisée passe maintenant par les contrats T4 de données/secrets,
puis revient à ce critère ; aucune borne locale n’est présentée comme révocation
instantanée du fournisseur.
