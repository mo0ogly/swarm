"""Pure witness validator for managed-resume quota fixtures (source only).

This module is a dictionary-comparison oracle for the declared fixture
resumption contract: exactly one additional finite global planner activation
and exactly one additional finite activation for one already-existing
selected scope, with every other limit, consumption, setting, task, control,
inbox entry, generation, lease and source-authority field unchanged — plus
the new planning quota audit, ``Work.revision + 1`` and a changed ``updated``
(exact producer: the public event application step; ``apply_quota_change``
updates a private Work without changing revision or updated; the public CLI emits raw Work).

Guarantees:

* Pure comparison of supplied dictionaries only: no IO, no processes, no SQL,
  no model calls, no timers, no dynamic evaluation, and no mutation of any
  input (restoration happens on private deep copies). The validator
  constructs no Work, scope or default quota objects; it only reads and
  compares what callers supply.
* It never grants human or planner authority. ``quota_authorized`` and
  ``runtime_executed`` are always ``False``; any actual budget authority must be established separately outside this
  validator, regardless of the supplied snapshot dictionaries. A supplied dictionary or a saved preview
  is not approval and is not proof that any runtime applied a quota.
* No fixture identifier, no specific Work revision, no fixed reviewer counts
  and no fixed budget pair are hardcoded: the one-step delta is derived from
  the supplied before/after data, request and preview.

Refusals raise ``ValueError`` whose message is exactly one stable code string
(see ``validate_post_quota``); malformed container shapes may additionally
surface as native ``KeyError``/``TypeError`` causes. There is no success
fallback and no executable CLI or runtime driver in this module.
"""

from copy import deepcopy

__all__ = ["validate_post_quota"]


def _go_trim_space(value):
    """Trim the exact protocol whitespace set from engine/src/text.rs.

    U+001C..U+001F and zero-width characters remain significant. The literal
    character set is Go text compatibility, not an operator text-length policy.
    A non-string input raises its original AttributeError; no coercion occurs.
    """
    return value.strip('\t\n\v\f\r \u0085\u00a0\u1680\u2000\u2001\u2002'
                       '\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a'
                       '\u2028\u2029\u202f\u205f\u3000')


def _require_int(value, code):
    """Return ``value`` only when ``type(value) is int``; bools/floats refuse."""
    if type(value) is not int:
        raise ValueError(code)
    return value


def _require_text(value, code):
    """Return ``value`` only when it is a nonempty ``str``."""
    if not isinstance(value, str) or value == "":
        raise ValueError(code)
    return value


def _require_int_or_null_map(mapping, code):
    """Refuse unless ``mapping`` is a dict whose values are ``None`` or ints."""
    if not isinstance(mapping, dict):
        raise ValueError(code)
    for value in mapping.values():
        if value is not None and type(value) is not int:
            raise ValueError(code)


def _compare_named_int_map(actual, expected, code):
    """Compare two flat quota maps field-by-field with strict integer typing.

    A missing key and an explicit ``None`` mean the same unconfigured value;
    every other value must be a genuine ``int`` (bool/float refused) on both
    sides and equal. Raises ``ValueError(code)`` on any difference or type
    violation. Neither mapping is modified.
    """
    if not isinstance(actual, dict) or not isinstance(expected, dict):
        raise ValueError(code)
    for key in set(actual) | set(expected):
        left = actual.get(key)
        right = expected.get(key)
        if left is None and right is None:
            continue
        if type(left) is not int or type(right) is not int or left != right:
            raise ValueError(code)


def _strict_equal(left, right):
    """Type-strict deep equality for JSON-shaped data.

    Unlike plain ``==`` it never coerces: ``True != 1``, ``1 != 1.0``, and
    dict key sets must match exactly. Used for the full-object restoration
    comparisons so numeric-bool drift can never pass as "unchanged".
    """
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left == right
    if isinstance(left, dict) and isinstance(right, dict):
        return set(left) == set(right) and all(
            _strict_equal(left[key], right[key]) for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            _strict_equal(one, other) for one, other in zip(left, right)
        )
    if type(left) is not type(right):
        return False
    return left == right


def _find_scope_index(scopes, scope_id):
    """Return the index of the unique scope entry whose ``id`` is ``scope_id``.

    Raises ``ValueError("quota-scope-missing")`` when the scope list is
    absent or holds no such id, and ``ValueError("quota-scope-ambiguous")``
    when the id appears more than once. Non-dict entries are never matches.
    """
    if scopes is None:
        raise ValueError("quota-scope-missing")
    matches = [
        index
        for index, entry in enumerate(scopes)
        if isinstance(entry, dict) and entry.get("id") == scope_id
    ]
    if not matches:
        raise ValueError("quota-scope-missing")
    if len(matches) > 1:
        raise ValueError("quota-scope-ambiguous")
    return matches[0]


def _work_view_limits(planning):
    """Project the flat QuotaView ``limits`` map from a raw Work planning dict.

    Mirrors the field mapping of the native ``quota_view`` constructor: the
    five planning maxima always, the reviewer context fields only when a
    reviewer exists, and ``judge_cycles`` only when a judge exists. Returns a
    new dict and never touches the input; absent optional structures yield
    ``None`` entries that the typed map comparison treats as unconfigured.
    """
    limits = {
        "planning_activations": planning.get("max_activations"),
        "planning_decisions": planning.get("max_decisions"),
        "planning_tasks": planning.get("max_tasks"),
        # models.rs serializes these scalar i64 fields with is_zero omission.
        # Zero is the wire default here, not a chosen operator policy fallback.
        "planning_scopes": planning.get("max_scopes", 0),
        "delegation_depth": planning.get("max_depth", 0),
    }
    reviewer = planning.get("reviewer")
    if isinstance(reviewer, dict):
        limits["review_calls"] = reviewer.get("max_calls")
        limits["review_context_tokens"] = reviewer.get("input_context_tokens", 0)
        limits["review_context_files"] = reviewer.get("max_context_files")
        limits["review_source_bytes"] = reviewer.get("max_source_bytes")
        limits["review_context_bytes"] = reviewer.get("max_context_bytes")
        limits["review_delivery_files"] = reviewer.get("max_files")
    judge = planning.get("judge")
    if isinstance(judge, dict):
        limits["judge_cycles"] = judge.get("max_cycles")
    return limits


def _refuse_wrapped_work(work):
    """Refuse a Work payload wrapped as ``{"work": {...}}``.

    The real quota apply emits a RAW Work object; a single ``work`` wrapper
    key is a shape this validator must never interpret.
    """
    if isinstance(work, dict) and set(work) == {"work"} and isinstance(work["work"], dict):
        raise ValueError("quota-work-wrapper")


def validate_post_quota(before_work, after_work, request, preview, scope_id):
    """Validate a post-quota resume witness using only the supplied dictionaries.

    Pure comparison oracle for the declared fixture resumption contract. It
    never applies, authorizes, observes or executes anything, never touches
    the filesystem, network or clock, never mutates its inputs, and never
    grants human or planner authority: a returned dictionary — like a saved
    preview — is not approval and is not proof that any runtime applied a
    quota.

    Parameters
    ----------
    before_work : dict
        Raw Work snapshot before the change (never ``{"work": ...}``).
    after_work : dict
        Raw Work snapshot after ``apply_quota_change`` plus public event
        publication (raw Work again; private candidate update changes
        neither ``revision`` nor ``updated`` — the public event application,
        the exact producer, performs ``revision + 1`` and a new ``updated``).
    request : dict
        The decoded QuotaChange dictionary.
    preview : dict
        Saved ``QuotaPreview{settings_revision, before, after}`` whose
        ``before``/``after`` are full QuotaView dictionaries with flat
        all-field ``limits`` maps (not the nested Work planning shape).
    scope_id : str
        The single selected, already-existing scope.

    Accepted contract
    -----------------
    * Request: ``schema_version == 1``; ``expected_revision`` equal to
      ``before_work["revision"]``; nonblank reason; ``scope_activations``
      containing exactly ``scope_id``; no ``review_timeout_seconds``
      alteration (key absent, or re-stating the saved value).
    * Exactly one additional finite global planner activation: the Work
      ``planning.max_activations`` and the preview limit/remaining pair each
      gain exactly ``+1``; every other limit field, all consumption, all
      remaining capacity, bounds, reviewer/judge settings, review timeout,
      manifest flags, tasks, controls, inbox, generations, leases and source
      authority are unchanged.
    * Exactly one additional finite activation for the selected
      already-existing scope: its ``max_activations`` gains exactly ``+1`` in
      the Work and the preview; every other scope field and every other
      scope is unchanged.
    * The new ``planning.quota_authorization`` audit matches the structural data of the preview's
      ``after.last_authorization`` and carries the preview's full before and
      after limit maps, the selected before/after scope-limit maps for
      exactly ``scope_id``, a reason equal to the safely trimmed request
      reason, and nonblank actor/at strings. Actor/at are unverified local
      audit data — never authenticated human identity. Apply observes its own
      actor/time; these may differ from the historical preview.
    * Work chain: ``after.revision == before.revision + 1`` and a changed
      nonempty ``updated``; restoring ONLY the known deltas (the two +1
      limits, the new audit, ``revision``, ``updated``) on a private deep
      copy reproduces ``before_work`` exactly — any unexplained difference
      refuses.

    Returns
    -------
    dict
        Descriptive result in which ``quota_authorized`` and
        ``runtime_executed`` are always ``False`` and
        ``external_authority_not_established`` is ``True``: these supplied
        dictionaries cannot establish actual permission or an applied event.

    Raises
    ------
    ValueError
        Message is exactly one stable code: ``quota-work-wrapper``,
        ``quota-request-invalid``, ``quota-revision-conflict``,
        ``quota-revision-delta-invalid``, ``quota-updated-invalid``,
        ``quota-updated-unchanged``, ``quota-planning-required``,
        ``quota-preview-invalid``, ``quota-limit-delta-invalid``,
        ``quota-scope-delta-invalid``, ``quota-scope-missing``,
        ``quota-scope-ambiguous``, ``quota-audit-mismatch`` or
        ``quota-unexplained-diff``, ``quota-consumption-mismatch`` or
        ``quota-remaining-mismatch``. Missing required keys or non-dict
        containers may instead surface as native ``KeyError``/``TypeError``
        causes. Booleans and floats are refused wherever an integer is
        required (``type(x) is int``); there is no success fallback.
    """
    _refuse_wrapped_work(before_work)
    _refuse_wrapped_work(after_work)
    _require_text(scope_id, "quota-request-invalid")

    before_planning = before_work["planning"]
    after_planning = after_work["planning"]
    if not isinstance(before_planning, dict) or not isinstance(after_planning, dict):
        raise ValueError("quota-planning-required")

    # -- request contract ---------------------------------------------------
    if _require_int(request["schema_version"], "quota-request-invalid") != 1:
        raise ValueError("quota-request-invalid")
    raw_reason = request["reason"]
    if not isinstance(raw_reason, str) or _go_trim_space(raw_reason) == "":
        raise ValueError("quota-request-invalid")
    trimmed_reason = _go_trim_space(raw_reason)
    expected_revision = _require_int(request["expected_revision"], "quota-revision-conflict")
    scope_activations = request["scope_activations"]
    if not isinstance(scope_activations, dict) or set(scope_activations) != {scope_id}:
        raise ValueError("quota-request-invalid")
    requested_scope_limit = _require_int(scope_activations[scope_id], "quota-request-invalid")
    request_limits = request["limits"]
    if not isinstance(request_limits, dict):
        raise ValueError("quota-request-invalid")
    if "review_timeout_seconds" in request:
        saved_reviewer = before_planning.get("reviewer")
        saved_timeout = (
            saved_reviewer.get("timeout_seconds", 0)
            if isinstance(saved_reviewer, dict)
            else None
        )
        if _require_int(
            request["review_timeout_seconds"], "quota-request-invalid"
        ) != _require_int(saved_timeout, "quota-request-invalid"):
            raise ValueError("quota-request-invalid")

    # -- Work-level known deltas --------------------------------------------
    before_revision = _require_int(before_work["revision"], "quota-revision-conflict")
    if expected_revision != before_revision:
        raise ValueError("quota-revision-conflict")
    after_revision = _require_int(after_work["revision"], "quota-revision-delta-invalid")
    if after_revision != before_revision + 1:
        raise ValueError("quota-revision-delta-invalid")
    before_updated = _require_text(before_work["updated"], "quota-updated-invalid")
    after_updated = _require_text(after_work["updated"], "quota-updated-invalid")
    if after_updated == before_updated:
        raise ValueError("quota-updated-unchanged")

    before_activation_limit = _require_int(
        before_planning["max_activations"], "quota-limit-delta-invalid"
    )
    after_activation_limit = _require_int(
        after_planning["max_activations"], "quota-limit-delta-invalid"
    )
    if before_activation_limit <= 0 or after_activation_limit != before_activation_limit + 1:
        raise ValueError("quota-limit-delta-invalid")

    before_scope_index = _find_scope_index(before_planning.get("scopes"), scope_id)
    after_scope_index = _find_scope_index(after_planning.get("scopes"), scope_id)
    before_scope_limit = _require_int(
        before_planning["scopes"][before_scope_index]["max_activations"],
        "quota-scope-delta-invalid",
    )
    after_scope_limit = _require_int(
        after_planning["scopes"][after_scope_index]["max_activations"],
        "quota-scope-delta-invalid",
    )
    if before_scope_limit <= 0 or after_scope_limit != before_scope_limit + 1:
        raise ValueError("quota-scope-delta-invalid")
    if requested_scope_limit != after_scope_limit:
        raise ValueError("quota-scope-delta-invalid")

    # -- saved preview: QuotaPreview{settings_revision, before, after} -------
    settings_revision = preview["settings_revision"]
    if not isinstance(settings_revision, str) or settings_revision == "":
        raise ValueError("quota-preview-invalid")
    preview_before = preview["before"]
    preview_after = preview["after"]
    if not isinstance(preview_before, dict) or not isinstance(preview_after, dict):
        raise ValueError("quota-preview-invalid")
    if _require_int(preview_before["revision"], "quota-preview-invalid") != expected_revision:
        raise ValueError("quota-preview-invalid")
    if _require_int(preview_after["revision"], "quota-preview-invalid") != expected_revision:
        raise ValueError("quota-preview-invalid")

    preview_before_limits = preview_before["limits"]
    preview_after_limits = preview_after["limits"]
    if not isinstance(preview_before_limits, dict) or not isinstance(preview_after_limits, dict):
        raise ValueError("quota-preview-invalid")
    preview_before_activation_limit = _require_int(
        preview_before_limits["planning_activations"], "quota-limit-delta-invalid"
    )
    preview_after_activation_limit = _require_int(
        preview_after_limits["planning_activations"], "quota-limit-delta-invalid"
    )
    if preview_after_activation_limit != preview_before_activation_limit + 1:
        raise ValueError("quota-limit-delta-invalid")
    preview_before_remaining_activations = _require_int(
        preview_before["remaining"]["planning_activations"], "quota-limit-delta-invalid"
    )
    preview_after_remaining_activations = _require_int(
        preview_after["remaining"]["planning_activations"], "quota-limit-delta-invalid"
    )
    if preview_after_remaining_activations != preview_before_remaining_activations + 1:
        raise ValueError("quota-limit-delta-invalid")

    # ties: the saved preview must describe exactly these Work snapshots
    if preview_before_activation_limit != before_activation_limit:
        raise ValueError("quota-preview-invalid")
    if preview_after_activation_limit != after_activation_limit:
        raise ValueError("quota-preview-invalid")
    preview_before_scope_index = _find_scope_index(preview_before.get("scopes"), scope_id)
    preview_after_scope_index = _find_scope_index(preview_after.get("scopes"), scope_id)
    preview_before_scope_limit = _require_int(
        preview_before["scopes"][preview_before_scope_index]["max_activations"],
        "quota-scope-delta-invalid",
    )
    preview_after_scope_limit = _require_int(
        preview_after["scopes"][preview_after_scope_index]["max_activations"],
        "quota-scope-delta-invalid",
    )
    if preview_after_scope_limit != preview_before_scope_limit + 1:
        raise ValueError("quota-scope-delta-invalid")
    if preview_before_scope_limit != before_scope_limit:
        raise ValueError("quota-preview-invalid")

    _compare_named_int_map(
        preview_before_limits, _work_view_limits(before_planning), "quota-preview-invalid"
    )
    _compare_named_int_map(
        preview_after_limits, _work_view_limits(after_planning), "quota-preview-invalid"
    )
    _compare_named_int_map(request_limits, preview_after_limits, "quota-request-invalid")

    for view in (preview_before, preview_after):
        _require_int_or_null_map(view["consumed"], "quota-preview-invalid")
        _require_int_or_null_map(view["remaining"], "quota-preview-invalid")
        _require_int_or_null_map(view["bounds"], "quota-preview-invalid")

    # QuotaView.scopes is the same serialized scope vector as Work.planning.
    # Equal before/after fabricated values alone do not establish a Work binding.
    for view, planning in ((preview_before, before_planning),
                           (preview_after, after_planning)):
        if not _strict_equal(view['scopes'], planning['scopes']):
            raise ValueError('quota-preview-invalid')
        used = {'planning_activations': planning['activations'],
                'planning_decisions': planning['decisions']}
        if isinstance(planning.get('reviewer'), dict):
            used['review_calls'] = planning['reviewer']['calls']
        for name, count in used.items():
            if (type(count) is not int or type(view['consumed'].get(name)) is not int
                    or view['consumed'][name] != count):
                raise ValueError('quota-consumption-mismatch')
        # Same saturating remainder as quotas.rs::quota_view, on strict ints.
        if view['remaining']['planning_activations'] != max(
                planning['max_activations'] - planning['activations'], 0):
            raise ValueError('quota-remaining-mismatch')

    preview_before_authorization = preview_before.get("last_authorization")
    if not _strict_equal(
        preview_before_authorization, before_planning.get("quota_authorization")
    ):
        raise ValueError("quota-preview-invalid")
    preview_after_authorization = preview_after.get("last_authorization")
    if not isinstance(preview_after_authorization, dict):
        raise ValueError("quota-preview-invalid")

    # -- saved audit versus preview, request and actual Work -----------------
    audit = after_planning.get("quota_authorization")
    if not isinstance(audit, dict) or not _strict_equal(
        {key: value for key, value in audit.items() if key not in ('actor', 'at')},
        {key: value for key, value in preview_after_authorization.items()
         if key not in ('actor', 'at')},
    ):
        raise ValueError("quota-audit-mismatch")
    _compare_named_int_map(
        preview_after_authorization["before"], preview_before_limits, "quota-audit-mismatch"
    )
    _compare_named_int_map(
        preview_after_authorization["after"], preview_after_limits, "quota-audit-mismatch"
    )
    for audit_map, expected_map in (
        (
            preview_after_authorization["before_scope_activations"],
            {scope_id: before_scope_limit},
        ),
        (
            preview_after_authorization["after_scope_activations"],
            {scope_id: after_scope_limit},
        ),
    ):
        if not isinstance(audit_map, dict):
            raise ValueError("quota-audit-mismatch")
        for value in audit_map.values():
            _require_int(value, "quota-audit-mismatch")
        if audit_map != expected_map:
            raise ValueError("quota-audit-mismatch")
    actor = audit["actor"]
    at = audit["at"]
    audit_reason = preview_after_authorization["reason"]
    if not isinstance(actor, str) or _go_trim_space(actor) == "":
        raise ValueError("quota-audit-mismatch")
    if not isinstance(at, str) or _go_trim_space(at) == "":
        raise ValueError("quota-audit-mismatch")
    if not isinstance(audit_reason, str) or audit_reason != trimmed_reason:
        raise ValueError("quota-audit-mismatch")

    # -- preview restoration: undo exactly the known deltas, then compare ----
    restored_preview = deepcopy(preview_after)
    restored_preview["limits"]["planning_activations"] = preview_before_activation_limit
    restored_preview["remaining"]["planning_activations"] = (
        preview_before_remaining_activations
    )
    restored_preview["scopes"][preview_after_scope_index]["max_activations"] = (
        preview_before_scope_limit
    )
    restored_preview["last_authorization"] = deepcopy(preview_before_authorization)
    if not _strict_equal(restored_preview, preview_before):
        raise ValueError("quota-preview-invalid")

    # -- Work restoration: undo exactly the known deltas, then compare -------
    restored_work = deepcopy(after_work)
    restored_work["revision"] = before_revision
    restored_work["updated"] = before_updated
    restored_planning = restored_work["planning"]
    restored_planning["max_activations"] = before_activation_limit
    restored_planning["scopes"][after_scope_index]["max_activations"] = before_scope_limit
    before_authorization = before_planning.get("quota_authorization")
    if before_authorization is None and "quota_authorization" not in before_planning:
        restored_planning.pop("quota_authorization", None)
    else:
        restored_planning["quota_authorization"] = deepcopy(before_authorization)
    if not _strict_equal(restored_work, before_work):
        raise ValueError("quota-unexplained-diff")

    return {
        "quota_authorized": False,
        "runtime_executed": False,
        "external_authority_not_established": True,
        "quota_application_allowed": False,
        "validated": True,
        "scope_id": scope_id,
        "settings_revision": settings_revision,
        "work_revision": {
            "before": before_revision,
            "after": after_revision,
            "delta": after_revision - before_revision,
        },
        "planning_activations_limit": {
            "before": before_activation_limit,
            "after": after_activation_limit,
            "delta": after_activation_limit - before_activation_limit,
        },
        "selected_scope_max_activations": {
            "before": before_scope_limit,
            "after": after_scope_limit,
            "delta": after_scope_limit - before_scope_limit,
        },
        "preview_consumed_identical": True,
        "reviewer_limits_unchanged": True,
        "audit_actor": actor,
        "audit_at": at,
        "audit_identity_verified": False,
        "checks_passed": (
            "request-shape",
            "revision-chain",
            "one-step-global-limit-delta",
            "one-step-scope-limit-delta",
            "preview-before-after-consistency",
            "preview-work-projection-ties",
            "audit-preview-request-consistency",
            "work-restoration",
        ),
        "notes": (
            "Witness is consistent with exactly one additional global planner "
            "activation and one additional activation for the selected "
            "already-existing scope; nothing else changed.",
            "This validator establishes no human budget authority and grants "
            "no permission to apply a quota.",
            "A supplied dictionary or a saved preview is not approval and is "
            "not proof that any runtime applied a quota.",
            "audit_actor/audit_at are unverified local audit data, never "
            "authenticated human identity.",
        ),
    }
