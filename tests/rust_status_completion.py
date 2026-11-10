"""Public readonly status after an owned ordinary native completion.

Called by the maintained logs journey after its inert local agent exits. No
provider invocation or SQL writes here. Real Root report files exercise absent,
attributable, ambiguous and recovered presentation; none is task acceptance.
"""
from pathlib import Path


def normalize_live_volumes(value, label, check):
    """Validate volatile statvfs facts in a caller-owned comparison copy.

    Original observations must already be retained. Require actual nonempty
    volumes and unsigned available counts within their current physical capacity;
    normalize only those two live counts. Paths, kinds, states and every budget/
    task/result remain untouched. IO/refusal propagates; this grants no acceptance.
    """
    import os
    volumes = value['runtime']['volumes']
    for volume in volumes:
        capacity = os.statvfs(volume['path'])
        for key, maximum in [('available_bytes', capacity.f_blocks * capacity.f_frsize),
                             ('available_inodes', capacity.f_files)]:
            assert type(volume[key]) is int and 0 <= volume[key] <= maximum, (key, volume)
            volume[key] = '<per-read statvfs>'
    check(label+'-volumes', bool(volumes))


def verify(server, owned, check):
    """Compare HTTP with actual CLI and preserve Work/events/agent/budget state.

    The caller supplies its owned Fixture and named assertion collector. All IO,
    CLI and HTTP failures propagate. Only this fixture's docs files are created
    and removed; callers retain the original failure and perform server cleanup.
    Per-read clock/free-space facts are validated before stable DTO comparison;
    no task, budget, volume identity or safety-state difference is normalized.
    """
    path = '/api/v1/mission?work='+owned.work+'&action=status'
    before = owned.snapshot()
    def read(label, state, report_state):
        """Require meaningful nonvalidated result, not merely transport success."""
        cli = owned.ok(['mission', 'status', owned.work])
        actual = server.request(path, observe=False)
        from rust_agent_model_route import publish
        publish(owned.output/('completion-'+label+'-observations.json'), dict(cli=cli,http=actual))
        # Each public read observes its own clock and live statvfs availability.
        # Retain originals, validate those observations against the actual volume
        # capacity, then compare every stable field, including paths/kinds/states.
        import copy, datetime
        comparable = []
        for value in [cli, actual]:
            current = copy.deepcopy(value)
            stamps = [current[key]['observed_at'] for key in ['runtime', 'spending']]
            check('completion-'+label+'-clock-'+str(len(comparable)),
                  stamps[0] == stamps[1] and bool(datetime.datetime.fromisoformat(stamps[0].replace('Z', '+00:00')).tzinfo))
            for key in ['runtime', 'spending']: current[key]['observed_at'] = '<per-read clock>'
            normalize_live_volumes(current, 'completion-'+label+'-'+str(len(comparable)), check)
            comparable.append(current)
        check('completion-'+label+'-CLI-HTTP', comparable[0] == comparable[1])
        row = actual['tasks'][0]
        check('completion-'+label+'-result', actual['validated'] == 0
              and row['result']['process_state'] == 'completed'
              and row['result']['validation_state'] == 'not_started'
              and row['result']['state'] == state
              and row['result']['report_state'] == report_state)
        check('completion-'+label+'-unchanged', owned.snapshot() == before)
        return actual
    read('absent', 'completed_unproven', 'absent_or_unattributed')
    docs = owned.root/'docs'; docs.mkdir(exist_ok=True)
    report = docs/'task.md'; other = docs/'task-other-handoff.md'
    assert not report.exists() and not other.exists(), 'owned report witness collision'
    from rust_agent_model_route import publish
    publish(report, b'Owned report witness: contents establish no acceptance.\n')
    value = read('attributable', 'result_to_review', 'attributable_unsubmitted')
    check('completion-report-path', value['tasks'][0]['result']['report_id'] == 'docs/task.md')
    publish(other, b'Second candidate: attribution ambiguous.\n')
    read('ambiguous', 'completed_unproven', 'absent_or_unattributed')
    other.unlink()
    read('recovered', 'result_to_review', 'attributable_unsubmitted')
    report.unlink()
    read('absent-again', 'completed_unproven', 'absent_or_unattributed')
