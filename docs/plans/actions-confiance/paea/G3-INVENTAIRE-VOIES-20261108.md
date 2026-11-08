# G3a — inventaire des voies de sortie (préparation confinement)

8 novembre 2026 · [English](../../../en/plans/actions-confiance/paea/G3-INVENTAIRE-VOIES-20261108.md)

Lot G3a du [plan de clôture des écarts](GAP-CLOSURE-PLAN-20261103.md) : la
première exigence de G3 est « Inventaire des voies ; tests synthétiques ;
barrières… ». Ce document établit l'inventaire — l'état vérifié des voies de
sortie d'un helper fournisseur et des titulaires d'information sensible —
pour que la campagne PA-04 « témoin puis contournement pour chaque chemin
inventorié » vise exactement ces chemins. Incrément d'analyse lié aux octets
lus ; aucun code modifié, aucune barrière nouvelle posée par ce document.

Attribution concurrente : la frontière commune I0 (BT02) est committée
(`6706e201b`) ; le refus Internet additif opérateur (`provider_internet_denied`,
`deny_internet`) est un flux en cours **non commité** au moment de cet
inventaire (arbre de travail partagé, propriétaire : flux G3 barrières). Les
constats ci-dessous le décrivent tel qu'observé dans l'arbre, sans présumer sa
livraison ni rejouer ses preuves.

## 1. Barrière obligatoire et point d'exécution commun

Un unique point d'exécution confine déjà tous les helpers CLI fournisseur :

- Profil Landlock ABI6 + seccomp : `rust/crates/swarm-biba-linux/src/kernel.rs`
  (`ruleset()` ligne 17 — `Ruleset{fs:ALL, net:0, scoped:3}` ; `seccomp()`
  ligne 53 — deny-list : ptrace/`process_vm_*`/`pidfd_getfd`, montages,
  `chroot`/`setns`/`unshare`, `open_by_handle_at`, io_uring/bpf/perf,
  trousseau de clés, métadonnées chmod/chown/utimensat/xattr, `socket`/
  `socketpair` AF_UNIX ; ioctl limité à FIOCLEX/FIONREAD/FIONBIO/TCGETS/
  TIOCGWINSZ).
- Le lecteur d'ancêtres `read_leaves` (lib.rs ligne 41) autorise la lecture de
  tout le système de fichiers **sauf** la racine privée `<root>/.swarm` ;
  `prepare_bound` (ligne 76) épingle le workspace par (dev, ino) et refuse les
  chevauchements racine/privée, hardlinks et alias de liens symboliques.
- `Sandbox::enforce` (ligne 110) exige helper mono-thread, non privilégié
  (`CapEff:0000000000000000`), entre par fd épinglé, pose `no_new_privs`,
  `landlock_restrict_self`, `close_range(3.., CLOEXEC)` puis seccomp. Tout
  échec impose la sortie du helper — aucun repli non confiné n'existe.
- Point d'exécution partagé : `PreparedExec::execute`
  (`rust/crates/swarm-engine/src/native_provider_exec.rs:207`) — `biba::prepare`
  → dup2 stdin (memfd scellé contenant le prompt) → `biba::enforce` → execve
  direct sans fallback shell/PATH/ENOEXEC.

Cinq lanceurs convergent vers ce point, tous derrière un canal privé avec
octet d'octroi, identité parent (pidfd/démarrage) et pair-credentials :

| Lanceur | Helper CLI | Gate (côté enfant) |
|---|---|---|
| Agents | `_agent_gate` (`agent_supervision.rs:357` spawn) | `agent_execution_gate.rs:222` |
| Planification | `_planning_gate` (`planning_native.rs:1358`) | `planning_native.rs:473` |
| Validation | `_validation_gate` (`task_validation/process.rs:454`) | `task_validation/process.rs:159` — commande admise du candidat, workspace géré obligatoire |
| Revue | `_reviewer_gate` (`reviewer_native/runtime.rs:184`) | `reviewer_native/gate.rs:199` |
| Préparation CLI | `_prepare_cli_exec` (`preparation_call_cli.rs`) | `preparation_call_cli.rs:620` |

## 2. Secrets : titulaires et voies d'accès

| Secret | Stockage | Voie d'accès inventoriée |
|---|---|---|
| Clé de connexion API (`ConnectionState.key`, `provider_connection_catalogue.rs:22`) | Root SQL privée | Construite en mémoire (`preparation_call.rs:1021-1022`) et transmise au worker API (§4). Jamais en argv, jamais dans les diagnostics (`native_provider_exec.rs:20`). |
| Clé web du cockpit (owner key) | Racine privée `.swarm` | Non lisible par un helper confiné : `.swarm` est exclu de `read_leaves`. Voie résiduelle : réseau loopback du helper (§5). |
| Tickets de connexion web (`issue_web_login_ticket`, root_store.rs:2146) | Fichier privé d'expiration | Même exclusion `.swarm` ; un ticket consommé est retiré. |
| Jetons télémétrie (`telemetry_store.rs`) | Racine privée | Variables d'environnement calculées moteur (`agent_execution_gate.rs:172-190`), jamais en argv. |
| Prompt/capsule d'envoi | memfd scellé (fd 0) | Sealed `SEAL\|SHRINK\|GROW\|WRITE` (`native_provider_exec.rs:65-105`) — le helper ne peut ni tronquer, ni réécrire, ni grandir son stdin ; seule lecture. |
| Données I0 de la tâche | Workspace I0 attribué | Seule feuille inscriptible ; instanciation par (dev, ino) gelée à la préparation (`biba-workspace-instance-changed`). |

Le profil seccomp refuse `ptrace`, `process_vm_readv/writev`, `pidfd_getfd`,
`add_key`/`request_key`/`keyctl` : un helper ne peut ni lire la mémoire d'un
autre processus du même UID, ni manipuler le trousseau noyau.

## 3. Voies résiduelles du helper confiné (cibles des tests synthétiques PA-04)

Chacune de ces voies est **réelle et non médifiée** — c'est là que la campagne
« témoin puis contournement » doit mesurer :

1. **Réseau sortant du helper CLI** : `net:0` ne restreint pas le réseau
   (attribut Landlock réseau jamais posé) ; un helper confiné peut créer des
   sockets AF_INET/AF_INET6. Le refus additif `deny_internet` en cours (flux
   étranger non commité) pose un refus seccomp cumulatif optionnel côté
   moteur, gelé par `provider_internet_denied_snapshot()` à la porte. Cette
   voie reste la cible PA-05 numéro un.
2. **Descripteurs hérités** : `close_range(3.., CLOSE_RANGE_CLOEXEC)` marque
   CLOEXEC, ne ferme pas. Un fd passé intentionnellement hérité (le canal de
   diagnostic) reste accessible — par conception ; tout autre fd hérité est
   un écart à reproduire.
3. **Environnement** : `env_clear` + liste explicite (`task_validation/biba.rs`
   et transport commun) ; les valeurs observées proviennent du lanceur, un
   nom absent reste absent ; conflit de variable moteur refusé
   (`agent-execution-environment-conflict`). Toute fuite héritée au-delà de la
   liste est un écart.
4. **`/proc`** : cmdline de l'hôte et des pairs lisible (constat mémoire du
   31 octobre re-vérifié : pas de médiations read procfs) — argv visible aux
   pairs du même UID. Les arguments porteurs de secret sont refusés en argv
   (`add_leading_arguments` exige sans secret).
5. **Fichiers hors racine privée** : lecture générale autorisée par
   `read_leaves` (home inclus : cookies de navigateur, clés SSH). Seule
   l'écriture est bornée au workspace I0. Constat C07 : « pas de confinement
   général des lectures/secrets ».
6. **Descendants** : le profil est hérité ; un enfant ne peut pas retrouver de
   droits (pas de `unshare`/`setns`/montages). Un descendant hérite aussi de
   la voie réseau (§3.1) et des lectures générales (§3.5).

## 4. Worker API (`_prepare_turn`) — hors frontière I0

Le worker de préparation `api-` (`preparation_worker.rs`, commande CLI
`_prepare_turn`, spawn `preparation_dispatch` `preparation_call.rs:851`)
**n'est pas confiné par le profil Biba** : il détient la clé de connexion
(`PreparationApiRequest.key`, `preparation_call.rs:237` alimenté depuis
`ConnectionState.key`) et doit joindre le vrai endpoint. Il est encadré par :
jeton nonce à usage unique via stdin fermé après lecture (ligne 13-44),
attribution PID/stamp avant libération du nonce (lignes 855-901), verrou
egress partagé à la réclamation et au dernier usage
(`preparation_worker_transport_gate`, `preparation_transport.rs:37` — re-vérifie
quarantaine, admission, attribution destination, no-write-down avant
toute ouverture de socket). Limite déclarée : c'est un contrôle applicatif,
pas un confinement OS — la clé est dans la mémoire du worker et dans les
requêtes sortantes, par nécessité fonctionnelle. Un écart PA-04/PA-05 ici est
un écart applicatif, pas un contournement noyau.

## 5. Transport moteur (serveur web) — superposition à l'inventaire helper

`rust/crates/swarm-web/src/access.rs` : liaison écoutée exclusivement
127.0.0.1/::1 (`from_listener` ligne 105 et suivants), session requise
(format Go `session-`+24). Les envois fournisseurs côté moteur passent le
verrou egress partagé (`with_preparation_egress`,
`preparation_confidentiality.rs:33`). La quarantaine
`preparation_quarantine` bloque l'egress contrôlé API/CLI, pas l'OS
(`swarm-confidentiality/src/lib.rs`, doc du champ).

## 6. Ce que cet inventaire ne conclut pas

- Aucune voie de §3-§5 n'est déclarée fermée ; chaque ligne est un chemin à
  témoin valide puis à contournement refusé (PA-04), avec octets observés à
  destination (PA-05).
- Le refus Internet additif observé n'est ni commité ni qualifié ; son
  propriétaire livre tests et preuves (`rust_biba_linux_boundary_internet.py`
  non commité visible dans l'arbre).
- Les prérequis AC-02 (helpers/mappings/comptes, H1) restent humains ; la
  qualification G3 barrières reste derrière G1+G3a (graphe de qualification).
- BT03/BT04 (contrôles depuis données I0, labels/promotions) restent ouverts
  dans [BIBA-TRANSVERSE-APEX](../BIBA-TRANSVERSE-APEX.md) ; cet inventaire
  fournit leurs entrées sans clôturer leurs critères.

## 7. Suite concrète (reprise)

1. Le propriétaire du flux `deny_internet` livre et commit sa tranche (hors
   périmètre de ce document).
2. Les tests synthétiques G3a par voie de §3 s'écrivent sur le modèle
   `rust_biba_linux_boundary.py` (témoin non confiné d'abord, refus ensuite,
   preuves sous `.swarm-fixtures/`), chacun relié à une ligne de §3.
3. Le parcours PA-05 réseau attend le verdict du flux en cours ; la voie
   lecture générale (§3.5) demande une décision opérateur de portée (AC-02)
   avant barrière.
