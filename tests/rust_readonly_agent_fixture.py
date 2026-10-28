"""Prepare a real historical Agent, explicitly cancel its unstarted intention.

Starting a reference Go server runs its conductor; a manual queued grant can be
runnable even without continuous mission consent. This helper grants no scope,
execution or acceptance: cancellation is a supported operation on an owned inert
fixture. Keep shared Fixture.prepare unchanged for intentional execution tests.
"""
from rust_agent_model_route import publish

def prepare_historical_agent(fixture):
    """Return a non-runnable recorded Agent before any server starts.

    Public prepare then public reconcile cancel the unregistered intention and
    release only its unspent reservation. No consumed usage is reset, provider
    started or mission accepted. Refused/uncertain cancellation propagates; never
    seed desired/status in SQL or start a server after a failed prerequisite.
    """
    prepared=fixture.prepare();identifier=prepared['agent']['id'];assert not list(fixture.root.rglob('model-received.json'))
    result=fixture.ok(['agent','reconcile',identifier]);agent=result['agent']
    assert result['cancelled_before_start'] and not result['completion_recovered'] and not result['accepted_by_cancellation']
    # Generated Agent wire omits zero PIDs; omission has that exact Go meaning.
    assert agent['status']=='interrupted' and agent['desired']=='stop' and agent.get('supervisor_pid',0)==agent.get('child_pid',0)==0 and agent['ended']
    assert not list(fixture.root.rglob('model-received.json'))
    publish(fixture.output/'historical-agent-admission.json',dict(agent_id=identifier,cancelled_before_start=True,accepted_by_cancellation=False,provider_started=False,reservation_observed=fixture.reservation(),scope='owned fixture; no manual SQL lifecycle edits'))
    return identifier
