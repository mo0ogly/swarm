# Candidate delivery — graph and automation

Status on October 6, 2026: **candidate awaiting qualification, not published**. This note describes content verified in the D01–D04 checkout; it is not a release, engine acceptance, or proof that the shared server is running this candidate.

## Candidate contents

- graph preparation and editing with preview, cycle detection, revisions and conflicts;
- agent journal and graph-linked evidence rendering;
- automation programs, idempotent requests, scheduling, pause and recovery;
- migration/archives and disabled imported programs;
- version diagnostics separating installed CLI, active server and candidate sources;
- French and English installation, backup and rollback documentation.

D01/D02 journeys use isolated roots and browsers without real provider autonomy. Provider cost remains unknown.

## Verified installation and version

The D04 test invokes the official native installer in a temporary prefix containing spaces. It checks mode `0755`, confirms that no server starts, then reads the stable identity through:

```sh
swarm --json version
```

It also checks embedded web assets, their content-based ETag and `304` revalidation so replacing the binary cannot silently retain a stale JavaScript entry point. This isolated check does not replace the supervised post-install comparison of installed CLI, active server and candidate identities.

## Upgrade and rollback

Before a real upgrade: wait until no agent or check is active, stop the server, back up `.swarm/`, the agent directory and the matching binary together, then verify their hashes. Never run an old binary against already migrated data; restore the compatible backup/binary pair.

The D04 recipe verifies that an update with invalid metadata fails without replacing the installed binary. It then checks an old→new upgrade and atomic restoration of the saved binary with the same SHA-256, without changing unrelated sentinel state. The complete operational commands remain in [INSTALL.md](INSTALL.md).

## Delivery authorization and limits

This note authorizes no commit, push, tag, package, public image, release or shared-server restart. Supervision may install the candidate only after:

1. the public D04 check runs against the final inputs;
2. an independent reviewer examines the same candidate;
3. the engine records D03 and then D04 acceptance;
4. no active agent or check remains;
5. the backup and rollback plan are ready; and
6. shared-server installation is explicitly authorized.

After installation, read the CLI identity and server health/version again, then run the targeted navigation journey. Any divergent identity, stale asset or graph regression requires stopping and following the documented rollback.

## Evidence status

- D01 and D02: historical checks/reviews are reported accepted, with fixture limits explicit.
- D03: a host receipt is present (single discovery of the Go suite, race, vet, npm and guards all exit 0), but the inspected local evidence does not yet establish a later review/acceptance of that receipt.
- D04: worker installation/version/assets/rollback checks exit 0; public check, independent review, supervised installation and engine closure remain pending.

See the [D04 report](../D04.md) and [dossier](../D04-dossier.md).
