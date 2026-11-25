"""Regress Python text-default decoding in the private fixture settings reader.

One named test launches isolated Python children under C locale with UTF-8 mode
on/off. All settings/source copies belong to this fixture; no live fallback.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import unittest
import fixture_area

_ENCODING_CASE = '"""Exercise the effective text-reader contract with private saved JSON only."""\nimport sys,json,locale,io,importlib.util\nfrom pathlib import Path\nroot=Path(sys.argv[1])\nspec=importlib.util.spec_from_file_location(\'fixture_area\',root/\'tests/fixture_area.py\')\nmodule=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)\nsaved=root/\'.swarm/engine-settings.json\'\nreports=[]\nfor name,value,escaped in [(\'ascii-control\',\'control\',True),(\'raw-accent\',\'\\u00e9vidence\',False),(\'escaped-equivalent\',\'\\u00e9vidence\',True),(\'ascii-recovery\',\'recovered\',True)]:\n    raw=json.dumps({\'values\':{\'runtime_temporary_directory\':value}},ensure_ascii=escaped).encode(\'utf-8\')\n    saved.write_bytes(raw)\n    report={\'case\':name,\'input_hex\':raw.hex()}\n    try:\n        old_value=json.loads(saved.read_text())[\'values\'][\'runtime_temporary_directory\']\n        report[\'old_reader\']={\'value\':old_value}\n    except UnicodeDecodeError as error:\n        report[\'old_reader\']={\'error\':type(error).__name__}\n    try:\n        events=[];actual=module.configured_area(root,observer=events.append)\n        report[\'candidate\']={\'value\':str(actual.relative_to(root))}\n        report[\'last_event\']=events[-1][\'kind\']\n    except UnicodeDecodeError as error:\n        report[\'candidate\']={\'error\':type(error).__name__}\n    reports.append(report)\nprint(json.dumps({\'utf8_mode\':sys.flags.utf8_mode,\'locale_encoding\':locale.getencoding(),\'text_encoding\':io.text_encoding(None),\'cases\':reports}))\n'

class Utf8ModeReader(unittest.TestCase):
    def setUp(self):
        """Create owned child sources and an explicit saved bootstrap value."""
        self.repo = fixture_area.fixture_root('fixture-area-encoding-')
        (self.repo/'tests').mkdir()
        (self.repo/'.swarm').mkdir()
        (self.repo/'tests/fixture_area.py').write_bytes(Path(fixture_area.__file__).read_bytes())
        defaults=self.repo/'rust/crates/swarm-engine/config'
        defaults.mkdir(parents=True)
        (defaults/'engine-settings.json').write_text(
            json.dumps({'runtime_temporary_directory':'.swarm-fixtures'}),encoding='utf-8')

    def tearDown(self):
        """Remove only this owned fixture, preserving retained launcher output."""
        shutil.rmtree(self.repo)

    def test_utf8_mode_text_reader_contract(self):
        """Compare old text reading with resolution across encoding modes.

        ASCII control precedes raw/escaped equivalent Unicode and ASCII recovery.
        Mode0 additionally preserves locale refusal; no encoding policy is added.
        Child stderr/exit are checked before treating its JSON as valid evidence.
        """
        reports=[]
        for mode in (1,0):
            saved=self.repo/'.swarm/engine-settings.json'
            saved.write_text(json.dumps({'values':{'runtime_temporary_directory':str(self.repo/'area')}}),encoding='utf-8')
            argv=[sys.executable,'-X','utf8='+str(mode),'-I','-B','-c',_ENCODING_CASE,str(self.repo)]
            result=subprocess.run(argv,cwd=self.repo,env=dict(os.environ,LC_ALL='C'),
                                  capture_output=True,text=True,encoding='utf-8')
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(result.stderr,'')
            report=json.loads(result.stdout)
            self.assertEqual(report['utf8_mode'],mode)
            reports.append(report)
            print(json.dumps(report),flush=True)
        for report in reports:
            for case in report['cases']:
                with self.subTest(mode=report['utf8_mode'],case=case['case']):
                    self.assertEqual(case['candidate'],case['old_reader'])
            if report['utf8_mode']==1:
                self.assertEqual(report['cases'][0]['old_reader'],{'value':'control'})
                self.assertEqual(report['cases'][1]['old_reader'],{'value':'\u00e9vidence'})
                self.assertEqual(report['cases'][2]['old_reader'],{'value':'\u00e9vidence'})
                self.assertEqual(report['cases'][3]['candidate'],{'value':'recovered'})

if __name__=='__main__':
    unittest.main(verbosity=2)
