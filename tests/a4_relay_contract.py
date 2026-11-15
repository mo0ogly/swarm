#!/usr/bin/env python3
"""Fictional A4 reference oracle; NOT a Swarm integration qualification.

No Swarm imports, processes, network, files, clocks or global watchdog. R1/R2
are in-memory doubles. Equality against operator-registered originals is not
authentication. Run only after assignment: python3 tests/a4_relay_contract.py -v.
Execution receipts are separate and hash-bound. See A4-RELAY-CONTRACT.md.
"""
from copy import deepcopy
from dataclasses import dataclass, replace
import unittest


class Refused(ValueError):
    """Stable fictional refusal code; no diagnostic state is persisted."""


@dataclass(frozen=True)
class Order:
    """Immutable fictional decision; stop_ref links a new resume to its stop.

    Fields are data, not identity/authorization proofs. Registration validates
    them; constructing a variant has no effects and grants no authority.
    """
    id: str
    action: str
    target: str
    issuer: str
    decision: str
    stop_ref: str = ""


@dataclass(frozen=True)
class Receipt:
    """A double's received payload and actual preceding receipt, or None at R1.

    Object linkage is an in-memory test observation, never a secure transport.
    """
    relay: str
    order: Order
    previous: object


class Reference:
    """Single-thread fictional oracle. Only guarded apply changes running state.

    Operator registration, transport receipts, decisions and effect observations
    are separate records. Refusals raise Refused before mutating any record.
    No storage, actual action execution, rollback or security boundary exists.
    """
    def __init__(self):
        """Create fresh active A/B and empty trusted, transport and effect records."""
        self.running = {"A": True, "B": True}
        self.originals = {}
        self.receipts = []
        self.applied = []
        self.effects = []
        self.events = []

    def authorize(self, order):
        """Register an operator-supplied original once; no action is applied.

        Inputs must be well-formed Order data. Refuse invalid/duplicate IDs before
        mutation. This explicit fixture operation does not authenticate an issuer.
        """
        if (not isinstance(order, Order)
                or any(not isinstance(v, str) or not v for v in
                       (order.id, order.action, order.target, order.issuer, order.decision))
                or order.target not in self.running
                or order.action not in ("stop", "resume")
                or not isinstance(order.stop_ref, str)
                or (order.action == "stop" and order.stop_ref)
                or (order.action == "resume" and not order.stop_ref)):
            raise Refused("invalid-original")
        if order.id in self.originals:
            raise Refused("duplicate-original")
        self.originals[order.id] = order
        self.events.append(("authorized", order))

    def receive(self, relay, source, changed=None):
        """Record one in-memory transport double without applying its payload.

        R1 receives Order; R2 receives the actual R1 Receipt. Optional changed is
        a LAB-ONLY altered payload, not a production hook. Invalid shapes refuse
        before append; content fidelity is checked later against the original.
        """
        if relay == "R1" and isinstance(source, Order):
            previous, order = None, source
        elif relay == "R2" and isinstance(source, Receipt) and source.relay == "R1":
            previous, order = source, source.order
        else:
            raise Refused("invalid-relay-input")
        payload = order if changed is None else changed
        if not isinstance(payload, Order):
            raise Refused("invalid-relay-payload")
        receipt = Receipt(relay, replace(payload), previous)
        self.receipts.append(receipt)
        self.events.append(("received", receipt))
        return receipt

    def apply(self, order, chain):
        """Validate original, ordered observed chain and lifecycle, then apply.

        Refuse unknown/altered/replayed orders, missing/reversed/unlinked receipts
        and resumes lacking a new linked authorization. All checks precede state,
        journal or effect mutation. Effects already recorded are never rewritten.
        """
        if not isinstance(order, Order) or self.originals.get(order.id) != order:
            raise Refused("original-mismatch")
        if not isinstance(chain, (tuple, list)) or len(chain) != 2:
            raise Refused("chain-shape")
        first, second = chain
        if (not isinstance(first, Receipt) or not isinstance(second, Receipt)
                or (first.relay, second.relay) != ("R1", "R2")
                or first.previous is not None or second.previous is not first
                or any(not any(r is item for r in self.receipts) for item in chain)
                or first.order != order or second.order != order):
            raise Refused("chain-fidelity")
        if any(old.id == order.id for old in self.applied):
            raise Refused("already-applied")
        if order.action == "resume":
            previous = next((o for o in reversed(self.applied)
                             if o.target == order.target), None)
            if (previous is None or previous.action != "stop"
                    or self.running[order.target] or order.stop_ref != previous.id
                    or order.decision == previous.decision):
                raise Refused("resume-authority")
        self.running[order.target] = order.action == "resume"
        self.applied.append(order)
        self.events.append(("applied", order))

    def action(self, actor, label, state="known"):
        """Append a fictional known/engaged/unknown effect if actor may act.

        Invalid inputs or stopped actors refuse without mutation. State is a
        scenario observation only, not a remote result or permission to reconcile.
        """
        if actor not in self.running or state not in ("known", "engaged", "unknown"):
            raise Refused("invalid-action")
        if not isinstance(label, str) or not label:
            raise Refused("invalid-action")
        if not self.running[actor]:
            raise Refused("actor-stopped")
        self.effects.append((actor, label, state))


class RelayContractTest(unittest.TestCase):
    """Reference-only witnesses, mutations, refusals and recovery; no native proof."""
    def fixture(self):
        """Return fresh A/B witnesses and stop order with observed R1→R2 chain.

        A separate clone proves that the SAME preconditions admit the healthy
        stop before any injection. Deep-copy model and chain together preserves
        receipt identity/linkage; this helper does not mutate the returned model.
        """
        model = Reference()
        model.action("A", "healthy-A")
        model.action("B", "healthy-B")
        order = Order("stop-A", "stop", "A", "operator", "decision-stop")
        model.authorize(order)
        first = model.receive("R1", order)
        chain = (first, model.receive("R2", first))
        control, original, received = deepcopy((model, order, chain))
        control.apply(original, received)
        self.assertEqual(control.running, {"A": False, "B": True})
        control.action("B", "healthy-B-after-stop")
        return model, order, chain

    def unchanged_refusal(self, model, code, operation):
        """Require expected refusal and exact complete state equality, including logs."""
        before = deepcopy(model.__dict__)
        with self.assertRaisesRegex(Refused, "^" + code + "$"):
            operation()
        self.assertEqual(model.__dict__, before)

    def test_healthy_ab_receipt_then_application(self):
        """Receiving two relays is not application; only apply stops A, preserving B."""
        model, order, chain = self.fixture()
        self.assertEqual(model.running, {"A": True, "B": True})
        self.assertEqual([e[0] for e in model.events], ["authorized", "received", "received"])
        model.action("A", "allowed-after-receipt")
        self.assertIs(chain[1].previous, chain[0])
        model.apply(order, chain)
        self.assertEqual(model.events[-1], ("applied", order))
        model.action("B", "allowed-after-application")
        self.unchanged_refusal(model, "actor-stopped", lambda: model.action("A", "new-A"))

    def test_altered_scope_sense_and_authority_at_each_relay(self):
        """Each fault has its own healthy control; original stays fixed and trusted."""
        changes = ({"target": "B"}, {"target": "global"}, {"action": "ban-discussion"},
                   {"issuer": "relay"}, {"decision": "relay-decision"})
        for at in ("R1", "R2"):
            for fields in changes:
                with self.subTest(relay=at, fields=fields):
                    model, order, _ = self.fixture()
                    altered = replace(order, **fields)
                    first = model.receive("R1", order, altered if at == "R1" else None)
                    second = model.receive("R2", first, altered if at == "R2" else None)
                    self.unchanged_refusal(model, "original-mismatch",
                                           lambda: model.apply(second.order, (first, second)))
                    self.assertIs(model.originals[order.id], order)
                    self.assertEqual(model.running, {"A": True, "B": True})

    def test_missing_reversed_and_broken_chain(self):
        """Coverage of two names is insufficient: order and predecessor must agree."""
        for kind in ("missing", "reversed", "unlinked", "altered-receipt"):
            with self.subTest(kind=kind):
                model, order, chain = self.fixture()
                first, second = chain
                if kind == "missing":
                    bad, code = (first,), "chain-shape"
                elif kind == "reversed":
                    bad, code = (second, first), "chain-fidelity"
                elif kind == "unlinked":
                    other = model.receive("R1", order)
                    bad, code = (first, model.receive("R2", other)), "chain-fidelity"
                else:
                    bad = (first, model.receive("R2", first, replace(order, target="B")))
                    code = "chain-fidelity"
                self.unchanged_refusal(model, code, lambda: model.apply(order, bad))
                model.apply(order, chain)

    def test_duplicate_does_not_reapply_or_duplicate_history(self):
        """Healthy application precedes replay refusal; registration is immutable too."""
        model, order, chain = self.fixture()
        model.apply(order, chain)
        self.unchanged_refusal(model, "already-applied", lambda: model.apply(order, chain))
        self.unchanged_refusal(model, "duplicate-original", lambda: model.authorize(order))

    def test_engaged_unknown_effects_survive_stop_and_resume(self):
        """Stop blocks new A, never deletes existing known/engaged/unknown observations."""
        model, order, chain = self.fixture()
        model.action("A", "dispatched-before-stop", "engaged")
        model.action("A", "response-not-known", "unknown")
        effects = tuple(model.effects)
        model.apply(order, chain)
        self.assertEqual(tuple(model.effects), effects)
        self.unchanged_refusal(model, "actor-stopped", lambda: model.action("A", "late"))
        model.action("B", "B-still-valid")
        self.assertEqual(tuple(model.effects[:len(effects)]), effects)
        self.resume(model, order)
        model.action("A", "new-authorized-A")
        model.action("B", "B-after-resume")
        self.assertEqual(tuple(model.effects[:len(effects)]), effects)

    def resume(self, model, stop):
        """Fixture operator authorizes a new linked decision and relays it before apply."""
        new = Order("resume-A", "resume", "A", "operator", "decision-resume", stop.id)
        model.authorize(new)
        first = model.receive("R1", new)
        model.apply(new, (first, model.receive("R2", first)))
        self.assertEqual(model.running, {"A": True, "B": True})

    def test_resume_requires_distinct_registered_linked_decision(self):
        """Unknown, same-decision and wrong-stop resumes refuse; new grant recovers."""
        for kind in ("unknown", "same-decision", "wrong-stop", "not-stopped"):
            with self.subTest(kind=kind):
                model, order, chain = self.fixture()
                if kind != "not-stopped":
                    model.apply(order, chain)
                control, control_stop, control_chain = deepcopy((model, order, chain))
                if kind == "not-stopped":
                    control.apply(control_stop, control_chain)
                self.resume(control, control_stop)  # Healthy recovery BEFORE each invalid variant.
                control.action("A", "healthy-resume-control")
                new = Order("resume-invalid", "resume", "A", "operator",
                            order.decision if kind == "same-decision" else "new-decision",
                            "wrong-stop" if kind == "wrong-stop" else order.id)
                if kind != "unknown":
                    model.authorize(new)
                first = model.receive("R1", new)
                received = (first, model.receive("R2", first))
                self.unchanged_refusal(model, "original-mismatch" if kind == "unknown"
                                       else "resume-authority", lambda: model.apply(new, received))
                if kind == "not-stopped":
                    model.apply(order, chain)
                self.resume(model, order)
                model.action("A", "authorized-recovery")
                self.assertEqual(model.applied[0], order)

    def test_lab_guard_omission_has_observable_consequence(self):
        """LAB MUTATION ONLY: bypassing guard stops B or wrongly resumes A, not Swarm."""
        model, order, _ = self.fixture()
        wrong = replace(order, target="B")
        model.running[wrong.target] = False  # Deliberate reference-oracle fault injection.
        self.assertNotEqual(model.running, {"A": False, "B": True})
        self.unchanged_refusal(model, "actor-stopped", lambda: model.action("B", "should-work"))
        model.action("A", "wrongly-allowed-A")
        model, order, chain = self.fixture()
        model.apply(order, chain)
        control, control_stop = deepcopy((model, order))
        self.resume(control, control_stop)  # Authorized recovery before the omission variant.
        control.action("A", "healthy-before-unauthorized-resume")
        model.running["A"] = True  # Deliberate unauthorized-resume omission.
        model.action("A", "unauthorized-new-effect")
        self.assertEqual(model.effects[-1], ("A", "unauthorized-new-effect", "known"))


if __name__ == "__main__":
    unittest.main()
