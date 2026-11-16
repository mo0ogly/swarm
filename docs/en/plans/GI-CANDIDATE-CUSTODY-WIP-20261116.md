# WIP — Gi producer candidate transition

Source reviewed, uncompiled and not runtime verified: do not merge as accepted.

First checkpoint7088121 counterexample R09-CUSTODY-1 is resolved at source through distinct holders. sources::request acquires/refreshes the preclaim candidate outside SQL; its callback retains preparation/claim transactions and records known Work/claim before close. sources::job then acquires custody of the actually recorded running review outside SQL; its callback retains owner/freshness/clock/expiry/history checks. No equality/I0/budget/TTL protection or limit removed. Errors after claim do not imply rollback.

```mermaid
flowchart LR
 A[Preclaim without review] --> B[Recorded claim]
 B --> C[Close preclaim holder]
 C --> D[Holder of exact review]
 D --> E[Checks and expiry]
 E --> F[Close and reports]
```

Reviewer09 APPROVE_SOURCE_ONLY: SHA6b3d83798f27900c0c035197cf96ef41e870693c99adeef52a15b554e0fc9263. Source5c0e1ab29b4eb87673b126f2314c34d5c8ef343d5ca957894d8a8b567f1ff14a; private evidence /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-custody-transition-u41ymcya. One separator blank line removed; no other function body changed.

Still required: new SDK qualification, actual healthy managed witness before wrong/nonrunning review and cleanup retaining knownWork, then controlled recovery. No build/test/W/SDK/F/installation/acceptance performed for this candidate. Failed Work w-83e055eae3e47c27a0f5043c, last observed rev14/calls0, is untouched. Historical scenarios are not replayed.
