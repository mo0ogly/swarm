# Verify a PAEA canonical export's correspondence

[Français](../architecture/PAEA-CANONICAL-VERIFIER.md) · [Contract](PAEA-CONTRACT.md)

The maintained `tools/paea/verify_canonical.py` checks an original
`{proposal, context}` document against an offline `protection canonicalize` export.
This is an initial local PA-01/PA-09 component, not the reserved `protection verify`
command or an effect-evidence verifier.

```sh
python3 tools/paea/verify_canonical.py --document document.json --result export.json
```

Requires POSIX, Python 3, `jsonschema >= 4`, two regular files and the versioned repository
schema. No Root, service, account, secret, network or policy is opened. Files are
read into memory with no product ceiling, retry or global watchdog. Symlinks to
regular files are allowed and confer no identity/ownership proof. Local helper
imports create no bytecode; input files are unchanged.

The reader owns its descriptors and refuses directories, FIFO and devices without
waiting on FIFO. Strict UTF-8 JSON refuses decoded duplicate keys, floats/exponents,
non-JSON constants, invalid Unicode and trailing content. Python interpreter
integer/depth technical limits can also refuse. Schema v1 checks document shape. Its identifier/profile and local-only references
are checked; nested base identifiers/external references refuse before resolution.
Schema corruption receives a schema diagnostic. Context fields are required
independently of schema validation, so fixture defaults cannot fill missing values. The existing Python oracle recomputes exact
canonical text, action digest and business intent. The complete export envelope
must match, including integer version (not bool), no-authority and offline scope.
Export JSON whitespace/key order can vary; canonical text must match exactly.

Exit0 reports `status=canonical-correspondence-verified`,
`validation=schema-and-canonical-correspondence-only`, hashes of both consumed
files and schema, schema identifier, action digest and intent. `authority` and `effect` remain `not-established`.
Request ID remains excluded and confidentiality labels remain a sorted set.

**A mutually forged but matching pair can pass.** Schema does not enforce all Rust
semantic admission rules: i64 monetary bounds, Git references, source relations,
resource existence and current policy remain outside scope. A matching out-of-i64
amount is retained as an explicit limit test. No authenticity, authorization,
signature, actual effect, omission detection, independence or E2/E3 assurance is
established. Shared Python vector/oracle dependencies are declared.

Exit2 emits a static JSON refusal on stderr without raw documents or paths:

| Code | Cause and recovery |
|---|---|
| `canonical-json-duplicate-key` | Duplicate decoded name; supply an unambiguous original. |
| `canonical-json-number-invalid` | Float/exponent/NaN/Infinity; use exact contract representation. |
| `canonical-json-invalid` | Syntax, Unicode or interpreter limit; correct the original. |
| `canonical-document-shape-invalid` | Unknown/incomplete shape; compare the versioned schema. |
| `canonical-correspondence-mismatch` | Substituted/incomplete/incompatible export; recover matching originals. |
| `canonical-regular-file-required` | Unsupported file type; supply an authorized regular copy. |
| `canonical-io-unavailable` | Read/close/write failed; repair source/destination then retry. |
| `canonical-verifier-schema-invalid` | Local schema invalid; repair candidate without admitting input. |

Missing arguments follow argparse usage diagnostics. Unbuffered standard stdout
writes handle partial progress; zero progress or OS failure refuses. The report
has no buffered interpreter-exit reflush. Partial bytes may already be published:
consume only a complete JSON report with exit0. Additional stderr failure retains
exit2 without claiming a published diagnostic. No output proves absence of an
effect in another system.

`tools/verification/recipes/paea-canonical-verifier.json` uses the same explicit
existing binary/hash/fresh transcript environment as the
[public recipe](PAEA-CONTRACT.md). Nine named tests cover 14 actual binary exports,
tampering, raw ambiguities, equivalence and explicit limits, refused reads,
real `/dev/full`, simulated short writes, failures and recovery, corrupt/weakened schemas and
standalone vector generation without bytecode cache writes. The schema digest
binds bytes, not authenticity; schema changes create a new candidate. Memory
exhaustion or external termination may interrupt normal diagnostic handling. Roots are isolated,
originals retained; no compilation, installation or model call occurs in the recipe.

[Increment 04 status, review and evidence](plans/actions-confiance/paea/INCREMENT-04.md).
