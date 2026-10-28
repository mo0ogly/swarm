"""Measure retained public recipe receipts; no runtime mutation or model calls."""
import argparse,datetime,json,subprocess
from pathlib import Path

def date(value):return datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
def duration(start,end):
 if not start or not end:return None
 seconds=(date(end)-date(start)).total_seconds()
 if seconds<0:raise ValueError('Negative recorded interval')
 return seconds

def measure(root):
 root=Path(root);result=json.loads((root/'result.json').read_text());assert result['status']=='PASS'
 frozen=json.loads((root/'frozen.json').read_text());public=json.loads((root/'final-public-work.json').read_text());spending=json.loads((root/'spending.json').read_text());agents=[x['agent'] for x in json.loads((root/'agents.json').read_text())]
 assert public['validation']['validated']==frozen['parameters']['tasks'] and public['validation']['stale']==0
 events=public['events'];reviews={e['payload']['id']:e['payload'] for e in events if e['kind']=='review.managed.result'}
 controls=[];integrations=[]
 for e in events:
  if e['kind']=='managed.integrated':
   p=e['payload'];entry={'agent':p['agent_id'],'at':e['at'],'candidate':p['candidate_commit']};integrations.append(entry)
   for task,checks in p['controls'].items():
    for c in checks:
     if c.get('executed'):controls.append(dict(producer=p['agent_id'],task=task,id=c['id'],started=c.get('started_at'),finished=c.get('finished_at'),seconds=duration(c.get('started_at'),c.get('finished_at'))))
 stages=[]
 for a in agents:
  matching=[r for r in reviews.values() if r.get('producer')==a['id']];matching.sort(key=lambda r:r['started']);r=matching[-1] if matching else {}
  done=next((i for i in integrations if i['agent']==a['id']),{})
  preceding=[c for c in controls if c['producer']==a['id'] and r and c['finished'] and date(c['finished'])<=date(r['started']) and date(c['started'])>=date(a['ended'])]
  first=min((c['started'] for c in preceding),default=None)
  stages.append(dict(task=a['task_id'],agent=a['id'],attempt=a['attempt_id'],process_seconds=duration(a['started'],a.get('ended')),post_process_before_control_seconds=duration(a.get('ended'),first),review_seconds=duration(r.get('started'),r.get('finished')),post_review_to_publication_seconds=duration(r.get('finished'),done.get('at')),tools=a['progress'].get('tool_calls'),review_id=r.get('id')))
 starts=[e for e in events if e['kind']=='mission.start'];judges=[e for e in events if e['kind']=='planning.decide' and any(o.get('kind')=='judge' and o.get('judge_verdict')=='end' for o in e['payload'].get('operations',[]))]
 assert len(starts)==1 and judges
 windows=[];previous={}
 for line in (root/'cli.jsonl').read_text().splitlines():
  row=json.loads(line)
  if row['exit']==0 and row['command'][-3:]==['agent','list',result['work']]:
   current={x['agent']['id']:x['agent'] for x in json.loads(row['stdout'])['agents']}
   advancing=[key for key,a in current.items() if key in previous and a['progress'].get('tool_calls',0)>previous[key]['progress'].get('tool_calls',0) and previous[key]['status'] in ('running','starting')]
   if len(advancing)>1:windows.append(dict(at=row['finished'],agents=advancing))
   previous=current
 tree=subprocess.check_output(['git','rev-parse',frozen['baseline']+'^{tree}'],cwd=frozen['project'],text=True).strip()
 return dict(work=result['work'],slots=frozen['parameters']['slots'],tasks=len(agents),baseline_tree=tree,binary_sha256=frozen['binary_sha256'],provider_config_sha256=frozen['provider_config_sha256'],model_policy=frozen['model_policy'],mission_start_to_judge_seconds=duration(starts[0]['at'],judges[-1]['at']),recipe_elapsed_seconds=result['elapsed_seconds'],observed_peak_workers=result['max_simultaneous_workers'],stages=stages,control_executions=len(controls),control_seconds=sum(c['seconds'] for c in controls if c['seconds'] is not None),review_executions=len(reviews),review_seconds=sum(duration(r.get('started'),r.get('finished')) for r in reviews.values()),multi_worker_tool_progress_windows=windows,window_poll_seconds=frozen['parameters']['poll_seconds'],rows=spending['rows'],calls=spending.get('calls'),operator_interventions_after_start=result['operator_interventions_after_start'],limits='Process intervals are not model inference time. Pre-control interval includes queue/preparation; publication tail includes validation. Reservation count includes manual setup claim. Missing billed cost remains unknown. Polling pause is nominal; intervals include CLI duration and common progress windows do not prove simultaneous inference. One observation per level, not p95 or endurance.')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('serial');p.add_argument('parallel');p.add_argument('output');a=p.parse_args();one=measure(a.serial);four=measure(a.parallel)
 for key in ['baseline_tree','binary_sha256','provider_config_sha256','model_policy']:assert one[key]==four[key],('Comparison inputs differ',key)
 out=dict(serial=one,parallel=four,wall_ratio_serial_over_parallel=one['mission_start_to_judge_seconds']/four['mission_start_to_judge_seconds'])
 Path(a.output).write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({k:{j:v[j] for j in ['work','slots','tasks','mission_start_to_judge_seconds','observed_peak_workers','control_executions','review_executions']} for k,v in [('serial',one),('parallel',four)]}))
