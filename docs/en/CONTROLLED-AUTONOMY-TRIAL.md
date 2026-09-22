# Preparing an autonomy trial with a controlled fault

Status: recipe components tested with simulated processes. This document does not
establish a live-model trial or acceptance of a mission.

## Business fault and expected evidence

The sample exposes `feature.py:is_expected(value)`, which must return the Boolean
true only for the string `expected`. The independent check tests `expected`,
`wrong`, an empty string and `None`.

The worker adapter waits for the first successful production process, verifies
that its output passes the check, then replaces the function with one returning
true for every input. It immediately demonstrates the resulting business failure.
Outside agent copies, it records mission/agent/attempt identities, original source,
before/after hashes, checker/injector hashes and exit codes. This is an explicit
trial fault, not a mistake attributed to the model.

The engine must reject the result through its own checks. The planner must decide
how to correct it within existing limits. Later productions are not modified.
An interruption between reservation and completed injection leaves pending evidence;
it requires examination, never silent reinjection.

## Components

- `tests/controlled_business_fault.py`: one-shot injection, business check and evidence.
- `tests/autonomy_worker_adapter.py`: identifies the worker through public `agent list`,
  preserves arguments, prompt and output, then injects only after successful exit.
  Verifies the engine binary hash before and after the producer runs.
- `test_controlled_business_fault.py` and `test_autonomy_worker_adapter.py`: local
  component tests with no AI provider.

Use the adapter only for the **worker** provider of a dedicated disposable Store.
Planners and reviewers keep their normal provider. Never attach it to a production
mission or user checkout. Path restrictions are trial guards, not a sandbox against
hostile agents running under the same operating-system account.

Store this adapter configuration outside copies:

```json
{
  "engine": "/absolute/path/frozen-swarm",
  "engine_sha256": "VERIFIED_BINARY_HASH",
  "store": "/absolute/path/disposable-store",
  "work": "ACTUAL_WORK_ID",
  "copies_root": "/absolute/path/managed-copies",
  "evidence_dir": "/absolute/path/injection-evidence",
  "producer": "/absolute/path/real-provider"
}
```

Invocation: `python3 tests/autonomy_worker_adapter.py CONFIG [provider arguments]`,
with the prompt on stdin. Preserve the executable identity expected by the provider
connector when creating its dedicated launcher. Never select this launcher for
planning or review. Engine limits remain enforced. The adapter neither creates
nor authorizes the mission.

## Local checks

```sh
python3 -m unittest discover -s tests -p test_controlled_business_fault.py -v
python3 -m unittest discover -s tests -p test_autonomy_worker_adapter.py -v
```

## Before a live trial

1. Freeze binary, engine commit, recipe commit and storage schema. Do not change
   engines during the trial.
2. Prepare a disposable repository and an engine check with the same business
   assertions, outside files editable by the worker. Ignore Python caches.
3. Separately configure and authorize planner/producer/reviewer budgets. This
   recipe does not reset exhausted limits of existing missions.
4. Attach the adapter only to the worker; verify configuration without model calls
   before the authorized launch.
5. Capture public events: real delegation, rejection of the injected candidate,
   real retry decision, corrected production, checks and independent review on
   the same SHA, then scope closure.
6. Observe after launch: the trial script must not manufacture `claim/decide/retry/close`
   decisions. Record external repairs; they prevent a claim of unaided autonomy.

The public campaign launcher and observer still need assembly before real calls
can be authorized. Ten local tests prove fixture components, not the ability of
real agents to finish the mission.
