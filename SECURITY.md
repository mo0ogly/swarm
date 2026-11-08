# Security reports

Do not disclose credentials, private session links or mission databases in issues.
For a suspected vulnerability, contact the repository owner through the contact
options on [their GitHub profile](https://github.com/mo0ogly) to arrange a private
report. If private vulnerability reporting is enabled in the repository Security
tab, use that channel. No response-time SLA is promised.

Swarm is a local tool. The private `.swarm/web-session-*` key grants cockpit access; keep it private.
`swarm open` passes a single-use sign-in link (short saved lifetime, consumed on
first use) to the browser opener; do not copy or share it while it is valid.
If the key may have leaked, run `./swarm.sh rotate`: it replaces the key and
refuses every earlier cookie and sign-in link (Sign out alone does not revoke).
Swarm-launched agents cannot read `.swarm/` (Landlock), but the sign-in link is
briefly visible in `/proc/*/cmdline` until the browser consumes it; a stolen
link makes your browser show "already used" — if you see it unexpectedly, run
`./swarm.sh rotate`.
Agent programs can execute commands with the configured user's permissions.
`.swarm/` and agent home directories can contain credentials and source material.
Docker mounts the selected project writable and uses Linux host networking;
it is not an isolation boundary for untrusted agent code.

Use the latest reviewed revision. Before upgrading, stop missions and agents and
back up the project state as described in [INSTALL.md](INSTALL.md). Older binaries
may refuse a newer database schema. Never downgrade by editing schema metadata.
