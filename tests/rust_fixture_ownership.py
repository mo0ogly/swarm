#!/usr/bin/env python3
"""Keep fixture birth attribution even when actual CLI initialization fails.

Healthy control uses the real supplied CLI, no provider call. The only double
exits2 on init: no fake positive Root/SQL/receipt is produced or accepted.
"""
import argparse,json,traceback
from pathlib import Path
import rust_agent_model_route as fixture


def marker(output):
    """Read the one unique birth entry, not a Root guessed by mtime/prefix."""
    found=list(output.glob('owned-root-*.json'));assert len(found)==1
    value=json.loads(found[0].read_bytes());assert value['schema_version']==1 and value['kind']=='owned_inert_fixture_birth' and not value['actor_authenticated'] and not value['runtime_permission']
    assert Path(value['root']).is_dir();return value


def main():
    """Exercise missing output directories/string paths, then failed CLI init.

    Metadata is attribution only; preserve both Roots and original command
    channels. No cleanup, native inference or permission is implied.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        real=a.binary.resolve();control_dir=out/'control'/'not-created';control=fixture.Fixture(str(real),control_dir,'birth-control');first=marker(control_dir);assert first['root']==str(control.root) and first['binary_sha256']==fixture.digest(real.read_bytes()) and (control.root/'.swarm/state.db').is_file() and not (control.root/'model-received.json').exists()
        stub=out/'init-failure';fixture.publish(stub,b'#!/usr/bin/python3\nimport sys,json\nprint(json.dumps(dict(failure=dict(code="owned-intentional-init-failure"))),file=sys.stderr)\nsys.exit(2)\n');stub.chmod(0o700)
        failed_dir=out/'failure'/'not-created'
        try:fixture.Fixture(str(stub),failed_dir,'birth-failed-init')
        except AssertionError as error:assert 'owned-intentional-init-failure' in str(error),error
        else:raise AssertionError('Failed init unexpectedly accepted')
        failed=marker(failed_dir);assert failed['binary_sha256']==fixture.digest(stub.read_bytes()) and not (Path(failed['root'])/'.swarm/state.db').exists() and not (Path(failed['root'])/'model-received.json').exists()
        assert list(failed_dir.glob('*.stderr')) and list(failed_dir.glob('*.stdout'))
        fixture.publish(out/'result.json',dict(status='PASS',healthy_root=first['root'],failed_root=failed['root'],missing_output_created=True,string_binary_supported=True,birth_before_cli=True,failed_init_attributed=True,provider_started=False));print('PASS fixture birth survives failed initialization',flush=True)
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
