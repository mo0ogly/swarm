# E7 — historical acceptance qualification

The history recipe passed six test cases/subcases on isolated stores (4.352 s).
A historical accepted status remains recorded, while the engine and public CLI
report it as not fresh if independent review is missing. Reading does not rewrite
the stored work. Existing tests also cover preserved result integration retries,
explicit missing historical evidence and new candidate review requirements.

Run `node tests/engine_acceptance.cjs --case history`. Review providers in these
tests are deterministic doubles, not an independent AI assessment of this change.

Use `work show` to distinguish historical status from current freshness. Preserve
receipts, events and candidate identities. `planning retry-integration` applies
only to eligible integration failures (see REVIEW-TIMEOUT.md); it is not a general
requalification command for an old accepted task without a reviewer.

E7 is PARTIAL: a general public re-review procedure for that historical case and
an independent assessment of this delivery are still missing. No production
mission was rewritten or marked accepted by this recipe.
