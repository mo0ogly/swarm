# Exact authority for PAEA effect services

[Français](../architecture/PAEA-EFFECT-AUTHORITY.md)

The Linux `swarm_enrollment_authority::paea_action` module separates PAEA effect
permissions from historical Trust commands. It provides a borrowed capability
while current personal-account and business authority reservations are held.
It does not yet provide a daemon, payment/Git service, authenticated transport or
actual Linux account separation. Qualification and delivery are tracked in PAEA
increment 65.

## Permission and binding

`Request::new(root, service, operation, action)` validates two canonical SHA-256
digests. The closed service variants are `FictitiousPayment` and `GitPromotion`;
operations are `Commit` and `Reconcile`. These constants name the protocol and
select no executable, URL or operator policy. The requested signed permission is:

```text
operation: effect.execute
resource: paea-action-v1/<root>/<payment.settle|repository.promote>/<commit|reconcile>/<action>
```

A request is data, never a grant. Its constructor checks no storage, actual
action or identity. The future service must derive `root` from a retained opened
storage witness and `action` from control-attributed canonical PAEA bytes. At use,
it must compare them with current service, project, plan, revision, policy,
approval and business terms. A `trust-action/...` permission, another service,
operation or nearby digest never matches. No wildcard or fallback is accepted.
`Reconcile` grants no new commitment.

## Use and failures

`AuthenticatedControl::with_paea_action` checks the configured account password.
The private shared gate checks principal and installation, retains the business
reservation, reads the current signed active grant, checks exact permission and
expiry, invokes the consumer once and rechecks time afterwards. Trust now uses
the same gate with its public API retained. Locks and reservations are not OS
confinement guarantees.

Callers cannot construct or deserialize `Identity`, or turn a copied principal
into a lasting grant. Getters expose the retained enrollment, signed project,
plan, revision, expiry and exact request; consumers retain their own validation
obligations. A fixture password establishes no actual human consent.

`Use` preserves `authority` and `action` independently. `action=None` means the
callback was not invoked. `Some(Err(...))` preserves the consumer refusal, which
may itself retain a known or uncertain effect. `Some(Ok(...))` survives a later
authority failure. The personal layer may additionally return `UseError::AfterUse`
containing that complete outcome. Inspect every branch: failure establishes no
rollback or absence of effect. Refusal and journal failures retain independent
causes. No automatic retry, timer, restart, initialization or reconciliation.

`tools/verification/recipes/paea-exact-effect-authority.json` uses isolated private
journals and explicit signed fixture grants. It checks exact permissions and
neighbors, Trust/PAEA namespaces, passwords, callback refusal, revocation,
after-use cancellation and CAS. It also reruns the three Trust regressions affected
by the shared gate. Local evidence does not qualify separated effects, protected
installation, global PAEA invariants or E2/E3.
