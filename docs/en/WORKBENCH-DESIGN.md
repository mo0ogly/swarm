# Integrated cockpit and trust-screen design

The [adopted mockups](../design/README.md) guide the real Rust cockpit and existing
forms. No preview adapter, sample transaction or provider integration is shipped.

**Graph / Lanes** is a presentation preference stored locally as
`swarm-conduite:view:v1` (two enum values). Storage refusal is visible and does not
block reading. No Work data or identifier is stored with this preference.

Lanes group actual tasks by `plan_role`, falling back to `owner`, and by received
state. Columns are recorded states, not business phases inferred from titles.
Missing roles/observations remain explicit. Search, role filtering and selection
are local. The inspector reads actual Work dependencies and projected reasons;
returning to Graph retains its existing editing controls and authorization gates.
Changing Work clears old cards before reading. Failure never becomes an empty
success; recorded-only cards keep their unavailable-status notice. A card/state
does not grant launch/acceptance rights or establish current activity.

Preparation keeps the real dialogue, document, questions, adoption and engine
operations on `/prepare.html`; the wider conversation and distinct document panel
stack on mobile. Native settings keep exact values, scopes, previews, history and
refusals. Mockup numeric rules and values do not become product policy/defaults.

Security belongs to Configuration; Audit and evidence belongs to Follow-up.
`/trust-actions.html?view=security` and `?view=audit` compose the existing Trust
page. Security tabs group actual Biba, Bell-LaPadula, role conflict, identity and
key controls. Arrow/Home/End keys select tabs; Escape retains native dialog
behavior. Existing IDs/listeners, I3 gates and actual preview/application APIs stay
in force. Audit presents existing identity inputs, transaction audit and public
verification. Empty/refused input never becomes a favorable example verdict.
Tokens never enter navigation links; existing BLP limits and simulated Trust
effects remain explicit. No sample identity, key or transaction is introduced.

Implementation: `web/swarm-workbench.js`, `web/trust-workbench.js`, shared CSS and
private assets in `service/native_admin.rs`. Regression:
`python3 tests/rust_native_cockpit_browser.py --binary BINARY --design --output
FRESH_EVIDENCE`. This uses public CLI fixtures and real Rust HTTP, without business
provider execution. Named client faults remain separate from server observations.
See the [integration record](../plans/DESIGN-INTEGRATION-20261108.md) for source,
installation and evidence limits. Local checks are not independent acceptance or
global BLP qualification. Mockups remain historical design references.

For partial reads, recorded `accepted`, `blocked` and `todo` codes receive FR/EN
labels. The original refusal and notice remain visible; this presentation does
not establish independent acceptance or restore native status calculation.

Cockpit links to Security/Audit and both Admin links retain language, theme and
the local limits fragment. Trust applies the incoming theme only for `etat` or
`sombre`; outgoing links reread the preference on click. These presentation
parameters carry no token.
