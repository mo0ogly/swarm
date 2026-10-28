# PAEA — increment 50: validate raw Linux credentials before use

The correction is installed and verified in **v0.1.1-dev.180**. This closes the credential representation defect, not PA-02 or the whole PAEA programme. Direct Codex only; no GLM, delegation, additional commit or push.

## Diagnosis and decision

While preparing the attributed channel, the [primary Rustix report](https://github.com/bytecodealliance/rustix/issues/1698) and installed 1.1.5 source showed nonzero `UCred.pid` despite legitimate kernel PID zero. A valid connected witness precedes each fault injection. Ephemeral user/PID/mount namespaces reproduce an invisible connection creator with a visible immediate parent. No host account, sysctl or namespace is changed.

The old 175 ELF refused all three gates before any grant or Root access but reported bogus `os-errno:0`; no successful exploit was observed. Original raw observations and gate failures remain in `raw-zero-peer-result.json` and `gate-before-fix.json`.

Rather than upgrade or fork a dependency, the common getter uses existing Nix 0.30.1 raw credential integers. It rejects nonpositive PID and invalid UID/GID sentinels before the unchanged parent, UID, PDEATH, grant, source, freshness and rights checks. Three production gates and the `cfg(test)` review SDK helper consume it. Actual syscall errors keep their errno; invalid metadata keeps a safe category without private values or invented errno. The `socket` feature adds only `memoffset` 0.9.1 and `autocfg` 1.5.1, with no existing lockfile version changed. No unsafe code or new operator policy, authority, deadline or retry is added.

SO_PEERCRED remains connection identity; it does not replace per-message SCM_CREDENTIALS/SCM_PIDFD or peer admission.

## Retained evidence

Evidence directory: `evidence/20261027-zero-peer50-7n84xnp1/` in the French plan tree.

| Check | Result and scope |
| --- | --- |
| Library `recipe-01` | Stable inputs; 4 engine and 6 CLI tests PASS. Overall recipe FAIL on four existing Rustdoc diagnostics, preserved. |
| Documentation `rustdoc-02` | PASS, stable sources, verified integrity, `-D warnings`. Two repairs change comments only; executable bodies are identical. |
| Normal profile | 581 effective inputs archived and verified, no fixture feature; 10 owned production input hashes bound. The test-only SDK is correctly absent. |
| Installed executable | PID 4016445 at closure, SHA256 `e274363a532af0a0fff6147efb099a1aff736f07593fc30d9ee63fef43168b91`; supported build/restart. |
| Same actual ELF | 5 personal journeys / 56 commands; 3 valid witnesses then 3 invisible-PID refusals; 56 FR/EN UI checks and failed-script guard. PASS, stable inputs and verified integrity. |
| Markdown consumer | Repaired an inherited inline English Mermaid fence; CommonMark now recognizes the diagram and following heading. Original retained. |

Original refusals remain separate: unavailable offline dependency before fetch; Rustdoc errors; archive comparison omitting hardlink members; concurrent build before four producer APIs existed; local collector incorrectly requiring test-only SDK in production inventory. Each recovery preserves cause and changed prerequisite. Only our regenerable cache was retired after all 86 executable files were archived and reread; shared caches, original evidence, sources and user data remain intact.

## Remaining work

Resume attributed channel and bound client. Installed wrappers still lack SO_PASSPIDFD/SCM_PIDFD coverage; the selected boundary must own received descriptors and close them safely on rejection/truncation. No declared-PID or connection-creator fallback.

`newuidmap`/`newgidmap` remain absent and need local administrator installation. Full functional SDK acceptance, human enrollment, personal daemon, OS separation, E2/E3, PA-13 and **0/17 global invariants** remain open. Three earlier missing migration receipts retain their separate owner/resumption condition; own PAEA links are checked separately. Communication gaps exceeding 60 seconds remain an open method defect. This is local review, not independent approval.
