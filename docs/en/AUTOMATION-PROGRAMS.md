## C04 web and CLI administration

In the cockpit, switch to expert mode and open **Programs**. Enter a name,
existing mission, type, IANA timezone and local date. Select **Preview without
effect**, inspect UTC times, profile, limits, authorization and cost (unknown
when the provider does not report it), then **Create disabled**. Creation starts
no agent. **Enable**, **Pause** and **Archive** require the current revision;
a conflict requires reloading. Archiving is permanent. **Cancel this wait** is
allowed only before claiming or an effect. A processed occurrence is not a
validated mission.

Text help opens a modal, supports Escape and restores focus.
Program states, actions, authorization reasons and errors follow the cockpit
language; machine codes remain unchanged in JSON responses for scripts. In
English, for example, an invalid timezone reports `explicit IANA timezone
required; UTC is accepted`.

**Versioned operational settings** exposes all eight lease, deadline, occurrence and external
event settings. Saving adds a revision and actor; concurrent writes against the
same revision are rejected without overwriting either the other operator's saved
values or the visible draft. Reload, review the new revision, then deliberately
apply the edit again. It raises no budget and changes no active attempt.

```sh
swarm automation help
swarm --lang en automation help
swarm automation list
swarm automation show PROGRAMME
swarm automation preview --input programme.json
swarm automation create --input creation.json
swarm automation enable PROGRAMME --input revision.json
swarm automation pause PROGRAMME --input revision.json
swarm automation archive PROGRAMME --input revision.json
swarm automation cancel OCCURRENCE --input revision.json
swarm automation params show
swarm automation params apply --input reglages.json
```

`creation.json` contains `schedule` (the previewed document) and the returned
`preview_token`. `revision.json` contains the object's `expected_revision`.
Settings contain `schema_version: 1`, `expected_revision` and `values` with all
eight current fields. CLI and HTTP adapters share one service; local HTTP retains
session and CSRF protection, without a new public port or external connector.
The authorized host still calls the tick: this page installs no autonomous poller.

## Screenshots and independent review

The four FR/EN dark/light screenshots are declared PNG files under `docs/screenshots/` in control inputs. The engine validates hashes, dimensions and sizes before review. A report link alone attaches no image. Codex receives the bytes through `--image` in a private temporary directory removed after process exit; Claude receives them in the structured message. Review tools remain disabled and the budget unchanged. Unverified API/provider adapters are refused. Fixtures do not prove subscription availability.
