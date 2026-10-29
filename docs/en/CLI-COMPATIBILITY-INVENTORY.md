# Static CLI route inventory

`tools/rust-migration/current_cli_inventory.py` binds reviewed candidates to exact
hashes of their declared CLI sources and three shared dispatchers. It establishes
neither behavioral parity, completeness of requirements nor acceptance. A refused
route may also be an invalid Go command. The 64 rows from October 28, 2026 are
therefore not a migration progress denominator. PAEA is out of scope.

```sh
python3 tools/rust-migration/current_cli_inventory.py capture \
  --rows docs/plans/migration-rust/CURRENT-CLI-ROWS-20261028.json \
  --out /new/path/inventory.json
python3 tools/rust-migration/current_cli_inventory.py check --manifest /new/path/inventory.json
```

The parent directory must exist. `capture` refuses an existing target; `check`
never rewrites hashes or classifications. Changed source, missing bindings,
invalid schema or an alias outside the checkout refuse. `wired`, `partial`,
`explicit_refusal`, `unknown` classifications come from review, never an automatic
execution proof.

Publication syncs an adjacent temporary file before a no-replace hard link.
An error after linking may leave the new file visible; sync and close causes
are retained. Reading sources twice is not an atomic snapshot or an ABA guarantee.
Six regressions cover drift, malformed input, an external alias, refusal before
publication, recovery and post-publication failure. The permanent
`cli-static-inventory` recipe retains these checks without a provider, Root or
compilation. Each candidate gap must then be checked against the actual Go
contract, any Rust replacement and a journey recipe before becoming remaining work.
