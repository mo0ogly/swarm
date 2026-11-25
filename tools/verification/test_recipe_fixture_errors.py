"""One targeted CLI regression retaining helper close/observer secondary causes.

An explicit private evidence root may be supplied by the harness. Otherwise use
the existing configured fixture allocator for a fresh root. No compiler/provider/
service; all result directories remain available for review.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import unittest

_DRIVER = '"""Invoke the actual runner CLI after an explicit fixture-only error seam.\n\nImport-time resolution runs normally; only the subsequent explicit saved read\nhas injected read/close/observer diagnostics. No product hook is added.\n"""\nimport sys,os,importlib.util\nfrom pathlib import Path\nfrom unittest.mock import patch\nrunner=Path(sys.argv[1]);fault=sys.argv[2];args=sys.argv[3:]\nspec=importlib.util.spec_from_file_location(\'recipe_cli\',runner)\nmodule=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)\noriginal_spec=importlib.util.spec_from_file_location\nreal_close=os.close\nclass Loader:\n    """Load the real helper, then wrap only its explicit resolution call."""\n    def __init__(self,delegate):\n        self.delegate=delegate\n    def create_module(self,spec):\n        return self.delegate.create_module(spec)\n    def exec_module(self,area):\n        """Run normal import before installing the owned runtime fault seam."""\n        self.delegate.exec_module(area)\n        original=area.configured_area\n        def resolve(*args,**kwargs):\n            """Forward a healthy call or retain injected primary/secondary errors."""\n            if fault==\'none\':\n                return original(*args,**kwargs)\n            observe=kwargs.get(\'observer\')\n            if fault==\'both\':\n                def observer(event):\n                    """Deliver the event before adding a synthetic observation failure."""\n                    if observe is not None:observe(event)\n                    raise RuntimeError(\'private-observer-message-must-not-leak\')\n                kwargs[\'observer\']=observer\n            def close_after_release(fd):\n                """Really release the FD, then raise a synthetic close diagnostic."""\n                real_close(fd)\n                raise OSError(5,\'private-close-message-must-not-leak\')\n            with patch.object(os,\'read\',side_effect=PermissionError(13,\'synthetic saved read refusal\')):\n                with patch.object(os,\'close\',close_after_release):\n                    return original(*args,**kwargs)\n        area.configured_area=resolve\n\ndef locate(name,path,*args,**kwargs):\n    """Wrap only the helper loader; leave other import specifications unchanged."""\n    value=original_spec(name,path,*args,**kwargs)\n    if name==\'fixture_area\':value.loader=Loader(value.loader)\n    return value\nsys.argv=[str(runner)]+args\nwith patch.object(importlib.util,\'spec_from_file_location\',locate):\n    raise SystemExit(module.main())\n'

class RunnerFixtureSecondaryErrors(unittest.TestCase):
    def test_runner_retains_resolution_close_causes_cli(self):
        """Control → injected read/close refusal → fresh recovery, twice.

        Both variants use the real CLI parser/output/result writer and helper.
        Faults are synthetic after actual FD release; combined variant also
        refuses observation. Messages of secondary causes must stay private.
        """
        declared=os.environ.get('SWARM_CD01_RUNNER_EVIDENCE')
        if declared is None:
            # Reusable recipe path: existing allocator, never an ad-hoc /tmp root.
            import importlib.util
            helper=Path(__file__).resolve().parents[2]/'tests/fixture_area.py'
            spec=importlib.util.spec_from_file_location('fixture_secondary_allocator',helper)
            area=importlib.util.module_from_spec(spec)
            spec.loader.exec_module(area)
            root=area.fixture_root('runner-secondary-')/'cases'
        else:
            root=Path(declared)
        root.mkdir(mode=0o700)
        (root/'tests').mkdir()
        source_root=Path(__file__).resolve().parents[2]
        shutil.copyfile(source_root/'tests/fixture_area.py',root/'tests/fixture_area.py')
        (root/'.swarm').mkdir()
        (root/'.swarm/engine-settings.json').write_text(json.dumps({'values':{
            'runtime_temporary_directory':str(root/'area')}}),encoding='utf-8')
        (root/'source.txt').write_text('declared owned source',encoding='utf-8')
        (root/'driver.py').write_text(_DRIVER,encoding='utf-8')
        marker=root/'action-count.txt'
        recipe=root/'recipe.json'
        code="from pathlib import Path; p=Path('action-count.txt'); p.write_text((p.read_text() if p.exists() else '')+'x'); print('owned-step PASS')"
        recipe.write_text(json.dumps({'schema_version':1,'id':'fixture-secondary-causes',
            'inputs':['source.txt','tests/fixture_area.py','driver.py'], 'environment':{},
            'steps':[{'id':'owned-action','command':[sys.executable,'-I','-B','-c',code],
                'expected_exit':0,'required_stdout':['owned-step PASS'],'required_stderr':[]}]}),encoding='utf-8')
        runner=Path(__file__).with_name('recipe.py')
        def invoke(label,fault):
            """Retain exact argv, exit and raw public CLI streams, no timeout."""
            output=root/label
            argv=[sys.executable,'-I','-B',str(root/'driver.py'),str(runner),fault,
                  'run','--root',str(root),'--recipe',str(recipe),'--output',str(output)]
            result=subprocess.run(argv,cwd=root,capture_output=True)
            (root/(label+'.stdout')).write_bytes(result.stdout)
            (root/(label+'.stderr')).write_bytes(result.stderr)
            (root/(label+'.command.json')).write_text(json.dumps({'argv':argv,'exit':result.returncode,'watchdog':None}),encoding='utf-8')
            self.assertEqual(result.stderr,b'')
            public=json.loads(result.stdout.splitlines()[-1])
            self.assertEqual(Path(public['evidence']),output)
            retained=json.loads((output/'result.json').read_bytes())
            return result.returncode,public,retained,output
        for mode in ('close','both'):
            control=invoke(mode+'-control','none')
            self.assertEqual(control[0],0)
            self.assertEqual(control[2]['status'],'PASS')
            self.assertEqual(control[2]['evidence_failures'],[])
            self.assertTrue(control[2]['source_stable'])
            before=marker.read_bytes()
            refused=invoke(mode+'-refused',mode)
            after=marker.read_bytes()
            recovery=invoke(mode+'-recovered','none')
            self.assertEqual(recovery[0],0)
            self.assertEqual(recovery[2]['status'],'PASS')
            self.assertEqual(recovery[2]['evidence_failures'],[])
            self.assertEqual(marker.read_bytes(),before+b'x')
            print(json.dumps({'case':mode,'control':control[2]['status'],
                'refusal':refused[2]['status'],'recovery':recovery[2]['status'],
                'effect_unchanged_on_refusal':before==after,
                'evidence':str(refused[3])}),flush=True)
            with self.subTest(mode=mode):
                self.assertEqual(refused[0],1)
                self.assertEqual(refused[1]['status'],'FAIL')
                self.assertEqual(refused[1]['steps'],0)
                self.assertEqual(refused[2]['failure']['type'],'PermissionError')
                self.assertEqual(refused[2]['failure']['errno'],13)
                self.assertIn('synthetic saved read refusal',refused[2]['failure']['cause'])
                self.assertEqual(refused[2]['steps'],[])
                self.assertEqual(after,before)
                self.assertTrue(refused[2]['source_stable'])
                self.assertFalse((refused[3]/'step-0001.stdout').exists())
                errors=refused[2]['evidence_failures']
                closes=[e for e in errors if e['phase']=='fixture-resolution-close']
                observers=[e for e in errors if e['phase']=='fixture-resolution-observation']
                self.assertEqual(closes,[{'type':'OSError','errno':5,'phase':'fixture-resolution-close'}]*3)
                self.assertEqual(len(observers),2 if mode=='both' else 0)
                self.assertTrue(all(e['type']=='RuntimeError' and e['errno'] is None for e in observers))
                raw=(refused[3]/'result.json').read_bytes()
                self.assertNotIn(b'private-close-message',raw)
                self.assertNotIn(b'private-observer-message',raw)

if __name__=='__main__':
    unittest.main(verbosity=2)
