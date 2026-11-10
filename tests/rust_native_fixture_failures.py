#!/usr/bin/env python3
"""Verify helper failure ownership with a real CLI refusal and controlled bad JSON.

Private fixture data only; no server/provider/live Root. A valid witness precedes
both injections. Preserve original private diagnostics and copy only these known
benign refusal outputs to replay evidence. Success cleanup is owned and checked.
"""
import argparse,json,tempfile,shutil,sys,traceback
from pathlib import Path
import rust_native_http_cli as native
import rust_agent_model_route as retained


def refusal(binary,root,arguments,expected_cause=None):
    """Require failure with exact original bytes in a private diagnostic directory."""
    try:native.cli(binary,root,arguments)
    except BaseException as error:
        directory=getattr(error,'evidence_directory',None);assert directory is not None,'CLI failure discarded original diagnostics'
        assert directory.stat().st_mode&0o777==0o700
        assert (directory/'stdout.bin').read_bytes()==error.result.stdout
        assert (directory/'stderr.bin').read_bytes()==error.result.stderr
        if expected_cause is not None:assert isinstance(error.__cause__,expected_cause)
        return error,directory
    raise AssertionError('Controlled refusal became success')


def main():
    """Bind source/binary inputs; retain failed Roots and every original error."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();binary=a.binary.resolve();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    retained.publish(output/'inputs.json',{str(binary):retained.digest(binary.read_bytes()),'helper':retained.digest(Path(native.__file__).read_bytes()),'driver':retained.digest(Path(__file__).read_bytes())})
    root=native.owned_root(binary);private=[]
    try:
        before=(root/'.swarm/engine-settings.json').read_bytes();assert native.cli(binary,root,['engine-settings','show'])['revision']=='1'
        error,directory=refusal(binary,root,['unknown-owned-fixture-operation']);private.append(directory);assert error.result.returncode==2
        # This deliberate unknown-command output has no session/provider data.
        retained.publish(output/'refused.stdout',error.result.stdout);retained.publish(output/'refused.stderr',error.result.stderr)
        assert native.cli(binary,root,['engine-settings','show'])['revision']=='1' and (root/'.swarm/engine-settings.json').read_bytes()==before
        print('PASS fixture failure real refusal retains original output and recovers',flush=True)
        fake=root/'controlled-json.py';source=b'#!/usr/bin/env python3\nimport sys\nprint("invalid fixture" if sys.argv[1]=="invalid" else "{}")\n';retained.publish(fake,source);fake.chmod(0o700);retained.publish(output/'controlled-json.py',source)
        assert native.cli(fake,root,['valid'])=={}
        error,directory=refusal(fake,root,['invalid'],json.JSONDecodeError);private.append(directory);assert error.result.returncode==0
        retained.publish(output/'malformed.stdout',error.result.stdout);assert native.cli(fake,root,['valid'])=={}
        print('PASS fixture failure malformed JSON preserves parse cause without fallback',flush=True)
        retained.publish(output/'result.json',dict(status='PASS',cases=2,scope='Real CLI refusal plus controlled JSON helper fixture; no Root mutation/provider',normal_binary_sha256=retained.digest(binary.read_bytes())))
    except BaseException as error:
        retained.publish(output/'failure.json',dict(status='FAIL',type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),retained_root=str(root)));raise
    for directory in private:shutil.rmtree(directory)
    shutil.rmtree(root)


if __name__=='__main__':main()
