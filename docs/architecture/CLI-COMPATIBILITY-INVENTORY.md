# Inventaire statique des routes CLI

`tools/rust-migration/current_cli_inventory.py` lie des candidats relus aux
empreintes exactes des sources CLI déclarées et de trois dispatchers communs.
Il ne déduit ni parité comportementale, ni exhaustivité du besoin, ni acceptation.
Une route « refusée » peut être une commande invalide également dans Go.
Les 64 lignes du relevé du 28 octobre 2026 ne sont donc pas un dénominateur
permettant de calculer l’avancement de la migration. PAEA reste hors périmètre.

```sh
python3 tools/rust-migration/current_cli_inventory.py capture \
  --rows docs/plans/migration-rust/CURRENT-CLI-ROWS-20261028.json \
  --out /chemin/neuf/inventaire.json
python3 tools/rust-migration/current_cli_inventory.py check --manifest /chemin/neuf/inventaire.json
```

Le dossier parent doit exister. `capture` refuse une cible existante ; `check`
ne réécrit ni empreinte ni classement. Source changée, liaison manquante, schéma
invalide ou lien extérieur au checkout refusent. Les classements `wired`,
`partial`, `explicit_refusal`, `unknown` viennent de la revue humaine/agent,
jamais d’une preuve d’exécution automatique.

La publication synchronise un temporaire adjacent avant un lien sans remplacement.
Une erreur après ce lien peut laisser le nouveau fichier visible ; les causes de
synchronisation et de fermeture sont conservées. La double lecture des sources
ne garantit pas un snapshot atomique ni l’absence de changement aller-retour.
Les six régressions couvrent dérive, données invalides, alias externe, refus
avant publication, reprise et échec après publication. La recette permanente
`cli-static-inventory` conserve ces preuves, sans fournisseur, Root ou compilation.
Chaque manque doit ensuite être confronté au vrai contrat Go, à son remplacement
Rust éventuel et à une recette de parcours avant d’entrer dans le reste à faire.
