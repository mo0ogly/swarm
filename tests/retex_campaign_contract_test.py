"""Reject misleading execution modes before filesystem or provider access."""
import subprocess,tempfile,unittest
from pathlib import Path

class CampaignModeContract(unittest.TestCase):
 def reject(self,flags,reason):
  with tempfile.TemporaryDirectory() as directory:
   out=Path(directory)/'never-created'
   command=['python3',str(Path(__file__).with_name('retex_real_campaign.py')),'--binary','missing-binary','--output',str(out),'--providers','missing-providers','--provider','codex','--scenario','parallel','--tasks','4','--slots','1','--activations','16','--decisions','16','--reviews','8','--observation-seconds','900','--cli-seconds','45','--control-seconds','15','--poll-seconds','2','--control-program','python3',*flags]
   result=subprocess.run(command,capture_output=True,text=True)
   self.assertEqual(result.returncode,2,result.stderr);self.assertIn(reason,result.stderr)
   self.assertNotIn('SETUP_ONLY',result.stdout);self.assertFalse(out.exists(),'invalid mode must not create or resume runtime state')
 def test_resume_cannot_be_claimed_as_free_setup(self):self.reject(['--setup-only','--resume-mission'],'not allowed')
 def test_prepared_execution_cannot_be_claimed_as_free_setup(self):self.reject(['--setup-only','--run-prepared'],'not allowed')
 def test_execution_resume_and_prepared_start_are_exclusive(self):self.reject(['--resume-mission','--run-prepared'],'not allowed')
 def test_partial_setup_requires_setup_only(self):self.reject(['--resume-setup'],'requires --setup-only')

if __name__=='__main__':unittest.main(verbosity=2)
