# PAEA — increment60: atomic controller creation

Resume59 verified dev208; Codex alone, no GLM/agent/commit/push. Full PA00..13 and
RETEX/global0of17 remain open. APEX/replan removes try_wait/pidfd_open window.
Stable Rust1.96 create_pidfd E0658 versus available GNU2.43 pidfd_spawn: choose
native capability lookup before use, no PID/PATH/shell fallback. Installed ABI
bits/spawn_ext.h/spawn.h, locked libc POSIX types, sole kernel FFI.
Primary source: https://sourceware.org/pipermail/glibc-cvs/2024q1/084106.html.

Explicit absolute executable/cwd, nonempty argv, exact env without inheritance,
three borrowed streams and caller-selected signal masks/defaults. No launch
policy/approval/UID/budget/image granted. Duplicate streams above0..2 then close
all other child FDs;3 is POSIX stream boundary, no operator ceiling. No file open/
truncate actions. Check pidfs64 before execution; missing symbol refuses first.

Return owned atomic handle even for already exited child. Every post-success
validation/cleanup failure retains process in Failure.child and actual invocation;
Err never means no effect. Wait only by pidfd, optional nonblocking observation
and explicit wait; no numeric PID. Close/drop never implicitly signal/reap.
Strict Service accepts nonforgeable Spawned and retains ALL UID/process/domain58/
current control/Reply/error stages.

Actual attributed socket, cwd/env/signals/FDs, fast exit, exec refusal/recovery,
post-spawn error ownership and strict composition controls; maintained recipes,
FR/EN/docstrings, exact-byte policies/functions/errors review, default delivery.
Administrator uidmap, actual ptrace/UID separation, human/confinement/independent
assessment open; no global activation or implicit delegation.

## Resume state: library verified, delivery pending

35distinct Rust PASS (9new+26existing), strict Rustdoc6exact sources/3pages,9HTML
originals, final trace11 CLONE_PIDFD and waitid(P_PIDFD), no child PID capture.
complete07/regressions08/trace11 integrity verified. Recipe06 refused a missing
observable assertion, fixed before07. Trace05 remains historical after docs
recipe changes. Initial verify --output typo corrected to --evidence.

Builds09/10/11 compiled but refused concurrently changed shared inputs:09CLI/
session_visit,10new unbound web/native-session-visit.js,11version notes and engine
sources. None installed. Dev208 remains last PAEA-qualified runtime, ELF
06592796d06752f478e03a82ac17d6c95c6ad177a364f321afa74dc79d1aaf90. Do not rebuild
unchanged: resume after concurrent source phase stabilizes or a supported candidate
binds67current PAEA inputs and notes. Hashes/originals in the French evidence tree,
20261027-atomic60-zzc42alw/replan-delivery-concurrency.json. Continue independent work.
GNU library only; no daemon/actual distinct UID/confinement/human/independent
acceptance. PA02partial/global0of17/E2/E3/PA13/RETEX open; administrator uidmap and
inter-UID ptrace pending, no same-UID fallback.


## Local atomic-library delivery

v0.1.1-dev.209, ELF `ebe65e0d253b012d2a6fb988ea7c3de03a857d750d7aa631c150ceb2021b2e74`, final-check PID1713359. Default profile
645 effective/67 PAEA inputs, no test-fixtures, cfg(test)
sources excluded.35distinct Rust PASS:9new+26existing, two private atomic tests
not counted twice. Strict Rustdoc6exact sources/3purposepages,9HTML originals.
Final native trace11 requires CLONE_PIDFD and waitid(P_PIDFD), no child-PID open.
Trace05 remains historical after documentation recipe changes;11 binds final inputs.

Same installed ELF:5calls/56commands,7personal+3guards,3peerpairs/6commands,
56FR/EN UI checks+guard. Existing native flows do not claim a new atomic daemon.
Version/notes/actual dirty commit verified. Originals in the French evidence tree
`evidence/20261027-atomic60-zzc42alw/`;byte-bound review/archive retained.
Build09 compiled then refused two concurrently changed shared inputs; no install.
Build10 refuses a newly added, unbound web asset; Build11 also refuses changed engine/notes. Common209 subsequently qualifies
645effective/67PAEA inputs through the same supported path; restart12 selects its SHA
without another compilation. Recipe06 refusal (no Rustdoc
observable assertion) precedes07 PASS. Initial verify --output typo refused, corrected
to --evidence. Synthetic post-spawn failure is not actual OS destroy failure.

PA02partial/global0of17/E2/E3/PA13/RETEX open. No daemon, independent launch/image
approval, human identity, actual account separation or confinement established.
Administrator uidmap and inter-UID ptrace remain open. Next protected consumer/
admission/PA03. Current-capability FR/EN documentation is reconciled with209. No commit/push/GLM/agent.
