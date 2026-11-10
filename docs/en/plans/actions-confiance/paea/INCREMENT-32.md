# PAEA32 — durable personal intent/checkpoint coordinator

30dev128 installed;31has43tests/5steps and Rustdoc PASS, normal dev129build in
progress. No effective31sources changed during compilation. Planned, not delivered.
PA-02requires exact durable intent before one-time bootstrap consumption and
independently pinned checkpoint after journal acknowledgement. Use a separate
coordinator over Source+Documents instead of mixing filesystem recovery into the
signed authority state machine. Codex owns technical work; human/E2/E3 independent.

32a: explicit paired cancellation contexts (files/journal), no timer/default.
Canonical closed checkpoint binds independently selected Expected/key/project;
positive head, bootstrap or account+pin, pending exact proposal only pre-account.
Reject context/schema/phase/head substitution or missing state; never learn head
from DB.32b: explicit initialization requires independent bootstrap head and real
active context validation, creates only absent checkpoint. No protected key/proof
provisioning or out-of-agent human delivery is fabricated.
32c: reserve files, validate checkpoint, prepare with actual installation proof,
fsync exact pending intent BEFORE Source.enroll; then atomically confirm account
head/pin. Explicit recovery restores only existing protected exact intent, replays
without new salt/signature/password and confirms. Missing/corrupt intent refuses,
never reconstructs authority from current DB.32d: preserve distinct journal
NotAttempted/UNKNOWN/Acknowledged and checkpoint publication phase on every error,
including acknowledgement followed by checkpoint/unlock failure. Retain both causes
and exact replay semantics; disposable phase fault hooks only, valid control first.
32e: authenticate reads/validates checkpoint, refuses pending, releases files and
checks finish BEFORE held Source authentication/callback. Journal checks fresh
head+pin and retains authority through use. No file lock across business effect;
AfterUse preserves successful/failed callback outcomes. Intervening revoke/conflict
refuses. Enrollment may hold its dedicated file lock before journal, has no external
callback, all reservations immediate.32f: API-red, persistence/reopen/crash before
and after each publication, isolated rollback/tamper, contention/cancel/recovery,
held authority/outcomes, wrong roots and no-mutation refusals; affected regressions,
Rustdoc, byte-bound policies/functions/errors review, FR/ENdocs, normal local delivery
and Version verification. Personal service/provisioning/transport/Admin/UX/confinement
and PA-03sinks follow; final PA-13RETEX never substitutes unfinished original need.


## Actual implementation progress

31installed/qualified dev129.32API-red preserved; first compile failed owned/borrowed
Record mismatch (0scenarios), corrected to actual producer and SQLtable contract.
First5scenarios PASS. Added canonical-context/rollback,actual held writer/revoke,
and subprocess abrupt-exit matrix; recipe31tests/5steps still to qualify.


### Qualification recovery

recipe-01FAIL:7of8new scenarios pass, including actual subprocess3crash/control
pairs. Contention harness expected busy text but real SQLite category stays sqlite;
use original typed is_busy and release/join worker before asserting. New sync-red
1test fails: business callback ran without fresh checkpoint fsync. Corrected
common authentication to fsync validated witness and verify release before use,
retaining Before refusal and both success/failure AfterUse. Recipe now32tests/5steps,
including9coordinator cases. First edit rejected formatter mismatch before writes;
actual source boundaries read before correction, no dependent check run on refusal.


### Verified library, local installation still open

recipe-02:32named tests/5steps PASS, integrity VERIFIED, Rustdoc and FR/ENrendered
diagrams PASS (PNGs visually inspected).34config tests from31bound to7unchanged
actual config source files;9control repeated. Normal130first refused missing locked
regex cache before compile;4locked archives fetched/checksummed, lock unchanged.
Second Cargo/profile499sources/isolated candidate PASS, but supported launcher refused
broad inventory change: new foreign root_store/preparation_capture.rs, no effective
source delta. Original full evidence retained. Other preparation chat is actively
correcting nested JSON duplicate/null budget semantics, so no unchanged build retry.
Current managed dev129 stays installed; no32installation claim. Continue independent
authenticated composition33; resume supported delivery on coherent stable current
source. Same-agent semantic source review is not independent acceptance.


### Additive current delivery34

Current internal sources included in qualified installed dev136;59current library
tests,515-source nofixture profile,6CLI harness tests and56readonly UIassertions
plus failure guard PASS. [Details34](INCREMENT-34.md). Original refusals/proofs
preserved; no actual personal service/human/confinement/independent acceptance.
