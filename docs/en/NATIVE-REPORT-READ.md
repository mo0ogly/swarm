# Native report reading

Status: candidate settings; reader/API/CLI and delivery remain incomplete.

Settings group47 introduces two distinct caps in the shared engine configuration
and Admin catalogue: report_preview_max_bytes limits retained display bytes;
report_verification_max_bytes limits complete attributed private-report hashing.
The initial preview262144 exists only in engine-settings.json. The initial
verification choice is explicit null, unlimited. No runtime fallback is added.

Historical absence or partial groups refuse consumption. Null removes only its
specific cap. Zero retains no file text, or rejects nonempty full verification.
The shared engine owns canonical decimal, negative and platform-representation
validation. API, CLI and the Admin dialog must preserve the same validation,
revision history, nonmutating preview and application readback. This independent
group never implicitly adopts missing unrelated execution policies.

The planned reader allows contained public docs or the private report attributed
to the selected current task attempt. Full SHA verification cannot succeed on an
excerpt. Planning-context limits are separate. Access/source/close errors must
never become empty text or partial success. No agent, spending, acceptance or
restore authority follows. These reader outcomes still need implementation and
verification; settings tests alone do not prove them.

~~~mermaid
flowchart LR
    Defaults[Versioned initial configuration] --> Settings[Persisted shared settings]
    Admin[Admin modal / API / CLI] --> Preview[Validate and preview]
    Preview --> Apply[Fresh revision and checked application]
    Apply --> Settings
    Settings --> Caps[Explicit preview and verification caps]
    Caps -. reader pending .-> File[Attributed regular file]
    File -. complete SHA when required .-> View[Display text is not proof acceptance]
~~~
