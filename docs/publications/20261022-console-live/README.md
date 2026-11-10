# Console live Swarm — présentation (22 octobre 2026)

Présentation de LIVE-UX-01 → 03 : tableau de bord des agents, fenêtre harnais,
télémétrie fournisseur (réception OTLP, lancement instrumenté, réglage Admin v30).

- Version consultable (privée, à partager depuis son menu) :
  <https://claude.ai/artifact/9zhRifWdcFtccKiBqEK2Pa> — export PPTX/PDF depuis la page.
- Sources dans ce dossier : `deck.json` (ordre, sections, polices) et
  `slides/*.html` (une diapositive par fichier, format Slides : éléments
  `x-icon`/`x-connector` rendus par cette application, pas par un navigateur seul) ;
  `images/` : captures réelles du binaire `swarm-rust` sur une Root de fixture privée,
  produites par `tests/rust_native_live_browser.py --token-fixture …` et
  `tests/rust_native_login_browser.py --scenario telemetry-settings`.

État au moment de la présentation : vérifications locales, sans revue
indépendante ni acceptation ; recette `live-monitor-native` 19/19 PASS, preuve vérifiée.
Détails et preuves : `docs/architecture/RUST-MIGRATION.md` § LIVE-UX-03 et
`docs/plans/migration-rust/PLAN.md`.

## English

Swarm live console presentation (LIVE-UX-01 → 03): agent dashboard, harness
window, provider telemetry (OTLP ingest, instrumented launch, Admin setting v30).
Viewable deck (private until shared, PPTX/PDF export from the page):
<https://claude.ai/artifact/9zhRifWdcFtccKiBqEK2Pa>. This folder holds its
sources (`deck.json`, `slides/*.html` in the Slides format) and the real
screenshots in `images/`. Status: local verification only, no independent
review or acceptance. Evidence: `docs/en/RUST-MIGRATION.md` § LIVE-UX-03.
