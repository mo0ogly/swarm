# Window identity and text hierarchy

The [shared design system](../UI-DESIGN.md) uses the unchanged canonical asset
`web/swarm-logo.png`. Dialog header titles show this decorative logo through
`swarm-design.css`, including dialogs created on demand for help, version and
settings. New dialogs must retain `dialog > header` or `dialog > form > header`
with an `h2`, an `aria-labelledby` reference, content and an accessible close action.
The logo does not replace the title or add a redundant accessible name. Its
reserved space is 44 px, reduced to 34 px on narrow screens.

Processes, trusted actions and agent sessions also show the brand. The cockpit
keeps it visible in its mobile header and when the desktop navigation is
collapsed. HTML tabs use only the canonical PNG favicon. The embedded log editor
keeps its full height; its containing window carries the identity. The transient
OIDC window retains its waiting message and shows the logo.

Dialog titles use local Georgia, subheadings use the readable link token, and
body text uses 1.65–1.7 line spacing. Bold text, quotations, inline code and
highlights use existing semantic pairs. Notices retain their information,
success, warning or error meaning, written labels and borders. Headings within
notices inherit the state color. Do not arbitrarily color user text or render
untrusted AI HTML to provide rich text.

Dense instructions and logs have their own [reading contract](DENSE-TEXT.md),
shared source/reading controls and dedicated interaction recipe. Window identity
checks alone do not verify document structure, exact source or log navigation.

Long informational dialogs start at the title. `SwarmShell.focusWindowStart(dialog)`
focuses the labelled title programmatically and resets the dialog scroll position.
Forms keep native input focus. Escape and close actions restore the opener focus.

Run `node tests/window_identity_ui.cjs BINARY EVIDENCE_DIRECTORY` and the existing
design recipe for complete states, forms and interaction checks. The focused
recipe checks FR/EN, both themes, desktop and mobile widths. It opens static
dialogs for geometry without submitting their forms; separate interactions
exercise the connection, version, help and method controls. No agent or paid
provider is needed. The [increment report](../reports/window-identity-20261017.md)
distinguishes local verification from a running installation and historical
training recordings.

Source integrity uses the current manifest chain. Run
`python3 tools/refresh_ui_source_manifest.py`, then, if the engine manifest exists,
`python3 tools/verification/refresh_engine_source_manifest.py`. The Go qualification
test selects the engine snapshot when present, rather than the UI parent alone.
Run `go test ./internal/engine -run TestGraphDeliveryD03QualificationManifest -count=1 -timeout=0`
to check the selected snapshot. Regeneration records current inputs, including
existing unrelated edits; it never renews historical acceptance.
