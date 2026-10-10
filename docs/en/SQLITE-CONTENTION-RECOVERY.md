# SQLite contention recovery

## What changes

Planning decisions use the existing bounded SQLite retry policy. For independent review, a provider result that has already been obtained is now stored in an attributable journal before SQLite persistence: after contention, the engine resumes that same verdict without calling the provider again, incrementing review calls, or creating an agent attempt.

The engine, CLI, and Administration UI share one policy: `busy_retries` (0–10), `busy_retry_delay_ms` (0–1,000), and `busy_timeout_ms` (1–60,000). `swarm storage-retry show` reports configured/effective values, source, persistence, `sqlite_busy`, and the total bound `(retries + 1) × busy_timeout + retries × delay`. `swarm storage-retry apply --input <file>` validates and atomically writes `.swarm/storage-retry.json`.

## Reproducible measurement

The worker actually ran:

```sh
python3 tests/sqlite_contention_final.py --timeout 300 --measure-only
```

The recipe extracts `HEAD` into a temporary root and runs the exact same test and parameters against that baseline and the current candidate. Each variant performs nine independent reviews with a deterministic local provider double, one held writer, `busy_timeout_ms=1`, zero automatic retries, and one causal resume.

| Variant | n | p50 | p95 | max | initial SQLite errors | durable business effects | business errors | review calls | attempts |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Before (`HEAD` 8923ba5) | 9 | 60.843 ms | 71.537 ms | 71.537 ms | 9 | 0 | 9 | 9 | 9 |
| After (dirty candidate) | 9 | 59.286 ms | 67.748 ms | 67.748 ms | 9 | 9 | 0 | 9 | 9 |

Normalized resumed-run result SHA-256: `4ed05b1bcd9cab73f7194ca82da02eb72cf489a42f09e62af239904a151143a1`.

The candidate removes the business failure in this scripted load. Its p50 is slightly lower while p95/max are slightly higher; nine samples support no general performance claim. This is neither a real-provider benchmark nor proof of autonomy: the provider is a local double and every root is isolated.

## Final host qualification

The host conductor, and only the host conductor, must run:

```sh
python3 tests/sqlite_contention_final.py --timeout 290 --baseline-ref <revision-avant-correction>
```

The final recipe runs the before/after measurement, required targeted race tests with no skips or zero-test success, `go vet ./...`, `npm test`, the workflow configuration check, `git diff --check`, and exactly one discovered Go-suite execution through the preserved `tests/supervision_go_suite.py` harness. Any optional skips are listed explicitly; required cases may not skip.

The final control, candidate manifest, report, and dossier must bind the same bytes before independent review. Technical review precedes administrative acceptance; global closure is never a prerequisite for the S04 review.

## Attributed retrospective

- Engine: the former review path converted a computed-but-unpersisted verdict into an interruption error. The candidate journals the attributable verdict and resumes it without another call.
- Benchmark: the 27 historical errors comprise 14 planning decisions and 13 reviews; the five provider timeouts remain separate from SQLite. The S04 scripted measurement does not rewrite those archives.
- Provider: real autonomy is not demonstrated. Calls are local and deterministic; real cost, quota, and availability remain unknown.
- Supervision: the first AI preparation response violated its JSON contract, so supervision authored and adopted the brief and plan. S03 also required correcting omitted browser arguments and a reload `ERR_ABORTED` classification; prior failures and budgets remain visible.
- S04 worker: the first race command revealed that the measurement test required an output even in the discovered suite. The test now asserts 9/9 candidate verdicts without an output and emits comparison data when the recipe supplies one; both targeted paths pass.

## Current limits

At worker handoff time, the global Go suite, the S04 host functional control, and the actual independent review have not run. Targeted evidence is therefore not engine acceptance, installation on a live cockpit, or mission closure.


## Main integration — 10 October 2026

Mission w-82cb75995c4b5ee02130797b publicly closed at revision 98 with four fresh accepted tasks and a passed independent review. Its isolated qualification discovered 1,072 tests; optional skips were explicitly excluded from passes. That evidence covers the isolated candidate, not automatically this integration. The port preserves main’s internal/engine modules, stable login and tutorials. The recipe now requires an explicit pre-correction Git revision; historical tests read archives from the repository root. Integration checks and installation remain separate. Measurements above are historical; nine samples do not establish a general latency improvement.
