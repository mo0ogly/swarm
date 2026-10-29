# PAEA85 — consommateur CLI public du vérificateur PA-09

Continuité glm-resume-20261029T064841Z, service v247 actif (ELF
`9caabf6c…`), incrément 84 clos (recette PASS/VERIFIED, installation vérifiée).
Aucun commit/push, aucun sous-agent, aucun appel payant, mission native
suspendue préservée.

## Besoin

Après l'incrément 84, le vérificateur PA-09
(`RootStore::paea_reconcile_effect_journals`) n'avait AUCUN consommateur public :
l'inventaire des consommateurs (web = 6 endpoints purs, CLI = aucun appel
d'effet) montrait que le premier parcours réel manquant était un accès CLI en
lecture seule. Ce consommateur est exigible sans les prérequis ouverts
(séparation UID, enrôlement humain, approbation indépendante) car il n'exécute
aucun effet, n'accorde aucune autorité et ne transporte rien publiquement ; il
fait progresser PA-08 (parcours CLI réel) et PA-10 (CLI isolé) sans les
fabriquer.

## Réalisation (code)

- **Garde de séparation** (`paea_effect_reconcile.rs`) :
  `paea_reconcile_effect_journals` exige maintenant deux racines physiquement
  distinctes via `bind_root()` + `instance()` (identité répertoire + base) ;
  coïncidence → `R::Refused("paea-effect-reconcile-separate-ledger-required")`.
  Un même stockage ne peut pas fournir les deux côtés du rapprochement, sinon un
  stockage compromis fabriquerait ses deux témoins. Fermetures appariées via
  `finished(result, close)`.
- **Binding old_oid** (défaut de couverture 84 repéré par warning dead_code) :
  l'`expected_old_oid` de l'intention durable doit égaler l'`old_oid` observé
  avant promotion consigné dans le reçu → code
  `paea-effect-reconcile-git-old-oid` : la transition promue doit partir de
  l'état réservé.
- **CLI** (`swarm-cli/src/paea.rs`, dispatch `lib.rs`,
  `#[cfg(target_os = "linux")]`) : `swarm-rust paea reconcile-effects LEDGER
  --root RACINE --json`. Garde d'usage stricte (`pos.len()==3`, options
  d'exécution `--server/--task/--input/--output` refusées sans ouvrir de
  racine), deux ouvertures `RootStore::open(path,false)`, rapport JSON v1
  publié via `write_output` seulement si TOUTES les fermetures réussissent ;
  erreurs appariées `CliError::RootCleanup{primary, cleanup}` ; rapport d'un
  rapprochement réussi retenu si la fermeture du grand livre échoue. Aides
  FR/EN ajoutées.

## Qualification

- `cargo test -p swarm-engine --test paea_effect_reconcile -- --ignored
  --test-threads=1` : **4 passed** (nouveau test
  `reconcile_refuses_self_reconciliation_of_one_root` ; harnais settlement des
  2 tests existants adapté à la garde avec racine service distincte, git
  restant `Uninitialized` dans ces phases).
- `cargo test -p swarm-cli --test paea_reconcile` : **3 passed** — rapport
  explicite `uninitialized` sans mutation (octets DB avant==après, FR+EN),
  refus de racine partagée (code exact en stderr), refus des options
  d'exécution sans ouverture.
- Recette permanente `tools/verification/recipes/paea-effect-reconcile.json`
  étendue : inputs `rust/crates/swarm-cli/**/*`, nouvelle étape CLI
  (`effect-reconcile-cli-consumer`), 4 tests engine attendus. Exécution :
  `docs/plans/actions-confiance/paea/evidence/20261029-fixture-regen-mvfdts57/
  reconcile-effect-85` (statut et intégrité à relever dans PROGRESS.md).
- Rustdoc `-D warnings` propre (engine + CLI).

## Documentation

`docs/architecture/PAEA-GIT-TRANSACTION.md` et `docs/en/PAEA-GIT-TRANSACTION.md`
: garde de séparation, binding old_oid, contrat du consommateur CLI, recette à
jour (4+3 tests).

## Limites non modifiées

Le consommateur CLI reste une qualification locale : il n'établit ni
authenticité des journaux (`verified=true` = correspondance recalculée), ni
séparation UID, ni enrôlement humain, ni approbation indépendante, ni
raccordement web/UX du vérificateur, ni PA-09 complet (ancres externes, revue
indépendante, observateur séparé restent ouverts). Aucun service accepté ;
PA02/03 partiels, 0/17, PA13/RETEX final ouverts.
