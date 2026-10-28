#!/usr/bin/env python3
"""Replay synthetic historical Go-format rows through the normal CLI and HTTP.

Owned inert Root only: seed retained data, not worker authority. No provider is
configured or contacted, and call policy remains disabled. ASCII request bytes
follow the actual Go PreparationSend field order/omitempty contract. This is not
an execution of the old Go binary or a migration of a living Go supervisor.
No global watchdog, silent retry, automatic refund or live Root mutation.
"""
import argparse
import hashlib
import json
import sqlite3
from pathlib import Path

import rust_agent_model_route as fixture
import rust_native_http_cli as native


def ledger_state(database):
    """Observe an absent historical ledger distinctly from an empty present one.

    Observe the actual preparation_reservations table, including all amounts/states. Schema creation is not part of replay. SQL errors propagate; only an actual
    sqlite_master observation establishes absence on this owned fixture.
    """
    present = database.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='preparation_reservations'").fetchone() is not None
    return (present, database.execute('SELECT * FROM preparation_reservations').fetchall() if present else None)


def main():
    """Assert exact replay of every historical status, conflicts and unchanged ledger."""
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    out = Path(args.output).resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    f = fixture.Fixture(Path(args.binary).resolve(), out, 'legacy-replay')
    f.environment.pop('SWARM_PREPARATION_DISABLED', None)
    f.settings_change(preparation_write_enabled='1')
    preparation = f.ok(['prepare', 'create'], dict(version=1, action='create',
        event_id='historical-create', expected_revision=0, text='Historical fixture'))
    checks = []
    server = None
    primary = None

    def check(name, condition):
        """Persist a named semantic assertion; failed prerequisites never pass."""
        assert condition, name
        checks.append(name)
        fixture.publish(out / 'checks.json', checks)
        print('PASS ' + name, flush=True)

    try:
        request = dict(level='high', version=1, preparation_id=preparation['id'],
            event_id='historical-turn', expected_revision=preparation['revision'],
            provider='api-historical', capability_hash='historical-capability',
            message='Exact historical intent')
        raw = json.dumps(request, separators=(',', ':')).encode()
        turn_id = 'pt-' + hashlib.sha256((preparation['id'] + '\0' + request['event_id']).encode()).hexdigest()[:32]
        body = dict(id=turn_id, preparation_id=preparation['id'], status='failed',
            source_revision=preparation['revision'], method_hash=preparation['method_hash'],
            provider=request['provider'], request_hash=hashlib.sha256(raw).hexdigest(),
            question=request['message'], error='Retained fixture failure',
            prompt='PRIVATE_HISTORICAL_PROMPT')
        database = f.root / '.swarm/state.db'
        with sqlite3.connect(database) as db:
            db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)',
                (turn_id, preparation['id'], 'failed', json.dumps(body).encode()))
            before = db.execute('SELECT id,preparation_id,status,body FROM preparation_turns').fetchall()
            db.execute('INSERT INTO preparation_reservations(turn_id,preparation_id,work_id,amount,work_amount,state) VALUES(?,?,?,?,?,?)',(turn_id,preparation['id'],'',2.5,0,'estimated'))
            ledger = ledger_state(db)
        observed = f.ok(['prepare', 'send', preparation['id']], request)
        check('legacy-terminal-cli-replay', observed['id'] == turn_id and observed['status'] == 'failed' and observed['error'] == body['error'])
        check('legacy-replay-public-prompt-omitted', 'prompt' not in observed and '_native' not in observed and 'PRIVATE_HISTORICAL_PROMPT' not in json.dumps(observed))
        server = native.Server(f.binary, f.root, out, 'historical-http', environment=f.environment)
        server.capture_async(out / 'historical-http.raw')
        csrf = server.request('/api/v1/native-session', observe=False)['csrf_token']
        result = server.request('/api/v1/preparations/send', request, csrf=csrf, observe=False)
        check('legacy-terminal-cli-http-equal', result == observed)
        changed = {**request, 'message': 'Different historical intent'}
        rc, error = f.call(['prepare', 'send', preparation['id']], changed)
        check('legacy-cli-event-conflict', rc != 0 and error['failure']['code'] == 'preparation-event-conflict')
        result = server.request('/api/v1/preparations/send', changed, csrf=csrf, status=409, observe=False)
        check('legacy-http-event-conflict', result['failure']['code'] == 'preparation-event-conflict')
        rc, error = f.call(['prepare', 'send', preparation['id']], {**request, 'event_id': 'new-high-level'})
        check('new-unsupported-level-still-refused', rc != 0 and error['failure']['code'] == 'preparation-send-invalid')
        with sqlite3.connect(database) as db:
            check('legacy-terminal-no-row-or-budget-write', before == db.execute('SELECT id,preparation_id,status,body FROM preparation_turns').fetchall() and ledger == ledger_state(db))
        for status in ['pending','running','stopping']:
            for state in ['reserved','estimated']:
                with sqlite3.connect(database) as db:
                    db.execute('UPDATE preparation_turns SET status=? WHERE id=?',(status,turn_id))
                    db.execute('UPDATE preparation_reservations SET state=? WHERE turn_id=?',(state,turn_id))
                    active=db.execute('SELECT id,preparation_id,status,body FROM preparation_turns').fetchall()
                    ledger=ledger_state(db)
                label=status+'-'+state
                rc,observed=f.call(['prepare','send',preparation['id']],request)
                check('legacy-active-cli-observation-'+label,rc==0 and observed['id']==turn_id and observed['status']==status)
                result=server.request('/api/v1/preparations/send',request,csrf=csrf,observe=False)
                check('legacy-active-http-observation-'+label,result==observed and 'prompt' not in result and '_native' not in result)
                result=server.request('/api/v1/preparations/send',changed,csrf=csrf,status=409,observe=False)
                check('legacy-active-conflict-'+label,result['failure']['code']=='preparation-event-conflict')
                with sqlite3.connect(database) as db:
                    check('legacy-active-no-write-or-budget-'+label,active==db.execute('SELECT id,preparation_id,status,body FROM preparation_turns').fetchall() and ledger==ledger_state(db))
        check('legacy-replay-no-provider-launched',not (f.root/'model-received.json').exists())
        check('legacy-replay-did-not-enable-calls', f.ok(['engine-settings', 'show'])['values']['preparation_call_enabled'] == '0')
        fixture.publish(out / 'result.json', dict(passed=True, assertions=checks,
            binary_sha256=fixture.digest(f.binary.read_bytes()), root=str(f.root),
            limits='Synthetic Go-format data only; no living legacy worker or real provider qualification.'))
        print('PREPARATION_LEGACY_REPLAY_PASS ' + str(len(checks)), flush=True)
    except BaseException as error:
        primary = error
    finally:
        if server is not None:
            native.finish(server, primary)
        elif primary is not None:
            raise primary


if __name__ == '__main__':
    main()
