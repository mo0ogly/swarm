#!/usr/bin/env python3
"""Proposed complete W/F campaign entry. SOURCE ONLY until root/CI allocation.

Two new public fixtures; one real witness review W, one expired claim and one
follow-on review F. No historical Root input, Cargo, cap/TTL change or acceptance.
Exit0 means this bounded fixture contract only, never recovery of historical377.
"""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import sqlite3
import sys
import rust_agent_model_route as fixture
import rust_native_managed_candidate as helpers
import rust_gieth_effective_dependency as binding
from rust_unstarted_review_process import Failure, OwnedRun, check, owners, report_terminal
from rust_unstarted_review_retention import retain_review

PREFIX = 'root_store::review_preview::reviewer_candidate::unstarted_sdk::'
PRODUCER = PREFIX+'unstarted_expired_claim_producer'
GENUINE = PREFIX+'unstarted_genuine_claim_refusal_rollback_retirement_and_replay'


def equal(left, right):
    """Compare actual retained JSON without bool/int coercion or normalization."""
    return json.dumps(left,sort_keys=True,allow_nan=False) == json.dumps(right,sort_keys=True,allow_nan=False)


def named_pass(run, selector):
    """Require the exact single libtest body, not exit0/collection/filter output."""
    text = run.raw('stdout.raw').decode('utf-8')
    check(len(re.findall(r'^test '+re.escape(selector)+r' \.\.\. ok$',text,re.M)) == 1
          and 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text, 'parent-one-named-pass-required')


def sdk_binding(args):
    """Bind original Cargo provenance and a qualified frozen copy without rewriting.

    Uses existing CI summary contract: original_ELF/frozen_ELF/sha/config sources.
    Copy content equality is bounded provenance, not an authenticity guarantee.
    Source/profile/collection drift refuses; no rebuild or list is performed.
    """
    profile = json.loads(args.sdk_profile.read_bytes())
    qualified = json.loads(args.sdk_qualification.read_bytes())
    artifact = profile['core_artifact']
    digest = fixture.digest(args.sdk.read_bytes())
    check(qualified['status'] == 'PASS_WORKSPACE_TEST_PROFILE' and qualified['source_stable'] is True
          and qualified['sha256'] == profile['executable_sha256'] == digest
          and qualified['original_ELF'] == artifact['executable']
          and args.sdk.resolve() in (Path(qualified['original_ELF']).resolve(),Path(qualified['frozen_ELF']).resolve()),
          'parent-sdk-frozen-original-binding')
    check(artifact['profile']['test'] is True and artifact['features'] == []
          and artifact['target']['name'] == 'swarm_engine' and profile['source_stable'] is True,
          'parent-sdk-profile')
    checkout = Path(artifact['manifest_path']).resolve().parents[3]
    inventory = profile['effective_inputs']
    check(len(inventory) == qualified['effective_workspace_inputs']
          and all(inventory.get(n) == h for n,h in qualified['effective_config_inputs'].items())
          and all(fixture.digest((checkout/n).read_bytes()) == h for n,h in inventory.items()),
          'parent-sdk-effective-inputs')
    receipt = json.loads(args.collection_receipt.read_bytes())
    check(receipt['same_ELF'] == digest and receipt['actual_list_sha256'] == fixture.digest(args.collection.read_bytes()),
          'parent-collection-image-binding')
    names = args.collection.read_text().splitlines()
    check(all(names.count(s+': test') == 1 for s in (PRODUCER,GENUINE)), 'parent-selected-body-not-unique')
    return dict(sdk_sha256=digest,checkout=str(checkout),inventory=inventory,
                original_ELF=artifact['executable'],executed_ELF=str(args.sdk),
                profile_sha256=fixture.digest(args.sdk_profile.read_bytes()),
                qualification_sha256=fixture.digest(args.sdk_qualification.read_bytes()))


def normal_checkout(args):
    """Resolve relative normal-profile inputs against their actual Cargo producer.

    Requires four normal core artifact records and this future entry colocated
    with its frozen helpers. Changing only this owned parent process cwd preserves
    historical relative inventory/config bytes; no provenance file is rewritten.
    """
    records=json.loads((args.normal_profile.parent/'artifacts.json').read_bytes())
    names={'swarm_engine','swarm_web','swarm_cli','swarm-rust'}
    core=[r for r in records if r['target']['name'] in names]
    check(len(core)==4 and {r['target']['name'] for r in core} == names
          and all(r['profile']['test'] is False and r['features'] == [] for r in core),'parent-normal-four-core-profile')
    engine=next(r for r in core if r['target']['name'] == 'swarm_engine')
    checkout=Path(engine['manifest_path']).resolve().parents[3]
    check(Path(__file__).resolve().parent.parent == checkout,'parent-helper-checkout-not-normal-producer')
    os.chdir(checkout)
    return checkout


def ledger(root, work, values, output):
    """Retain read-only actual review/call/event rows; no positive SQL or fallback.

    SQLite busy wait mirrors the actual validated Admin value, not Python's
    implicit5seconds. Queries/each cursor close/connection close retain causes.
    Deferred review-table absence is explicit, never an empty successful ledger.
    """
    timeout = values['storage_busy_timeout_ms']
    check(not isinstance(timeout,bool) and str(timeout).isdigit(), 'parent-observer-storage-timeout-shape')
    timeout = int(timeout)
    check(timeout <= 2147483647, 'parent-observer-sqlite-timeout-range') # same SQLite integer representation
    db, cursors, errors, data = None, [], [], {}
    try:
        db = sqlite3.connect((Path(root)/'.swarm/state.db').as_uri()+'?mode=ro',uri=True,timeout=timeout/1000)
        def rows(sql,params=()):
            """Collect a single explicit snapshot query; cursor is owned until close."""
            cursor = db.cursor()
            cursors.append(cursor)
            cursor.execute(sql,params)
            return cursor.fetchall()
        begin=db.cursor();cursors.append(begin);begin.execute('BEGIN')
        tables = {r[0] for r in rows("SELECT name FROM sqlite_master WHERE type='table'")}
        check({'planning_calls','events'} <= tables, 'parent-observer-schema-required')
        data['planning_calls'] = rows('SELECT id,scope_id,CAST(estimate AS TEXT),state FROM planning_calls WHERE work_id=? ORDER BY id',(work,))
        data['review_jobs_present'] = 'native_review_jobs' in tables
        data['review_jobs'] = rows('SELECT id,task_id,owner_pid,owner_stamp,owner_host,permission,child_pid,phase,receipt_sha256,published_revision FROM native_review_jobs WHERE work_id=? ORDER BY id',(work,)) if data['review_jobs_present'] else None
        data['events'] = rows('SELECT id,revision,kind FROM events WHERE work_id=? ORDER BY revision,id',(work,))
    except BaseException as error:
        errors.append(error)
    finally:
        for cursor in reversed(cursors):
            try:
                cursor.close()
            except BaseException as error:
                errors.append(error)
        if db is not None:
            for action in (db.rollback,db.close):
                try:
                    action()
                except BaseException as error:
                    errors.append(error)
    if errors:
        raise Failure('parent-observer-read-close',errors)
    fixture.publish(output,data)
    return data


class Campaign:
    """Supervise one future explicit allocation, keeping child/effect ownership.

    No resume/root parameter accepts historical missions. All operations below
    remain dormant until main is explicitly invoked after review/allocation.
    """
    def __init__(self,args):
        """Own one fresh output directory; create no Root or process here."""
        self.args, self.output = args,args.output.resolve()
        self.output.mkdir(mode=0o700,exist_ok=False)
        self.runs, self.fixtures, self.phase = [],{},'preflight'
        self.sdk, self.normal_inputs, self.helper_inputs = None,None,None
        self.normal_profile_sha = fixture.digest(args.normal_profile.read_bytes())

    def run(self,label,argv,environment=None,observe=False):
        """Transfer the actual owned child to a typed failure on every wait error."""
        owner = OwnedRun(self.output/label,argv,environment,observe)
        self.runs.append(owner)
        return owner.run()

    def cli(self,tag,label,arguments,request=None,observe=False):
        """Execute only the public normal CLI and retain original channels/status."""
        spec = self.fixtures[tag]['spec']
        argv = [str(self.args.normal),*arguments,'--root',spec['root'],'--json']
        if request is not None:
            path = self.output/(label+'.input.json')
            fixture.publish(path,request)
            argv += ['--input',str(path)]
        environment = dict(os.environ,CODEX_HOME=str(Path(spec['root'])/'codex-home'))
        run = self.run(label,argv,environment,observe)
        return json.loads(run.raw('stdout.raw')), run

    def show(self,tag,label):
        """Read current Work by public CLI, never infer state from stale prep files."""
        value,_ = self.cli(tag,label,['work','show',self.fixtures[tag]['spec']['work']])
        return value['work']

    def stable(self):
        """Recheck actual normal/SDK/helper/config bytes before each consuming stage."""
        check(fixture.digest(self.args.normal_profile.read_bytes()) == self.normal_profile_sha,'parent-normal-profile-drift')
        if self.sdk is not None:
            now = sdk_binding(self.args)
            check(now == self.sdk, 'parent-sdk-binding-drift')
        if self.normal_inputs is not None:
            check(all(fixture.digest(Path(p).read_bytes()) == h for p,h in self.normal_inputs.items()),'parent-normal-source-drift')
        if self.helper_inputs is not None:
            check(all(fixture.digest(Path(p).read_bytes()) == h for p,h in self.helper_inputs.items()),'parent-helper-drift')
        check(fixture.digest(self.args.normal.read_bytes()) == json.loads(self.args.normal_profile.read_bytes())['sha256'], 'parent-normal-binary-drift')
        check(all(fixture.digest(Path(p).read_bytes()) == h for p,h in json.loads(self.args.normal_profile.read_bytes())['cargo_config_inputs'].items()),'parent-normal-cargo-config-drift')

    def prepare(self,tag):
        """Create one genuinely new public fixture via unchanged preparer06.

        Both exit0 and final result are required; producer-input alone is early.
        No reviewer claim is created here. Program/reply copies must match Root.
        """
        self.phase = 'prepare-'+tag
        self.stable()
        path = self.output/('prepared-'+tag)
        driver = Path(__file__).with_name('rust_unstarted_claim_prepare.py')
        self.run('prepare-'+tag,[sys.executable,str(driver),'--binary',str(self.args.normal),
                 '--profile',str(self.args.normal_profile),'--output',str(path)])
        final = json.loads((path/'result.json').read_bytes())
        spec = json.loads((path/'producer-input.json').read_bytes())
        check(final['status'] == 'PUBLIC_CONTROLS_READY_FOR_RESERVED_PRODUCER'
              and final['claim_created'] is False and final['provider_review_started'] is False
              and final['root'] == spec['root'] and final['work'] == spec['work'], 'parent-preparation-exit-result')
        root = Path(spec['root']).resolve()
        check((root/'.swarm/owned-unstarted-review-fixture').read_text() == spec['token'], 'parent-new-fixture-marker')
        check(all(root != Path(v['spec']['root']).resolve() for v in self.fixtures.values()),'parent-sibling-Root-distinct')
        check((root/'codex').read_bytes() == (path/'inert-reviewer.py').read_bytes(),'parent-prepared-program-binding')
        self.fixtures[tag] = dict(path=path,spec=spec)
        work = self.show(tag,'current-'+tag)
        reviewer = work['planning']['reviewer']
        check(reviewer['calls'] == 0 and reviewer['max_calls'] == 2,'parent-existing-two-call-contract')
        self.fixtures[tag]['before'] = work
        values,_ = self.cli(tag,'settings-'+tag,['engine-settings','show'])
        self.fixtures[tag]['settings'] = values['values']
        self.fixtures[tag]['settings_revision'] = values['revision']
        budget,_ = self.cli(tag,'budget-'+tag,['budget','show',spec['work']])
        self.fixtures[tag]['budget'] = budget['budget']['budget']
        ledger(root,spec['work'],values['values'],self.output/('ledger-before-'+tag+'.json'))
        return work

    def equivalence(self):
        """Enforce explicit W/F precondition equality, preserving distinct originals.

        Only provider command Root prefix and its derived per-Root digest/call
        history are compared by an explicit projection; IDs/receipts stay separate.
        Any other relevant mismatch refuses, never relaxed after a failure.
        """
        W,F = self.fixtures['W'],self.fixtures['F']
        projections = []
        for item in (W,F):
            root = Path(item['spec']['root'])
            program = (root/'codex').read_bytes()
            check(program == (item['path']/'inert-reviewer.py').read_bytes(),'parent-equivalence-program-drift')
            providers = json.loads((root/'.swarm/providers.json').read_bytes())
            provider = providers['providers']['fixture']
            check(Path(provider['command']).resolve() == root/'codex','parent-equivalence-fixture-command')
            provider['command'] = '<new-owned-root>/codex' # comparison only; original never rewritten
            reviewer = dict(item['before']['planning']['reviewer'])
            reviewer.pop('provider_digest',None)
            reviewer.pop('calls',None)
            task = next(t for t in item['before']['tasks'] if t['id'] == item['spec']['request']['task_id'])
            budget = item['budget']
            projections.append(dict(program_sha256=fixture.digest(program),reply=json.loads((item['path']/'inert-reply.json').read_bytes()),
                provider=providers,reviewer=reviewer,settings=item['settings'],
                budget={k:budget[k] for k in ('limit_usd','reserve_per_launch_usd','estimate_source','reference_date')},
                checks=item['before']['planning']['checks'],
                task={k:task.get(k) for k in ('title','criteria','deliverable','requirements')}))
        fixture.publish(self.output/'equivalence.json',dict(W=projections[0],F=projections[1],
            expected_differences='IDs/paths/events/receipt/provenance/owners differ and are not reused; only program command prefix normalized for comparison',
            scope='distinct healthy witness, not same physical Root or attempt'))
        check(equal(*projections),'parent-WF-equivalence-mismatch')

    def producer(self):
        """Run exact producer once, then validate actual waited owner and proofs."""
        self.phase = 'producer-F'
        self.stable()
        item = self.fixtures['F']
        root, path, spec = Path(item['spec']['root']),item['path'],item['spec']
        current = self.show('F','F-before-producer')
        check(current == item['before'] and current['planning']['reviewer']['calls'] == 0,'parent-F-prepared-state-stale')
        check((root/'codex').read_bytes() == (path/'inert-reviewer.py').read_bytes(),'parent-F-program-changed')
        evidence = self.output/'producer-originals'
        env = dict(os.environ,CODEX_HOME=str(root/'codex-home'),SWARM_UNSTARTED_PRODUCER_INPUT=str(path/'producer-input.json'),SWARM_UNSTARTED_PRODUCER_EVIDENCE=str(evidence))
        owner = self.run('producer',[str(self.args.sdk),PRODUCER,'--exact','--ignored','--nocapture'],env,True)
        named_pass(owner,PRODUCER)
        observed = json.loads((evidence/'producer-observation.json').read_bytes())
        final = json.loads((evidence/'producer-finalization.json').read_bytes())
        fresh = json.loads((evidence/'fresh-witness.json').read_bytes())
        check(final['complete'] is True and final['failure'] is None and observed['failure'] is None,'parent-producer-final-proof')
        result,job = observed['result'],observed['result']['job']
        check(result['root'] == str(root) and result['work'] == spec['work'] and result['token'] == spec['token']
              and result['review'] == job['id'] and job['work_id'] == spec['work']
              and job['task_id'] == spec['request']['task_id'] and job['event_id'] == spec['request']['event_id'], 'parent-producer-fixture-binding')
        check(equal(job,fresh['job']) and equal(job,observed['known_claim']) and equal(result['funding'],fresh['funding']), 'parent-producer-fresh-history')
        check(observed['claim_attempted'] is True and observed['claim_outcome'] == 'recorded-work-known'
              and equal(final['known_recorded_work'],observed['known_recorded_work'])
              and observed['known_recorded_work'] == dict(work=spec['work'],revision=fresh['work']['revision']), 'parent-producer-recorded-Work')
        check(job['settings_revision'] == item['settings_revision'],'parent-F-settings-generation-changed')
        check(owner.identity is not None and type(job['owner_pid']) is int and job['owner_pid'] == owner.process.pid
              and job['owner_stamp'] == owner.identity['stamp'] and job['owner_host'] == owner.identity['host'], 'parent-producer-kernel-owner')
        check(job['phase'] == 'claimed' and type(job['permission']) is int and job['permission'] == 0
              and all(job[k] is None for k in ('child_pid','parent_pid','group_id','child_stamp','receipt_sha256','published_revision')), 'parent-producer-prelaunch-shape')
        check(result['original_expiry'] == job['preflight_until'] == fresh['original_expiry']
              and result['owner_departure_verified'] is False and fresh['owner_alive'] is True, 'parent-producer-unchanged-original-expiry')
        item['expired'] = {key:result[key] for key in ('root','work','token','review')}
        item['claimed_work'] = self.show('F','F-claimed-after-wholewait')
        check(item['claimed_work']['planning']['reviewer']['calls'] == 1,'parent-one-claim-call-debit')
        item['claimed_ledger'] = ledger(root,spec['work'],item['settings'],self.output/'F-claimed-ledger.json')
        fixture.publish(self.output/'genuine-sdk-spec.json',item['expired'])
        self.stable()

    def execute(self):
        """Full future public sequence, fail closed before each dependent stage."""
        check(__debug__,'parent-helper-oracles-require-assertions')
        check(hasattr(os,'P_PIDFD') and hasattr(os,'waitid') and hasattr(os,'WNOWAIT')
              and signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,'parent-linux-sole-waiter-prerequisite')
        normal_root=normal_checkout(self.args)
        self.helper_inputs = helpers.helper_inputs(Path(__file__))
        driver = Path(__file__).with_name('rust_unstarted_claim_prepare.py')
        self.helper_inputs.update(helpers.helper_inputs(driver))
        fixture.publish(self.output/'helper-inputs.before.json',self.helper_inputs)
        profileout = self.output/'normal-profile';profileout.mkdir()
        self.normal_inputs = binding.qualify(self.args.normal,self.args.normal_profile,profileout)
        self.sdk = sdk_binding(self.args)
        fixture.publish(self.output/'sdk-binding.json',self.sdk)
        shared=set(self.sdk['inventory']) & set(self.normal_inputs)
        engine=[p for p in shared if p.startswith('rust/crates/swarm-engine/')]
        required={'rust/crates/swarm-engine/src/root_store/reviewer_native/'+n for n in ('claim.rs','guard.rs','store.rs','unstarted.rs')}
        check(required <= set(engine) and all(self.sdk['inventory'][p] == self.normal_inputs[p] for p in shared),
              'parent-normal-sdk-common-engine-input-mismatch')
        fixture.publish(self.output/'cross-profile-inputs.json',dict(engine_common=sorted(engine),common=sorted(shared),
            normal_checkout=str(normal_root),sdk_checkout=self.sdk['checkout'],scope='common bytes only, not whole web/CLI graph equality'))
        self.prepare('W')
        self.phase = 'normal-witness-W'
        self.stable()
        w = self.fixtures['W'];request = dict(w['spec']['request'],event_id='parent12-healthy-review')
        check(self.show('W','W-immediate-before-review') == w['before'],'parent-W-prepared-state-stale')
        check((Path(w['spec']['root'])/'codex').read_bytes() == (w['path']/'inert-reviewer.py').read_bytes(),'parent-W-program-changed')
        reply,owner = self.cli('W','W-review',['task','review',w['spec']['work']],request,True)
        after = self.show('W','W-after-review')
        check(after['planning']['reviewer']['calls'] == 1,'parent-W-one-real-call')
        retained=retain_review(w['spec']['root'],after,request['task_id'],reply,owner,self.output/'W-originals')
        check(retained['settings_revision'] == w['settings_revision'],'parent-W-settings-generation-changed')
        replay,_ = self.cli('W','W-reconcile',['task','review-reconcile',reply['review_id']])
        check(replay['replayed'] is True and replay['review_id'] == reply['review_id'] and replay['verdict_applicable'] is True
              and replay['task_accepted'] is False,'parent-W-engine-reconciliation')
        ledger(Path(w['spec']['root']),w['spec']['work'],w['settings'],self.output/'ledger-after-W.json')
        self.prepare('F')
        self.equivalence()
        self.producer()
        self.phase = 'genuine-F'
        self.stable()
        f = self.fixtures['F'];root=Path(f['spec']['root'])
        capture=self.output/'followon-capture' # genuine SDK creates this only at real CLI follow-on
        env=dict(os.environ,CODEX_HOME=str(root/'codex-home'),SWARM_UNSTARTED_REVIEW_FIXTURE=str(self.output/'genuine-sdk-spec.json'),
                 SWARM_UNSTARTED_REVIEW_BINARY=str(self.args.normal),SWARM_UNSTARTED_REVIEW_PROFILE=str(self.args.normal_profile),
                 SWARM_UNSTARTED_REVIEW_EVIDENCE=str(capture))
        owner=self.run('genuine',[str(self.args.sdk),GENUINE,'--exact','--ignored','--nocapture'],env)
        named_pass(owner,GENUINE)
        after=self.show('F','F-after-genuine')
        task=next(t for t in after['tasks'] if t['id'] == f['spec']['request']['task_id'])
        beforetask=next(t for t in f['claimed_work']['tasks'] if t['id'] == task['id'])
        check(after['planning']['reviewer']['calls'] == 2 and after['planning']['reviewer']['max_calls'] == 2
              and task['status'] == 'submitted' and task['independent_review']['state'] == 'passed'
              and task['independent_review']['id'] != f['expired']['review']
              and equal(task['attempts'],beforetask['attempts']) and equal(task['automatic_validation'],beforetask['automatic_validation'])
              and any(r['id'] == f['expired']['review'] and r['state'] == 'stale' for r in task['previous_reviews']), 'parent-genuine-preserved-history')
        current=ledger(root,f['spec']['work'],f['settings'],self.output/'F-after-ledger.json')
        check(all(row in current['planning_calls'] for row in f['claimed_ledger']['planning_calls']), 'parent-genuine-reserve-history')
        capture_result=json.loads((capture/'result.json').read_bytes())
        followon=json.loads((capture/'stdout.raw').read_bytes())
        check(capture_result['capture_complete'] is True and capture_result['child_wait_confirmed'] is True
              and capture_result['exit_success'] is True and capture_result['status_code'] == 0
              and followon['review_recorded'] is True and followon['verdict_applicable'] is True
              and followon['task_accepted'] is False and followon['review_id'] == task['independent_review']['id'], 'parent-followon-real-CLI')
        self.stable()
        fixture.publish(self.output/'result.json',dict(status='PASS_BOUNDED_NEW_WF_FIXTURES',
            W=w['spec']['work'],F=f['spec']['work'],expired_review=f['expired']['review'],followon_review=followon['review_id'],
            review_requests=3,task_accepted=False,historical377_recovered=False,global_acceptance=False,
            departed_guard_scope='existing genuine SDK preface and retirement guards, no standalone receipt',
            process_waits=[r.summary() for r in self.runs]))
        return 0

    def failure_observations(self):
        """Retain safe known/unknown effects without replacing a primary failure.

        No further process is launched while an owned wait is unknown. Per-Root
        public Work reads and read-only rows are attempted only for this run's
        successfully prepared fresh Roots, never a user-supplied/history Root.
        A newly pending owner is handed back immediately with earlier facts and
        unread Roots marked unknown; later observations require settlement first.
        """
        observed,errors=[],[]
        pending_roots=list(self.fixtures)
        for tag,item in self.fixtures.items():
            try:
                value=self.show(tag,'failure-read-'+tag)
                observed.append(dict(fixture=tag,observed_work=value['id'],observed_revision=value['revision'],state='public-read-observation-not-acceptance'))
                ledger(Path(item['spec']['root']),item['spec']['work'],item['settings'],self.output/('failure-ledger-'+tag+'.json'))
            except BaseException as error:
                errors.append(error)
                if any(o.pending() for o in owners(error)):
                    # Transfer immediately; no F command while W ownership or
                    # consumed-wait validation is still unresolved. Prior facts
                    # remain attached, unread Roots are explicitly unknown.
                    raise Failure('parent-observer-pending-owner',errors,facts=dict(
                        phase=self.phase,effects=observed,unread_roots=pending_roots,
                        unread_state='unknown, no later subprocess started'))
            pending_roots.remove(tag)
        if 'F' in self.fixtures:
            for name in ('producer-observation.json','producer-finalization.json','fresh-witness.json'):
                path=self.output/'producer-originals'/name
                try:
                    value=json.loads(path.read_bytes())
                    metadata=value.get('known_recorded_work')
                    if metadata is not None:
                        check(isinstance(metadata,dict) and metadata['work'] == self.fixtures['F']['spec']['work']
                              and type(metadata['revision']) is int and metadata['revision'] > 0,'parent-failure-recorded-metadata-shape')
                        observed.append(dict(recorded_work_observation=metadata,source_sha256=fixture.digest(path.read_bytes()),
                                             scope='reported recorded Work, may include replay, not acceptance'))
                    native=value.get('failure')
                    if isinstance(native,list):
                        codes=[]
                        for node in native:
                            for field in ('io_errno','sqlite_code'):
                                number=node.get(field)
                                if number is not None:
                                    check(type(number) is int,'parent-failure-native-numeric-shape')
                                    codes.append(dict(kind=field,value=number))
                        observed.append(dict(producer_native_numeric_causes=codes,original_sha256=fixture.digest(path.read_bytes())))
                except BaseException as error:
                    errors.append(error)
        return Failure('parent-failure-state-observations',errors,facts=dict(phase=self.phase,effects=observed,unobserved_effects='unknown, never rollback'))


def main():
    """Final supervised consumer: preserve owners/typed failures through safe exit.

    No execution is authorized by preparing this source. Root/CI must review and
    allocate the three-request W/F experiment and actual profiles/storage first.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    for option in ('normal','normal-profile','sdk','sdk-profile','sdk-qualification','collection','collection-receipt','output'):
        parser.add_argument('--'+option,type=Path,required=True)
    args=parser.parse_args()
    for key,value in vars(args).items():setattr(args,key,value.resolve())
    campaign=None
    try:
        campaign=Campaign(args)
        return campaign.execute()
    except BaseException as error:
        # The terminal controller alone receives owner handback, settles, then
        # may perform readonly observations. It never reruns a product action.
        return report_terminal(error,campaign.output if campaign is not None else None,
                               campaign.failure_observations if campaign is not None else None)


if __name__ == '__main__':
    try:
        status=main()
    except SystemExit:
        raise  # argparse exits occur before any resource/Root/process ownership
    except BaseException as error:
        status=report_terminal(Failure('parent-bootstrap-failed',[error]),None)
    raise SystemExit(status)
