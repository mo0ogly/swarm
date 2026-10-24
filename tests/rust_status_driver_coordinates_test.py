#!/usr/bin/env python3
"""Reproduce relative evidence paths breaking CLI input after child cwd change."""
from pathlib import Path
import json,os,sys,tempfile,unittest
import rust_mission_status_contract as shared


class DriverCoordinates(unittest.TestCase):
    """Owned inert Python child only; no engine, SQLite, provider or timeout."""
    def test_relative_evidence_survives_child_root_cwd(self):
        """A real child reads the retained JSON request despite a different cwd."""
        fixture=Path(tempfile.mkdtemp(prefix='swarm-driver-path-')).resolve()
        print('OWNED_DRIVER_FIXTURE '+str(fixture),flush=True)
        root=fixture/'root';root.mkdir();out=fixture/'evidence';out.mkdir()
        previous=Path.cwd()
        try:
            os.chdir(fixture)
            driver=shared.Driver(Path(sys.executable),None,Path('evidence'),root,'fixture',os.environ.copy())
            program="import sys,json;from pathlib import Path;p=Path(sys.argv[sys.argv.index('--input')+1]);print(json.dumps(json.loads(p.read_text())))"
            code,raw,error=driver.cli('request',['-c',program],dict(message='owned request'))
            self.assertEqual(code,0,error.decode())
            self.assertEqual(json.loads(raw),dict(message='owned request'))
            command=json.loads((out/'001-request.command.json').read_text())['argv']
            self.assertTrue(Path(command[command.index('--input')+1]).is_absolute())
            self.assertEqual(list(root.iterdir()),[])
        finally:os.chdir(previous)

if __name__=='__main__':unittest.main(verbosity=2)
