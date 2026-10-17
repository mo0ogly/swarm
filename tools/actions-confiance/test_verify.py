import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SOURCE=Path(__file__).with_name('verify.py').resolve()

class VerifierContract(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        (self.root/'tools/actions-confiance').mkdir(parents=True)
        (self.root/'tools/actions-confiance/verify.py').write_bytes(SOURCE.read_bytes())
        (self.root/'tests').mkdir()

    def invoke(self,result,task='T2',mode='c1'):
        recipe=self.root/f'tests/actions_confiance_t{task[1:]}.py'
        recipe.write_text('print('+repr(json.dumps(result))+')\n')
        return subprocess.run([sys.executable,str(SOURCE),task,mode,'--timeout-seconds','5'],cwd=self.root,capture_output=True,text=True)

    def result(self,task='T2',criterion=1):
        return {'version':1,'task':task,'criterion':criterion,'cases':[{'id':'refusal','status':'pass','assertions':2,'observed':{'before':0,'after':0}}]}

    def test_content_and_rejection(self):
        valid=self.result()
        p=self.invoke(valid)
        self.assertEqual(p.returncode,0,p.stderr)
        self.assertIn('"cases": 1',p.stdout)
        for field,value in [('task','T3'),('criterion',0),('cases',[])]:
            with self.subTest(field=field):
                bad=dict(valid);bad[field]=value
                self.assertNotEqual(self.invoke(bad).returncode,0)
        for field,value in [('assertions',0),('assertions',True),('assertions',-1),('observed',{}),('observed','pass'),('status','fail')]:
            with self.subTest(field=field,value=value):
                bad=self.result();bad['cases'][0][field]=value
                self.assertNotEqual(self.invoke(bad).returncode,0)
        bad=self.result();bad['cases']*=2
        self.assertNotEqual(self.invoke(bad).returncode,0)

    def test_operator_copy_and_containment(self):
        local=self.root/'tools/actions-confiance/verify.py'
        local.write_text('modified')
        p=self.invoke(self.result())
        self.assertNotEqual(p.returncode,0)
        self.assertIn('differs',p.stderr)
        local.write_bytes(SOURCE.read_bytes())
        target=self.root/'tests/actions_confiance_t2.py'
        outside=self.root/'outside.py';outside.write_text('print("{}")')
        target.unlink();target.symlink_to(outside)
        p=subprocess.run([sys.executable,str(SOURCE),'T2','c1','--timeout-seconds','5'],cwd=self.root,capture_output=True,text=True)
        self.assertNotEqual(p.returncode,0)
        self.assertIn('Regular candidate evidence',p.stderr)

    def test_ebpf_absence_is_environment_failure(self):
        result=self.result('T7',4);result['kernel_observation']={'available':False}
        p=self.invoke(result,'T7','c4')
        self.assertEqual(p.returncode,3,p.stderr)
        result['kernel_observation']={'available':True,'source':'double','events':[{}],'owned_pids':[1]}
        self.assertNotEqual(self.invoke(result,'T7','c4').returncode,0)

    def test_missing_recipe_fails(self):
        p=subprocess.run([sys.executable,str(SOURCE),'T8','delivery','--timeout-seconds','5'],cwd=self.root,capture_output=True,text=True)
        self.assertNotEqual(p.returncode,0)
        self.assertIn('Regular candidate evidence',p.stderr)

if __name__=='__main__':
    unittest.main(verbosity=2)
