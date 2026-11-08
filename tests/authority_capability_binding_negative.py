#!/usr/bin/env python3
"""Reusable negative guard for the input-binding helper's refusals.

Purpose — the binding helper `tests/authority_capability_inputs.py` gates the
whole capability/owner-key recipe; a refusal lost to a refactor (a wrong type
silently accepted, a precondition check dropped) would let an incoherent
batch run the guard and the owned UI. This guard runs the REAL helper as a
subprocess (no import of the module under a monkey-patched state, no stub at
the CLI boundary) against a VALID witness first, then against one minimal
crafted perturbation per refusal class, and finally re-runs the witness to
prove a refusal leaves no state behind. Fault classes, each expected to exit
2 with `AUTHORITY_CAPABILITY_INPUTS_REFUSED` naming the divergent entry and
an EMPTY stdout (no BOUND marker before a refusal):

  - precondition fields of the normal profile: `status` (False, wrong type,
    absent) and each of `no_fixture_feature`, `all_core_features_empty`,
    `all_core_test_false` (False, wrong type, absent — wrong type covers
    int 1 and the string "true");
  - binary/profile mismatch: the profile's `sha256` replaced by 64 zeros;
  - inputs: an entry absent from `after.json`, then an entry whose recorded
    hash diverges from the real workspace bytes.

Decoupling — no copied ELF and no archive is built or read. The witness
"binary" IS a small real workspace file hashed at runtime with the same
streaming 1 MiB algorithm; the witness profile declares that real digest in
its `sha256` field, and the witness `after.json` records the real digests of
the helper's `BOUND_INPUTS` (imported from the helper itself, never
retyped), computed from the actual files. Only the fault cases perturb one
value at a time. The crafted profiles are tiny JSON files under a fresh
owned `fixture_area.fixture_root('authority-capability-binding-')` root —
never `/tmp`, never the real profile directory, nothing under the repository
except published code. Output — per-case progress lines, then
`AUTHORITY_CAPABILITY_BINDING_NEGATIVE_PASS cases=<n>` (the count is
informational, not an assertion) and
`AUTHORITY_CAPABILITY_BINDING_NEGATIVE_WITNESS_SHA256 <sha>` on stdout;
`results.json` retains every observed exit/stdout/stderr in the fixture
root. Errors — any unexpected exit, unexpected stdout/stderr shape or
missing marker raises AssertionError and fails the process (exit 1, traceback
retained). Limits — this guard establishes that the helper REFUSES what it
must refuse and accepts the crafted witness; it does not qualify the image,
the effective graph or the real 330 profile (the recipe's separate steps and
Codex's own runs cover those), grants no right and runs no network, shell or
watchdog. Executed by Codex only; authored UNEXECUTED by GLM.
"""
import argparse,json,subprocess,sys
from pathlib import Path
sys.path.insert(0,'tests')
from authority_capability_inputs import BOUND_INPUTS,digest
from fixture_area import fixture_root

WITNESS_INPUT='version_history.json'
# One divergent entry for the input faults; its real hash stays in after.json
# for every other case so a refusal can only name this entry.
DIVERGENT_INPUT='web/native-runtime.js'

# Per-field perturbations: `absent` removes the key, `false` sets the strict
# bool False, `wrong-type` sets a value of another real JSON type. `status`
# is a string in the valid profile, so its wrong type uses an int.
FAULTS={
 'status':[('false',False),('wrong-type',5),('absent',None)],
 'no_fixture_feature':[('false',False),('wrong-type','true'),('absent',None)],
 'all_core_features_empty':[('false',False),('wrong-type',1),('absent',None)],
 'all_core_test_false':[('false',False),('wrong-type','true'),('absent',None)],
}


def case_profiles(root, witness_sha, input_hashes):
    """Craft one tiny per-case profile directory and return its case list.

    Purpose: give every refusal class its own minimal `result.json` (witness
    bytes with exactly one perturbation) beside an `after.json` recording the
    real input digests — the helper reads `after.json` from the profile's
    parent, so each case needs its own directory. Inputs: `root`, the fresh
    fixture directory; `witness_sha`, the runtime digest of the real witness
    file; `input_hashes`, the real per-input digests. Output: a list of
    `(case name, profile path, expected exit, expected stderr substring)`
    tuples in execution order, witness first, recovery last. Errors: OSError
    propagates. Limits: creates only tiny JSON files (no ELF copy, no
    archive); mutates nothing outside `root`.
    """
    witness=dict(status='VERIFIED_DEFAULT_NATIVE_PROFILE',no_fixture_feature=True,
                 all_core_features_empty=True,all_core_test_false=True,
                 sha256=witness_sha)

    def write(name,profile,after=None):
        """Create one case directory with its result.json and after.json."""
        directory=root/('case-'+name);directory.mkdir(mode=0o700)
        (directory/'result.json').write_text(json.dumps(profile,indent=1)+'\n')
        (directory/'after.json').write_text(json.dumps(after if after is not None else input_hashes,indent=1)+'\n')
        return directory/'result.json'

    cases=[('witness',write('witness',witness),0,'')]
    for field,perturbations in FAULTS.items():
        for kind,value in perturbations:
            broken=dict(witness)
            if value is None:broken.pop(field)
            else:broken[field]=value
            cases.append((field+'-'+kind,write(field+'-'+kind,broken),2,
                          'AUTHORITY_CAPABILITY_INPUTS_REFUSED profile '+field))
    mismatch=dict(witness,sha256='0'*64)
    cases.append(('binary-sha-mismatch',write('binary-sha-mismatch',mismatch),2,'REFUSED binary sha256'))
    missing=dict(input_hashes);missing.pop(DIVERGENT_INPUT)
    cases.append(('missing-input',write('missing-input',witness,missing),2,'REFUSED input not recorded in profile after.json: '+DIVERGENT_INPUT))
    wrong=dict(input_hashes);wrong[DIVERGENT_INPUT]='0'*64
    cases.append(('wrong-input',write('wrong-input',witness,wrong),2,'REFUSED workspace input differs from profile after.json: '+DIVERGENT_INPUT))
    cases.append(('recovery',write('recovery',witness),0,''))
    return cases


def main():
    """Run the real binding helper per case and retain observed evidence.

    Purpose: keep the helper's refusal contract permanently guarded. Inputs:
    none (arguments are refused by argparse). Output: per-case progress lines
    and the two final markers on stdout; `results.json` in a fresh owned
    fixture root with every observed exit/stdout/stderr. Errors:
    AssertionError on any unexpected observation (process fails with its
    traceback); OSError from the fixture area propagates. Effects: creates
    and leaves the fixture evidence in the owned area (retention, not
    cleanup, is the area's contract). Limits: asserts the helper's refusal
    behavior only, on crafted tiny profiles and real workspace digests — no
    image qualification, no acceptance, no network/shell/watchdog; the
    witness "binary" is a real small workspace file, never a copied ELF.
    """
    argparse.ArgumentParser(description=__doc__).parse_args()
    binary=(Path('.')/WITNESS_INPUT).resolve(strict=True)
    witness_sha=digest(binary)
    input_hashes={name:digest(Path(name)) for name in BOUND_INPUTS}
    root=fixture_root('authority-capability-binding-')/'negative-guard'
    root.mkdir(mode=0o700)
    cases=case_profiles(root,witness_sha,input_hashes)
    results=[]
    bound=('AUTHORITY_CAPABILITY_INPUTS_BOUND '+witness_sha+'\n').encode()
    for name,profile,expected_exit,expected_stderr in cases:
        run=subprocess.run([sys.executable,'tests/authority_capability_inputs.py',
                            '--binary',str(binary),'--profile',str(profile)],
                           capture_output=True,cwd='.')
        if expected_exit==0:
            # Success shape is exact: only the BOUND line on stdout, nothing
            # on stderr — a refusal accidentally turned into success cannot
            # hide behind an empty expectation.
            ok=run.returncode==0 and run.stdout==bound and run.stderr==b''
        else:
            ok=(run.returncode==2 and run.stdout==b''
                and expected_stderr.encode()in run.stderr)
        if not ok:
            raise AssertionError('binding-negative '+name+': exit='+str(run.returncode)
                                 +' expected='+str(expected_exit)
                                 +' stdout='+repr(run.stdout)+' stderr='+repr(run.stderr))
        print('NEGATIVE',name,'exit='+str(run.returncode),flush=True)
        results.append(dict(case=name,exit=run.returncode,stdout=run.stdout.decode(),stderr=run.stderr.decode()))
    (root/'results.json').write_text(json.dumps(results,indent=1)+'\n')
    print('AUTHORITY_CAPABILITY_BINDING_NEGATIVE_PASS cases='+str(len(results)),flush=True)
    print('AUTHORITY_CAPABILITY_BINDING_NEGATIVE_WITNESS_SHA256 '+witness_sha,flush=True)


if __name__=='__main__':main()
