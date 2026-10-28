"""Isolated regression checks for permanent recipe execution and evidence refusal.

No provider, live root, retry or global timeout; subprocesses are finite fixtures.
"""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('recipe', Path(__file__).with_name('recipe.py'))
recipe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recipe)


class RecipeTests(unittest.TestCase):
    """Exercise successful witnesses before failure, tamper and recovery variants."""
    def setUp(self):
        """Create a private source and explicit observable recipe, never a live DB."""
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root/'source.txt').write_text('original\n')
        self.path = self.root/'recipe.json'
        self.value = dict(schema_version=1, id='fixture', inputs=['source.txt'], environment={}, steps=[self.step("print('named-test PASS')")])
        self.write()

    def step(self, code, name='witness'):
        """Build a finite subprocess assertion, with no shell or temporal deadline."""
        return dict(id=name, command=[sys.executable, '-c', code], expected_exit=0, required_stdout=['named-test PASS'], required_stderr=[])

    def write(self):
        """Persist the current isolated recipe before running its frozen source."""
        self.path.write_text(json.dumps(self.value))

    def run_fixture(self, name):
        """Execute once into fresh evidence and return observed result plus location."""
        output = self.root/name
        return recipe.run_recipe(self.root, self.path, output), output

    def test_pass_verify_and_exclusive_history(self):
        """PASS requires a named assertion; replay never overwrites prior evidence."""
        result, output = self.run_fixture('pass')
        self.assertEqual(result['status'], 'PASS')
        self.assertEqual(recipe.verify_evidence(output)['recorded_status'], 'PASS')
        original = (output/'result.json').read_bytes()
        with self.assertRaises(ValueError):
            recipe.run_recipe(self.root, self.path, output)
        self.assertEqual((output/'result.json').read_bytes(), original)
        with self.assertRaises(FileExistsError):
            recipe.publish(output/'result.json', b'overwrite')
        self.assertEqual((output/'result.json').read_bytes(), original)

    def test_exit_failure_retains_cause_and_stops_then_recovers(self):
        """A valid witness precedes refusal; stderr survives and later steps stay idle."""
        self.assertEqual(self.run_fixture('witness')[0]['status'], 'PASS')
        self.value['steps'] = [self.step("import sys; print('cause', file=sys.stderr); sys.exit(7)"), self.step("print('named-test PASS')", 'later')]
        self.write()
        result, output = self.run_fixture('failure')
        self.assertEqual(result['status'], 'FAIL')
        self.assertEqual(result['steps'][0]['exit'], 7)
        self.assertEqual(result['failure']['cause'], 'recipe-unexpected-exit')
        self.assertEqual((output/'step-0001.stderr').read_text(), 'cause\n')
        self.assertFalse((output/'step-0002.stdout').exists())
        self.assertEqual(recipe.verify_evidence(output)['recorded_status'], 'FAIL')
        self.value['steps'] = [self.step("print('named-test PASS')")]
        self.write()
        self.assertEqual(self.run_fixture('recovered')[0]['status'], 'PASS')

    def test_zero_tests_and_source_mutation_cannot_pass(self):
        """Exit zero alone and changed declared source both fail observable checks."""
        self.assertEqual(self.run_fixture('witness')[0]['status'], 'PASS')
        self.value['steps'] = [self.step("print('running 0 tests')")]
        self.write()
        self.assertEqual(self.run_fixture('zero')[0]['failure']['cause'], 'recipe-observable-assertion-missing')
        self.value['steps'] = [self.step("from pathlib import Path; Path('source.txt').write_text('changed'); print('named-test PASS')"), self.step("print('named-test PASS')", 'later')]
        self.write()
        result, output = self.run_fixture('mutation')
        self.assertEqual(result['failure']['cause'], 'recipe-declared-inputs-changed')
        self.assertFalse(result['source_stable'])
        self.assertFalse((output/'step-0002.stdout').exists())

    def test_tampering_and_missing_hash_table_refuse(self):
        """A valid retained witness is invalidated by output or manifest tampering."""
        _, output = self.run_fixture('pass')
        recipe.verify_evidence(output)
        stdout = output/'step-0001.stdout'
        original = stdout.read_bytes()
        stdout.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            recipe.verify_evidence(output)
        stdout.write_bytes(original)
        result = json.loads((output/'result.json').read_bytes())
        result['artifacts'] = {}
        (output/'result.json').write_text(json.dumps(result))
        with self.assertRaisesRegex(ValueError, 'table-incomplete'):
            recipe.verify_evidence(output)

    def test_invalid_manifests_refuse_before_output(self):
        """Reject ambiguous schema, unknown fields, bool exits and empty assertions."""
        for mutation in ('bool', 'unknown', 'assertions', 'missing'):
            with self.subTest(mutation=mutation):
                candidate = json.loads(json.dumps(self.value))
                if mutation == 'bool': candidate['steps'][0]['expected_exit'] = True
                if mutation == 'unknown': candidate['unknown'] = 1
                if mutation == 'assertions': candidate['steps'][0]['required_stdout'] = []
                if mutation == 'missing': candidate['inputs'] = ['absent']
                self.path.write_text(json.dumps(candidate))
                with self.assertRaises(ValueError):
                    recipe.run_recipe(self.root, self.path, self.root/mutation)
                self.assertFalse((self.root/mutation).exists())
        with self.assertRaisesRegex(ValueError, 'duplicate-key'):
            recipe.load_recipe(b'{"schema_version":1,"schema_version":1}')

    def test_relative_executable_is_bound_to_fixture_root(self):
        """Record and run the same root-relative launcher, independent of caller cwd."""
        launcher = self.root/'bin'/'witness'
        launcher.parent.mkdir()
        launcher.write_text('#!'+sys.executable+'\nprint("named-test PASS")\n')
        launcher.chmod(0o700)
        self.value['steps'][0]['command'] = ['./bin/witness']
        self.write()
        result, output = self.run_fixture('relative')
        self.assertEqual(result['status'], 'PASS')
        self.assertEqual(result['steps'][0]['executable'], str(launcher))
        self.assertEqual(recipe.verify_evidence(output)['recorded_status'], 'PASS')


    def test_symlink_launcher_keeps_invocation_name_and_target_identity(self):
        """Preserve multicall argv0 for absolute, relative and PATH aliases.

        The direct real launcher is the healthy control. All three aliases must
        report their own invoked name while metadata hashes the resolved target;
        target identity and invocation semantics are separate observations.
        """
        real=self.root/'bin'/'real-tool';real.parent.mkdir()
        real.write_text('#!'+sys.executable+'\nfrom pathlib import Path\nimport sys\nprint("named-test PASS "+Path(sys.argv[0]).name)\n');real.chmod(0o700)
        alias=real.with_name('alias-tool');alias.symlink_to(real.name)
        (self.root/'bin-link').symlink_to('bin',target_is_directory=True)
        self.value['inputs']+=['bin/real-tool','bin/alias-tool','bin-link/alias-tool']
        self.value['steps'][0]['command']=[str(real)]
        self.value['steps'][0]['required_stdout']=['named-test PASS real-tool'];self.write()
        self.assertEqual(self.run_fixture('real-control')[0]['status'],'PASS')
        for name,invocation,search,parent in [('absolute',str(alias),'{root}/bin','bin'),('relative','./bin/alias-tool','{root}/bin','bin'),('path','alias-tool','{root}/bin','bin'),('relative-path-link','alias-tool','bin-link','bin-link')]:
            with self.subTest(invocation=name):
                self.value['steps'][0]['command']=[invocation]
                self.value['steps'][0]['required_stdout']=['named-test PASS alias-tool']
                self.value['environment']={'PATH':search};self.write()
                result,output=self.run_fixture(name)
                self.assertEqual(result['status'],'PASS',(output/'step-0001.stdout').read_text())
                self.assertEqual(Path(result['steps'][0]['command'][0]).name,'alias-tool')
                self.assertEqual(Path(result['steps'][0]['command'][0]).parent.name,parent)
                self.assertEqual(result['steps'][0]['executable'],str(real))
                self.assertEqual(result['steps'][0]['launcher_sha256'],recipe.digest(real.read_bytes()))
                self.assertEqual(recipe.verify_evidence(output)['recorded_status'],'PASS')

if __name__ == '__main__':
    unittest.main(verbosity=2)
