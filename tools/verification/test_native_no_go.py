"""Owned finite no-Go wrapper fixtures, including an ignored refused tool.

No real Go/Cargo, provider, live Root or deadline is used. The broad native
recipe separately exercises Rust. Fresh roots/results remain for diagnostics.
"""
import json, sys, tempfile, unittest
from pathlib import Path
from native_no_go import run


class NoGoTests(unittest.TestCase):
    """Require a valid witness before explicit dependency/failure variants."""
    def setUp(self):
        """Keep an owned tiny source fixture; no real project data is copied."""
        self.root=Path(tempfile.mkdtemp(prefix='native-no-go141-'))
        (self.root/'source.txt').write_text('fixture input\n');self.path=self.root/'recipe.json'
        self.value=dict(schema_version=1,id='fixture',inputs=['source.txt'],environment={},steps=[self.step("print('named fixture PASS')")])
        self.write()

    def step(self,code):
        """Explicit finite fixture program, not engine policy or permission."""
        return dict(id='fixture',command=[sys.executable,'-c',code],expected_exit=0,required_stdout=['named fixture PASS'],required_stderr=[])

    def write(self):
        """Only replace this test's own recipe before its immutable run."""
        self.path.write_text(json.dumps(self.value))

    def invoke(self,name):
        """Retain component and wrapper decisions separately for actual inspection."""
        return run(self.root,self.path,self.root/name)

    def test_witness_and_exclusive_history(self):
        """Passing finite native command has no trap marker; history is exclusive."""
        result=self.invoke('valid');self.assertEqual(result['status'],'PASS');self.assertFalse(result['go_path_invoked'])
        before=(self.root/'valid/result.json').read_bytes()
        with self.assertRaises(ValueError):self.invoke('valid')
        self.assertEqual((self.root/'valid/result.json').read_bytes(),before)

    def test_ignored_go_or_gofmt_refusal_still_blocks_claim(self):
        """Positive witness then ignored trap status; a green component is insufficient."""
        self.assertEqual(self.invoke('valid')['status'],'PASS')
        for tool in ('go','gofmt'):
            self.value['steps']=[self.step("import subprocess; subprocess.run(["+repr(tool)+",'version'],check=False); print('named fixture PASS')")];self.write()
            result=self.invoke(tool);self.assertEqual(result['status'],'FAIL');self.assertTrue(result['go_path_invoked'])
            self.assertEqual(result['component_status'],'PASS')
            self.assertEqual(json.loads((self.root/tool/'go-invocation.json').read_bytes())['tool'],tool)

    def test_declared_go_and_failed_component_refuse(self):
        """A valid witness precedes missing/explicit forbidden dependency and exit7."""
        self.assertEqual(self.invoke('valid')['status'],'PASS')
        self.value['inputs'].append('go.mod');self.write()
        with self.assertRaisesRegex(ValueError,'go-input'):self.invoke('declared-input')
        self.assertFalse((self.root/'declared-input').exists());self.value['inputs']=['source.txt']
        self.value['steps'][0]['command']=['/not-executed/go','version'];self.write()
        with self.assertRaisesRegex(ValueError,'go-command'):self.invoke('declared-command')
        self.value['steps']=[self.step("import sys; print('original fixture failure',file=sys.stderr); sys.exit(7)")];self.write()
        result=self.invoke('failed');self.assertEqual(result['status'],'FAIL');self.assertEqual(result['component_status'],'FAIL')
        component=json.loads((self.root/'failed/component/result.json').read_bytes());self.assertEqual(component['steps'][0]['exit'],7)
        self.assertEqual((self.root/'failed/component/step-0001.stderr').read_text(),'original fixture failure\n')


if __name__=='__main__':unittest.main(verbosity=2)
