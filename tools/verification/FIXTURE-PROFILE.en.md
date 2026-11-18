# Fixture profile compile qualification

`qualify_fixture_profile.py` prepares one Cargo integration-test target using
`--offline --locked --no-run`. It executes no test body. Cargo may change the
cache and rebuild required dependencies. The adapter does not schedule a managed
default CLI-image build.

The coordinator separately authorizes each actual compile. Inert tests provide
neither that authority nor evidence of native compilation.

## Explicit inputs

The CLI requires `--config PATH --config-sha256 DIGEST`; it exposes no helper
injection flags. Version1 JSON requires exactly: `schema_version`, `root`,
`frozen`, `workspace_root`, `target`, `output`, `manifest_path`, `source_map`,
`registry_map`, `cargo_lock`, `external_targets_policy`, `space_policy`, `package`,
`test`, `features`. The five bindings contain `path`/`sha256`. The source map
maps paths relative to `frozen` to SHA-256; the registry map uses absolute paths.
The lock must be `workspace_root/Cargo.lock`. Persistent policies stay under
`root/.cargo`. The external target must already be admitted; the adapter neither
registers a target nor lowers the reserve.

Output must be fresh, outside repository/source/target, on the admitted mount.
Admission and reserve checks precede its creation. The volume descriptor stays
held through the child, postchecks and publication. Preserve qualified default
images and referenced old cache artifacts separately before sequential cache
reuse. A cache pathname does not reserve against every external producer.

## Evidence and errors

Receipts bind configuration, raw maps, before/after sources, policies,
stdout/stderr, helper outcome, Cargo artifacts and selected test profile.
Relative depfile paths resolve from the Cargo workspace. The inventory is the
observed union of cache depfiles, not an exhaustive binary input closure.
Observed generated files remain distinct from prebound inputs. The retained ELF
is copied, read back and made owner-executable; it is not installed.

Publication uses an adjacent synchronized temporary and an exclusive link.
Failure after linking preserves an uncertain application observation, never a
rollback claim. Read, process, storage and publication causes survive, including
secondary failures. A known child exit stays recorded when the helper raises;
it does not establish controller success. No global timeout, retry or additional
margin is added. Cargo offline mode is not a network sandbox.

## Verification and limits

The maintained `recipes/fixture-profile-inert.json` recipe checks guards using
synthetic files and injected helpers. It invokes no compiler, provider or service.
Retain its result and verify it with `recipe.py`. Publication refusals and drift
cases have valid controls in the tests.

This does not qualify the C/header/system graph, build-script/proc-macro effects,
runtime, installation or business acceptance. Independent review and a separate
allocation remain required before the intended native use.
See [Français](FIXTURE-PROFILE.md).

Depfiles and generated files are archived from the exact buffer used for their
hash and analysis, without rereading the source. This does not establish that
the source stays immutable after that observation.

Depfile `..` components resolve from the Cargo workspace before classification.
Their presence alone does not establish escape: the canonical target must remain
declared and hash-bound, or be an admitted-cache generated file. Unbound external
paths, including symlink escapes, remain refused.
