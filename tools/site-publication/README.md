# Article publication and sitemap

The maintained Swarm static-site publisher regenerates `sitemap.xml` from the assembled candidate **before activation**. The addition preparer and private editorial site builder call the same generator. A failed generation/write, stale expected manifest or postactivation public mismatch makes publication fail; the last case restores the prior release and sitemap.

`publication_sitemap.py` includes only six known presentation routes and bilingual `articles/swarm-*/{fr,en}.html` pairs linked from their respective public homepages. Pages must exist, match their language and not be noindex. Declared canonicals must match. Private/review/unlinked HTML is not discovered by recursive crawling. This eligibility check is not access control; new public route categories need an explicit contract change and tests.

## Maintained entry

Installed outside the public release on the existing VPS:

```sh
python3 /var/www/swarm-agentic/publication-tools/current/deploy_article.py CONFIG.json
```

Read-only live consistency check, **on that VPS**:

```sh
python3 /var/www/swarm-agentic/publication-tools/current/deploy_article.py --check-current
```

This is not a localhost preview command. The production root is an existing deployment contract, not a configurable engine execution policy. The initial installed tool version is `publication-c19289ede93d`; both installed source hashes match these maintained files. No content release, application restart or Nginx reload was needed to install the tools.

The reviewed JSON input binds `baseline`, archive/hash, final file `inventory`, `deltaInventory`, `articleSlug`, `releaseId`, explicit `operation` (`add`, `update`, `remove`), `removedPaths` and `sitemapGeneratorSha256`. Baseline includes public files, current release, configuration hashes, service identities and established LIA coexistence witnesses. Missing generator identity refuses before staging. Python optimization (`-O`) is forbidden: publication guards include assertions.

`prepare_article_addition.py SPEC.json` prepares the existing private editorial package from its verified USB baseline and bundle. It automatically generates the sitemap and records the generator hash before binding the archive. Its SPEC/evidence paths follow the existing editorial directory convention; it does not deploy. Private author bundles, editorial archives, operational baseline JSON and media remain excluded from Git under `docs/communication/`.

An explicit retirement must acknowledge all removed files of the named article, update home links (including any featured link), and provide the reviewed archive/final inventory. The deployer supports this operation; no real article was created or retired by this change. This does not introduce a public retirement UI or infer editorial authorization.

Under the existing deployment lock, the publisher clones the exact baseline, applies the scoped delta/removals, regenerates and validates the sitemap, then checks the complete final inventory before switching the current symlink. Forgotten sitemap changes are not silently accepted: regeneration makes a stale expected manifest fail. Once active, full HTTPS hashes, configuration/service identities and LIA witnesses are checked. Rollback preserves both original and secondary errors. No new timeout, retry budget or execution policy is added.

`install_tools.py ARCHIVE SHA256 BASELINE` is the verified **initial** installation path: it refuses an existing tools tree, validates a three-file package, runs the installed read-only check and selects its version. An upgrade requires a separately reviewed package/install step; do not overwrite current tools, remove locks or replace the public webroot. Historical incoming copies and unrelated specialized deployers are not rewritten. A root administrator can bypass this maintained entry; no OS confinement is claimed.

## Verification

```sh
python3 tools/site-publication/test_publication_sitemap.py
```

Nine named tests exercise valid add/retirement, private exclusion, stale sitemap, generation/write refusal and recovery, language/noindex conflict, generator identity, removal acknowledgement and postactivation rollback. Service/network checks are explicit doubles; archive assembly, filesystem generation, activation pointers and rollback logic run in isolated roots. A retained original publisher fixture reproduces the previous sitemap-delta rejection. These tests never create or retire a production article.

A fresh full site build and the installed live check match the current public 60-URL sitemap. All 420 public files, release, configuration/service identities and three established LIA witnesses remained unchanged during installation. This is not future production add/remove acceptance, global LIA validation or a Google indexing guarantee.

[Notes françaises](README.fr.md)
