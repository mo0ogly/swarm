#!/usr/bin/env python3
"""Input-consistency binding for the authority capability/owner-key contract.

Purpose — the consumer guard `node tests/authority_capability_consumer_test.cjs`
reads the THREE JS consumer sources (`web/native-authority-admin.js`,
`web/cockpit-authority.js`, `web/native-runtime.js`), the real generated
`web/i18n-en.js` and `locales/en.json` from the WORKSPACE, while the
owned-image UI script `tests/blp_admission_ui.py` reads the qualified image
and its profile archive. Without a binding step, a run could exercise the
guard on sources that are no longer the ones embedded in the image the UI
verified, or hand the UI a binary/profile pair that does not belong together.
This helper refuses BEFORE any later recipe step runs when:

  - the supplied binary's real SHA-256 does not equal the profile's
    `sha256` field, or the profile is not of the normal qualified shape —
    the four preconditions `status=VERIFIED_DEFAULT_NATIVE_PROFILE`,
    `no_fixture_feature`, `all_core_features_empty` and `all_core_test_false`
    must each be present with the strict expected type and value (string,
    then exactly True for the three booleans; a wrong type, False or an
    absent key is a named refusal);
  - any of the real workspace consumer inputs — the three JS consumer
    sources, `web/i18n-en.js`, `locales/en.json`,
    `rust/crates/swarm-web/src/service.rs` and `version_history.json` —
    no longer matches the hash recorded for it in the profile's effective
    inputs (`after.json` next to the profile).

Success prints one marker line and exits 0; every mismatch is an explicit
nonzero refusal naming the divergent entry. The helper performs read-only
hashing and validation only: it never rebuilds, activates, writes or
re-qualifies anything, and establishes input coherence only — not effective
graph qualification, independent review or any product right.

Inputs — `--binary` (real image path), `--profile` (qualified profile
`result.json`); `after.json` is read from the profile's directory, matching
the layout `tests/blp_admission_ui.py` already consumes — that UI script,
NOT this helper, reads `effective-sources.tar.gz`. Output — stdout marker
only; no evidence directory is created. Errors —
FileNotFoundError/JSONDecodeError propagate with their original message for
an unreadable path or malformed profile; `SystemExit(2)` with a named cause
for a binding mismatch. Limits — a match proves only that the checked bytes
are identical at binding time; a later workspace or image change invalidates
the result (the recipe runner re-snapshots declared inputs around each
step). No secret, environment capture or network use.
"""
import argparse,hashlib,json,sys
from pathlib import Path

# Exactly the consumer inputs the guard reads from the workspace and the
# image-serving/embedded files the owned UI exercises; all must be recorded
# in the profile's effective inputs.
BOUND_INPUTS=[
 'locales/en.json',
 'rust/crates/swarm-web/src/service.rs',
 'version_history.json',
 'web/cockpit-authority.js',
 'web/i18n-en.js',
 'web/native-authority-admin.js',
 'web/native-runtime.js',
]


def digest(path):
    """Hash complete file bytes with a 1 MiB buffer, like the receiving scripts.

    Purpose: byte-exact comparison between workspace inputs and the profile's
    recorded hashes, and between the binary and the profile's `sha256`.
    Inputs: `path`, an existing file. Output: lowercase hex SHA-256 string.
    Errors: OSError propagates (an unreadable input is a refusal, not a skip).
    Limits: hashes the named file only — no directory walk, no archive
    extraction; `after.json` hashes are trusted as the qualification wrote
    them, this function never recomputes archive members.
    """
    value=hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda:source.read(1024*1024),b''):value.update(chunk)
    return value.hexdigest()


def main():
    """Validate the binary/profile pair and workspace inputs against the profile.

    Purpose: refuse an incoherent capability/owner-key verification batch
    before the guard or the owned UI runs. Inputs: `--binary` image path and
    `--profile` qualified profile path (both required); the four profile
    preconditions (`status` string equal to
    `VERIFIED_DEFAULT_NATIVE_PROFILE`; `no_fixture_feature`,
    `all_core_features_empty`, `all_core_test_false` each exactly True) are
    validated with strict types before anything else; `BOUND_INPUTS` are
    resolved against the CURRENT WORKING DIRECTORY, so the caller must run
    from the repository root (the recipe runner does — `subprocess.run` with
    `cwd=root`). Output: on success one stdout line
    `AUTHORITY_CAPABILITY_INPUTS_BOUND <binary sha256>` and exit 0.
    Errors: original OSError/ValueError from unreadable/malformed files
    propagate; a binding mismatch prints `AUTHORITY_CAPABILITY_INPUTS_REFUSED
    <cause>` to stderr immediately and exits 2. Effects: none beyond reads.
    Limits: time-of-check coherence only — no rebuild, activation,
    qualification or acceptance; the caller re-checks nothing afterwards
    (recipe source snapshots cover later drift of declared files, not of the
    binary); `after.json` hashes are trusted as the qualification wrote them
    (the helper recomputes workspace hashes, never archive members).
    """
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--binary',type=Path,required=True)
    ap.add_argument('--profile',type=Path,required=True)
    args=ap.parse_args()
    binary=args.binary.resolve(strict=True)
    profile_path=args.profile.resolve(strict=True)
    profile=json.loads(profile_path.read_bytes())
    if type(profile.get('status'))is not str or profile.get('status')!='VERIFIED_DEFAULT_NATIVE_PROFILE':
        raise SystemExit(refused('profile status is not the normal qualified one: '+repr(profile.get('status'))))
    for field in ('no_fixture_feature','all_core_features_empty','all_core_test_false'):
        value=profile.get(field)
        if type(value)is not bool or value is not True:
            raise SystemExit(refused('profile '+field+' is not exactly True: '+repr(value)))
    binary_sha=digest(binary)
    if binary_sha!=profile.get('sha256'):
        raise SystemExit(refused('binary sha256 '+binary_sha+' does not match profile sha256 '+str(profile.get('sha256'))))
    inputs=json.loads((profile_path.parent/'after.json').read_bytes())
    for name in BOUND_INPUTS:
        workspace=digest(Path(name))
        recorded=inputs.get(name)
        if recorded is None:
            raise SystemExit(refused('input not recorded in profile after.json: '+name))
        if workspace!=recorded:
            raise SystemExit(refused('workspace input differs from profile after.json: '+name+' workspace='+workspace+' profile='+recorded))
    print('AUTHORITY_CAPABILITY_INPUTS_BOUND '+binary_sha,flush=True)


def refused(cause):
    """Print the named refusal immediately and yield exit code 2.

    Purpose: every divergence must be an explicit, attributable stop, never a
    warning or a silent continue. Inputs: `cause`, a short human-readable
    description naming the divergent entry. Output: the integer 2 — callers
    raise `SystemExit(refused(cause))`, so the value becomes the process exit
    code; the `AUTHORITY_CAPABILITY_INPUTS_REFUSED <cause>` line is printed
    to stderr immediately (flushed), not on interpreter teardown. Errors:
    none. Limits: diagnostic helper only; the cause is not machine-parsed by
    anything else; the message carries no secret (causes name fields and
    hashes only).
    """
    print('AUTHORITY_CAPABILITY_INPUTS_REFUSED '+cause,file=sys.stderr,flush=True)
    return 2


if __name__=='__main__':main()
