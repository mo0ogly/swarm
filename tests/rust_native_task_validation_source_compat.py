#!/usr/bin/env python3
"""Compare actual scoped source observations across two explicit native binaries.

An owned public planner/worker fixture is built by the retained earlier binary.
Both previews read the same real Root and revision. No controls, source rewrites,
provider account, seed, live Root or global watchdog. The extra ordinary authority
DTO must leave every prior scoped value and source hash unchanged.
"""
import argparse, json, traceback
from pathlib import Path
import rust_native_task_validation as native


def main():
    """Retain exact binaries, public command channels and source bytes at both ends."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--old-binary',required=True,type=Path)
    p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path)
    a=p.parse_args();old=a.old_binary.resolve();new=a.binary.resolve();out=a.output.resolve();out.mkdir(exist_ok=False)
    inputs=dict(old_binary=str(old),old_sha256=native.fixture.digest(old.read_bytes()),binary=str(new),
        binary_sha256=native.fixture.digest(new.read_bytes()),source_sha256=native.fixture.digest(Path(__file__).read_bytes()))
    native.fixture.publish(out/'inputs.json',inputs)
    try:
        setup=native.public.flow(old,out,control_script=native.CONTROL);root=Path(setup['root']);work=setup['work_id']
        r=dict(schema_version=1,event_id='source-compat',expected_revision=setup['revision'],task_id='task')
        code,before=native.invoke(old,root,out,'before',['work','show',work]);assert code==0
        code,previous=native.invoke(old,root,out,'previous',['task','validation-preview',work],r);assert code==0,previous
        code,current=native.invoke(new,root,out,'current',['task','validation-preview',work],r);assert code==0,current
        assert current.pop('authority_mode')=='planning-task-policy'
        assert current==previous and current['source_sha256']==previous['source_sha256']
        code,after=native.invoke(old,root,out,'after',['work','show',work]);assert code==0 and after==before
        assert not (root/'validation-executed.json').exists()
        assert native.fixture.digest(old.read_bytes())==inputs['old_sha256'] and native.fixture.digest(new.read_bytes())==inputs['binary_sha256']
        assert native.fixture.digest(Path(__file__).read_bytes())==inputs['source_sha256']
        result=dict(status='PASS',scoped_source_unchanged=True,prior_preview_values_preserved=True,
            no_control_execution=True,no_work_mutation=True,old_binary_sha256=inputs['old_sha256'],binary_sha256=inputs['binary_sha256'])
        native.fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:
        native.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
