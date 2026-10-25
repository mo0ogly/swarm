# PAEA — increment 06: Rust CLI integration, qualification deferred

[Français](../../../../plans/actions-confiance/paea/INCREMENT-06.md) · [Plan](../PAEA-APEX.md)

The resumed APEX/replan work connects increment 05's pure Rust comparison to the
existing CLI reader, using two positional regular files instead of a new envelope
or global argument parser change:

```sh
swarm-rust protection compare /PATH/before.json /PATH/after.json --json --lang en
```

**Implemented in source only: uncompiled, unexecuted, uninstalled.** User's
no-compilation/no-restart instruction remains active. Existing canonicalize output
and earlier evidence cannot qualify the new command. PA-00…13 remains open.

Both reads reuse existing regular-file rules and the existing 16 MiB per-document
transport bound, without a new business limit, attempt or timeout. `-` refuses
for compare to avoid shared-stdin framing; a file named `-` can use `./-`.
Symlinks to regular files retain existing behavior without ownership/confinement
claims. Sequential reads are not an atomic snapshot; hashes bind consumed bytes,
not filesystem freshness.

Nonempty `--input`, nondefault root, server, nonempty output/task and non-delivery
phase refuse. Historical empty option values retain their behavior without
conferring authority. No Root, secret, network, effect service or project setting
is opened. API/UX and protected authority remain unimplemented.

The JSON envelope carries `schema_version:1`, `scope:"offline-data-only"` and
`comparison`: before/after document/action/intent hashes, sorted fixed changed
paths without values, content/intent/transport-change booleans. Authority, effect
and approval validity remain `not-established`. Equal content cannot establish
continued approval; changed content needs future protected-control review. Hashes
and change metadata still require recipient permissions.

`schemas/paea/comparison-v1.schema.json` describes this output, not yet verified
against a binary. Historical input schema remains unchanged. No new Python component.
Read/inspect/close failures preserve `CommandInputError`; validation preserves
`Protection(Error)`. Serialization precedes output; existing `write_output` retains
write/flush errors. Nonzero can follow partial published bytes. No business effect
is created and no automatic retry occurs.

Four CLI tests added, **not executed**: both domains FR/EN with unchanged files;
invalid JSON/missing file on each side, valid control/recovery; runtime-option
refusals; real `/dev/full` plus recovery. Linux target `protection` now contains
eight tests including four historical tests. Three library comparison tests
remain written/unexecuted. Permanent `paea-contract` recipe updates targets/counts
but is **not run**. No global watchdog is added.

Static byte-bound policy/function/error review is retained in `source-review.json`.
Guessed arguments/help/error paths were resolved to their actual producer in
`swarm-cli/src/lib.rs`. Only two Protection help strings change in that shared
file; foreign work remains intact. SCALING-S2 stays in progress per inspected plan.
No commit/push/publication/install, mission resumption or foreign process changes.

Codex must qualify exact `comparison`/`protection` targets, affected foundation and
Rustdoc after no-compilation is lifted. Future API/UX must preserve diagnostic vs
approval distinction. IA3/AC-02 handoffs and sponsor-provided E3 evaluator/team
remain open; internal opinions and historical Go evidence cannot replace them.
Global program and retrospective remain unfinished.

## GLM disposition

Internal call finished `stop`: 16,825 input / 22,302 output / 39,127 total reported
tokens, 138.412 seconds. No client cap or imposed effort. API requested/reported
`glm-5.2` does not attest backend version. No execution/independent acceptance.

Integrated naming correction: `canonical_content_changed` replaces the ambiguous
`action_content_changed`, since context belongs to compared bytes. Rust, tests
and output schema change before any delivery; historical evidence stays intact.
Envelope version now uses the shared constant. Added unknown-private-field,
stdin-marker, extra-argument and safe ENOSPC assertions remain unexecuted.

Fixture-path and double-close claims are rejected using actual path resolution
and nix 0.30.1 `close<Fd: IntoRawFd>`, which consumes File ownership. This is static
evidence, not compilation. Git 2.51.0 `check-ref-format refs/heads/@` returns 0,
refuting the proposed branch restriction. Error consumers publish safe JSON
categories/labels, not private serde messages; future unknown-field tests cover
that boundary. Historical compact PAEA output remains; new report holds fixed
ASCII fields/metadata. Without-JSON/explicitly empty options retain existing
behavior. Only Linux qualification is planned; no non-Unix compatibility claimed.
