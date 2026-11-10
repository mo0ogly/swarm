# Codex private signal IPC / canal privé de signaux

Correctif source proposé pour Tokio 1.52.3 utilisé par Codex 0.166.0 : remplacer
uniquement son canal de réveil de signaux UnixStream par un pipe anonyme
non bloquant/CLOEXEC. Les filtres Swarm, droits filesystem, IPC externes, réseau,
approbations et sandbox Codex ne doivent pas changer. Pas de shim ni de patch
du binaire installé ; candidat compilé séparément, jamais déclaré livré sans
vérification de l’image, de ses ressources et du parcours réel.

`probe/` reproduit l’initialisation, la réception SIGUSR1, le reap d’un enfant
et la recréation d’un réacteur. Sous le profil réel, la version amont doit
échouer ; le candidat doit réussir en conservant les refus UnixStream,
UnixDatagram et écriture hors workspace. Un témoin non confiné précède la faute.
Aucun modèle n’est appelé par ce composant ; il ne qualifie pas le vrai parcours.

Le suivi canonique et les preuves sont dans
`docs/plans/actions-confiance/REAL-MODEL-TOOL-QUALIFICATION.md`, reprise RF.
Les sources amont/crates vérifiées sont conservées dans l’aire de build ; le
patch et la procédure de reconstruction doivent rester versionnés ici.

Sources : Codex tag `rust-v0.166.0`, Tokio `1.52.3` au checksum du Cargo.lock.
`tokio-1.52.3-signal-pipe.patch` modifie les trois fichiers du pilote privé ;
`patch-inputs.json` lie octets amont et candidat. Le pilote conserve une extrémité
de lecture dupliquée/possédée par réacteur ; Mio assure son deregister/drop.

`codex-0.166.0-alias-permissions.patch` évite seulement le chmod redondant d’un
répertoire déjà exactement 0700. Un mode incorrect exige toujours la correction
réussie, sinon le setup refuse. `client-patch-inputs.json` lie ce changement.
Cela traite le warning d’alias observé sous le filtre métadonnées inchangé.

Le candidat utilise l’entrée officielle `codex-exec`, qui inclut le même moteur
et le helper Linux. Le raccord attesté retire uniquement le mot `exec` de la
CLI racine, conserve tous les arguments de protection et exécute l’image par FD
après son hash complet. Il ne choisit ni ne lance lui-même l’outil du modèle.

Le build V8 garde le commit exact
`350c5a859a9b3c84ab8701b3ad51e3b7699b30c0` ; une archive de cet arbre remplace
seulement la récupération Git récursive des historiques. Le build amont utilise
normalement une bibliothèque précompilée et ses bindings ; aucune feature V8,
`v8_enable_sandbox`, mode de build ou garde ne doit être désactivé pour passer.

Le probe vérifie aussi la véritable API `ctrl_c` et la stabilité des descripteurs
après recréation du réacteur. Ces contrôles restent des preuves de composant,
distinctes du client complet et du vrai modèle.

English: source-only candidate replacing Tokio's private signal wakeup socket
with a nonblocking close-on-exec pipe. Native/Codex protections must remain
unchanged. The probe checks real signals, child reaping and reactor recreation,
plus unchanged socket/filesystem refusals. No real-model or delivery claim.
