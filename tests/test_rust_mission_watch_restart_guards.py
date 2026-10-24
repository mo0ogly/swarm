"""W2 admission comparators: inert local values only, no engine/SDK process."""
import copy
import json
import unittest
import rust_mission_watch_restart as restart


class RestartGuardTests(unittest.TestCase):
    """Valid controls precede changed id, amount, request, attempt and turn cases."""
    def witness(self):
        """Build explicit typed ledger and native provenance fixture values."""
        rows=dict(agents=[[['text','a'],['text','task'],['text','/owned/work'],['blob','request']]],
                  reservations=[[['text','a'],['text','w'],['real',float(1).hex()],['text','reserved']]])
        queued=dict(id='a',attempt_id='attempt',task_id='task',native_mission={'schema_version':4,'generation':1},status='queued')
        running={**queued,'status':'running'}
        costs=dict(attempts_without_cost=1,attempts_with_cost=0,reported_usd=0)
        turn=dict(seq='1',work='w',task='task',agent='a',state='acquired')
        return rows,queued,running,costs,turn

    def test_same_admission_witness_and_changed_rows_refused(self):
        before,queued,running,costs,turn=self.witness()
        restart.verify_same_admission(before,copy.deepcopy(before),queued,running,costs,costs,turn,turn)
        for kind in ('duplicate','amount','request'):
            active=copy.deepcopy(before)
            if kind=='duplicate':active['agents'].append(copy.deepcopy(active['agents'][0]))
            if kind=='amount':active['reservations'][0][2]=['real',float(2).hex()]
            if kind=='request':active['agents'][0][-1]=['blob','changed']
            with self.subTest(kind=kind),self.assertRaises(RuntimeError):
                restart.verify_same_admission(before,active,queued,running,costs,costs,turn,turn)
        restart.verify_same_admission(before,before,queued,running,costs,costs,turn,turn)

    def test_same_agent_attempt_fifo_and_consumption_are_required(self):
        before,queued,running,costs,turn=self.witness()
        restart.verify_same_admission(before,before,queued,running,costs,costs,turn,turn)
        for key,value in [('id','other'),('attempt_id','other'),('native_mission',{'generation':2})]:
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                restart.verify_same_admission(before,before,queued,{**running,key:value},costs,costs,turn,turn)
        with self.assertRaises(RuntimeError):
            restart.verify_same_admission(before,before,queued,running,costs,{**costs,'attempts_without_cost':2},turn,turn)
        with self.assertRaises(RuntimeError):
            restart.verify_same_admission(before,before,queued,running,costs,{**costs,'reported_usd':False},turn,turn)
        with self.assertRaises(RuntimeError):
            restart.verify_same_admission(before,before,queued,running,costs,costs,turn,{**turn,'seq':'2'})

    def test_one_terminal_settlement_without_refund_or_second_admission(self):
        before,*_=self.witness();final=copy.deepcopy(before)
        final['reservations'][0][3]=['text','estimated']
        restart.verify_terminal_history(before,final,'a')
        for kind in ('refund','duplicate','row-missing'):
            invalid=copy.deepcopy(final)
            if kind=='refund':invalid['reservations'][0][2]=['real',float(0).hex()]
            if kind=='duplicate':invalid['agents'].append(copy.deepcopy(invalid['agents'][0]))
            if kind=='row-missing':invalid['reservations']=[]
            with self.subTest(kind=kind),self.assertRaises(RuntimeError):
                restart.verify_terminal_history(before,invalid,'a')
        restart.verify_terminal_history(before,final,'a')

    def test_seed_declares_stopped_owner_not_provider_or_ledger_authority(self):
        value=dict(schema_version=1,status='QUEUED_RESTART_SEED',work='w',agent_id='a',task_id='task',attempt_id='attempt',
                   generation=1,binding={'schema_version':4,'generation':1},before_conductor_stop={'receipt_count':1},
                   after_conductor_stop={'receipt_count':1},worker_projections_unchanged=True,
                   lease_released=True,lease_closed=True,root_closed=True,
                   provider_spawn_called=False,agent_cancelled=False,consumption_reset=False,
                   observation_is_permission=False,reservation_rows_verified=False)
        self.assertEqual(restart.verify_seed(value,'w'),value)
        for key in ('provider_spawn_called','consumption_reset','reservation_rows_verified'):
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                restart.verify_seed({**value,key:True},'w')
        with self.assertRaises(RuntimeError):restart.verify_seed({**value,'lease_closed':False},'w')
        self.assertEqual(restart.verify_seed(value,'w'),value)

    def test_actual_adoption_event_matches_one_append_only_chain_before_spawn(self):
        events=[dict(schema_version=1,event='mission-watch-started',work='w',mission_complete=False,observation_is_permission=False),
                dict(schema_version=1,event='mission-worker-adopted',work='w',agent_id='a',generation=2,event_id='adopt',created=True,provider_start_verified=False,mission_complete=False,observation_is_permission=False),
                dict(schema_version=1,event='mission-worker-started',work='w',agent_id='a',session_state='pending',provider_start_verified=False,mission_complete=False),
                dict(schema_version=1,event='mission-watch-stopped',work='w',reason='sigint',children_still_running=1,lease_released=True,root_closed=True,mission_complete=False)]
        rows=[dict(work='w',agent='a',event_id='adopt',generation=2,previous_generation=1,
                   request=dict(schema_version=1,agent_id='a',event_id='adopt'),
                   receipt=dict(schema_version=1,work='w',agent='a',event_id='adopt',original_generation=1,previous_generation=1,generation=2))]
        def verify(actual_events,actual_rows):
            return restart.verify_restart_events(b'\n'.join(json.dumps(value).encode() for value in actual_events),'w','a',1,2,actual_rows)
        self.assertEqual(len(verify(events,rows)),4)
        for invalid in ([],rows*2,[{**rows[0],'previous_generation':2}]):
            with self.subTest(rows=invalid),self.assertRaises(RuntimeError):verify(events,invalid)
        with self.assertRaises(RuntimeError):verify([events[0],events[2],events[1],events[3]],rows)
        self.assertEqual(len(verify(events,rows)),4)

        # A real prior successful adoption may exist while the original
        # Binding4 still names generation1; the watch appends generation3.
        prefix=copy.deepcopy(rows)
        newer_events=copy.deepcopy(events);newer_events[1]['generation']=3
        newer_events[1]['event_id']='watch-adopt'
        latest=copy.deepcopy(rows[0]);latest.update(event_id='watch-adopt',generation=3,previous_generation=2)
        latest['request']['event_id']='watch-adopt'
        latest['receipt'].update(event_id='watch-adopt',generation=3,previous_generation=2)
        raw=b'\n'.join(json.dumps(value).encode() for value in newer_events)
        self.assertEqual(len(restart.verify_restart_events(raw,'w','a',1,3,[*prefix,latest],prefix)),4)
        rewritten=copy.deepcopy(prefix);rewritten[0]['request']['event_id']='rewritten'
        with self.assertRaises(RuntimeError):
            restart.verify_restart_events(raw,'w','a',1,3,[*rewritten,latest],prefix)

    def test_real_api_checker_requires_all_refusals_and_exact_replay_recovery(self):
        snapshot=dict(agent_sha256='a'*64,work_sha256='b'*64,controls_sha256='c'*64,
                      queue_turn_count=1,receipt_count=1,reservation_rows_verified=False)
        corpus=[('stale-revision','revision_conflict'),('paused-permission','planning-mission-paused'),
                ('positive-adoption',None),('exact-replay',None),('replay-conflict','mission-adoption-replay-conflict'),
                ('recovery-replay',None),('same-epoch-new-event','mission-adoption-new-lease-required'),('stopped-lease','mission-lease-lost')]
        checks=[]
        for name,code in corpus:
            check=dict(name=name,public_projections_unchanged=True,reservation_rows_verified=False,
                       before=copy.deepcopy(snapshot),after=copy.deepcopy(snapshot))
            if code is None:check.update(verdict='ADOPTED',event_id='checker-adoption',generation=2,created=name=='positive-adoption')
            else:check.update(verdict='REFUSED',expected_code=code,cause_codes=[dict(labels=[code])])
            checks.append(check)
        value=dict(schema_version=1,status='ADOPTION_REFUSALS_VERIFIED',work='w',agent_id='a',generation=2,
                   adoption_event_id='checker-adoption',created=True,exact_replay_created=False,recovery_replay_created=False,
                   reservation_rows_verified=False,provider_spawn_called=False,agent_cancelled=False,consumption_reset=False,
                   observation_is_permission=False,lease_released=True,lease_closed=True,root_closed=True,checks=checks)
        restart.verify_adoption_api_result(value,'w','a')
        for kind in ('missing','empty-cause','wrong-code','bad-replay','changed-projection'):
            invalid=copy.deepcopy(value)
            if kind=='missing':invalid['checks']=invalid['checks'][:-1]
            if kind=='empty-cause':invalid['checks'][0]['cause_codes']=[]
            if kind=='wrong-code':invalid['checks'][0]['cause_codes']=[dict(labels=['other'])]
            if kind=='bad-replay':invalid['checks'][3]['created']=True
            if kind=='changed-projection':invalid['checks'][0]['after']['work_sha256']='d'*64
            with self.subTest(kind=kind),self.assertRaises(RuntimeError):restart.verify_adoption_api_result(invalid,'w','a')
        restart.verify_adoption_api_result(value,'w','a')

    def test_fresh_witness_refuses_revision_changed_by_control_mutations(self):
        request = restart.base64.b64encode(json.dumps(dict(expected_revision=8)).encode()).decode()
        rows = dict(agents=[[['text','a'],['text','task'],['text','/owned'],['blob',request]]])
        restart.verify_fresh_execution_revision(rows, 'a', 9)
        for changed in (11, True, '9'):
            with self.subTest(revision=changed), self.assertRaises(RuntimeError):
                restart.verify_fresh_execution_revision(rows, 'a', changed)
            restart.verify_fresh_execution_revision(rows, 'a', 9)

    def test_cell_integer_real_text_blob_are_distinct(self):
        self.assertNotEqual(restart.typed_cell(1),restart.typed_cell(1.0))
        self.assertNotEqual(restart.typed_cell('x'),restart.typed_cell(b'x'))
        self.assertNotEqual(restart.typed_cell(b'x'),restart.typed_cell(b'x\x00'))
        with self.assertRaises(RuntimeError):restart.typed_cell(True)


if __name__=='__main__':unittest.main(verbosity=2)
