# PAEA — increment 51: Linux boundary for the attributed channel

Resume after verified dev180 delivery. PA-02 objective: receive a SOCK_SEQPACKET packet with the kernel-supplied sender pidfd, hold process namespaces through that descriptor, and pass data to independent admission. Transport alone grants no mandate, human session or confinement evidence.

Installed Nix 0.30.1 has connection SO_PEERPIDFD, not SO_PASSPIDFD/SCM_PIDFD; cmsgs() refuses truncation before iteration although received FDs must be closed. Rustix 1.1.5 does not cover the required operations. Never substitute connection creator or a claimed PID.

Compared waiting for a qualified upstream wrapper, maintaining a complete Nix fork, and a small explicit Linux FFI crate. Select the explicit boundary: borrowed socket, initialized bytes, unique ownership of kernel-received FDs, no safe API accepting caller-chosen raw descriptor numbers. Existing Swarm crates retain forbid(unsafe_code); the new FFI is explicitly declared, documented and reviewed, never hidden in a macro or presented as a wholly unsafe-free repository. Use locked libc types/constants and verify missing UAPI names against Linux sources. No listener/command activation before qualification or new operator policy/deadline/retry.

Codex owns: (1) minimal seqpacket configuration/guarded receive/held pidfd and namespace API with syscall/refusal/cleanup causes; (2) real positive witnesses then payload/ancillary truncation, wrong socket, unexpected rights, empty packet, missing options, close/recovery, FD counts/identity, actual child sender on parent-created connection, and exited sender refusal; (3) strict Rustdoc, byte-bound policy/function/error/unsafe review and maintained recipe without arbitrary watchdog; (4) only then service/bound-consumer admission and delivery. A library without a consumer remains a library, not an installed daemon.

uidmap helpers, distinct OS account, human enrollment, personal daemon, E2/E3, PA-13 and 0/17 globals remain open. Tests and FFI review are not independent evaluation. Direct Codex only, no GLM/delegation/commit/push. Preserve original failures and require a changed prerequisite before retries.

## Library result — integration remains open

Implemented and verified on x86_64 / Linux 6.17.0-41-generic. Nine Rust cases:
eight in `recipe-07`, then the named nonblocking/shutdown case in `nonblocking-10`.
Current lib/kernel bytes exactly match 07; its eight original test functions and
helpers remain an exact prefix of the current file. Strict private Rustdoc PASS.
After ABI/ownership review the fixture was rebuilt (`fixture-build-08`), then both
visible/invisible-subject journeys replayed in `native-09` on new ELF SHA256
`76bc13227691bb8c460538ea59325eee97e1ed00a601ad02ec504477a3342086`.
All are stable with verified integrity. `red-01` is expected missing-feature stub
refusal; `recipe-03` compiled zero tests before missing uio diagnosis; `recipe-04`
retains eight PASS then Rustdoc FAIL. uio is test-only, no existing dependency
version changed. Compile-time c_int/ucred/cmsghdr layout checks and explicit private
kernel-output ownership preconditions protect this FFI boundary.

Credentials identify the kernel-checked subject under Linux privilege rules, not
absolute physical sender identity or a human. Namespace/pidfd observations are
admission data. Partial second-ioctl failure preserves separate cleanup causes in
code, but that exact kernel fault window was not injected. No Miri, sanitizer,
other-architecture or independent-review claim. FR/EN diagrams rendered/viewed and
actual Markdown fence recognition checked.

Evidence: `evidence/20261027-channel51-nunzg4wm/` in the French plan tree. Byte-bound
policy/function/error/unsafe review in `source-review.json`, source archive and Linux
headers retained. No secret keys, personal proof or live mission fixtures. No public
consumer yet: grouped integration/local delivery remains 52 work, not an activated
library claim. Last server qualified by this task is dev180 (50 point-in-time proof).
UID/human prerequisites and global assurance remain open.

Expanded documentation check:177 historical missing references to90paths across README and both large migration pages, allbefore51; none in currentindex/HEAD. REPLAYS alreadymarks originals absent. Inventory/diagnosis retained with migrationowner and verifiedoriginalarchive resumptioncondition; no allGit-history/backupaudit claim. Own51links checkedseparately; whole-documentconformance remains open.
