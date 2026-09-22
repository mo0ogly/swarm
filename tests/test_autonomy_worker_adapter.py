import json
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from controlled_business_fault import GOOD, check_feature

ADAPTER = Path(__file__).with_name('autonomy_worker_adapter.py').resolve()


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.copy = self.base / 'copies' / 'first'
        self.copy.mkdir(parents=True)
        self.evidence = self.base / 'evidence'
        self.engine = self.base / 'fake-engine'
        self.producer = self.base / 'fake-producer'
        self.agent = dict(id='agent-1',work_id='w-1',attempt_id='a-1',role='worker',
                          status='running',workspace=str(self.copy))
        self.config = self.base / 'adapter.json'
        self.config.write_text(json.dumps(dict(engine=str(self.engine), store=str(self.base),
            work='w-1',copies_root=str(self.copy.parent),evidence_dir=str(self.evidence),
            producer=str(self.producer))))
        self.set_agents([self.agent])
        self.producer.write_text('#!'+sys.executable+'\nfrom pathlib import Path\nimport sys\n'
            'assert sys.argv[1:]==["--fixture"]\nassert sys.stdin.read()=="worker prompt"\n'
            'Path("feature.py").write_text('+repr(GOOD)+')\nprint("producer event",flush=True)\n')
        self.producer.chmod(0o700)

    def set_agents(self, agents):
        self.engine.write_text('#!'+sys.executable+'\nimport sys\n'
            'assert sys.argv[-3:]==["agent","list","w-1"]\nprint('+repr(json.dumps({'agents':agents}))+')\n')
        self.engine.chmod(0o700)
        config=json.loads(self.config.read_text())
        config['engine_sha256']=hashlib.sha256(self.engine.read_bytes()).hexdigest()
        self.config.write_text(json.dumps(config))

    def run_adapter(self):
        return subprocess.run([sys.executable,str(ADAPTER),str(self.config),'--fixture'],
                              cwd=self.copy,input='worker prompt',text=True,capture_output=True,timeout=10)

    def test_stream_prompt_and_identity_preserved_then_fault_injected_once(self):
        first = self.run_adapter()
        self.assertEqual(first.returncode,0,first.stderr)
        self.assertEqual(first.stdout,'producer event\n')
        self.assertNotEqual(check_feature(self.copy/'feature.py').returncode,0)
        record=json.loads((self.evidence/'injection.json').read_text())
        self.assertEqual(record['agent'],'agent-1')
        self.assertEqual(record['attempt'],'a-1')
        second=self.run_adapter()
        self.assertEqual(second.returncode,0,second.stderr)
        self.assertEqual(check_feature(self.copy/'feature.py').returncode,0)

    def test_ambiguous_owner_or_planner_never_starts_producer(self):
        for agents in [[self.agent,self.agent],[dict(self.agent,role='planner')],[]]:
            self.set_agents(agents)
            p=self.run_adapter()
            self.assertEqual(p.returncode,78)
            self.assertFalse((self.copy/'feature.py').exists())
            self.assertFalse((self.evidence/'injection.json').exists())

    def test_changed_engine_refuses_before_producer(self):
        self.engine.write_text(self.engine.read_text()+'\n# changed\n')
        p=self.run_adapter()
        self.assertEqual(p.returncode,78)
        self.assertIn('engine changed',p.stderr)
        self.assertFalse((self.copy/'feature.py').exists())

    def test_failed_producer_does_not_consume_injection(self):
        self.producer.write_text('#!'+sys.executable+'\nraise SystemExit(3)\n')
        p=self.run_adapter()
        self.assertEqual(p.returncode,3)
        self.assertFalse((self.evidence/'injection.json').exists())


if __name__=='__main__':
    unittest.main()
