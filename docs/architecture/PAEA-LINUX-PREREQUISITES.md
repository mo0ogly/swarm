# Prérequis Linux de PAEA

Le contrôle disponible est une **observation**, pas un certificat de confinement
ni une autorisation de lancement. Depuis le checkout :

```sh
python3 tools/paea/linux_prerequisites.py
python3 tools/paea/linux_prerequisites.py --output /tmp/paea-prerequis-nouveau
```

Le dossier de sortie doit être neuf. Le lecteur observe euid/gid/groupes, les
mappings du namespace courant, les plages propres dans `/etc/subuid` et
`/etc/subgid`, et la garde de fichier des helpers installés. Il ne choisit aucune
UID, ne réserve aucune plage et ne lance aucun namespace, sudo, paquet, compte ou
service. Un échec de lecture ou format invalide reste distinct d'une liste vide.
Les observations de namespace ne sont pas des descripteurs détenus ni des identités
durables. Le rapport ne contient ni mot de passe, ni clé, ni preuve d'installation.

`launch_verified`, `separate_host_uid_verified`, `confinement_verified` et
`human_verified` restent faux, même si les helpers et plages sont présents. Leurs
métadonnées seules ne prouvent pas leur fonctionnement, une attribution exclusive,
le nettoyage des groupes/capacités/FD, les montages, le réseau ou la reprise.

## État constaté le 27 octobre 2026

Le processus contrôleur observé utilise euid1001, sans capacité effective. Les
plages de cet utilisateur existent :165536+65536 pour UID et GID. `unshare` et
`bwrap` sont présents. Deux essais éphémères, précédés d'un témoin valide sur un
fichier synthétique0700/0600, ont donné UID0 **interne**, mappée vers UID1001
**externe**, et conservé la lecture du fichier privé. Ce profil ne sépare donc pas
les clés d'un agent partageant cette UID. Il n'est pas un remplacement admissible
pour le profil protégé exigeant une UID distincte.

`newuidmap` et `newgidmap` sont absents ; le paquet `uidmap` n'est pas installé.
Le catalogue local propose1:4.17.4-2ubuntu3. La lecture de deux préférences apt a
été refusée à l'utilisateur ; ce constat ne remplace pas le choix fait par apt
avec les droits administrateur. `sudo --non-interactive --list` répond qu'une
authentification interactive est requise. Aucun privilège ni système modifié.

## Action administrateur délimitée

L'administrateur de cette machine peut installer le paquet des deux helpers :

```sh
sudo apt-get install uidmap
```

L'authentification se fait dans son terminal local ; ne transmettre aucun mot de
passe au chat, à un fichier du projet ou à une recette. Cette commande n'est pas
exécutée par le contrôle et ne crée pas de compte/service PAEA. Elle ne donne pas
un mandat métier. Après installation, relancer le lecteur et conserver un nouveau
rapport ; ne pas modifier les rapports ou preuves antérieurs.

La suite reste à vérifier par le lanceur prévu : helpers réels détenus/épinglés,
mappings autorisés, UID externes effectivement distinctes, suppression des groupes
et capacités non nécessaires, racines/FD détenus et hors portée de l'agent,
montages/PID/réseau/descendants et reprise. Aucune plage ni UID n'est choisie par ce
guide. L'activation du service et l'enrôlement humain restent des opérations
séparées. Sans cette preuve, l'usage protégé doit refuser ; aucun fallback vers
l'UID partagée et aucun changement automatique de sysctl/politique d'hôte.

[Plan et preuves45](../plans/actions-confiance/paea/INCREMENT-45.md).

## Création atomique disponible dans la bibliothèque GNU

`swarm_linux_identity_channel::atomic_spawn::launch` crée le processus et son
pidfd en une seule opération GNU `pidfd_spawn`, résolue dans la libc déjà chargée.
Les symboles requis et pidfs64 sont contrôlés avant invocation ; absence signifie
refus, sans capture ultérieure par PID. La capture conditionnelle d'un `Child`
existant reste une API distincte et ne prouve pas cette atomicité.

L'appelant fournit un exécutable et un répertoire absolus, argv non vide,
l'environnement exact, les trois flux POSIX et les ensembles de signaux explicites.
Aucun héritage d'environnement, recherche PATH ou shell n'est ajouté. Les flux sont
dupliqués ; les autres descripteurs hérités sont fermés dans l'enfant. Aucun compte,
changement de credentials, confinement, approbation ou politique opérateur n'est créé.
Le consommateur doit établir ces préconditions avant l'appel.

`Spawned` conserve le handle même après une sortie rapide. `try_wait` et `wait`
observent ce pidfd ; une récolte externe peut produire ECHILD, jamais un succès vide.
Toute erreur après création conserve le propriétaire dans `Failure.child`, avec les
causes indépendantes de validation et nettoyage. `invoked` sans handle ne prouve pas
l'absence d'un enfant transitoire ou d'effets. `close`/Drop ferment seulement le FD :
attente, annulation et reprise appartiennent explicitement au consommateur.

`Service::dispatch_atomic_controller_packet` exige ce type atomique puis conserve
les contrôles UID, processus, domaine de namespaces, décodage et autorité courante.
Cette bibliothèque n'est pas encore un daemon installé ni une preuve de deux UID.
La recette `paea-atomic-controller.json` couvre neuf nouveaux tests Rust ; la recette
`paea-atomic-controller-native-trace.json` exige CLONE_PIDFD et waitid(P_PIDFD), sans
pidfd_open du PID enfant. Ces preuves locales ne valent pas acceptation indépendante.
[Plan et preuves60](../plans/actions-confiance/paea/INCREMENT-60.md).

## Retour de réponse sur un paquet

`Channel::send_packet` envoie un enregistrement non vide par un seul `send`, après
relecture du type de socket et des options requises. Il n'ajoute aucun descripteur
ni credentials déclarés. `MSG_NOSIGNAL` évite SIGPIPE pour cet appel seulement ;
aucune disposition globale n'est modifiée. Le blocage suit les flags du socket
fourni : le consommateur configure ses politiques avant usage, sans délai/tentative
ou plafond de taille caché dans la bibliothèque. Les limites de transport et errno
restent distinctes d'une politique opérateur.

Le succès signifie acceptation locale du nombre exact d'octets, jamais réception
ou admission du pair. `SendFailure` conserve cause, tentative native et éventuel
compte accepté après retour court inattendu ; aucune erreur ne prouve un rollback.
Le socket et le buffer restent à l'appelant, sans copie, fermeture ou reprise.

`service_response::send_packet` encode l'observation canonique puis appelle ce
transport exactement une fois. Réponse originale et observation sont conservées
aussi après refus d'encodage ou d'envoi. Lire le résultat métier indépendamment de
la cause de sortie, notamment après un effet déjà enregistré. L'ancienne API
`service_response::send` sur `Write` conserve son contrat write_all/flush ; ne pas
l'utiliser comme adaptateur implicite qui fragmenterait un paquet.

Recette `paea-packet-response.json` : aller-retour/deux enregistrements, refus avant
envoi et reprise, EPIPE réel, SIGPIPE par défaut débloqué dans un worker isolé,
consommateur lié à la demande et résultat conservé/relu après échec. Aucun nouveau
daemon, pair authentifié, compte distinct ou effet distribué n'est établi.
[Plan61](../plans/actions-confiance/paea/INCREMENT-61.md).
Sources primaires : [send(2)](https://man7.org/linux/man-pages/man2/send.2.html),
[socket(2)](https://man7.org/linux/man-pages/man2/socket.2.html).

La recette observe également `SO_SNDBUF` puis vérifie EMSGSIZE sans paquet reçu,
et la saturation réelle en mode non bloquant (EAGAIN), le vidage exact et la reprise.
Elle ne définit aucun plafond applicatif ou nombre arbitraire de tentatives.

## Inspection par le parent propriétaire

Un contrôle local distingue les capacités effectives dans le namespace du parent
et les capacités que celui-ci possède dans un namespace utilisateur enfant dont
il est propriétaire. La source Linux6.17 décrit ce second droit ; les ioctls
pidfd de namespaces conservent leur contrôle `PTRACE_MODE_READ_FSCREDS`.

La recette `paea-namespace-owner.json` utilise deux profils éphémères avec **la
même UID externe** et le jeu effectif de capacités du parent nul :

| Profil de l'enfant | dumpable=1, témoin | dumpable=0 | restauration à1 |
| --- | --- | --- | --- |
| Namespace utilisateur du parent | accès | EACCES | accès |
| Namespace utilisateur enfant et montages privés | accès | accès | accès |

Les six phases sont observées via le pidfd atomique du processus réellement
émetteur et les objets de namespaces détenus. Cela vérifie une permission locale
d'inspection, sans créer de droit dans le namespace parent. Ce n'est ni une
preuve inter-UID, ni un profil de déploiement : `--map-root-user` conserve ici
l'UID externe et ne sépare pas les clés du service. Les helpers de mapping,
la vérification de deux UID distinctes et le confinement restent requis.
Ne pas en déduire qu'une capacité globale doit être accordée au service.

Un second test conserve l'erreur native d'un enfant après un témoin valide :
le puits nul la perd, la capture privée la garde. L'exécutable de test est figé
et identifié ; ses canaux et son résultat sont liés aux empreintes de la recette.
L'altération d'une copie des preuves est refusée. Ces tests ne livrent aucun
daemon ni nouvelle API de production ; la livraison61 reste suivie séparément.

[Preuves62](../plans/actions-confiance/paea/INCREMENT-62.md) ·
[Propriétaire d'un user namespace](https://man7.org/linux/man-pages/man7/user_namespaces.7.html) ·
[Linux6.17, capacités](https://raw.githubusercontent.com/torvalds/linux/v6.17/security/commoncap.c) ·
[contrôle ptrace](https://raw.githubusercontent.com/torvalds/linux/v6.17/kernel/ptrace.c) ·
[ioctls pidfs](https://raw.githubusercontent.com/torvalds/linux/v6.17/fs/pidfs.c).
La référence amont n'est pas le source exact du noyau Ubuntu observé ; le témoin
local est limité au noyau6.17.0-41-generic exercé.

La règle de base du propriétaire porte sur son UID et le namespace parent, pas
sur le seul PID du lanceur. Cette UID doit rester hors du domaine agent ; placer
une autorité dans un namespace contrôlé par une UID non fiable ne protège pas
ses clés par le seul changement d’UID interne. Cette conséquence de la règle
amont reste à éprouver avec un observateur non parent et deux UID externes dans
le profil de lancement retenu ;62ne prouve pas ces variantes.

## Un échange contrôlé complet

`Service::exchange_atomic_controller_packet(ctx, channel, buffer, controller,
expected)` reçoit exactement un paquet, applique le contrôle atomique existant,
ferme explicitement le paquet puis envoie une réponse canonique sur le même canal.
L'appelant conserve le canal, le contrôleur et les namespaces approuvés ; il fournit
la capacité de réception et gère flags, blocage, annulation et nettoyage final.
Aucune boucle, initialisation implicite, tentative ou politique nouvelle.

Un refus de réception ne déclenche pas le dispatch. Un refus de dispatch ou de
fermeture du paquet supprime l'envoi, même si un résultat métier est déjà connu.
L'erreur typée conserve ce résultat via `known_reply()` et toutes les branches
indépendantes via `error_branches()`, dont les deux erreurs `AfterUse`. La fermeture
du paquet est toujours tentée après le dispatch. Une erreur d'envoi conserve la
réponse et son observation. Une réponse absente ne prouve ni absence d'effet ni
permission de rejeu ; un succès exige encore de lire `Sent.reply`.

La recette `paea-controlled-exchange.json` couvre l'échange noyau réel, un enfant
atomiquement identifié avec domaine parent capturé indépendamment, les refus sans
sortie, la reprise et la conservation des causes. Le callback métier positif reste
synthétique/same-UID ; le Service public couvre le refus UID sans mutation puis
réouverture. Ni daemon protégé ni succès métier entre deux comptes n'est établi.
[Plan et preuves63](../plans/actions-confiance/paea/INCREMENT-63.md).

## Observation native version2 — livraison locale216

Les sources proposent `identity linux-prerequisites --json [--lang fr|en]`,
`GET /api/v1/protection/linux-prerequisites` et **Prérequis Linux de PAEA**, liée
aux Vérifications PAEA. Ces interfaces sont livrées et vérifiées localement dans216 ; consulter
le [plan64](../plans/actions-confiance/paea/INCREMENT-64.md) et le dialogue Version
pour la livraison qualifiée. L'observation n'active aucune protection et ne lance
ni helper, interpréteur, service, compte, mapping ou requête NSS réseau.

Le composant `swarm-linux-prerequisites`, réexporté par
`swarm_engine::linux_prerequisites`, produit le même type pour CLI et API.
Il lit UID/GID/groupes du noyau, passwd **local** (sans NSS), seuls subuid/subgid du
nom ou UID courant, uid_map/gid_map courants et premier candidat exécutable du PATH.
Les erreurs IO gardent leur cause typée ; le JSON expose seulement code/errno.
Un format invalide et une source indisponible restent distincts d'une liste vide.
Les plages valides vides sont affichées comme telles. Aucun utilisateur, plage,
réservation, configuration ou identité de lancement n'est choisi automatiquement.

Le schéma `schemas/paea/linux-prerequisites-v2.schema.json` décrit cette version2,
distincte du format1 Python historique. Il contrôle la forme, pas les droits ni
l'effet. Les helpers de ce lecteur suivent le profil de métadonnées setuid root :
fichier régulier, bits exécution/setuid, propriétaire et ancêtres root non
modifiables par groupe/autres. ACLs, capacités de fichier, exécution effective,
custodie stable et exclusivité ne sont pas établies. La page refuse les
contradictions élémentaires d'un booléen `metadata_safe` ; elle ne prouve pas la
protection. Tout est local au namespace du processus observé, sans FD détenus.

La commande exige JSON explicite et refuse les entrées/Root/sorties/options
incompatibles, même avec valeur par défaut. Elle observe le processus CLI.
L'API observe le **serveur**, sous Host/session, par GET sans query ni body ; les
autres méthodes et sélecteurs sont refusés, sans lecture de projet ni effet Root.
Le navigateur observe ce rapport du serveur, jamais sa machine ni un agent futur.
La page garde les erreurs/préconditions visibles, efface un résultat périmé sur
échec, conserve la navigation au clavier et exporte les octets validés uniquement.
Même sans métadonnée manquante, les quatre preuves lancement/UID distincts/
confinement/humain restent fausses. Les privilèges ou acceptations restent des
opérations séparées ; aucune reprise permissive sur UID partagée.

Depuis la version216, ouvrir **Vérifications PAEA → Prérequis Linux de PAEA** puis
**Observer le serveur Linux**. Pour le processus CLI depuis le checkout installé :

```sh
./bin/swarm identity linux-prerequisites --json --lang fr
```

Les réponses réseau doivent être UTF-8 valides sans BOM : le lecteur conserve les
octets valides et refuse le remplacement Unicode ou la suppression silencieuse du
BOM. Un échec efface le précédent résultat/export ; un nouveau témoin valide peut
être demandé. Recettes maintenues : `paea-linux-prerequisites-core.json`,
`paea-linux-prerequisites-ui.json`, `paea-linux-prerequisites-native.json`.
Leur portée reste locale/observatoire. Les captures de216 attestent précisément
son inventaire compilé ; des sources ultérieures du dépôt peuvent rester non livrées.
