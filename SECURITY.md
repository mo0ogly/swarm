# Security reports

Do not disclose credentials, private session links or mission databases in issues.
For a suspected vulnerability, contact the repository owner through the contact
options on [their GitHub profile](https://github.com/mo0ogly) to arrange a private
report. If private vulnerability reporting is enabled in the repository Security
tab, use that channel. No response-time SLA is promised.

Swarm is a local tool. Its session URL grants cockpit access; keep it private.
Agent programs can execute commands with the configured user's permissions.
`.swarm/` and agent home directories can contain credentials and source material.
Docker mounts the selected project writable and uses Linux host networking;
it is not an isolation boundary for untrusted agent code.

Use the latest reviewed revision. Before upgrading, stop missions and agents and
back up the project state as described in [INSTALL.md](INSTALL.md). Older binaries
may refuse a newer database schema. Never downgrade by editing schema metadata.
