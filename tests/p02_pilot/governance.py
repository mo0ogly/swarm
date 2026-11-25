"""Fictional P02 delegations and decisions, bound to one recipe run and its evidence."""
from copy import deepcopy
from hashlib import sha256
from hmac import compare_digest, digest
import json

ROLES = {
    "service_rights": ("fixture-service-authority", "fixture-rights-validator"),
    "confidentiality": ("fixture-data-authority", "fixture-confidentiality-validator"),
}
SCOPE = {
    "environment": "local-fixture", "project": "fictional-invoices",
    "batch": "qualification", "audience": "local-receiver", "reference": "ref",
}


def encode(value):
    """Canonical JSON for fixture evidence identity."""
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def fingerprint(value):
    """Hash the canonical representation of one evidence object."""
    return sha256(encode(value)).hexdigest()


def signed(body):
    """Tag an act with a deliberately public, reproducible fixture key."""
    key = sha256(encode(["P02 public fixture key", body["run"], body["actor"]])).digest()
    return {"body": body, "tag": digest(key, encode(body), "sha256").hex()}


def verify_transport(result):
    """Check the retained qualification or pilot observation before using it."""
    cases = {case["label"]: case for case in result["cases"]}
    expected = {
        "match": ("confirmed", "recorded", 0),
        "wrong-project": ("refused_before_send", "no_draft", 0),
        "wrong-batch": ("refused_before_send", "no_draft", 0),
        "wrong-audience": ("refused_before_send", "no_draft", 0),
        "wrong-reference": ("refused_before_send", "no_draft", 0),
        "revoked": ("refused_before_send", "no_draft", 0),
        "expired": ("refused_before_send", "no_draft", 0),
        "wrong_peer": ("peer_refused", "no_draft", 0),
        "wrong_host": ("unconfirmed", "wrong_host", 0),
        "unexpected_field": ("raw_probe", "invalid_draft", 0),
        "crash_before_commit": ("unconfirmed", "", 71),
        "crash_after_commit": ("unconfirmed", "", 72),
        "replay_after_readback": ("confirmed", "replayed", 0),
        "conflicting_replay": ("raw_probe", "conflict", 0),
    }
    outcomes = {
        label: (case["state"], case["receiver"], case["receiver_exit"])
        for label, case in cases.items()
    }
    if result["status"] != "PASS" or len(result["cases"]) != len(expected) or outcomes != expected:
        raise ValueError("incomplete-transport")
    rows = result["rows_before_purge"]
    if (rows != [
            ["different", 0, cases["replay_after_readback"]["digest"].upper()],
            ["same", 1, cases["match"]["digest"].upper()],
        ] or result["rows_after_purge_reopen"] != 0
            or any(cases[label]["reply"] for label in ("unexpected_field", "conflicting_replay"))
            or b"fictitious-invoice-account" in encode(result) or b"account_token" in encode(result)):
        raise ValueError("invalid-receiver-evidence")


def fixture_acts(run, qualification):
    """Create signatures, acceptances and opinions for the two distinct roles."""
    verify_transport(qualification)
    evidence = fingerprint(qualification)
    acts = []
    for role, (issuer, delegate) in ROLES.items():
        for stage, actor, value in (
            ("delegation", issuer, "signed"),
            ("acceptance", delegate, "accepted"),
            ("opinion", delegate, "favorable_test"),
        ):
            acts.append(signed(dict(run=run, scope=SCOPE, role=role, stage=stage,
                                    actor=actor, value=value, evidence=evidence)))
    return acts


def admission(run, qualification, acts):
    """Require the entire ordered, role-bound favorable chain before admission."""
    expected = fixture_acts(run, qualification)
    if len(acts) != len(expected):
        raise ValueError("missing-or-replayed-act")
    for recorded, witness in zip(acts, expected, strict=True):
        if not compare_digest(recorded["tag"], signed(recorded["body"])["tag"]):
            raise ValueError("invalid-test-signature")
        if recorded["body"] != witness["body"]:
            raise ValueError("act-outside-delegation")
    return dict(run=run, scope=SCOPE, simulated=True, decision="admitted_in_fixture",
                acts=fingerprint(acts), qualification=fingerprint(qualification))


def launch(run, recorded, expected):
    """Bind a fictional launch to exact admission without granting runtime authority."""
    if recorded != expected:
        raise ValueError("admission-mismatch")
    return dict(run=run, scope=SCOPE, simulated=True, decision="launch_fixture",
                admission=fingerprint(recorded), real_execution_authorized=False)


def closure(run, recorded, expected, qualification, pilot):
    """Accept a closed fixture only when launch and the full replay agree."""
    if recorded != expected:
        raise ValueError("launch-mismatch")
    verify_transport(pilot)
    if pilot != qualification:
        raise ValueError("replay-different")
    return dict(run=run, scope=SCOPE, simulated=True, decision="closed_favorable",
                launch=fingerprint(recorded), pilot=fingerprint(pilot), real_execution_authorized=False,
                technical_cases=len(pilot["cases"]), rows_after_purge=pilot["rows_after_purge_reopen"])


def verify_closure(recorded, expected):
    """Refuse a persisted closure that differs from the recomputed fixture decision."""
    if recorded != expected:
        raise ValueError("closure-readback-mismatch")


def negative_controls(run, qualification, acts, admitted, launched, pilot, closed):
    """Reject bad signatures, missing and foreign acts, and invented runtime effects."""
    forged = deepcopy(acts)
    forged[2]["body"]["value"] = "unfavorable_test"
    foreign_role = deepcopy(acts)
    foreign_role[2]["body"]["actor"] = ROLES["confidentiality"][1]
    foreign_role[2] = signed(foreign_role[2]["body"])
    adverse = deepcopy(acts)
    adverse[-1]["body"]["value"] = "unfavorable_test"
    adverse[-1] = signed(adverse[-1]["body"])
    foreign_lot = deepcopy(admitted)
    foreign_lot["scope"]["batch"] = "another-batch"
    foreign_run = deepcopy(acts)
    foreign_run[2]["body"]["run"] = "another-run"
    foreign_run[2] = signed(foreign_run[2]["body"])
    wrong_evidence = deepcopy(acts)
    wrong_evidence[2]["body"]["evidence"] = "0" * 64
    wrong_evidence[2] = signed(wrong_evidence[2]["body"])
    partial = dict(pilot, cases=pilot["cases"][:-1])
    checks = {
        "invalid_signature": (lambda: admission(run, qualification, forged), "invalid-test-signature"),
        "wrong_role": (lambda: admission(run, qualification, foreign_role), "act-outside-delegation"),
        "unfavorable_opinion": (lambda: admission(run, qualification, adverse), "act-outside-delegation"),
        "missing_opinion": (lambda: admission(run, qualification, acts[:-1]), "missing-or-replayed-act"),
        "replayed_act": (lambda: admission(run, qualification, acts + acts[:1]), "missing-or-replayed-act"),
        "foreign_run": (lambda: admission(run, qualification, foreign_run), "act-outside-delegation"),
        "wrong_evidence": (lambda: admission(run, qualification, wrong_evidence), "act-outside-delegation"),
        "missing_admission": (lambda: launch(run, None, admitted), "admission-mismatch"),
        "altered_closure": (lambda: verify_closure(dict(closed, real_execution_authorized=True), closed),
                             "closure-readback-mismatch"),
        "foreign_admission": (lambda: launch(run, foreign_lot, admitted), "admission-mismatch"),
        "runtime_activation": (lambda: closure(run, dict(launched, real_execution_authorized=True),
                                                 launched, qualification, pilot), "launch-mismatch"),
        "incomplete_replay": (lambda: closure(run, launched, launched, qualification, partial),
                              "incomplete-transport"),
    }
    for name, (check, refusal) in checks.items():
        try:
            check()
        except ValueError as error:
            if str(error) != refusal:
                raise
        else:
            raise AssertionError(f"accepted invalid fixture: {name}")
    if closed["real_execution_authorized"] is not False:
        raise ValueError("invalid-closure")
    return list(checks)
