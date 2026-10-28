# Swarm positioning and security references

October 25, 2026 — [Français](../SECURITY-POSITIONING.md).

## Product positioning

**Swarm is a multi-agent orchestration engine with deterministic controls. It
aims to link each action to an identity, authority, approved content and a
verifiable outcome, with supervision and recovery mechanisms. Its assurances
are established per workflow, for an identified version and configuration.**

The shared engine organizes objectives, plans, tasks, dependencies, responsibilities,
checks and evidence. Its intended uses are development projects and business
processes; their separation and maturity remain defined in the
[product architecture, FR](../PRODUCT-ARCHITECTURE.md). Agents propose operations;
the engine checks the conditions of the transitions it controls. That function
alone does not establish confinement of tools, processes or direct external access.

The desired property is continuity between authority, decision, commitment and
recovery: authorization must remain bound to the exact action and current state
until its effect can be committed. Rust and determinism support implementation
properties; they establish neither correct policies, defect freedom nor security
of the complete deployment.

## Roles of existing references

These references have different purposes. Citing them establishes no Swarm
compliance. This is a documentary positioning map, without a selected baseline
or an exhaustive normative assessment.

| Reference | Nature and contribution | Intended or documented use in Swarm |
| --- | --- | --- |
| [ISO/IEC 27001:2022](https://www.iso.org/standard/27001) and [42001:2023](https://www.iso.org/standard/42001) | Requirements for information security and AI management systems | Organize responsibilities, risks, incidents and changes in the operating organization; no Swarm certification claimed |
| [ISO/IEC 23894:2023](https://www.iso.org/standard/77304.html) | AI risk management guidance | Assess risks by use, data and consequences; no clause-by-clause evaluation performed here |
| [OWASP Top 10 for Agentic Applications 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) | Agentic risk catalogue | Select threats and adversarial scenarios; a risk catalogue is not a complete qualification protocol |
| [OWASP Agent Control Standard](https://genai.owasp.org/resource/agent-control-standard-acs/) and [ACS repository](https://github.com/GenAI-Security-Project/agent-control-standard) | Runtime interception and control specification | Study contract mappings, then test any claimed compatibility subset; this page establishes no ACS compatibility |
| [NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) | Zero Trust architecture | Examine separation of policy decisions and resource enforcement; no global compliance inferred |
| [SPIFFE](https://spiffe.io/docs/latest/spiffe/concepts/) and its [SPIRE implementation](https://spiffe.io/docs/latest/spire-about/spire-concepts/) | Verifiable workload identities | Possible process identity reference; no SPIFFE/SPIRE integration announced. Workload authentication does not define business permissions |
| OIDC and OAuth | Authentication and authorization protocols and mechanisms | The [target contract](ACTIONS-CONFIANCE.md) describes historical choices; the current Rust core uses local identities. No real federation requalified by this page |
| [OWASP ASVS 5.0.0](https://asvs.owasp.org/) | Web application and service verification requirements | Proposed reference for APIs, sessions, permissions and configuration; no ASVS level evaluated here |
| [NIST SP 800-53 Rev. 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final) | Security and privacy control catalogue | Twelve selected controls already map to contextual evidence in the [R19 matrix](actions-confiance-referentiels.md), without a score or global compliance |
| Biba | Integrity flow control model | I0–I3 monitor in the [Rust trusted actions core](actions-confiance-rust.md); neither content truth, general semantic injection protection nor Linux access control |

The OWASP ACS page is dated September 1, 2026. The inspected repository README
already describes a deterministic layer with optional LLM involvement. Swarm
must therefore explain its contribution through contracts and evidence rather
than claim to have invented this separation. A future comparison must pin the
ACS revision, applicable requirements and executed checks.

NIST announced its [AI Agent Standards Initiative on February 17, 2026](https://www.nist.gov/news-events/news/2026/02/announcing-ai-agent-standards-initiative-interoperable-and-secure).
Its [September 29 identity and authorization update](https://www.nist.gov/news-events/news/2026/09/comments-software-and-agentic-ai-identity-concept-paper)
describes feedback and an initial DevSecOps implementation use case, not a final
agent certification standard. OWASP dates the “2026” Top 10 page December 9,
2025: its edition year differs from its publication date.

## Trust boundaries to demonstrate

| Function | Responsibility | Expected evidence |
| --- | --- | --- |
| Rules guardian | Check identity, mandate, current policy, exact content and required approvals | A positive decision is consumable only for the authorized operation and state; AI analysis creates no permissions |
| Keys guardian | Retain secrets and issue capabilities limited to the authorized consumer | An agent can neither read critical secrets nor obtain broader capabilities through another route |
| Enforcement points | Apply controls at the relevant connectors, processes, files and network accesses | Inventoried direct paths, descendants and alternative accesses cannot bypass a denial |

These are architectural functions. Three modules sharing privileges and able to
change one another’s policies or keys do not establish three independent
boundaries. Logs or post-effect eBPF observation do not replace preventive
enforcement. The demonstration must declare its threat model, including limits
against the same UID or a compromised privileged host.

## Documented state on October 25, 2026

| Scope | Recorded state | Claim boundary |
| --- | --- | --- |
| Rust Biba trusted actions | The [BIBA-RUST plan, FR](../plans/migration-rust/PLAN.md#livré-localement--biba-rust-g7-r09t2-25-octobre-paris) records local delivery of the monitor, mandates, exact approvals, simulated effects and recovery, with CLI/API/page and FR/EN Admin | `trust-actions` scope; no actual payment, external permission or OS confinement. This documentary analysis does not rerun the recorded checks |
| Agent identity and authority | [Internal contracts and components](AGENT-IDENTITY-CONTRACT.md), with explicit integration gaps | Does not establish authentication and authorization of every launch, memory use and tool |
| Memory | [Versions, sources and usage conditions](PROJECT-MEMORY.md); [Rust FPA02A recovery, FR](../plans/migration-rust/FPA02A-MEMOIRE-20261025.md) of historical memory references | The reference slice closes neither permanent versioned memory nor all read, summarize and transmit permissions. Withdrawal does not erase received contexts |
| General confinement and secret mediation | [AC-02 recorded, not started](plans/actions-confiance/securite-execution-apex.md) | OpenShell references or YAML policies establish no active AC-02 protection; compatibility and enforcement remain to be demonstrated |
| Control mapping | [Contextual R19](actions-confiance-referentiels.md): nine partial, two unassessed, one out of scope | Illustrative subset, without a selected baseline, certification, global compliance or independent evaluation |

## Proposed agentic execution assurance profile

This profile is a **documentary proposal**, not an adopted new standard, active
policy, completed qualification or authorization to start AC-02. It extends the
existing matrix rather than replacing it. PA identifiers support tracking;
external requirement mappings remain to be assessed.

| Proposed requirement | Observable qualification criterion | Starting point and follow-up owner |
| --- | --- | --- |
| PA-01 Identity and delegation | Every action traces to a current principal and mandate; delegation cannot exceed the parent’s delegable permissions | Identity contract; identity integration owner, then deployment integrator |
| PA-02 Integrity and admission | External data and client fields create no permissions; admission creates an attributed object linked to its original | Scoped Biba core; Trust owner and future consumers |
| PA-03 Governed memory | Sources, versions and usage permissions are retained; withdrawal blocks relevant new uses without rewriting received history | Memory contracts; G8 owner and identity/memory integrator |
| PA-04 Control at commitment | Changed approved content or revocation before the serialization point prevents the effect; control failure denies the sensitive action | Simulated actions and effect contracts; effect service and policy owner |
| PA-05 Capabilities and access routes | The agent obtains no critical secret and cannot bypass a denial through a tool, file, descendant or network path | AC-02; confinement owner and environment operator after its starting prerequisites |
| PA-06 Effects and recovery | A lost response retains a known or unknown outcome; reconciliation and idempotency prevent a second effect for the same intent | Two simulated connectors; owner of each real connector before automation |
| PA-07 Human validation | When required by policy, AI advice cannot replace authenticated human approval bound to the exact operation; required separation of duties is checked | Validation contracts; identity/approval owner and business owner |
| PA-08 Evidence and qualification | Decisions and observed effects link to inputs, revisions and originals; tampering and omissions are detected under declared assumptions | Local recipes and evidence; qualification owner, then mandated independent evaluator |

An evaluation record must declare binary version and digest, configuration,
threat model, scope, external requirement/version, applicability, mechanism,
named test, valid control, denial, recovery, evidence, limit, owner and state.
Unit checks, simulation, actual integration and independent assessment retain
separate statuses. An average percentage cannot offset a failed required invariant.

The proposed sequence is to pin scope and references, link verifiable existing
work to R19, then address missing integrations and protections through their
authorized plans. Each integration must exercise applicable changed approvals,
concurrent revocation, lost responses, control failure and bypass routes. Stop
or revocation blocks relevant new operations without claiming to undo an
already committed effect. Missing evidence leaves the criterion open.

## Communication vocabulary

| Wording | Required justification |
| --- | --- |
| Inspired by | Explicit reference and reused principle |
| Documented mapping | Selected requirements, context, gaps and linked evidence |
| Compatible with a subset | Pinned revision, published scope and executed compatibility checks |
| Requirements met within an assessed scope | Applicable criteria, configuration and current evidence, with failures and limits declared |
| Certified | Verifiable actual certification, body, scope and validity; none claimed for Swarm here |

A signature does not establish memory truth. Biba does not guarantee
confidentiality, and authenticated identity is not business authorization.
“Enforceable” describes a demonstrated technical constraint within its scope,
without presuming legal evidentiary value.

For critical uses, the proposed engineering position conditions autonomous
commitment on context-required assurances. It does not describe a general legal
ban. Regulatory and sector-specific deployment evaluation belongs to the
mandated responsible party.

The [French LinkedIn dossier and Word](../communication/linkedin-swarm-securite-20261025/README.md)
preserve the complete supplied original article and its accompanying material.
This positioning analysis remains separate from the author's text and does not
replace it with a summary. This documentation starts no implementation, changes no
execution policy and requalifies no runtime.
