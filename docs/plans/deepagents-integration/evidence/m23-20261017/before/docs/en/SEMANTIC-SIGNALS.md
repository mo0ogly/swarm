# Shared context seals and semantic signals

A reusable presentation component inspired by a wax seal at the foot of a
parchment. **The component and isolated catalogue are delivered; existing
application screens do not load them yet.** A seal grants no rights, signs no
document and certifies no identity. [Français](../SEMANTIC-SIGNALS.md).

## Shape and meaning

The central imprint represents the memory type. An irregular rim, double ring
and subtle shadow suggest wax. Adjacent text separates memory type, usage
protection and identity state. Owner, author and reference remain text when
supplied. Missing data produces **Rights not established** and **Identity not
attested**. Gold is a presentation accent; it carries no authority.

All colours use existing Swarm semantic tokens. Shapes and labels accompany
colour, with 29 bilingual signals and descriptions:

| Family | Signals and meaning |
| --- | --- |
| Memory, 5 | Unknown type; experience; procedure; framing; extractive summary. |
| Concept, 1 | Facts, a semantic-memory marker, **not a new persisted type**. Current catalogue types remain `experience` and `procedure`. |
| Version, 5 | Unknown; active in the catalogue; historical; withdrawn; unavailable. Active does not mean authorized. |
| Protection, 5 | Not established; pending; authorized for the checked content/recipient/duration; denied; revoked. |
| Identity, 5 | Not attested; pending; attested; suspended; revoked. Attestation grants no implicit rights. |
| Actor, 3 | Person; agent; service. Categories do not prove authentication. A service is not human approval. |
| Usage, 3 | Read; summarize; transmit, as distinct permissions. |
| Attribution, 2 | Owner: responsibility/management; author: origin of the contribution. |

Withdrawal and revocation preserve historical authorship, text and receipt.
A refusal reason must remain visible in the screen using the seal.

## Sources and build

Canonical sources are [signals.js](../../frontend/signals/signals.js),
[signals.css](../../frontend/signals/signals.css) and
[catalogue.html](../../frontend/signals/catalogue.html), with its local JS/CSS.
The library has no dependency; SVG geometry is built through DOM APIs. Labels,
descriptions and presentation states are local and bilingual.

```sh
node frontend/build-signals.mjs /tmp/my-context-seals
```

The parent must exist and the output must be new and outside the checkout.
The build copies five component/demo files, two existing theme sheets and the
local logo, then records hashes in `build-manifest.json`. Existing outputs and
paths resolving into the checkout, including symlinks, are refused. This avoids
changing embedded resources while the identity workstream is active. It neither
rebuilds Go nor deploys a page. The standalone entry is `catalogue.html`; it uses
no application API.

## Reuse by the identity workstream and other views

Load `signals.css` after theme tokens and `signals.js` as a local script before
the consumer. For embedded views, deliver these two assets through the agreed
frontend path, rebuild, then verify the affected screen. Reuse the component;
do not duplicate icons or edit generated copies. Shipping the demo is optional.

```js
// In the view's local JS after loading the shared component.
const seal = SwarmSignals.seal({memory: 'procedure', lang: 'en'});
container.append(seal); // Identity and permission remain unknown by default.
const read = SwarmSignals.badge('usage-read', {lang: 'en'});
const unknown = SwarmSignals.state('identity', undefined, {lang: 'en'});
```

`seal` also accepts `protection`, `identity`, `owner`, `author`, `reference`.
The last three are text, never authority. Memory values are `experience`,
`procedure`, `dialogue`, `summary`, `knowledge`; an unrecognized type is unknown.
Protection states are `unknown`, `pending`, `allowed`, `denied`, `revoked`.
Identity states are `unknown`, `pending`, `verified`, `suspended`, `revoked`.

`badge(id, {lang, iconOnly})` and `state(family, value, options)` create fresh DOM.
Families are `identity`, `protection`, `version`. Missing state means unknown;
unsupported IDs/states/languages throw `RangeError`, never a successful-looking
replacement. Languages are `fr` (default), `en`; `document` may supply a DOM.

Consumers must map states to verified engine data. Never derive `verified` from
a name, provider, ID or Admin role, or `allowed` from authentication, a hash,
an active version or historical approval. The
[target memory contract](../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md)
(French) remains separate; this component does not implement its authorization.

## Accessibility and verification

Badges have visible text; decorative SVGs are hidden from assistive technology
and keyboard focus. `iconOnly` supplies `role="img"` and a localized accessible
name. The seal retains visible labels. Fields use `textContent`; provenance
cannot insert HTML. Descriptions are visible in the legend and available as
badge titles. No CDN, remote font, generated bitmap, inline script/handler or
application network access is used.

```sh
node tests/semantic_signals_ui.cjs /tmp/my-context-seal-check
```

The recipe uses local Chrome (`CHROME_BIN`, else `/usr/bin/google-chrome`), a
temporary build and a test static server; browser/server are closed afterwards.
It checks states, reset, hostile text, FR/EN, both themes, widths
1440/1024/760/390/320, badge contrast, keyboard/focus and console/network errors.
Receipts and screenshots are retained. Each real screen still needs integration
testing; this isolated check does not establish engine permission enforcement.
