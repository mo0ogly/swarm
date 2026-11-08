"""Exercise both real installer recipe entry points without installer/daemon execution."""
import importlib.util,pathlib,sys,tempfile,unittest
from unittest.mock import patch
class StopBeforeExternal(RuntimeError):pass
class OutputContract(unittest.TestCase):
 def check_recipe(self,name,missing,existing):
  with tempfile.TemporaryDirectory() as temp:
   output=pathlib.Path(temp)/('missing/parent/result' if missing else 'result')
   if existing:output.mkdir();(output/'keep').write_bytes(b'preserve')
   spec=importlib.util.spec_from_file_location(name,pathlib.Path(__file__).parent/name);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
   with patch.object(sys,'argv',[name,'--output',str(output)]),patch.object(module.subprocess,'run',side_effect=StopBeforeExternal) as run:
    if existing:
     with self.assertRaises(FileExistsError):module.main()
     run.assert_not_called();self.assertEqual(list(output.iterdir()),[output/'keep']);self.assertEqual((output/'keep').read_bytes(),b'preserve')
    else:
     with self.assertRaises(StopBeforeExternal):module.main()
     self.assertTrue(output.is_dir());run.assert_called_once()
for name in ['rust_native_install.py','rust_docker_install.py']:
 for label,missing,existing in [('existing_parent',False,False),('missing_parents',True,False),('refuse_existing',False,True)]:
  setattr(OutputContract,'test_'+name.replace('.py','')+'_'+label,lambda self,n=name,m=missing,e=existing:self.check_recipe(n,m,e))
if __name__=='__main__':unittest.main(verbosity=2)
