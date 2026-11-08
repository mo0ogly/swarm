# PAEA88 — consommateur CLI gardé du transfer call (deux phases interactives)

Reprise 06:48 UTC du 2026-10-29 (dossier glm-resume-20261029T064841Z) après
closion de l'incrément 83. Objectif actif : prochain incrément utile du plan
PA00–PA13, uniquement GLM, sans commit/push ni notification intermédiaire.
L'état courant laissait le raccordement consommateur du `paea_git_transfer_call`
ouvert (motif CLI d'abord, puis web — même ordre que les incréments 85/86).

## Choix de conception

Le motif deux phases de `trust enrollment-call` est appliqué tel quel :
l'appel préparé détient des témoins mémoire (instances physiques des racines,
nonce) non sérialisables, donc préparation et consommation ont lieu dans le
même processus CLI ; l'opérateur provisionne la permission exacte hors bande
entre les deux (aucun auto-grant : `Document.permissions` de l'enrôlement
signé accepte `paea-action-v1/...`, le CLI n'émet que la permission à
provisionner). L'opération préparée reste Reconcile : aucun effet de ref,
cohérent avec la frontière CONTINUITE-80 (le Commit réel reste un incrément
séparé avec ses prérequis propres).

Grammaire : `paea transfer-call SERVICE PROJECT WORK CHILD PIN STAGING
DECLARATION --input REQUESTFILE` — neuf positions car la déclaration protégée
provisionnée de l'opérateur est un chemin supplémentaire (le pin de journal Git
reste un argument public, la frame privée porte le pin de déclaration + le
mot de passe personnel). stdin ne doit pas être un terminal.

## Réalisation

- `rust/crates/swarm-cli/src/paea_transfer.rs` (nouveau) : ouverture writables
  service puis projet (chaque refus ferme les acquéreurs précédents en
  retenant leur cause), préparation via `RootStore::paea_git_transfer_call`,
  ligne publique `paea-transfer-prepared`, lecture frame zeroizing via
  `personal_secret_frame::read_packet` (borne transport 16 MiB existante),
  refus `paea-transfer-call-frame`/`-input` avant toute ouverture de
  déclaration, démarrage autorité `authenticated_startup::open` sur la
  déclaration, consommation `call_paea_git_transfer`, ligne
  `paea-transfer-result` (candidate, pack_bytes, pack_sha256). Fermetures
  explicites service/projet/préparé/autorité sur toute issue, chaque échec de
  fermeture retenu (RootCleanup apparié, motif `paea.rs`).
- `rust/crates/swarm-cli/src/lib.rs` : variante `CliError::PaeaTransfer`
  (code/Display/source via `PaeaGitTransferCallError`), enregistrement du
  module, dispatch `paea transfer-call`, USAGE + HELP_FR/HELP_EN avec la
  nouvelle commande.
- `rust/crates/swarm-cli/tests/paea_transfer.rs` (nouveau) : harnais deux
  phases réel — fixture enrôlement signée courante
  (`Fixture::build_for_trust` sur la permission exacte émise), déclaration
  copiée 0600 après l'événement prepared, frame sur stdin privé, assertions
  d'absence du mot de passe dans stdout/stderr.
- Documentation : `docs/architecture/PAEA-GIT-TRANSACTION.md` §
  « Consommateur CLI gardé (deux phases interactives) » et miroir EN
  `docs/en/PAEA-GIT-TRANSACTION.md` § "Guarded CLI consumer".
- Recette : `tools/verification/recipes/paea-transfer-call-cli-rust.json`
  (deux étapes, env SWARM_PAEA_SOURCE_FIXTURE déclarée).

## Deux défauts trouvés et corrigés pendant la vérification (replan)

1. Indices de grammaire : la grammaire avait été étendue à neuf positions mais
   `run()` ouvrait encore pos[1] (« transfer-call ») comme racine service →
   échec `{"error":"path","causes":["path","io-kind:NotFound","os-errno:2"]}`
   après reconstruction. Cause établie par instrumentation temporaire
   (DIAG-*, retirée) puis indices corrigés (project=pos[3], service=pos[2],
   work/child/pin/staging=pos[4..7], déclaration=pos[8]).
2. Contrat de frame : le check exigeait `packet.len()==cut+1` (pin seul, motif
   serve de `trust enrollment-serve`) alors que le transfer call porte
   toujours un password après le séparateur ; désormais
   `packet.len() > cut+1` + header + séparateur + pin hex64. Un pin seul ou un
   mauvais header refuse sans consommation.

Leçon de procédé confirmée : un `cargo test` peut ne pas relinker la lib
modifiée (rmeta plus ancienne que le src corrigé) — comparer les mtimes
src/artefact avant de conclure à un échec sémantique ; forcer la
reconstruction (touch + build) quand le doute existe.

## Preuves

- `cargo test -p swarm-cli --test paea_transfer -- --ignored` → 2 passed :
  `transfer_call_cli_consumes_exact_preparation_once` (consommation exacte
  une fois, reçu figé rapporté) et
  `transfer_call_cli_refuses_bad_frame_without_consumption` (stderr
  `paea-transfer-call-frame`, trust-actions.json inchangé, aucun stdout).
- Recette `paea-transfer-call-cli-rust.json` : PASS 2/2 étapes, preuves
  `evidence/20261028-acquisition81-mvfdts57/glm-resume-20261029T064841Z/
  recipe-paea-transfer-call-cli`, `verify` → integrity VERIFIED (08:38 UTC).
- Binaire réel reconstruit : help FR/EN et USAGE listent `paea
  transfer-call` (vérifié dans `rust/target/debug/swarm-rust`).

## Limites restantes

Aucune installation de service n'a lieu dans cet incrément (le consommateur
est un CLI opérateur ; voir décision locale du 24 octobre pour l'installation
de l'avancée). Raccord web/API (motif inc 85/86 : CLI d'abord), effet de
Commit réel (CONTINUITE-80), PA-09 complet, E3, revue indépendante et
acceptation globale restent ouverts ; ils ne sont pas fabriqués ici.
