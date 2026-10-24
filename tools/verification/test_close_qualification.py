"""Isolated closure source-resolution regressions; no compiler, provider or live root."""
import importlib.util
import hashlib
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('close_qualification',Path(__file__).with_name('close_qualification.py'))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class ClosureTests(unittest.TestCase):
    """Valid witness precedes changed, missing and unresolved input refusals."""
    def test_all_namespaces_and_tamper_refusal(self):
        """Every source namespace is hashed; changing any retained file invalidates closure."""
        with tempfile.TemporaryDirectory() as directory:
            base=Path(directory);root=base/'root';goroot=base/'goroot';registry=base/'registry';external=base/'generated'
            files=[root/'code.rs',goroot/'src/runtime.go',registry/'lib.rs',external/'generated.go']
            for path in files:path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'original')
            keys=['code.rs','go-toolchain/src/runtime.go','registry/fixture-1/lib.rs','external-go/'+hashlib.sha256(str(external).encode()).hexdigest()+'/generated.go']
            inputs={key:hashlib.sha256(b'original').hexdigest()for key in keys}
            metadata=dict(packages=[dict(name='fixture',version='1',source='registry',manifest_path=str(registry/'Cargo.toml'))])
            records=[dict(Dir=str(external),CompiledGoFiles=['generated.go'])]
            args=(root,inputs,dict(GOROOT=str(goroot)),metadata,records)
            module.verify_inputs(*args)
            for path in files:
                path.write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError,'missing-or-changed'):module.verify_inputs(*args)
                path.write_bytes(b'original');module.verify_inputs(*args)
            files[0].unlink()
            with self.assertRaisesRegex(ValueError,'missing-or-changed'):module.verify_inputs(*args)

    def test_unresolved_and_traversal_never_skip(self):
        """A valid witness precedes invalid paths; no missing external hash is ignored."""
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/'code.rs';path.write_bytes(b'original')
            sha=hashlib.sha256(b'original').hexdigest();args=(root,{'code.rs':sha},{'GOROOT':str(root)},dict(packages=[]),[])
            module.verify_inputs(*args)
            for key in ['../outside','/outside','external-go/unknown/file.go','registry/unknown/lib.rs']:
                with self.subTest(key=key),self.assertRaises(ValueError):module.verify_inputs(root,{key:sha},args[2],args[3],args[4])

    def test_recorded_commands_are_data_not_execution_authority(self):
        """Valid recorded inventories pass; arbitrary executables/flags/hosts refuse."""
        host='x86_64-unknown-linux-gnu'
        valid=['go','list','-compiled','-deps','-test','-json','./cmd/swarm','./internal/engine','./tools/rust-migration/work-layout']
        self.assertEqual(module.inventory_command('go-inputs-normal',valid,host),valid)
        for bad in [['python3','-c','raise SystemExit(0)'],valid+['-exec','untrusted'],['go','test'],[]]:
            with self.assertRaisesRegex(ValueError,'command-invalid'):module.inventory_command('go-inputs-normal',bad,host)
        for bad_host in ['--help','host;command','../host']:
            with self.assertRaisesRegex(ValueError,'host-invalid'):module.inventory_command('go-inputs-normal',valid,bad_host)


if __name__=='__main__':unittest.main(verbosity=2)
