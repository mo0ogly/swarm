# PAEA91 — observation de capacité web du git-call

Continuité glm-resume-20261029T064841Z après l'incrément 90 (consommateur web
gardé, installé v254). Limite 90 levée : `PaeaGitCallAuthority::capability()`
portait `#[allow(dead_code)]` (« surface UX plus tard ») et la route ne savait
querefuser `paea-git-call-source-unavailable` — l'opérateur ne pouvait pas
observer qu'une source est liée sans consommer l'appel préparé one-shot,
confondant configuré/appliqué/observé (exigence PA-08).

## Réalisation

- `rust/crates/swarm-web/src/service/paea_git_call.rs` :
  - `paea_git_call_status()` : `GET /api/v1/paea/git-call` sous la même garde
    session/Host/CSRF retourne la vue JSON v1 `paea-git-call-status` —
    `bound:false` + capability nulle sans source liée (200 explicite, PAS un
    refus), sinon la capacité publique de démarrage (`scope`,
    `single_prepared_git_call`, `call`, `service`, `operation`,
    `root_digest`, `action_digest`). Aucun IO de racine, aucune
    consommation, aucun secret.
  - `#[allow(dead_code)]` de `capability()` supprimé (consommateur réel).
  - GET/HEAD sur la route existante délégant à l'observation ; POST et autres
    méthodes inchangés.
- Motif : `identity_authentication.held_enrollment` de la vue show trust
  (service/trust.rs) — enrichir une vue publique de la capacité détenue sans
  authentifier l'appel suivant ni promettre que la permission reste courante.
- Tests `tests/paea_git_call.rs` : sans source → 200 `bound:false` + capability
  nulle ; avec source → capacité exacte (`scope`, `call=transfer`,
  `action_digest` du préparé, capturé avant le move de la source) ; la
  consommation one-shot et ses refus restent inchangés.
- Docs : `docs/architecture/PAEA-GIT-TRANSACTION.md` § observation
  (incrément 91) + miroir EN.

## Vérification

- 3 tests web PASS (4.20 s) sur la fixture native writer78b.
- Recette `tools/verification/recipes/paea-git-call-web-rust.json` réexécutée
  → PASS 3/3, `recipe.py verify` → integrity VERIFIED ; preuves
  `evidence/20261029-git-call-status-mvfdts57`.

## Limites restantes

L'observation n'est pas un état moteur persisté : elle reflète la source liée
au démarrage du service, pas la validité courante de la permission (le recheck
moteur sous autorité reste le seul arbitre à la consommation). La surface
navigateur (Admin/cockpit) de cette vue reste un incrément séparé. Séparation
UID, enrôlement humain, approbation indépendante, PA-09 complet, PA-11/12
(E3), revue indépendante et acceptation globale restent ouverts ; ils ne sont
pas fabriqués ici. Aucun commit/push ; installation locale v255 dans la suite
immédiate du même incrément.
