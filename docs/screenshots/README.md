# Documentation screenshots

Captured on 29 September 2026 from the local Swarm binary using an isolated,
temporary project. French images are here; English images are in `en/`.

Regenerate from the repository root after building `bin/swarm`:

```sh
node scripts/readme-screenshots.cjs
node scripts/readme-screenshots.cjs --lang=en
```

The script uses Puppeteer and Chrome (`CHROME_BIN` can select another executable).
It creates three manual tasks through the public CLI, verifies visible dependency
arrows, captures both themes, task details, the connection form and preparation.
It makes no AI calls, never uses the live mission database and stops its temporary
server afterwards. Missing roles and unconfigured providers are intentional:
these images illustrate preparation, not a completed autonomous run.

No credentials or authenticated session URLs should appear in public captures.
The English interface does not translate arbitrary user-entered identifiers.
