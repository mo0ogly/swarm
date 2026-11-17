"""Synthetic unit tests for ``rust_managed_resume_quota.validate_post_quota``.

All fixtures are built in memory here; no repository fixture file, no IO, no
subprocess and no model is used. Every forbidden-mutation test first builds a
fresh witness the validator accepts, then applies exactly one mutation and
asserts the stable refusal code. These tests exercise dictionary comparison
only: they cannot and do not prove that any runtime applied a quota, and they
grant no permission and demonstrate no human approval. The synthetic witness
deliberately uses budget numbers different from the real fixture (global
2->3, selected scope 1->2, revision 5->6) to show the one-step delta is
derived, never hardcoded.
"""

from copy import deepcopy

import pytest

import rust_managed_resume_quota as _module

validate_post_quota = _module.validate_post_quota

SCOPE_ID = "child"
REASON = "  synthetic resume witness reason  "
TRIMMED_REASON = "synthetic resume witness reason"


def _limits(planning_activations):
    """Flat all-field QuotaView limits map for the synthetic planning state."""
    return {
        "planning_activations": planning_activations,
        "planning_decisions": 3,
        "planning_tasks": 2,
        "planning_scopes": 0,
        "delegation_depth": 0,
        "judge_cycles": 0,
        "review_calls": 2,
        "review_context_tokens": 0,
        "review_context_files": 24,
        "review_source_bytes": 98304,
        "review_context_bytes": 131072,
        "review_delivery_files": 100,
    }


def _work(revision, updated, activation_limit, scope_limit, authorization):
    """Raw synthetic Work snapshot with one root and one child scope."""
    return {
        "revision": revision,
        "updated": updated,
        "tasks": [],
        "inbox": [],
        "leases": {},
        "planning": {
            "max_activations": activation_limit,
            "max_decisions": 3,
            "max_tasks": 2,
            "max_scopes": 0,
            "max_depth": 0,
            "activations": 2,
            "decisions": 1,
            "quota_authorization": authorization,
            "reviewer": {
                "max_calls": 2,
                "calls": 1,
                "input_context_tokens": 0,
                "max_context_files": 24,
                "max_source_bytes": 98304,
                "max_context_bytes": 131072,
                "max_files": 100,
                "timeout_seconds": 0,
            },
            "judge": {"max_cycles": 0, "cycle": 1},
            "scopes": [
                {
                    "id": "root",
                    "parent": "",
                    "max_activations": 2,
                    "activations": 2,
                    "generation": 4,
                    "state": "ready",
                },
                {
                    "id": SCOPE_ID,
                    "parent": "root",
                    "max_activations": scope_limit,
                    "activations": 1,
                    "generation": 2,
                    "state": "ready",
                },
            ],
        },
    }


def _audit(limits_before, limits_after, scope_before, scope_after):
    """QuotaAuthorization audit with trimmed reason and full limit maps."""
    return {
        "before_scope_activations": {SCOPE_ID: scope_before},
        "after_scope_activations": {SCOPE_ID: scope_after},
        "before": deepcopy(limits_before),
        "after": deepcopy(limits_after),
        "actor": "local-fixture-operator",
        "at": "2026-01-01T01:02:03Z",
        "reason": TRIMMED_REASON,
    }


def _view(revision, limits, remaining_activations, scopes, authorization):
    """QuotaView-shaped dictionary for the synthetic saved preview."""
    return {
        "revision": revision,
        "limits": deepcopy(limits),
        "consumed": {
            "planning_activations": 2,
            "planning_decisions": 1,
            "planning_tasks": 0,
            "planning_scopes": 2,
            "delegation_depth": 2,
            "review_calls": 1,
            "judge_cycles": 1,
        },
        "remaining": {
            "planning_activations": remaining_activations,
            "planning_decisions": 2,
            "planning_tasks": 2,
            "planning_scopes": None,
            "delegation_depth": None,
            "review_calls": 1,
            "judge_cycles": None,
        },
        "bounds": {
            "planning_activations": 0,
            "planning_decisions": 0,
            "review_calls": None,
        },
        "last_authorization": deepcopy(authorization),
        "scopes": scopes,
        "review_manifest_transport_bytes": None,
        "review_manifest_supported": False,
        "context_window_override_supported": False,
        "review_timeout_seconds": 0,
    }


def _witness():
    """Build a fully valid synthetic (before, after, request, preview) tuple."""
    before_limits = _limits(2)
    after_limits = _limits(3)
    audit = _audit(before_limits, after_limits, scope_before=1, scope_after=2)
    before = _work(
        revision=5,
        updated="2026-01-01T00:00:00Z",
        activation_limit=2,
        scope_limit=1,
        authorization=None,
    )
    after = _work(
        revision=6,
        updated="2026-01-01T00:05:00Z",
        activation_limit=3,
        scope_limit=2,
        authorization=deepcopy(audit),
    )
    request = {
        "schema_version": 1,
        "event_id": "synthetic-resume-witness-0001",
        "expected_revision": 5,
        "scope_activations": {SCOPE_ID: 2},
        "limits": deepcopy(after_limits),
        "reason": REASON,
    }
    preview = {
        "settings_revision": "1",
        "before": _view(
            revision=5,
            limits=before_limits,
            remaining_activations=0,
            scopes=deepcopy(before["planning"]["scopes"]),
            authorization=None,
        ),
        "after": _view(
            revision=5,
            limits=after_limits,
            remaining_activations=1,
            scopes=deepcopy(after["planning"]["scopes"]),
            authorization=deepcopy(audit),
        ),
    }
    return before, after, request, preview


def _refusal_code(before, after, request, preview, scope_id=SCOPE_ID):
    """Call the validator and return its stable ValueError code string."""
    with pytest.raises(ValueError) as excinfo:
        validate_post_quota(before, after, request, preview, scope_id)
    return str(excinfo.value)


def test_valid_witness_is_accepted_and_reports_pending_quotas():
    """Check valid witness is accepted and reports pending quotas on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    result = validate_post_quota(before, after, request, preview, SCOPE_ID)
    assert result["quota_authorized"] is False
    assert result["runtime_executed"] is False
    assert result["external_authority_not_established"] is True
    assert result["quota_application_allowed"] is False
    assert result["validated"] is True
    assert result["scope_id"] == SCOPE_ID
    assert result["work_revision"] == {"before": 5, "after": 6, "delta": 1}
    assert result["planning_activations_limit"] == {"before": 2, "after": 3, "delta": 1}
    assert result["selected_scope_max_activations"] == {"before": 1, "after": 2, "delta": 1}
    assert result["preview_consumed_identical"] is True
    assert result["reviewer_limits_unchanged"] is True


def test_result_dictionary_is_not_approval_or_runtime_proof():
    """Check result dictionary is not approval or runtime proof on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    result = validate_post_quota(before, after, request, preview, SCOPE_ID)
    assert result["audit_identity_verified"] is False
    assert isinstance(result["notes"], tuple) and result["notes"]
    joined = " ".join(result["notes"])
    assert "not approval" in joined
    assert "establishes no human budget authority" in joined


def test_restoration_of_known_deltas_reproduces_before_work():
    """Check restoration of known deltas reproduces before work on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    restored = deepcopy(after)
    restored["revision"] = before["revision"]
    restored["updated"] = before["updated"]
    restored["planning"]["max_activations"] = before["planning"]["max_activations"]
    for scope in restored["planning"]["scopes"]:
        if scope["id"] == SCOPE_ID:
            scope["max_activations"] = 1
    restored["planning"]["quota_authorization"] = before["planning"]["quota_authorization"]
    assert restored == before  # only the known deltas separate after from before
    result = validate_post_quota(before, after, request, preview, SCOPE_ID)
    assert result["validated"] is True


def test_inputs_are_not_mutated_on_success_or_refusal():
    """Check inputs are not mutated on success or refusal on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    snapshot = (deepcopy(before), deepcopy(after), deepcopy(request), deepcopy(preview))
    validate_post_quota(before, after, request, preview, SCOPE_ID)
    assert (before, after, request, preview) == snapshot
    broken = deepcopy(after)
    broken["planning"]["activations"] = 0  # consumed reset on a private copy
    with pytest.raises(ValueError):
        validate_post_quota(before, broken, request, preview, SCOPE_ID)
    assert (before, after, request, preview) == snapshot


def test_unexplained_work_changes_are_refused():
    """Check unexplained work changes are refused on inert dictionaries; no runtime authority."""
    def reset_consumption(work):
        work["planning"]["activations"] = 0

    def bump_generation(work):
        work["planning"]["scopes"][1]["generation"] = 3

    def add_task(work):
        work["tasks"].append({"id": "task-1"})

    def add_inbox_event(work):
        work["inbox"].append("evt-1")

    def add_lease(work):
        work["leases"]["holder"] = "scope-a"

    def plant_extra_field(work):
        work["planning"]["legacy_flag"] = True

    def grant_other_scope(work):
        work["planning"]["scopes"][0]["max_activations"] = 9

    for mutate in (
        reset_consumption,
        bump_generation,
        add_task,
        add_inbox_event,
        add_lease,
        plant_extra_field,
        grant_other_scope,
    ):
        before, after, request, preview = _witness()
        assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
        mutate(after)
        assert _refusal_code(before, after, request, preview) == (
            "quota-consumption-mismatch" if mutate is reset_consumption
            else "quota-preview-invalid" if mutate in (bump_generation, grant_other_scope)
            else "quota-unexplained-diff")


def test_extra_global_limit_change_is_refused_via_preview_tie():
    """Check extra global limit change is refused via preview tie on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
    after["planning"]["max_decisions"] = 4  # extra quota beyond the one-step oracle
    assert _refusal_code(before, after, request, preview) == "quota-preview-invalid"


def test_preview_changes_are_refused():
    """Check preview changes are refused on inert dictionaries; no runtime authority."""
    def reset_consumed(preview_obj):
        preview_obj["after"]["consumed"]["planning_activations"] = 0

    def flip_control_flag(preview_obj):
        preview_obj["after"]["review_manifest_supported"] = True

    def blank_settings_revision(preview_obj):
        preview_obj["settings_revision"] = ""

    def change_reviewer_limit(preview_obj):
        preview_obj["after"]["limits"]["review_calls"] = 5

    def change_timeout_setting(preview_obj):
        preview_obj["after"]["review_timeout_seconds"] = 30

    for mutate in (
        reset_consumed,
        flip_control_flag,
        blank_settings_revision,
        change_reviewer_limit,
        change_timeout_setting,
    ):
        before, after, request, preview = _witness()
        assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
        mutate(preview)
        assert _refusal_code(before, after, request, preview) == (
            "quota-consumption-mismatch" if mutate is reset_consumed else "quota-preview-invalid")


def test_preview_missing_selected_scope_is_refused():
    """Check preview missing selected scope is refused on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
    preview["before"]["scopes"] = [preview["before"]["scopes"][0]]
    assert _refusal_code(before, after, request, preview) == "quota-scope-missing"


def test_request_rejections_are_refused():
    """Check request rejections are refused on inert dictionaries; no runtime authority."""
    def bool_schema_version(req):
        req["schema_version"] = True

    def bool_scope_activation(req):
        req["scope_activations"] = {SCOPE_ID: True}

    def extra_scope_key(req):
        req["scope_activations"] = {SCOPE_ID: 2, "root": 9}

    def timeout_alteration(req):
        req["review_timeout_seconds"] = 5

    def bool_limit_value(req):
        req["limits"]["planning_activations"] = True

    def float_limit_value(req):
        req["limits"]["planning_activations"] = 3.0

    for mutate in (
        bool_schema_version,
        bool_scope_activation,
        extra_scope_key,
        timeout_alteration,
        bool_limit_value,
        float_limit_value,
    ):
        before, after, request, preview = _witness()
        assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
        mutate(request)
        assert _refusal_code(before, after, request, preview) == "quota-request-invalid"


def test_revision_contract_rejections():
    """Check revision contract rejections on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
    request["expected_revision"] = 4
    assert _refusal_code(before, after, request, preview) == "quota-revision-conflict"

    before, after, request, preview = _witness()
    after["revision"] = 7  # event publication must add exactly one revision
    assert _refusal_code(before, after, request, preview) == "quota-revision-delta-invalid"

    before, after, request, preview = _witness()
    after["updated"] = before["updated"]  # public event application must change updated
    assert _refusal_code(before, after, request, preview) == "quota-updated-unchanged"


def test_audit_mismatches_are_refused():
    """Check audit mismatches are refused on inert dictionaries; no runtime authority."""
    def change_reason(work, preview_obj):
        work["planning"]["quota_authorization"]["reason"] = "other reason"

    def blank_actor(work, preview_obj):
        work["planning"]["quota_authorization"]["actor"] = ""
        preview_obj["after"]["last_authorization"]["actor"] = ""

    def rewrite_before_limits(work, preview_obj):
        work["planning"]["quota_authorization"]["before"]["planning_decisions"] = 9
        preview_obj["after"]["last_authorization"]["before"]["planning_decisions"] = 9

    def extra_scope_entry(work, preview_obj):
        work["planning"]["quota_authorization"]["before_scope_activations"]["root"] = 2
        preview_obj["after"]["last_authorization"]["before_scope_activations"]["root"] = 2

    def drop_audit(work, preview_obj):
        work["planning"]["quota_authorization"] = None

    for mutate in (
        change_reason,
        blank_actor,
        rewrite_before_limits,
        extra_scope_entry,
        drop_audit,
    ):
        before, after, request, preview = _witness()
        assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
        mutate(after, preview)
        assert _refusal_code(before, after, request, preview) == "quota-audit-mismatch"


def test_scope_activation_delta_must_be_exactly_one():
    # A fully consistent +2 (request, preview, audit and Work all agree).
    """Check scope activation delta must be exactly one on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)["validated"] is True
    after["planning"]["scopes"][1]["max_activations"] = 3
    request["scope_activations"][SCOPE_ID] = 3
    preview["after"]["scopes"][1]["max_activations"] = 3
    preview["after"]["last_authorization"]["after_scope_activations"][SCOPE_ID] = 3
    after["planning"]["quota_authorization"]["after_scope_activations"][SCOPE_ID] = 3
    assert _refusal_code(before, after, request, preview) == "quota-scope-delta-invalid"

    # The request value must equal the value actually applied to the Work.
    before, after, request, preview = _witness()
    request["scope_activations"][SCOPE_ID] = 5
    assert _refusal_code(before, after, request, preview) == "quota-scope-delta-invalid"


def test_unknown_scope_id_is_refused():
    """Check unknown scope id is refused on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    request["scope_activations"] = {"ghost": 2}
    assert (
        _refusal_code(before, after, request, preview, scope_id="ghost")
        == "quota-scope-missing"
    )


def test_wrapped_work_payload_is_refused():
    """Check wrapped work payload is refused on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    assert _refusal_code(before, {"work": after}, request, preview) == "quota-work-wrapper"
    assert _refusal_code({"work": before}, after, request, preview) == "quota-work-wrapper"


def test_restating_saved_review_timeout_is_not_an_alteration():
    """Check restating saved review timeout is not an alteration on inert dictionaries; no runtime authority."""
    before, after, request, preview = _witness()
    request["review_timeout_seconds"] = 0  # identical to the saved value: no alteration
    result = validate_post_quota(before, after, request, preview, SCOPE_ID)
    assert result["validated"] is True
    assert result["quota_authorized"] is False
    assert result["runtime_executed"] is False


def test_zero_scalars_omitted_by_model_serialization_remain_valid():
    """Removing omitted scalar zeros must preserve a valid supplied witness."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    for work in (before, after):
        for key in ('max_scopes', 'max_depth'):
            del work['planning'][key]
        del work['planning']['reviewer']['input_context_tokens']
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']


def test_public_apply_audit_time_is_distinct_from_preview_time():
    """Apply observes its own audit metadata; an old preview timestamp is no pin."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    after['planning']['quota_authorization']['at'] = '2026-01-01T01:03:04Z'
    after['planning']['quota_authorization']['actor'] = 'another-unverified-local-actor'
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']


def test_unchanged_preview_consumption_must_match_work():
    """Equal fabricated consumed values cannot contradict both supplied Works."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    for view in (preview['before'], preview['after']):
        view['consumed']['planning_activations'] = 0
    assert _refusal_code(before, after, request, preview) == 'quota-consumption-mismatch'


def test_inherited_scope_zero_is_not_one_more_finite_activation():
    """A zero inherited cap cannot be called an increase to one finite call."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    before['planning']['scopes'][1]['max_activations'] = 0
    after['planning']['scopes'][1]['max_activations'] = 1
    preview['before']['scopes'][1]['max_activations'] = 0
    preview['after']['scopes'][1]['max_activations'] = 1
    for audit in (after['planning']['quota_authorization'], preview['after']['last_authorization']):
        audit['before_scope_activations'][SCOPE_ID] = 0
        audit['after_scope_activations'][SCOPE_ID] = 1
    request['scope_activations'][SCOPE_ID] = 1
    assert _refusal_code(before, after, request, preview) == 'quota-scope-delta-invalid'


def test_omitted_zero_timeout_can_be_restated_without_change():
    """The serialized omission of timeout_seconds means the native scalar zero."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    del before['planning']['reviewer']['timeout_seconds']
    del after['planning']['reviewer']['timeout_seconds']
    request['review_timeout_seconds'] = 0
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']


def test_reason_trimming_preserves_go_non_whitespace_separator():
    """Go trim_space keeps U+001C; Python's broader default strip must not erase it."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    text = '\x1c audit reason \x1c'
    request['reason'] = text
    for audit in (after['planning']['quota_authorization'], preview['after']['last_authorization']):
        audit['reason'] = text
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']


def test_equal_remaining_delta_must_match_work_arithmetic():
    """A fictional 100-to-101 remainder is not justified by a valid +1 delta."""
    before, after, request, preview = _witness()
    assert validate_post_quota(before, after, request, preview, SCOPE_ID)['validated']
    preview['before']['remaining']['planning_activations'] = 100
    preview['after']['remaining']['planning_activations'] = 101
    assert _refusal_code(before, after, request, preview) == 'quota-remaining-mismatch'
