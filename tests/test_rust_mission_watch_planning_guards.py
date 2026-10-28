"""Pure witness/refusal/recovery checks for automatic planning evidence; no Root runtime."""
import copy
import json
import unittest
import errno
import tempfile
from pathlib import Path
from unittest.mock import patch
import rust_mission_watch_planning as replay


class PlanningGuards(unittest.TestCase):
    """Keep valid public evidence distinct from plausible empty or forged output."""

    def fixture(self):
        """Supply exact two-claim witnesses while independently stubbing old event checks."""
        claims = [dict(scope=scope,events=[f'input-{index}'],claim_id=f'claim-{index}')
                  for index,scope in enumerate(('root','child'))]
        events = [dict(schema_version=1,event='mission-planning-step',work='work',status='applied',
            selected_scope=c['scope'],input_event=c['events'][0],claim_id=c['claim_id'],
            settings_revision="1",work_revision=index+2,decision_applied=True,mission_complete=False,
            automatic_reviewer=False,automatic_validation=False,observation_is_permission=False)
            for index,c in enumerate(claims)]
        return claims,events+[dict(event='mission-worker-started')]

    def validate(self, claims, events):
        """Exercise the actual new validator; legacy event checks have their own suite."""
        with patch.object(replay.public,'public_watch_events'):
            return replay.planning_events(b'\n'.join(json.dumps(e).encode() for e in events),'work','agent','sigint',claims)

    def test_two_real_claim_witness(self):
        claims,events=self.fixture()
        self.assertEqual(self.validate(claims,events),events)

    def test_empty_and_duplicate_refuse_then_recover(self):
        claims,events=self.fixture();self.validate(claims,events)
        for invalid in ([],events[:1],events+[events[0]]):
            with self.assertRaises(RuntimeError):self.validate(claims,invalid)
        self.validate(claims,events)

    def test_wrong_scope_claim_input_and_revision_refuse(self):
        claims,events=self.fixture();self.validate(claims,events)
        for key,value in [('selected_scope','root'),('claim_id','forged'),('input_event','unclaimed'),
                          ('work','other'),('decision_applied',False),('schema_version',True),('work_revision',True)]:
            bad=copy.deepcopy(events);bad[1][key]=value
            with self.assertRaises(RuntimeError):self.validate(claims,bad)
        self.validate(claims,events)

    def test_setting_count_string_control_refusal_recovery(self):
        claims,events=self.fixture();self.validate(claims,events)
        for value in (1,True,'01','0','١','',None):
            bad=copy.deepcopy(events);bad[0]['settings_revision']=value
            with self.assertRaises(RuntimeError):self.validate(claims,bad)
        self.validate(claims,events)

    def test_ancestor_activation_control_refusal_recovery(self):
        claims=[dict(scope='root'),dict(scope='child')]
        work=dict(planning=dict(scopes=[dict(id='root',activations=2),
            dict(id='child',activations=1,parent='root',requirements=['req-1'])]))
        replay.activation_accounting(work,claims)
        for root_count,child_count in ((1,1),(2,True),(True,1),(2,2)):
            bad=copy.deepcopy(work)
            bad['planning']['scopes'][0]['activations']=root_count
            bad['planning']['scopes'][1]['activations']=child_count
            with self.assertRaises(RuntimeError):replay.activation_accounting(bad,claims)
        replay.activation_accounting(work,claims)

    def test_setup_execution_refuses_after_valid_control(self):
        good=[dict(argv=['swarm','planning','enable','work'])]
        replay.setup_calls(good)
        for command in (['planning','step'],['planning','run'],['agent','run']):
            with self.assertRaises(RuntimeError):replay.setup_calls(good+[dict(argv=['swarm',*command,'work'])])
        replay.setup_calls(good)

    def test_unknown_status_refuses_after_valid_control(self):
        claims,events=self.fixture();self.validate(claims,events)
        with self.assertRaises(RuntimeError):self.validate(claims,events+[dict(event='mission-planning-step',status='refused',work='work')])
        self.validate(claims,events)

    def test_program_contains_both_roles_without_replacement(self):
        first=replay.provider_program('nonce');second=replay.provider_program('nonce')
        self.assertEqual(first,second)
        compile(first,'<stable-provider>','exec')
        self.assertIn('watch-planner.sock',first)
        self.assertIn('release-worker.json',first)
        self.assertIn('unknown owned fixture role',first)

    def test_terminal_replay_requires_real_work_after_valid_control(self):
        work=dict(id='work',revision=17)
        good=dict(schema_version=1,claim_id='claim',accepted=False,replayed=True,decision_applied=True,work=work)
        replay.require_terminal_replay(good,work,'claim')
        for invalid in ({},dict(good,replayed=False),dict(good,work=dict(work,revision=16)),
                        dict(good,decision_applied=1),dict(good,claim_id='other'),dict(good,accepted=True)):
            with self.assertRaises(RuntimeError):replay.require_terminal_replay(invalid,work,'claim')
        replay.require_terminal_replay(good,work,'claim')

    def test_backup_failure_preserves_original_and_partial_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            source=Path(directory)/'original.db'
            connection=replay.sqlite3.connect(source)
            try:
                connection.execute('CREATE TABLE witness(value TEXT NOT NULL)')
                connection.execute('INSERT INTO witness VALUES(?)',['owned SQLite witness'])
                connection.commit()
            finally:
                connection.close()

            def valid_backup(name):
                """Use this exact source for real backup/control and recovery reads."""
                target=Path(directory)/name
                replay.backup_owned_database(source,target)
                self.assertEqual((source.stat().st_nlink,target.stat().st_nlink),(1,1))
                db=replay.sqlite3.connect(target.as_uri()+'?mode=ro',uri=True)
                try:
                    self.assertEqual(db.execute('SELECT value FROM witness').fetchall(),[('owned SQLite witness',)])
                finally:
                    db.close()

            valid_backup('valid-control.db')
            original=(source.stat().st_ino,source.read_bytes())
            destination=Path(directory)/'failed-copy.db'
            cause=OSError(errno.ENOSPC,'explicit fixture backup refusal')
            with patch.object(replay.sqlite3,'connect',side_effect=cause):
                with self.assertRaises(BaseExceptionGroup) as failure:
                    replay.backup_owned_database(source,destination)
            self.assertIn(cause,failure.exception.exceptions)
            self.assertEqual((source.stat().st_ino,source.read_bytes()),original)
            self.assertTrue(destination.exists())
            self.assertEqual(destination.stat().st_mode & 0o777,0o600)
            valid_backup('valid-recovery.db')
            self.assertEqual((source.stat().st_ino,source.read_bytes()),original)

    def test_backup_busy_locked_refuse_without_counter_success(self):
        for status in (replay.sqlite3.SQLITE_OK,replay.sqlite3.SQLITE_DONE):
            replay.backup_progress(status,0,0)
        for status in (replay.sqlite3.SQLITE_BUSY,replay.sqlite3.SQLITE_LOCKED,True):
            with self.assertRaisesRegex(RuntimeError,'offline-backup-sqlite-status-' + str(status)):
                replay.backup_progress(status,0,0)
        replay.backup_progress(replay.sqlite3.SQLITE_DONE,0,0)

    def test_directory_sync_failure_is_not_swallowed(self):
        with tempfile.TemporaryDirectory() as directory:
            replay.sync_directory(Path(directory))
            cause=OSError(errno.ENOSPC,'explicit fixture sync refusal')
            with patch.object(replay.os,'fsync',side_effect=cause):
                with self.assertRaises(OSError) as failure:replay.sync_directory(Path(directory))
            self.assertIs(failure.exception,cause)
            replay.sync_directory(Path(directory))

    @replay.contextmanager
    def swap_fixture(self):
        """Create owned single-link files, with saved absent, for move-only checks."""
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);private=root/'private';private.mkdir(mode=0o700)
            database=root/'state.db';copy=private/'copied.db';saved=private/'original.db'
            database.write_bytes(b'owned original inode');copy.write_bytes(b'fresh copied inode')
            database.chmod(0o600);copy.chmod(0o600)
            yield database,saved,copy,replay.physical(database),replay.physical(copy)

    def restored(self, state):
        """Require exact original/copy inode, content and nlink1 after restoration."""
        database,saved,copy,original,copied=state
        self.assertEqual(replay.physical(database),original)
        self.assertEqual(replay.physical(copy),copied)
        self.assertEqual(database.read_bytes(),b'owned original inode')
        self.assertEqual(copy.read_bytes(),b'fresh copied inode')
        self.assertEqual((database.stat().st_nlink,copy.stat().st_nlink),(1,1))
        self.assertFalse(saved.exists())

    def valid_swap(self, state):
        """Witness copied state.db and staged original both single-link, then restore."""
        database,saved,copy,original,copied=state
        self.restored(state)
        with replay.swapped_database(*state):
            self.assertEqual(replay.physical(database),copied)
            self.assertEqual(replay.physical(saved),original)
            self.assertEqual((database.stat().st_nlink,saved.stat().st_nlink),(1,1))
        self.restored(state)

    def leaf_causes(self, error):
        """Read retained primary and cleanup causes without dropping nested groups."""
        if isinstance(error,BaseExceptionGroup):
            return [leaf for cause in error.exceptions for leaf in self.leaf_causes(cause)]
        return [error]

    def test_move_only_swap_body_refusal_and_recovery(self):
        with self.swap_fixture() as state:
            self.valid_swap(state)
            cause=RuntimeError('explicit owned offline operation refusal')
            with self.assertRaises(BaseExceptionGroup) as failure:
                with replay.swapped_database(*state):
                    self.assertEqual(state[0].stat().st_nlink,1)
                    raise cause
            self.assertIn(cause,self.leaf_causes(failure.exception))
            self.restored(state)
            self.valid_swap(state)

    def test_move_only_install_rename_failures_restore_original(self):
        with self.swap_fixture() as state:
            real_replace=replay.os.replace
            for stage,after_effect in ((1,False),(1,True),(2,False),(2,True)):
                self.valid_swap(state)
                calls=[];cause=OSError(errno.ENOSPC,'explicit owned rename refusal')
                def refuse_once(source,destination):
                    """Inject once before/after the selected actual installation move."""
                    calls.append((source,destination))
                    if len(calls)==stage:
                        if after_effect:real_replace(source,destination)
                        raise cause
                    return real_replace(source,destination)
                with patch.object(replay.os,'replace',side_effect=refuse_once):
                    with self.assertRaises(BaseExceptionGroup) as failure:
                        with replay.swapped_database(*state):self.fail('rename failure reached operation')
                self.assertIn(cause,self.leaf_causes(failure.exception))
                self.restored(state)
                self.valid_swap(state)

    def test_move_only_sync_failures_restore_staged_original(self):
        with self.swap_fixture() as state:
            real_sync=replay.sync_directory
            for stage in (1,3):
                self.valid_swap(state)
                calls=[];cause=OSError(errno.ENOSPC,'explicit owned post-rename sync refusal')
                def refuse_once(directory):
                    """Fail sync once with original already staged or backup installed."""
                    calls.append(directory)
                    if len(calls)==stage:raise cause
                    return real_sync(directory)
                with patch.object(replay,'sync_directory',side_effect=refuse_once):
                    with self.assertRaises(BaseExceptionGroup) as failure:
                        with replay.swapped_database(*state):self.fail('sync failure reached operation')
                self.assertIn(cause,self.leaf_causes(failure.exception))
                self.restored(state)
                self.valid_swap(state)

    def test_move_only_restore_refusal_preserves_original_then_recovers(self):
        with self.swap_fixture() as state:
            self.valid_swap(state)
            real_replace=replay.os.replace;calls=[]
            primary=RuntimeError('explicit owned operation refusal')
            cleanup=OSError(errno.ENOSPC,'explicit owned retention move refusal')
            def refuse_retention(source,destination):
                """Keep original private if retaining the installed copy cannot proceed."""
                calls.append((source,destination))
                if len(calls)==3:raise cleanup
                return real_replace(source,destination)
            with patch.object(replay.os,'replace',side_effect=refuse_retention):
                with self.assertRaises(BaseExceptionGroup) as failure:
                    with replay.swapped_database(*state):raise primary
            leaves=self.leaf_causes(failure.exception)
            self.assertIn(primary,leaves);self.assertIn(cleanup,leaves)
            database,saved,copy,original,copied=state
            self.assertEqual(replay.physical(saved),original)
            self.assertEqual(saved.read_bytes(),b'owned original inode')
            self.assertEqual(saved.stat().st_nlink,1)
            self.assertEqual(replay.physical(database),copied)
            replay.restore_database_move(*state)
            self.restored(state)
            self.valid_swap(state)


if __name__=='__main__':unittest.main()
