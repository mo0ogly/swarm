# PAEA — increment 05: Rust comparison, qualification deferred

[Français](../../../../plans/actions-confiance/paea/INCREMENT-05.md) · [Plan](../PAEA-APEX.md)

Sources now expose `compare_documents`, `DocumentComparison` and `DocumentIdentity`
in the existing `swarm-protection-contract` Rust crate. They reuse strict Rust
validation and canonicalization of two raw documents. Changed paths omit values;
content, business intent and transport differences remain separate. Approval,
authority and effect stay unestablished. No CLI/API operation is added.

**Written and statically examined only: not compiled, Rust tests not executed,
not installed.** User's no-compilation/no-restart instruction remains active.
Historical evidence for increments 01…04 does not qualify these source bytes.
PA-01/08/09 stay partial; PA-08 operator integration remains open.

## Method recovery

Codex initially wrote another Python comparison tool to reuse the existing offline
verifier and execute without compilation. User twice challenges this language
choice. No-compilation does not forbid writing Rust; peripheral tools cannot
replace engine integration. Five new Python/tool/test/recipe/doc files are withdrawn
from maintained sources and archived under `withdrawn-python/`. Three passing
Python recipe runs remain historical and do not qualify Rust. Increment 04's
existing Python verifier is unchanged. No earlier delivery is deleted.

The retained `replan-language.json` records cause/decision and a documentation
mistake: the actual English queue is `docs/en/APEX-QUEUE.md`. Keep execution
constraints separate from language selection; future functional changes belong
in the Rust engine, with uncompiled status until authorized qualification.

## Contract and static review

The Rust function validates both supplied documents, including existing i64/OID/
source invariants, then compares computed canonical bytes. It consumes neither
exports nor external JSON schema and adds no duplicate admission implementation.
SHA256 identities bind raw document bytes. Paths come only from serialized typed
protocol fields; source arrays compare whole, label order is normalized and source
order preserved. Equal content never proves a still-valid approval or mandate.

No new Cargo dependency, operator policy/limit, wait, IO, clock, credential or effect.
Decoder/canonicalizer errors retain the existing safe `Error`. Internal object-shape
mismatch uses static `protection-comparison-shape-invalid`; no partial report is
returned. There is no output publication in this pure library function. Hashes and
paths remain metadata requiring appropriate recipient permissions.

Three Rust integration tests are written, **not executed**: both domains;
transport/labels/source ordering; rejection on either side with recovery.
Byte-bound policy/function/error review is retained in `source-review.json`,
without implying Cargo/Rustdoc or independent acceptance.

## GLM and resumption

One authorized SKYNET call finished `stop`, no client cap or effort imposed:
7,656 input / 30,301 output / 37,957 total reported tokens, 162.217 seconds.
Requested/reported ID `glm-5.2` does not establish backend version. Its Python
proposal is historical after user correction, not a review of final Rust. Codex
reuses the content/intent/transport distinctions, whole-source arrays and domain
cases in Rust; no unchecked proposal is applied. Original request/answer retained.

When compilation is authorized, Codex must run the crate's `comparison` integration
target, affected targeted checks/Rustdoc, then connect shared CLI/API/operator
consumers. IA3/AC-02 protected handoffs remain prerequisites for PA-02/03. Other
session's SCALING-S2 work remains untouched. No build/restart/install, commit,
push, publication or new mission. Sponsor must still appoint evaluator and
third-party team; PA-00…13 program and final retrospective remain open.
