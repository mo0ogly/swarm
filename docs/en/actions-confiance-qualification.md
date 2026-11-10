# Trusted Actions qualification

Qualification uses an identified candidate, frozen inputs and isolated roots. Each receipt distinguishes Go tests, CLI/API flows, browser checks, independent review and limitations. Units are not combined into a score. Historical records remain under `docs/plans/actions-confiance/verification-globale-20261016`; the corrected candidate packet links its binary, results and review decision.

## Reproduce

`python3 tests/actions_confiance_campaign.py --binary BINARY --candidate-receipt BUILD_RECEIPT --build-inputs BUILD_HASHES --evidence-dir NEW_DIRECTORY --timeout-seconds 180` runs nine stages: connectors, signatures/checkpoint/archives, laboratory, legacy broker, active data-policy broker, helper substitution, pilot, Processes and control mapping.

The build receipt contains `binary_sha256`. Hashes are a path/digest object or a list of `path`/`sha256` objects. `--prepare-only` validates without execution. `--resume` requires identical inputs and binary, intact results and an observed exit0; failures never become success. The timeout is a recipe watchdog, not product policy.

`tests/actions_confiance_effect_proof.py` also checks rejection of effects on a corrupt chain, policy/intent/result contexts, reconciliation snapshots, both connectors and historical proofs. The browser recipe `tests/actions_confiance_evidence_ui.cjs BINARY DIRECTORY --effect-proof` exercises an amount above 2^53, its signature and exact verification.

## Scope and decision

The user authorized a separate Codex review context and deferred positive native eBPF tests. Deferral does not qualify R20: real collection, attribution, exclusion of a second owned fixture, loss/restart/cleanup and overhead still require an explicitly authorized environment. The proposed launch remains in `kernel-positive-launch.md` under the qualification folder. No Linux privilege or retry is implicit.

The retrospective closes the agreed scope with this explicit deferral. Local simulation, integrity checks, code review and business acceptance remain distinct. No third-party account, payment or general provider is qualified. Same-UID, swap/privileged tracing and third-party time limitations remain documented in T4/T6/T7.
