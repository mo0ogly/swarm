import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from controlled_autonomy_campaign import assess, cli, prepare, run, LIMITS
from controlled_business_fault import CHECK


class OracleTests(unittest.TestCase):
    def evidence(self):
        command=['python3','-B','-c',CHECK,'feature.py']
        m={'work':'w-1','control_command':command}
        task={'id':'t','status':'accepted','attempts':[{'id':'a1'},{'id':'a2'}],
              'independent_review':{'state':'passed','candidate_commit':'sha','attempt':'a2','producer':'p2','reviewer':'reviewer://trial-planner'},
              'automatic_validation':{'state':'accepted','candidate_sha':'sha','attempt_id':'a2','producer_agent_id':'p2','controls':[{'command':command,'executed':True,'passed':True,'exit_code':0}]}}
        w={'id':'w-1','tasks':[task],'planning':{'max_activations':12,'max_tasks':1,'activations':5,
            'reviewer':{'max_calls':4,'calls':1},'repository':{'candidate_commit':'sha'},
            'scopes':[{'state':'closed'},{'state':'closed'}]}}
        snap={'work':w,'validation':{'tasks':{'t':{'fresh':True}}},'events':[]}
        history=[{'id':'planning-claim-1-decision','operations':[{'kind':k} for k in ['delegate','task','retry','close']]}]
        agents=[{'id':i,'attempt_id':a,'role':'worker','status':'completed'} for i,a in [('p1','a1'),('p2','a2')]]
        injection={'state':'injected','work':'w-1','agent':'p1','attempt':'a1','precheck_exit':0,'postcheck_exit':1}
        failures=[{'producer':'p1','attempt':'a1','control':{'command':command},'result':{'executed':True,'passed':False,'exit_code':1}}]
        return [m,snap,history,agents,injection,failures]

    def test_complete_fixture_is_eligible_for_artifact_verification(self):
        self.assertEqual(assess(*self.evidence())['status'],'PASS')

    def test_exhausted_planner_null_operations_remain_an_incomplete_result(self):
        data=self.evidence()
        data[2]=[{'id':'planning-claim-exhausted-decision','operations':None}]
        data[1]['work']['tasks'][0]['status']='blocked'
        result=assess(*data)
        self.assertEqual(result['status'],'INCOMPLETE')
        self.assertIn('real decision trajectory incomplete',result['missing'])
        self.assertIn('fresh engine acceptance missing',result['missing'])

    def test_false_green_states_are_rejected(self):
        for scenario in ['missing-injection','no-refusal','old-sha','stale','manual-retry','fake-decision','budget-raised','unfinished-child','wrong-attempt','third-producer','missing-identities','missing-controls','wrong-reviewer']:
            with self.subTest(scenario=scenario):
                data=self.evidence();m,s,h,a,i,f=data;t=s['work']['tasks'][0];p=s['work']['planning']
                if scenario=='missing-injection':i.clear()
                if scenario=='no-refusal':f.clear()
                if scenario=='old-sha':t['automatic_validation']['candidate_sha']='old'
                if scenario=='stale':s['validation']['tasks']['t']['fresh']=False
                if scenario=='manual-retry':s['events'].append({'kind':'review.retry'})
                if scenario=='fake-decision':h[0]['id']='operator-decision'
                if scenario=='budget-raised':p['reviewer']['max_calls']=12
                if scenario=='unfinished-child':p['scopes'][1]['state']='open'
                if scenario=='wrong-attempt':t['independent_review']['attempt']='a1'
                if scenario=='third-producer':a.append(copy.deepcopy(a[0]))
                if scenario=='missing-identities':
                    t['independent_review'].pop('attempt');t['automatic_validation'].pop('attempt_id')
                if scenario=='missing-controls':t['automatic_validation'].pop('controls')
                if scenario=='wrong-reviewer':t['independent_review']['reviewer']='worker'
                self.assertNotEqual(assess(*data)['status'],'PASS')

    def test_mutating_planner_commands_not_exposed_by_launcher(self):
        for action in ['claim','decide','retry-review','authorize-recovery']:
            with self.assertRaises(ValueError):cli({},['planning',action,'w'])


@unittest.skipUnless(os.environ.get('SWARM_CAMPAIGN_TEST_ENGINE'),'set frozen test engine path')
class PublicCampaignTests(unittest.TestCase):
    def test_public_campaign_with_deterministic_provider(self):
        # Runs actual Swarm processes with a deterministic provider, never a model.
        with tempfile.TemporaryDirectory(prefix='swarm-campaign-test-') as tmp:
            base=Path(tmp);provider=base/'claude';review=base/'review.py'
            source=Path(__file__).resolve().parent.parent/'managed_review_test.go'
            fixture=source.read_text().split('const managedReviewerFixture = `',1)[1].split('\n`',1)[0]
            review.write_text(fixture)
            provider.write_text('#!'+sys.executable+'\n'+r'''
import json, pathlib, re, subprocess, sys
text=sys.stdin.read()
if 'SWARM_MANAGED_REVIEW' in text:
 p=subprocess.run([sys.executable,str(pathlib.Path(__file__).with_name('review.py'))],input=text,text=True)
 raise SystemExit(p.returncode)
if '--json-schema' in sys.argv:
 ctx=None
 for line in text.splitlines():
  try:
   value=json.loads(line)
   if isinstance(value,dict) and 'scope' in value and 'events' in value:ctx=value
  except ValueError:pass
 assert ctx is not None,text[-200:]
 op=None
 if ctx['scope']['id']=='root':
  if not ctx['children']:op={'kind':'delegate','id':'child','title':'Owner','requirements':['req-1'],'next':'Create feature.py and report'}
  elif all(c['state']=='closed' for c in ctx['children']):op={'kind':'close'}
 elif not ctx['tasks']:
  op={'kind':'task','id':'implement','title':'Implement function','requirements':['req-1'],'deliverable':'feature.py et docs/implement.md','criteria':['expected only'],'next':'Write feature.py and docs/implement.md'}
 elif ctx['tasks'][0]['status']=='blocked':
  op={'kind':'retry','id':'implement','next':'Correct rejected feature.py, preserve expected-only behavior, update report'}
 elif ctx['tasks'][0]['status']=='accepted':op={'kind':'close'}
 reply={'input_events':[e['id'] for e in ctx['events']],'reason':'Deterministic fixture response to observed context','operations':[op] if op else []}
 print(json.dumps({'type':'result','result':json.dumps(reply)}))
else:
 pathlib.Path('feature.py').write_text('def is_expected(value):\n    return value == "expected"\n')
 pathlib.Path('docs').mkdir(exist_ok=True)
 delivery_text=text.split('BILAN DE LIVRAISON REQUIS',1)[1]
 delivery=json.JSONDecoder().raw_decode(delivery_text[delivery_text.index('\n{')+1:])[0]
 delivery['outcome']='complete'
 for criterion in delivery['criteria']:
  criterion.update(status='pass',reason='Fixture executed business implementation before injection',evidence=['feature.py'])
 pathlib.Path('docs/implement.delivery.json').write_text(json.dumps(delivery))
 pathlib.Path('docs/implement.md').write_text('Fixture implementation for '+delivery['attempt'])
 print(json.dumps({'type':'result','result':'Fixture worker completed'}))
''')
            provider.chmod(0o700)
            config=base/'providers.json';config.write_text(json.dumps({'providers':{'claude':{'command':str(provider),'args':['-p','--output-format','stream-json','--verbose']}}}))
            dest=base/'campaign'
            m=prepare(dest,os.environ['SWARM_CAMPAIGN_TEST_ENGINE'],config,'claude')
            before=cli(m,['work','show',m['work']])['work']
            self.assertEqual(before['planning']['activations'],0)
            self.assertEqual(before['planning']['reviewer']['calls'],0)
            self.assertEqual(cli(m,['agent','list',m['work']])['agents'],[])
            result=run(dest,seconds=150)
            # Preserve failed fixture artifacts for diagnosis outside TemporaryDirectory.
            evidence=os.environ.get('SWARM_CAMPAIGN_EVIDENCE')
            if evidence:
                import shutil
                target=Path(evidence)/base.name
                target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copytree(dest,target)
                (target.parent/'latest.txt').write_text(str(target))
            self.assertEqual(result['status'],'PASS',result)
            with self.assertRaises(FileExistsError):run(dest,seconds=1)
            journal=[json.loads(line) for line in (dest/'cli.jsonl').read_text().splitlines()]
            self.assertFalse(any(x['args'][:2] in [['planning','claim'],['planning','decide'],['planning','retry-review']] for x in journal))


if __name__=='__main__':unittest.main()
