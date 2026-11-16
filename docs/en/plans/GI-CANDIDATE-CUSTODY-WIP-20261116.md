# WIP — Gi producer candidate custody

Unfinished source checkpoint, uncompiled and not mergeable.

Only cfg(test) producer borrows candidate through sources::request for preparation/claim/freshness. Reviewer09 REQUEST_CHANGES: preclaim holder carries review=None, postclaim guard requires Some(review), refusing review-managed-held-record-changed. This P2 remains OPEN. Never remove I0/equality checks. Next correction: explicit transition to holder of the actually recorded claim, outside SQL/settings, followed by new review and targeted checks.

Source report SHA57dde4f568fb90876e8fc90cd5c8b43cd0e76f95931387431159a7b58e6c237b, private evidence /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-candidate-custody-cecms474. Base061be0a7ed7d4017e86677f4589e9bd4aeabd12e; sourcef9cc3d9d83a860412cb7a6fee58e558dfad7c8bd94850f8bcaa77c6d4829b016. No test, build, W/SDK/F, installation or acceptance established. Failed Work w-83e055eae3e47c27a0f5043c rev14/calls0 unchanged; no DB, budget, TTL, clock or engine guard changes.
