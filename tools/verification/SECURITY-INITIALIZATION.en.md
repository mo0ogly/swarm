# Security initialization guard

15 November status: source and guard integrated in the checkout; image380 qualified and installed locally;
the separate human Authority session remains unverified. The adapted targeted
entry passed seven scenarios with the two candidate assets. The permanent recipe
has not been run as a whole ; its steps have not been launched. This
does not by itself qualify any native image; the separate380 results are recorded below.

The test uses real Chrome and versioned HTML/scripts, with every request
intercepted: resource/session/show GETs and explicit synthetic config-preview POST
only. No backend, Authority, provider or config-apply; fixture values are test data.
Cases cover normal control, both data/layout orders, draft/token/preview retention,
initial refusal/recovery and edit-before-response/recovery. Initial hydration is
observed without Admin click; recovery requires an explicit gesture. Observable
conditions release held responses, with no sleep, retry or global timeout.
Unexpected diagnostics interrupt waits. Page/browser close errors remain failures.

After integration run from the repository root with a private JSON config
containing output (fresh admitted directory), optionally cases and asset_root:

```sh
SWARM_SECURITY_INIT_CONFIG=/private/config.json python3 tools/verification/recipe.py run --recipe tools/verification/recipes/security-initialization-ui.json --output /admitted/fresh-recipe
python3 tools/verification/recipe.py verify --evidence /admitted/fresh-recipe
```

The run/verify syntax matches the current parser; example paths require
replacement and storage admission. Default cases cover all seven; a direct Node invocation with
cases selects only new scenarios and always adds the normal valid control. The
full recipe expects seven, so a partial direct run must report its actual names.
asset_root is an explicit private-candidate source override, recorded in result;
missing files come from web/. Omit it for final repository recipe evidence. This
never establishes embedded/protected resource bytes or an installed version.

Actual served source/harness hashes are compared before/after. The shared browser
runtime helper collects Node/npm/Chrome inputs; an explicitly supplied successful
SWARM_VERIFICATION_NPM_BASELINE is fully read back and compared, without inheriting
runtime proof. Without one, normal collection applies. Keep profiles private and
exclude them from handoff archives. The recipe retains declared inputs and must
be verified with the supported verifier.

Synthetic API checks prove no backend journal/publication, human Authority session
or certification. New assets still require qualified-image/protected-path checks.
The separate380 delivery and FR/EN notes are recorded in the delivery receipt; this synthetic guard does not establish them. Historical53/152
campaigns are not rerun merely to package this focused guard.

## Review corrections ORACLE-02 and READY-03

Historical reproduce/guards/early-edit reports wrote adminClicks:0 as a constant.
Those fields are NOT MEASURED; original logs remain unchanged. The maintained
driver instruments clicks before navigation and records initial phase0 separately
from recovery phase1. No frontend observation is inferred from the legacy field.

The seventh case injects a synthetic post-assignment render failure, then delayed
layout: it is not a claim that a backend emits malformed state. The previous
source fails the no-automatic-fields assertion after a valid normal control. The
new readiness flag is cleared before each read and set only after successful
validation and render; delayed layout cannot promote partial state. No fields
automatically appear after failure; one explicit valid reread recovers. No backend
configuration mutation is established by these doubles; config-apply is refused.
Current private revision02 passes seven cases; original47-assertion proof is
historical and superseded for the changed load/readiness contract. No53/152 rerun.

Revision03 clarifies load's docstring: refusal prevents initial hydration
readiness, not every control. Existing gates and explicit Admin reread remain
unchanged. Only that block comment differs; executable bytes outside the block
match revision02. Its61-assertion evidence retains the exact revision02 sources;
no new execution or current-image qualification is inferred from this wording.

The separate native BLP check covers both orders by continuing real GETs, without synthetic API responses, then the existing four journeys. Executed on380:59 assertions PASS and recipe integrity VERIFIED.

The maintained Version check accepts clean or modified builds according to explicit expected boolean facts. It checks absence or presence of the local-change notice itself; non-boolean provenance is refused. The `version-provenance-oracle` recipe retains clean/modified controls, inconsistent notices, refusal without mutation and recovery. It establishes neither installation nor an Authority session.
