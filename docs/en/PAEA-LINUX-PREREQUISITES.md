# PAEA Linux prerequisites

This tool produces an **observation**, never confinement certification or launch
authority. Run from the checkout:

```sh
python3 tools/paea/linux_prerequisites.py
python3 tools/paea/linux_prerequisites.py --output /tmp/paea-prerequisites-new
```

The output directory must be new. It reads current namespace euid/gid/groups/maps,
own `/etc/subuid` and `/etc/subgid` ranges, and installed mapping-helper file custody.
It selects/reserves no UID, starts no namespace/sudo/package/account/service and
keeps unreadable/invalid data distinct from empty assignments. Namespace labels
are not held descriptors or durable identities. No password/key/proof is reported.
Launch/separate-host-UID/confinement/human verification indicators stay false even
when helpers/ranges exist: metadata alone establishes no functioning/exclusive
mapping, clean groups/caps/FDs, mounts/network/descendants or recovery.

## Observed on27October2026

Currenteuid1001, no effective capabilities; assignedUID/GID ranges165536+65536.
Unshare/bwrap exist. Each ephemeral probe follows a valid private synthetic-file
witness. InnerUID0 maps to outerUID1001 and can still read the protected same-UID
file: this profile does not separate trusted keys from that agent UID and cannot
replace the required distinct-UID profile. Newuidmap/newgidmap and the uidmap package
are absent. Local apt metadata lists1:4.17.4-2ubuntu3; two preferences are unreadable
to this user, so this is not the administrator apt decision. Noninteractive sudo
policy inspection requires interactive authentication. No privilege/system changed.

## Bounded administrator step

A machine administrator can install the two mapping helpers:

```sh
sudo apt-get install uidmap
```

Authenticate locally, never send the password to chat/repository/recipe. The
observer does not execute this command or create a PAEA account/service, and package
installation confers no business mandate. Rerun into fresh evidence afterwards;
preserve earlier reports. The future launcher must still prove owned/pinned real
helpers, authorized mapping, distinct outerUIDs, clean groups/caps/FDs, protected
roots, mounts/PID/network/descendants and recovery. This guide chooses no UID/range.
Service activation and real human enrollment remain separate. Required protection
missing means refusal, no same-UID fallback or automatic hostpolicy/sysctl change.

[Plan and evidence45](plans/actions-confiance/paea/INCREMENT-45.md).

## Atomic creation available in the GNU library

`swarm_linux_identity_channel::atomic_spawn::launch` creates the process and pidfd
in one GNU `pidfd_spawn` operation, resolved from the already loaded libc. Required
symbols and pidfs64 are checked before invocation; missing support refuses without
later child-PID capture. Conditional capture of an existing `Child` remains a
separate API and does not establish atomic creation.

The caller supplies absolute executable/cwd, nonempty argv, the exact environment,
three POSIX streams and explicit signal sets. No ambient environment, PATH search
or shell is added. Streams are duplicated and other inherited child descriptors
close. No account, credential change, confinement, approval or operator policy is
created. The consumer must establish those prerequisites before invoking launch.

`Spawned` retains the handle even after fast exit. `try_wait`/`wait` observe that
pidfd; an external reaper may cause ECHILD, never empty success. Every post-creation
failure retains ownership in `Failure.child`, with independent validation/cleanup
causes. `invoked` without a handle does not prove no transient child or effects.
`close`/Drop release only the FD: waiting, cancellation and recovery remain explicit
consumer responsibilities.

`Service::dispatch_atomic_controller_packet` requires the atomic type and preserves
UID, process, namespace domain, decoding and current-authority checks. This library
is not an installed daemon or distinct-UID proof. `paea-atomic-controller.json`
covers nine new Rust tests; `paea-atomic-controller-native-trace.json` requires
CLONE_PIDFD and waitid(P_PIDFD), without pidfd_open of the child PID. These local
proofs do not establish independent acceptance.
[Plan and evidence60](plans/actions-confiance/paea/INCREMENT-60.md).

## One-packet response output

`Channel::send_packet` sends one nonempty record with one native `send`, rechecking
socket kind/options. No caller controls/descriptors or declared credentials are
attached. Per-call `MSG_NOSIGNAL` suppresses SIGPIPE without changing global signal
policy. Blocking follows supplied socket flags: the consumer configures its policy
before use, with no hidden library timeout/retry/size ceiling. Transport limits and
actual errno remain distinct from operator policy.

Success means local exact-byte acceptance, never peer receipt/admission.
`SendFailure` retains cause, native invocation and any observed unexpected short
accepted count; no error establishes rollback. Caller retains socket/buffer, with
no copy, close or retry. `service_response::send_packet` encodes canonical public
observation then calls this transport once, retaining original Reply/Observation
on encode/send errors. Inspect business outcome independently of output cause,
especially after an already recorded effect. Legacy `service_response::send` keeps
its Write/write_all/flush contract; do not implicitly adapt it to fragmented packets.

`paea-packet-response.json` checks roundtrip/two records, pre-send refusals/recovery,
actual EPIPE, default unblocked SIGPIPE in an isolated worker, request-bound consumer
and known result retained/reopened after send failure. No daemon/authenticated peer/
actual distinct account/distributed effect is established.
[Plan61](plans/actions-confiance/paea/INCREMENT-61.md).
Primary references: [send(2)](https://man7.org/linux/man-pages/man2/send.2.html),
[socket(2)](https://man7.org/linux/man-pages/man2/socket.2.html).

The recipe also observes `SO_SNDBUF`, requires EMSGSIZE with no received packet,
and checks actual nonblocking saturation (EAGAIN), exact drain and recovery.
It defines no application ceiling or arbitrary attempt count.

## Inspection by the parent namespace owner

Local qualification distinguishes the parent's effective set from capabilities
it holds in an owned child user namespace. Linux6.17 describes that owner rule;
pidfd namespace ioctls keep their `PTRACE_MODE_READ_FSCREDS` check.
`paea-namespace-owner.json` uses two ephemeral profiles with **the same external
UID** and a zero effective set in the parent's current namespace:

| Child profile | dumpable=1 control | dumpable=0 | restored to1 |
| --- | --- | --- | --- |
| Parent user namespace | allowed | EACCES | allowed |
| Owned child user namespace and private mounts | allowed | allowed | allowed |

Six phases use the actual sender's atomic pidfd and held namespace objects.
This proves a local inspection permission, not a capability in the parent
namespace. It is neither cross-UID evidence nor a deployment profile:
`--map-root-user` here keeps the external UID and does not isolate service keys.
Mapping helpers, actual distinct UIDs and confinement remain required. Do not
infer a need to grant a global capability to the service.

A second test retains a child's native failure after a valid control: a null
sink loses it, private capture preserves it. The test executable is frozen and
identified; compiler/test channels and metadata bind to recipe hashes. Altering
a private proof copy refuses. No daemon or production API is delivered by these
tests;61delivery is tracked separately.

[Evidence62](plans/actions-confiance/paea/INCREMENT-62.md) ·
[User namespace owner](https://man7.org/linux/man-pages/man7/user_namespaces.7.html) ·
[Linux6.17 capabilities](https://raw.githubusercontent.com/torvalds/linux/v6.17/security/commoncap.c) ·
[ptrace checks](https://raw.githubusercontent.com/torvalds/linux/v6.17/kernel/ptrace.c) ·
[pidfs ioctls](https://raw.githubusercontent.com/torvalds/linux/v6.17/fs/pidfs.c).
Upstream source is not the exact observed Ubuntu build; actual runtime evidence
is limited to the exercised6.17.0-41-generic kernel.

The base owner rule binds UID and parent user namespace, not just the launcher
PID. Keep that UID outside the agent domain; putting an authority in a namespace
controlled by an untrusted UID does not isolate its keys merely by changing an
inner UID. This upstream-rule consequence still needs nonparent-observer and
distinct-external-UID probes in the chosen launch profile;62does not prove them.

## One complete controlled exchange

`Service::exchange_atomic_controller_packet(ctx, channel, buffer, controller,
expected)` receives one packet, applies the existing strict atomic controller,
explicitly closes the packet then sends a canonical response over the same channel.
The caller retains channel, controller and approved namespaces, supplies receive
capacity and owns flags, blocking, cancellation and final cleanup. No loop, implicit
initialization, retry or new policy is added.

Receive refusal never dispatches. Dispatch or packet-close refusal suppresses
output even when a business result is already known. The typed error retains it
through `known_reply()` and all independent causes through `error_branches()`,
including both `AfterUse` errors. Packet close is always attempted after dispatch.
Send refusal retains response and observation. Missing output proves neither no
effect nor permission to replay; successful transport still requires `Sent.reply`.

`paea-controlled-exchange.json` covers real kernel exchange, an atomically bound
child with independently captured parent domain, no-output refusals, recovery and
retained causes. Positive business callback remains synthetic/same-UID; public
Service covers UID refusal without mutation and supported reopen. Neither a
protected daemon nor business success across separate accounts is established.
[Plan and evidence63](plans/actions-confiance/paea/INCREMENT-63.md).

## Native version2 observation — local delivery216

Sources provide `identity linux-prerequisites --json [--lang fr|en]`, protected
`GET /api/v1/protection/linux-prerequisites` and **PAEA Linux prerequisites**, linked
from PAEA checks. These interfaces are delivered and verified locally in216; consult [plan64](plans/actions-confiance/paea/INCREMENT-64.md)
and the Version dialog for qualified delivery. No activation, helper/interpreter/
service/account/mapping execution or NSS network lookup occurs.

`swarm-linux-prerequisites`, re-exported as `swarm_engine::linux_prerequisites`,
provides one report type for CLI/API. Read kernel UID/GID/groups, **local** passwd,
only current username/numeric UID subuid/subgid assignments, current UID/GID maps
and first executable-looking PATH helper candidate. Original IO causes stay typed;
JSON exposes code/errno only. Invalid/unavailable sources never become successful
empty assignments. Valid empty ranges are explicit. No identity, allocation,
reservation, configuration or launch principal is selected automatically.

`schemas/paea/linux-prerequisites-v2.schema.json` describes version2, distinct from
historical Python version1. Shape is no authority/effect proof. Metadata uses the
root-setuid helper profile: regular executable-looking setuid file, root owner and
root-owned ancestors without group/other write. ACLs, file capabilities, actual
execution, stable custody and exclusivity are unproved. The UI rejects elementary
`metadata_safe` contradictions without establishing protection. All values belong
to the observed process namespace; no namespace/executable descriptors are held.

CLI requires explicit JSON and refuses incompatible input/Root/output/options,
even explicit defaults. API observes the **server**, under Host/session: GET only,
no query/body/selectors, no project read/Root effect. Browser observes server data,
not its own machine or a future agent. Missing prerequisites/errors remain visible;
failure clears stale results; keyboard navigation is preserved and download keeps
exact validated bytes. Even with complete metadata, launch/distinctUID/confinement/
human flags stay false. Privilege/acceptance operations remain separate, with no
permissive shared-UID fallback.

From216, open **PAEA checks → PAEA Linux prerequisites**, then **Observe the Linux
server**. For the CLI process from the installed checkout:

```sh
./bin/swarm identity linux-prerequisites --json --lang en
```

Network reports must be valid UTF-8 without BOM: retain valid body bytes and reject
replacement decoding or silent BOM removal. Failure clears stale report/export;
request a fresh valid observation to recover. Maintained recipes:
`paea-linux-prerequisites-core.json`, `paea-linux-prerequisites-ui.json`,
`paea-linux-prerequisites-native.json`. Local observation only. Delivery216 captures
bind its compiled inventory; later workspace sources may remain undelivered.
