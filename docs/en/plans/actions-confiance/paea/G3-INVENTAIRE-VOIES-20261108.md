# G3a — egress-path inventory (confinement preparation)

8 November 2026 · [Français](../../../../plans/actions-confiance/paea/G3-INVENTAIRE-VOIES-20261108.md)

Lot G3a of the [gap-closure plan](GAP-CLOSURE-PLAN-20261103.md): the first G3
requirement is "Path inventory; synthetic tests; barriers…". This document
establishes the inventory — the verified state of egress paths available to a
provider helper and of sensitive-information holders — so the PA-04 campaign
"valid flow then bypass for each inventoried path" targets exactly these
paths. Analysis increment tied to the bytes read; no code changed, no new
barrier posed by this document.

Concurrent attribution: the common I0 boundary (BT02) is committed
(`6706e201b`); the additive operator Internet refusal (`provider_internet_denied`,
`deny_internet`) is an **uncommitted** stream in flight at inventory time
(shared working tree, owner: the G3 barriers stream). The findings below
describe it as observed in the tree, without presuming its delivery or
replaying its evidence.

## 1. Mandatory boundary and shared exec point

A single exec point already confines all CLI provider helpers:

- Landlock ABI6 + seccomp profile: `rust/crates/swarm-biba-linux/src/kernel.rs`
  (`ruleset()` line 17 — `Ruleset{fs:ALL, net:0, scoped:3}`; `seccomp()`
  line 53 — deny list: ptrace/`process_vm_*`/`pidfd_getfd`, mounts,
  `chroot`/`setns`/`unshare`, `open_by_handle_at`, io_uring/bpf/perf,
  kernel keyring, chmod/chown/utimensat/xattr metadata, AF_UNIX `socket`/
  `socketpair`; ioctl limited to FIOCLEX/FIONREAD/FIONBIO/TCGETS/TIOCGWINSZ).
- The ancestor reader `read_leaves` (lib.rs line 41) allows reading the whole
  filesystem **except** the private root `<root>/.swarm`; `prepare_bound`
  (line 76) pins the workspace by (dev, ino) and refuses root/private overlaps,
  hardlinks and symlink aliases.
- `Sandbox::enforce` (line 110) requires a single-threaded, unprivileged
  helper (`CapEff:0000000000000000`), enters via the pinned fd, sets
  `no_new_privs`, `landlock_restrict_self`, `close_range(3.., CLOEXEC)`, then
  seccomp. Any failure forces helper exit — no unconfined fallback exists.
- Shared exec point: `PreparedExec::execute`
  (`rust/crates/swarm-engine/src/native_provider_exec.rs:207`) — `biba::prepare`
  → dup2 stdin (sealed memfd holding the prompt) → `biba::enforce` → direct
  execve with no shell/PATH/ENOEXEC fallback.

Five launchers converge on this point, all behind a private channel with a
grant byte, parent identity (pidfd/start stamp) and peer credentials:

| Launcher | CLI helper | Gate (child side) |
|---|---|---|
| Agents | `_agent_gate` (`agent_supervision.rs:357` spawn) | `agent_execution_gate.rs:222` |
| Planning | `_planning_gate` (`planning_native.rs:1358`) | `planning_native.rs:473` |
| Validation | `_validation_gate` (`task_validation/process.rs:454`) | `task_validation/process.rs:159` — admitted candidate command, managed workspace mandatory |
| Review | `_reviewer_gate` (`reviewer_native/runtime.rs:184`) | `reviewer_native/gate.rs:199` |
| CLI preparation | `_prepare_cli_exec` (`preparation_call_cli.rs`) | `preparation_call_cli.rs:620` |

## 2. Secrets: holders and inventoried access paths

| Secret | Storage | Inventoried access path |
|---|---|---|
| API connection key (`ConnectionState.key`, `provider_connection_catalogue.rs:22`) | Private Root SQL | Built in memory (`preparation_call.rs:1021-1022`) and passed to the API worker (§4). Never in argv, never in diagnostics (`native_provider_exec.rs:20`). |
| Cockpit owner key (web key) | Private root `.swarm` | Unreadable by a confined helper: `.swarm` is excluded from `read_leaves`. Residual path: helper loopback network (§5). |
| Web login tickets (`issue_web_login_ticket`, root_store.rs:2146) | Private expiry file | Same `.swarm` exclusion; a consumed ticket is removed. |
| Telemetry tokens (`telemetry_store.rs`) | Private root | Engine-computed environment variables (`agent_execution_gate.rs:172-190`), never argv. |
| Prompt/send capsule | Sealed memfd (fd 0) | Sealed `SEAL\|SHRINK\|GROW\|WRITE` (`native_provider_exec.rs:65-105`) — the helper cannot truncate, rewrite or grow its stdin; read-only. |
| Task I0 data | Attributed I0 workspace | Only writable leaf; instantiation frozen by (dev, ino) at preparation (`biba-workspace-instance-changed`). |

The seccomp profile refuses `ptrace`, `process_vm_readv/writev`, `pidfd_getfd`,
`add_key`/`request_key`/`keyctl`: a helper can neither read another same-UID
process's memory nor manipulate the kernel keyring.

## 3. Residual paths of the confined helper (PA-04 synthetic-test targets)

Each of these paths is **real and unmediated** — this is where the "valid flow
then refused bypass" campaign must measure:

1. **Outbound network of the CLI helper**: `net:0` does not restrict network
   (the Landlock network attribute is never set); a confined helper can create
   AF_INET/AF_INET6 sockets. The in-flight additive `deny_internet` (foreign
   uncommitted stream) poses an optional cumulative seccomp refusal, frozen
   engine-side by `provider_internet_denied_snapshot()` at the gate. This
   path remains PA-05 target number one.
2. **Inherited descriptors**: `close_range(3.., CLOSE_RANGE_CLOEXEC)` marks
   CLOEXEC, it does not close. An intentionally inherited fd (the diagnostic
   channel) stays accessible — by design; any other inherited fd is a
   reproducible deviation.
3. **Environment**: `env_clear` + explicit list (`task_validation/biba.rs` and
   the common transport); observed values come from the launcher, a missing
   name stays absent; an engine-variable conflict refuses
   (`agent-execution-environment-conflict`). Any inherited leak beyond the
   list is a deviation.
4. **`/proc`**: host and peer cmdline readable (31 October memory finding
   re-verified: no procfs read mediation) — argv is visible to same-UID peers.
   Secret-bearing arguments are refused in argv (`add_leading_arguments`
   requires no secret).
5. **Files outside the private root**: general reading allowed by
   `read_leaves` (home included: browser cookies, SSH keys). Only writing is
   bounded to the I0 workspace. C07 finding: "no general read/secret
   confinement".
6. **Descendants**: the profile is inherited; a child cannot recover rights
   (no `unshare`/`setns`/mounts). A descendant also inherits the network path
   (§3.1) and general reads (§3.5).

## 4. API worker (`_prepare_turn`) — outside the I0 boundary

The `api-` preparation worker (`preparation_worker.rs`, CLI command
`_prepare_turn`, spawn `preparation_dispatch` `preparation_call.rs:851`)
**is not confined by the Biba profile**: it holds the connection key
(`PreparationApiRequest.key`, `preparation_call.rs:237` fed from
`ConnectionState.key`) and must reach the real endpoint. It is bounded by: a
single-use nonce token via stdin closed after reading (lines 13-44), PID/stamp
attribution before nonce release (lines 855-901), the shared egress lock at
claim and last use (`preparation_worker_transport_gate`,
`preparation_transport.rs:37` — re-checks quarantine, admission, destination
attribution, no-write-down before any socket opens). Declared limit: this is
application control, not OS confinement — the key sits in worker memory and
outgoing requests, by functional necessity. A PA-04/PA-05 deviation here is
an application deviation, not a kernel bypass.

## 5. Engine transport (web server) — layered onto the helper inventory

`rust/crates/swarm-web/src/access.rs`: listener bound exclusively to
127.0.0.1/::1 (`from_listener` line 105 onward), session required (Go
`session-`+24 format). Engine-side provider sends pass the shared egress lock
(`with_preparation_egress`, `preparation_confidentiality.rs:33`). The
`preparation_quarantine` flag blocks controlled API/CLI egress, not the OS
(`swarm-confidentiality/src/lib.rs`, field doc).

## 6. What this inventory does not conclude

- No path in §3-§5 is declared closed; each line is a path needing a valid
  flow then a refused bypass (PA-04), with observed bytes at destination
  (PA-05).
- The observed additive Internet refusal is neither committed nor qualified;
  its owner delivers the tests and evidence (`rust_biba_linux_boundary_internet.py`
  uncommitted, visible in the tree).
- The AC-02 prerequisites (helpers/mappings/accounts, H1) remain human; G3
  barrier qualification stays behind G1+G3a (qualification graph).
- BT03/BT04 (controls from I0 data, labels/promotions) remain open in
  [BIBA-TRANSVERSE-APEX](../BIBA-TRANSVERSE-APEX.md); this inventory feeds
  their inputs without closing their criteria.

## 7. Concrete next steps (resumption)

1. The `deny_internet` stream's owner delivers and commits its tranche
   (outside this document's scope).
2. G3a synthetic tests per §3 path are written on the
   `rust_biba_linux_boundary.py` model (unconfined control first, refusal
   next, evidence under `.swarm-fixtures/`), each tied to a §3 line.
3. The PA-05 network path awaits the in-flight stream's verdict; the general
   read path (§3.5) needs an operator scope decision (AC-02) before a barrier.
