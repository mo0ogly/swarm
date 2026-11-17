"""Pure custody helper for trace witness outcome retention.

One public raising function: it re-raises a following failure while carrying
the already-observed tracer/primary evidence supplied by its caller. No I/O,
no subprocess work, no parsing, no retries, no controller: every retained
fact is a caller-supplied JSON-safe snapshot or a document rendered by the
unchanged native_build_outcome.failure_document.
"""
from native_build_outcome import failure_document

__all__ = ['raise_with_custody']

_SCOPE = 'verification-trace-custody'
_GROUP_MESSAGE = ('verification-trace-custody: following failure retained '
                  'alongside distinct earlier failure')


def raise_with_custody(later, *, tracer_result, native_receipt=None,
                       primary_observation=None, earlier_failure=None):
    """Raise ``later`` while carrying custody of already-observed facts.

    Pure raising helper for the trace witness consumers: it never returns,
    never retries and never suppresses; it performs no I/O, no subprocess
    work and no environment inspection. Every retained fact is a
    caller-supplied snapshot or a document rendered by the unchanged
    ``native_build_outcome.failure_document``.

    Caller-trusted, JSON-safe inputs:
      later: failure observed after the executor call (validation refusal,
        interruption, ...); must be a BaseException instance.
      tracer_result: dict of actually known numeric executor facts (e.g.
        returncode/signal/execution_attempted) cached from a direct executor
        return or from its error carrier. Shallow copied into the carrier as
        'subprocess'. None or an empty dict becomes the explicit unknown
        marker ``{'known': False, 'reason': 'tracer-result-not-cached'}`` --
        never 0, never False; no inference that tracer 0 implies primary 0.
      native_receipt: complete original executor details when available,
        else None; shallow copied as 'native_executor_receipt' (None when
        unknown).
      primary_observation: caller-owned JSON-safe facts recorded only after
        an exact trace comparison. None or empty becomes the explicit
        unknown marker ``{'known': False, 'reason': 'primary-not-verified'}``;
        primary facts stay UNKNOWN until actually verified. No timestamp,
        PID or publication status is invented here.
      earlier_failure: distinct pre-existing failure (for example the native
        executor ValueError) or None.

    Carrier and metadata mutation: ``failure_document(later)`` is snapshotted
    BEFORE any mutation, so a pre-existing ``later.native_outcome`` is
    preserved inside the carrier's 'following_failure' document and no
    self-reference or cycle is created. ``later.native_outcome`` is then
    replaced by the carrier dict with keys 'scope'
    (``'verification-trace-custody'``), 'subprocess',
    'native_executor_receipt', 'primary_observation', 'following_failure'
    and, when given, 'earlier_failure' (a failure_document of the earlier
    failure). No compiler or behavior admission is granted. Supplied dicts
    are trusted to be JSON-safe and are not re-verified.

    Raise forms (original exception objects keep their identity; existing
    cause chains are never silently erased):
      - earlier_failure is None or the same object as later: ``raise later``
        with no new cause.
      - both are ordinary Exceptions and later has no distinct pre-existing
        cause or unsuppressed context: ``raise later from earlier``.
      - an interruption (BaseException that is not an Exception) participates
        with a distinct earlier failure, or chaining would silently erase a
        distinct pre-existing cause of later: a ``BaseExceptionGroup`` of
        both original objects is raised with the carrier attached to the
        group (and to later). Interruptions are never downgraded to ordinary
        exceptions or success.

    Errors and limits: TypeError if later/earlier_failure is not a
    BaseException instance, or if tracer_result/native_receipt/
    primary_observation is neither None nor a dict. Side effects are limited
    to setting ``native_outcome`` on later and on the newly built group. No
    OS confinement, sandboxing or execution authority is provided or
    claimed.
    """
    if not isinstance(later, BaseException):
        raise TypeError('later must be a BaseException instance')
    if earlier_failure is not None and not isinstance(earlier_failure, BaseException):
        raise TypeError('earlier_failure must be a BaseException instance or None')
    for name, value in (('tracer_result', tracer_result),
                        ('native_receipt', native_receipt),
                        ('primary_observation', primary_observation)):
        if value is not None and not isinstance(value, dict):
            raise TypeError(name + ' must be a dict or None')

    earlier = earlier_failure
    subprocess_facts = dict(tracer_result) if tracer_result else {
        'known': False, 'reason': 'tracer-result-not-cached'}
    primary_facts = dict(primary_observation) if primary_observation else {
        'known': False, 'reason': 'primary-not-verified'}

    # Snapshot before any mutation so the document cannot contain the carrier.
    carrier = {'scope': _SCOPE,
               'subprocess': subprocess_facts,
               'native_executor_receipt': (None if native_receipt is None
                                           else dict(native_receipt)),
               'primary_observation': primary_facts,
               'following_failure': failure_document(later)}
    if earlier is not None:
        carrier['earlier_failure'] = failure_document(earlier)

    later.native_outcome = carrier

    if earlier is not None and earlier is not later:
        interruption = (not isinstance(later, Exception)
                        or not isinstance(earlier, Exception))
        existing = later.__cause__
        if existing is None and not later.__suppress_context__:
            existing = later.__context__
        if interruption or (existing is not None and existing is not earlier):
            group = BaseExceptionGroup(_GROUP_MESSAGE, [later, earlier])
            group.native_outcome = carrier
            raise group
        raise later from earlier
    raise later
