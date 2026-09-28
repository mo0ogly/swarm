import json
import os
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import patch, Mock
import controlled_autonomy_campaign as campaign


class LifecycleTests(unittest.TestCase):
    def test_interruption_stops_agents_and_server_and_cannot_restart(self):
        for interruption in ('signal', 'sigint', 'keyboard', 'observation'):
            with self.subTest(interruption=interruption), tempfile.TemporaryDirectory() as directory:
                root=Path(directory)
                (root/'campaign.json').write_text(json.dumps({'engine':'unused','store':'unused','work':'w','profile':{}}))
                commands=[]
                lists=0
                def cli(m,args,data=None):
                    nonlocal lists
                    commands.append(args)
                    if args[:2]==['agent','list']:
                        lists+=1
                        return {'agents':[{'agent':{'id':'a','status':'running' if lists==1 else 'interrupted'}}]}
                    if args[:2]==['work','show']: return {'work':{'tasks':[]}}
                def observe(m):
                    if interruption in ('signal','sigint'): os.kill(os.getpid(), signal.SIGTERM if interruption=='signal' else signal.SIGINT)
                    if interruption=='keyboard': raise KeyboardInterrupt()
                    raise ValueError('broken evidence')
                server=Mock();server.poll.return_value=None
                old=signal.getsignal(signal.SIGTERM)
                with patch.object(campaign,'frozen'), patch.object(campaign,'cli',side_effect=cli), patch.object(campaign,'observe',side_effect=observe), patch.object(campaign.subprocess,'Popen',return_value=server):
                    result=campaign.run(root,seconds=1)
                    self.assertEqual(result['status'],'FAIL')
                    self.assertEqual(result['interrupted'],interruption!='observation')
                    self.assertIn(['mission','stop','w'],commands)
                    self.assertIn(['agent','stop','a'],commands)
                    server.terminate.assert_called_once()
                    server.wait.assert_called_once()
                    self.assertEqual(json.loads((root/'result.json').read_text()),result)
                    before=len(commands)
                    with self.assertRaises(FileExistsError):campaign.run(root,seconds=1)
                    self.assertEqual(len(commands),before)
                self.assertEqual(signal.getsignal(signal.SIGTERM),old)

    def test_unconfirmed_cleanup_overrides_success_and_preserves_server(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'campaign.json').write_text(json.dumps({'engine':'unused','store':'unused','work':'w','profile':{}}))
            def cli(m,args,data=None):
                if args[:2]==['mission','stop']: raise RuntimeError('stop unavailable')
            server=Mock();server.poll.return_value=None
            with patch.object(campaign,'frozen'), patch.object(campaign,'cli',side_effect=cli), patch.object(campaign,'observe',return_value=({'status':'PASS'},{})), patch.object(campaign.subprocess,'Popen',return_value=server):
                result=campaign.run(root,seconds=1)
            self.assertEqual(result['status'],'FAIL')
            self.assertIn('cleanup not confirmed',result['missing'][0])
            server.terminate.assert_not_called()
            self.assertEqual(json.loads((root/'result.json').read_text()),result)

    def test_failed_atomic_publication_preserves_prior_result(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'result.json'
            campaign.write(path,{'status':'RUNNING'})
            with patch.object(campaign.os,'replace',side_effect=OSError('disk failure')):
                with self.assertRaises(OSError): campaign.write(path,{'status':'PASS'})
            self.assertEqual(json.loads(path.read_text()),{'status':'RUNNING'})
            self.assertEqual(list(Path(directory).iterdir()),[path])

if __name__=='__main__':unittest.main()
