# PAEA83 — promotion de référence Git sous autorité courante signée (raccord CONTINUITE-80)

Reprise du 29oct (continuité glm-resume-20261029T051714Z), service v245 réellement
actif (ELF e0a86760…, commit 86c5ab2f3+dirty), incrément 82 qualifié le matin
(recette reconcile-82 PASS 4/4). Aucun commit/push, aucun sous-agent, aucun appel
payant, mission native suspendue préservée.

## Besoin

La frontière CONTINUITE-80 demandait le raccordement protégé
admission/intention/OLD/commit/réconciliation sans jamais exposer un service
accepté avant ses prérequis réels. Les incréments 79–82 avaient produit le
protocole Git sous verrou (`run_git_reference_transaction`), le journal
d'intentions (`paea_git_intent`), l'association privée (`PaeaGitRepository`) et
le transfert qualifié (`paea_git_transfer_call`) — mais aucun consommateur
production ne publiait encore une ref sous OLD fencing.

## Réalisation (code)

Nouveau module `rust/crates/swarm-engine/src/root_store/paea_git_promote_call.rs`
(miroir du contrat settlement et de `paea_git_transfer_call`) :

- `paea_initialize_git_promotions(project_id, project_digest)` : journal d'effets
  `paea_git_promote_meta` + `paea_git_promote_effects` sur la racine service, une
  transaction explicite, journal d'intentions préexistant exigé, refus de schéma
  existant/partiel (aucune adoption implicite d'historique).
- `paea_git_promote_call(...)` : préparation unique Commit/Reconcile sur
  `Service::GitPromotion` ; attribution policy (révision + empreinte
  `paea-trust-policy-v1`) depuis la Trust réelle sous `reserve_trust(false)` pour
  Commit (Reconcile conserve la contexte historique, miroir settlement) ; témoins
  `RootBinding` des deux racines ; racines séparées exigées.
- `call_paea_git_promote(...)` : sous `with_paea_action` — permission/service/
  opération exacts, liaisons physiques revérifiées, plan d'identité courant
  (V1/V2, stamp, scope), rôle d'effet courant sous tx IMMEDIATE projet +
  réservation Trust, Biba I2 Invoke, séparations refusées au Commit, fraîcheur
  policy au Commit. Commit : autorité humaine non expirée ; intention durable
  réservée AVANT tout Git ; replay sans Git si effet identique (récupération de
  crash) ; custody + `inspect_for_effect` (candidat courant réel sous l'écrivain
  projet) ; `run_git_reference_transaction` avec décision sous verrou Git (OLD
  exact, création si OLD absent, ref déjà au candidat → Abort+enregistrement,
  OLD différent → `paea-git-promote-old-changed`) ; OID réellement observé ;
  effet enregistré ; COMMIT incertain conservé
  (`paea-git-promote-commit-uncertain`). Reconcile : lecture pure du journal,
  `ReconciledNotCommitted` explicite sans aucun Git.
- Accesseur `PaeaGitRepository::directory_path()` (pub(super)) : chemin
  /proc fd du répertoire retenu pour la commande Git durcie ; jamais un droit
  d'effet ni un chemin opérateur.

`cargo check -p swarm-engine` 0 erreur/0 warning du module ; `cargo doc
--no-deps` propre.

## Qualification (tests + recette)

`tests/paea_git_promote_call.rs` : harnais miroir de `paea_git_transfer_call.rs`
sur la fixture native vivante writer78b (SOURCE, racine /tmp/swarm-model83-ln5x265w,
reçu natif lié à l'inode réel). Le harnais exécute l'ordre métier réel :
transfert qualifié (incrément 82) → promotion, sans copie ni raccourci — le
diagnostic initial « update-ref refuse nonexistent object » a établi que
l'objet candidat doit être importé avant l'écriture de ref. Deux défauts de
harnais corrigés avec cause établie (aucun défaut produit) : (1) grant de
transfert signé sur la révision Trust post-config réelle (plus 1 révision) ;
(2) la préparation Reconcile réutilise la contexte attribuée du Commit
(`commit_prepared.context()`, miroir `historical_context = request.context()`
du test settlement) — sinon digest canonique divergent et refus
`paea-git-promote-intent-conflict`.

4 tests `--ignored --test-threads=1` PASS sur fixture vivante :
commits_once_and_records_effect (ref créée au candidat sous OLD fencing + effet
enregistré, `replayed=false`, ref bare vérifiée au candidat),
replays_recorded_effect_and_reconciles (replay commit `replayed=true` puis
reconcile `Recorded{replayed:true}`), reconcile_before_commit_reports_not_committed
(négatif explicite, aucun Git), refuses_other_service_root_without_effect (refus
`paea-git-promote-root-binding`, rien consommé).

Recette permanente `tools/verification/recipes/paea-owned-git-promote.json` :
étape cargo test des 4 tests + Rustdoc `-D warnings`, fixtures writer78b/current71c,
sans watchdog global. Première exécution FAIL `recipe-observable-assertion-missing`
sur un artefact de présentation (`--nocapture` intercale la ligne
GIT_REFERENCE_TOOL entre « ... » et « ok » ; les 4 tests réels étaient passés,
preuve intégrale conservée dans promote-83/), commande corrigée puis
promote-83-attempt2.

## Documentation

`docs/architecture/PAEA-GIT-TRANSACTION.md` et `docs/en/PAEA-GIT-TRANSACTION.md` :
nouvelle section « Consommateur gardé de promotion de référence » décrivant le
contrat réel (initialisation, préparation, consommation, outcomes, erreurs,
frontière CONTINUITE-80 restante) et la recette.

## Limites non modifiées

Aucun service accepté, aucune séparation UID, aucun enrôlement humain, aucune
approbation indépendante, aucune clôture globale. La promotion n'est joignable
que par l'API bibliothèque qualifiée en fixtures ; le raccordement au parcours
public (CLI/API/UX) reste un incrément séparé sous les prérequis réels.
Installation locale ./swarm.sh et notes FR/EN traitées dans la suite de ce
dossier de continuité.
