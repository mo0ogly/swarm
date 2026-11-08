# Swarm — agent-control PoCs retained for later execution

[Français](../../plans/AGENT-CONTROL-POCS.md).

Fabrice's decision of 7 November 2026: retain these scenarios until the code has
advanced sufficiently and he requests resumption. **Deferred; scope must be
qualified before implementation. No test has run and no product capability is
validated by this note.** Fabrice owns priority and resumption; Codex owns the
inventory and subsequent technical scoping. The engine does not decide business
authority. No schedule, automatic execution or additional development is launched.

Later decision on the same day: Fabrice authorized POC-06 development with
supervised GLM, Biba and documentation; the other PoCs remain deferred. Follow-up:
[POC-06 development](../../plans/actions-confiance/POC06-TASK-MANDATE.md).

On resumption, inspect actual public contracts and existing tests/recipes first.
Identify the connector or simulator, authority, decision/evidence storage and
external-effect observer. A missing integration is a missing prerequisite, not a
failed test. Start with POC-01, then select according to available integrations.

## Retained scenarios

| ID | Scenario and control | Expected observable outcome | Origin |
| --- | --- | --- | --- |
| POC-01 | Approved export to A succeeds; change destination to B while presenting A's approval through the normal and another available fixture path. | Reject before transmission to B; observe no forbidden effect at the destination. Resume only with applicable authority and retained history. | [Amit / A-B case](https://x.com/fabrice76346592/status/2119081585081188545), X-IDEE-02. |
| POC-02 | Approved action succeeds under unchanged state; change a simulated schema before a migration while keeping the command unchanged. | Detect invalidated relevant preconditions before mutation; reassess under control rather than blindly reusing approval. | [alexi](https://x.com/aleximarkett/status/2119108869460771185), X-IDEE-01/02, batch 8. |
| POC-03 | Initial attempt fails and the second succeeds; deliver a late event for the first, including an unknown-outcome variant. | Attribute the event to its attempt, preserve success, avoid unjustified re-emission and reconcile uncertainty under the simulator contract. | [Portotify](https://x.com/portotify/status/2119117960287559720), X-IDEE-07, batch 7. |
| POC-04 | Valid mandate permits an action; revoke before emission and during controlled emission, including child work if supported. | Block unauthorized new actions; retain and reconcile committed/unknown effects. Compensation needs applicable separate authority; preserve history. | [Ashik](https://x.com/AI_Ashik07/status/2119135017482011007), X-IDEE-06/07, batch 8. |
| POC-05 | Initially permitted access; remove the direct right while retaining an explicit fixture access path, then remove that path with authority. | First report incomplete revocation; subsequently confirm only the paths actually checked, preserving chronology and coverage. | [Offboard / Defi Preacher](https://x.com/DefiPreacherr/status/2119121712260522123), X-IDEE-04/06, batch 7. |
| POC-06 | Valid A operation succeeds; the same agent presents mandate A from task B, then receives applicable B authority from the competent role. | Reject reuse before effect, keep the target unchanged; permit B only under its applicable rights and retain A history. | Fabrice/Codex conversation, 7 November 2026, request to add this to Swarm; target contract below. |

[Phillip Shoemaker's contribution](https://x.com/pbsIdentity/status/2119120731334156403)
(X-IDEE-02, batch 8) applies to POC-01/02: the approver must see the relevant data,
destination and consequences corresponding to the proposed effect. A button alone
does not prove consent bound to the executed action.

## POC-06 — identity, task mandate and current policy

**Isolated PoC development authorized on 7 November 2026; GLM delegated,
integration and verification still pending.** This establishes
neither a current defect nor a delivered protection. Codex owns scoping; Fabrice
owns priority and resumption.

Identity establishes who acts; the mandate specifies what that identity may do,
on which target and under which conditions. At each operation, effective rights
must remain within the intersection of current identity rights, the mandate
applicable to the task/attempt and current policy. A task, its text or a prompt
creates no additional authority. Biba remains a distinct mandatory integrity
control alongside other applicable controls; a mandate cannot bypass it. See
[current Biba scope and limits](../BIBA-RUST.md).

Qualify the existing implementation before choosing a representation: bind verified
identity, installation/project context, task, attempt, action, target, preconditions,
authorized decision and relevant versions. Define validity, cancellation/revocation
and explicit delegation without scope expansion. Agent-supplied parameters do not
prove these bindings. Invent no validity duration or competing mandate mechanism:
reuse and verify contracts actually available.

```mermaid
flowchart TD
    R[Operation requested within a task and attempt] --> I[Verified identity and current rights]
    I --> M[Applicable mandate: action, target, scope and validity]
    M --> P[Current policy and applicable Biba controls]
    P --> V{Bindings and preconditions valid at application?}
    V -->|Yes| E[Apply only the authorized effect and retain the result]
    V -->|No or unknown before emission| F[Reject before effect and retain the cause]
```

**Target diagram to verify**, not an implemented end-to-end flow. Bind checking
to application so a concurrent target, mandate or policy change cannot invalidate
the check unnoticed. For external connectors, identify the actual controlled
boundary and retain committed or unknown outcomes: this diagram guarantees no
distributed atomicity.

| Case | Valid control followed by injection | Expected observable criterion |
| --- | --- | --- |
| A → B | Valid A mandate on A; same agent presents it from B. | Reject before B effect, no forbidden mutation/transmission; B subsequently works under applicable B authority. |
| Expanded action or target | Reading/suggestion permitted; request application or a different target using the same mandate. | No capability elevation; reject with target unchanged. A mandate permitting the exact action provides the positive control. |
| Cancellation and new attempt | Valid operation; cancel/revoke before emission or move to another attempt, then replay the previous mandate. | Reject inapplicable old authority; no implicit transfer to the new attempt. Distinguish returning an already recorded result from new execution. |
| Delegation | Valid explicit in-scope delegation; transfer to another identity without applicable delegation or with expanded scope. | Reject unauthorized transfer; no wrong-agent attribution or amplification of rights. |
| Concurrent change | Stable-state control; change target, mandate or policy between check and application at a deterministic fixture barrier. | No effect under authority invalidated before emission; observe rejection/reassessment. For already committed effects, retain actual/unknown state and reconcile with authority, without falsely claiming no effect. |
| Invalid bindings | Valid request; remove or falsify a required identity, task, attempt or mandate binding. | Explicit rejection without empty success or effect; no authority inferred from an agent-asserted identifier. |

Reuse POC-01 for destinations and POC-04 for revocation rather than duplicating
recipes. Retain observed identity, task/attempt, mandate/decision, checked versions,
consumer state before/after and recovery condition for every variant. Permission
to read does not permit application; terminal status proves neither identity nor
absence of effect.

## Shared execution criteria and follow-up

- Isolated fixtures, fictional data and simulated effects; no live mission,
  production database, banking transaction or actual access revocation.
- Valid control before each fault with matching preconditions; relevant invalid
  inputs, forbidden variants and recovery after correction.
- Check consumer effects and state, not only rejection text, exceptions or logs.
  Retain decision, mandate, parameters, attempt and exact coverage.
- State which alternative paths are controlled. A simulator or engine test
  establishes neither OS/network confinement nor universal tool coverage.
- Preserve uncertainty for committed actions; process termination or missing
  acknowledgement does not prove absence of effect.
- Reuse permanent recipes, fresh evidence and inventory verification. Bind results
  to actual sources, profile and binary; local PoCs are distinct from installation
  and global acceptance.

Original ideas: [French discussion register](../../communication/x-swarm-20261107/IDEES-DISCUSSIONS.md)
and dated `discussion-replies*.json`, `discussions-lot7-evidence.json` and
`discussions-lot8-evidence.json` in that campaign directory. These exchanges establish
neither Swarm defects nor qualification of third-party products.

On resumption, track each scenario as **to qualify → decided → implemented →
verified**, with evidence, discrepancies, limits and remaining work. All six
scenarios were initially deferred at Fabrice's request. POC-01 through POC-05
remain deferred; the subsequent POC-06 resumption is tracked separately.

Additional retained identity separation case from
[Cortexx](https://x.com/AI_cortexx/status/2119148103571792165), 7 November:
inspect effective delegated-session authority and policy-modification paths in
POC-01/04 (ideas 02/10/11). Require a valid human control and valid delegated
business action, then reject self-expansion with no policy mutation. This is a
hypothesis to qualify, not an established Swarm defect; owner and resumption
conditions remain unchanged. Dated evidence:
`docs/communication/x-swarm-20261107/lot8-audience-cortexx-context.json`.
