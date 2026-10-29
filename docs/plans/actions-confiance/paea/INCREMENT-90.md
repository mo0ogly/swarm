# PAEA90 — consommateur web gardé du transfer/promote call

Continuité glm-resume-20261029T064841Z après l'incrément 89 (consommateur CLI
de promotion, v251). Objectif actif : le raccord web/API resté ouvert par les
incréments 85/86/89 (« CLI d'abord, web ensuite ») — joindre le transfer call
et la promotion au parcours web enrôlé, uniquement GLM, sans commit/push ni
notification intermédiaire.

## Choix de conception

Extension du mécanisme single-intent éprouvé (`TrustEnrollmentSource`, v49),
PAS un champ de réglage : un appel préparé porte des témoins physiques vivants
(instances des deux racines + nonce) impossibles à persister — un réglage ne
désignerait qu'un chemin et recréerait l'adoption implicite rejetée aux
incréments 80/83.

Replan consigné en cours d'incrément : le préparé ne porte PAS le chemin
projet (seuls témoins `RootBinding` d'inode). La racine projet est donc une
**entrée startup de confiance** : fournie à la construction de la source web,
jamais un champ de requête, et le bind compare les témoins préparés aux DEUX
racines réellement ouvertes (service du bind + projet ouvert read-only depuis
le chemin startup) AVANT toute publication de session. Ce check de transport
(`binding_matches`, ajouté sur `PaeaGitTransferCall` et `PaeaGitPromoteCall`)
précède sans jamais remplacer le recheck moteur sous l'autorité maintenue.

## Réalisation

- `rust/crates/swarm-engine/src/root_store.rs` : variantes
  `RootStoreError::PaeaGitTransferCall|PaeaGitPromoteCall(Box<…>)` (cfg linux)
  avec arms `code()`/`source()` ; Display via le défaut existant.
- `rust/crates/swarm-engine/src/root_store/paea_git_transfer_call.rs` et
  `paea_git_promote_call.rs` : accessors publics
  `binding_matches(&project, &service)` — check des DEUX témoins physiques,
  documenté « transport uniquement, ne remplace jamais le recheck moteur ».
- `rust/crates/swarm-web/src/service/paea_git_call.rs` (nouveau) :
  `PaeaGitCallSource` (préparé + service_path + project_path trusted startup,
  constructeurs `new_transfer|new_promote` sans IO, `open(declaration, pin)`
  via `authenticated_startup`, fermeture du préparé possédé à tout refus de
  démarrage avec cause retenue) ; `PaeaGitCallAuthority` (`check_roots` des
  deux témoins, `call(kind, raw, password)` : kind + sha256(corps) ==
  action_digest, ouverture writables des deux racines, recheck, consommation
  one-shot, fermetures appariées ; `close()` joint autorité + préparé sans
  perdre de cause).
- `rust/crates/swarm-web/src/service.rs` : champ `paea_git_call`,
  `open_checked_source` (check des deux racines au bind), `open_git_call`,
  variante `HttpServiceError::GitCallSource`, `close()` joignant git_call +
  enrollment + binding sans perte de cause, route
  `POST /api/v1/paea/git-call?call=transfer|promote` (header unique
  `Authorization: Enrollment <base64url>` zeroizing, `check_command_input_size`,
  refus `paea-git-call-kind-required`/`-credential`/`-request`/`-kind`/
  `-root-binding`, sans source → `paea-git-call-source-unavailable` sans
  consommation).
- `rust/crates/swarm-web/src/service/listener.rs` : `bind_git_call` ;
  `bind_source` étendu (check du projet read-only depuis le chemin startup).
- Consommation moteur dans `service.rs` :
  `call_paea_git_transfer`/`call_paea_git_promote` recheckent les deux
  témoins sous callback d'autorité ; erreurs wrappées via les nouvelles
  variantes `RootStoreError`.
- `rust/crates/swarm-web/tests/paea_git_call.rs` (nouveau) : 3 tests réels
  (1 toujours actif + 2 avec fixture native retenue writer78b).
- Documentation : `docs/architecture/PAEA-GIT-TRANSACTION.md` §
  « Consommateur web gardé (incrément 90) » + miroir EN § "Guarded web
  consumer".
- Recette `tools/verification/recipes/paea-git-call-web-rust.json` (3 étapes).

## Défauts trouvés et corrigés pendant la vérification (replan)

1. **TempDir service non retenu** : le test de consommation échouait en
   `paea-git-call-root-binding` — « exact bind refused: path ». Cause : le
   `tempfile::TempDir` service n'était pas retenu dans `Owned` (seul
   `service_path: PathBuf`) → répertoire supprimé à la fin de `owned()` →
   `RootStore::open` échouait (`RootStoreError::Path`, Display réduit à
   « path »). Établi par sonde directe dans le test avant correction ;
   corrigé en retenant `_service_dir: TempDir` (ordre des champs pour éviter
   borrow-after-move) ; test vert ensuite ; sondes supprimées.

## Preuves

- `cargo build -p swarm-engine` / `-p swarm-web` : 0 erreur.
- 3 tests web verts : refus sans source (5 refus), refus de bind sur mauvaise
  racine avant publication + bind exact réussi, consommation exacte complète
  (mot de passe erroné sans consommation, corps `{}` refusé par digest, corps
  canonique exact consommé → reçu gelé, second appel exact refusé one-shot).
- Recette `paea-git-call-web-rust` PASS 3/3, verify → integrity VERIFIED,
  preuves `evidence/20261029-git-call-web-mvfdts57`.
- Installation (décision 24 octobre) : v0.1.1-dev.253 (SHA 22d48838…) puis
  v0.1.1-dev.254 (SHA 90707b21…) après correction des notes
  version_history ; ELF réel vérifié == candidat, 0 entrée modifiée depuis le
  candidat, notes FR/EN 90/89/88 servies par /api/v1/native-version (cookie
  scopé). DÉFAUT de suivi corrigé au passage : les notes version_history des
  incréments 88 et 89 n'avaient jamais été ajoutées (top item = incrément 86).

## Limites restantes

La surface UX (exposition de `capability()`, actuellement `#[allow(dead_code)]`)
reste un incrément séparé. Séparation UID, enrôlement humain, approbation
indépendante, PA-09 complet, PA-11/12 (E3), revue indépendante et acceptation
globale restent ouverts ; ils ne sont pas fabriqués ici. Aucun commit/push ;
installation locale v253 dans la suite immédiate du même incrément.
