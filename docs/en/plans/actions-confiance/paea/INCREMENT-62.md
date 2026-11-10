# PAEA — increment62: namespace-owner inspection permission

61library27tests PASS, delivery pending shared-source stability;209/60lastqualified.
Independent test-only investigation. Linux6.17 capability-owner/ptrace/pidfs source
suggests creator in parent can inspect its owned child userns without initial
CAP_SYS_PTRACE, including nondumpable target. Not distinct-UID proof.
Twoactualsame-outer-UID profiles, parentCapEff0: hostdumpable1control/0EACCES/1recovery,
then ownedprivateuserns+mount with same3phases, expected access at0. Atomicpidfd,
kernel packet subject, actual mount objects, explicit closes/reap even onassertion.
No host/sysctl/account/helper change or business admission. Only fixture/test/recipe/
docs change, no productionAPI/version;61stillpending. Maphelpers/actualUIDseparation/
confinement/human/E2/E3/global0of17 remainopen. Constants3/4installedprctlUAPI.
Primary sources Linuxv6.17 security/commoncap.c/kernel/ptrace.c/fs/pidfs.c and
https://man7.org/linux/man-pages/man7/user_namespaces.7.html. Googlesource access
refused; officialtorvalds/linux tag readable. Upstream differs from exactUbuntu
6.17.0-41build; actualruntimewitness required.

## Verified local result

Two named Rust tests pass from the frozen test executable: six permission phases
across two profiles, plus native EINVAL after valid controls in both null/capture
configurations. Current owner09; owner01–08 retain historical exact sources.
Parent effective set remains zero; external UIDs remain equal. No account/helper/
host capability/sysctl/live mission/datastore change. Parent ownership is a locally
qualified launcher path; actual cross-UID permission still needs mapping helpers.
No gate58–61 weakened, first-caller adoption or business authority inferred.

Fixture stderr loss was corrected. Original prctl/unshare errors stay in private
capture; deliberate null-sink counterexample and capture recovery observe the same
EINVAL after valid control. Earlier null-stderr fixtures keep a diagnostic followup,
not rewritten history. Known child on launch refusal is reaped/closed; protocol
assertions close endpoints and wait/close pidfd before rethrowing.

Collector retains normal rlib(features empty/testfalse) and explicitly test-only
executable. Original compiler/test channels, frozen binary and metadata in frozen07
bind via resultSHA printed into owner09 stdout. Local verification checks six fixed
regular files;5intact controls and3private-copy byte/kind refusals pass, originals
intact. Nine recipe integrities pass. Local hashes prove no signed provenance,
independent acceptance or full effective compiler graph. Three upstreamLinux6.17
reference sources retain URL/SHA/SPDX, no remote code executed/integrated. Exact
Ubuntu source not established; actual exercised kernel6.17.0-41-generic.

Byte-bound review/archive cover policies/functions/errors/docs. No library Rustdoc
required because no productionAPI changed; documented fixture helpers actually
compile/run. No62runtime version/install: test/fixture/recipe/docs only, excluded
from observed normal path.61delivery still pending stable candidate,209/60 last
qualified by this task. PA02partial/global0of17/E2/E3/PA13/RETEX remainopen.

Metadata-symlink acceptance was reproduced before correction, then refused while
the original still verifies. Permanent guards cover changed bytes and artifact
symlink too:5controls/3refusals, originals untouched. See replan-metadata-kind.json
and frozen07/verification-copies.

After this qualification,61delivery verified inv0.1.1-dev.211, ELF
`20444629f554417d220d882f09113ff76cf7e48a17bc40b4285a878fcc2825f7`. All12increment62inputs are excluded
from qualified normal211compiler graph per retained supplement. Earlier61pending
statements above are historical.

The permanent manifest no longer fixes a past output directory. Collector creates
a fresh private child of an explicit parent; two unedited permanent runs(owner08/09)
produce distinct directories and pass all checks. Integrity guards precede final
receipt. Actual path/anchor in portable-replays.json and owner09 stdout; earlier
frozen07 remains historical. No unsupported recipe output placeholder is assumed.
