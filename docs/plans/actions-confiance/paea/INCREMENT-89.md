# PAEA89 — consommateur CLI gardé de promotion de référence (raccord CONTINUITE-80)

Continuité glm-resume-20261029T064841Z après l'incrément 88 (consommateur CLI
du transfer call, installé v251). Objectif actif : prochain incrément utile du
plan PA00–PA13, uniquement GLM, sans commit/push ni notification
intermédiaire. La limite explicite de l'incrément 83 (« la promotion n'est
joignable que par l'API bibliothèque qualifiée en fixtures ; le raccordement
au parcours public (CLI/API/UX) reste un incrément séparé sous les prérequis
réels ») est le besoin repris ici pour la partie CLI.

## Choix de conception

Même motif interactif que l'incrément 88 : témoins mémoire vivants pendant le
provisionnement hors bande de la permission exacte, frame privée stdin
`swarm-trust-source-v1\n<pin64>\n<password>`, déclaration enrôlée de
l'opérateur. Différences contractuelles : l'opération est un argument
protocolaire explicite (`commit`|`reconcile`) et le résultat est un outcome
différencié — `recorded` (intent_id, action_digest, target_ref, new_oid,
replayed) ou `reconciled_not_committed` (intent_id, target_ref), le résultat
négatif explicite jamais rendu comme succès vide. Pour Commit, la transaction
de ref réelle avec OLD exact a lieu ; le parcours métier impose l'ordre transfert
qualifié préalable (import du pack candidat) → promotion, sinon Git refuse avec
un objet inexistant. Le test le reproduit : la fixture consomme d'abord le
transfer call réel, puis la promotion CLI.

## Réalisation

- `rust/crates/swarm-cli/src/paea_promote.rs` (nouveau) : ouverture writables
  service (pos[3]) puis projet (pos[4]), préparation
  `paea_git_promote_call` avec l'opération demandée, ligne publique
  `paea-promote-prepared` (service repository.promote, opération, root_digest,
  action_digest, permission exacte, authority:false, restart_reusable:false),
  frame zeroizing et refus `paea-promote-call-frame`/`-input` avant toute
  ouverture de déclaration, consommation `call_paea_git_promote`, ligne
  `paea-promote-result`. Fermetures explicites et causes retenues (RootCleanup
  apparié) sur toute issue.
- `rust/crates/swarm-cli/src/lib.rs` : variante `CliError::PaeaPromote`
  (code/Display/source via `PaeaGitPromoteCallError`), module, dispatch
  `paea promote-call`, USAGE + HELP_FR/HELP_EN.
- `rust/crates/swarm-cli/tests/paea_promote.rs` (nouveau) : harnais deux
  phases réel avec transfert qualifié préalable consommé par l'API bibliothèque
  (permission signée courante, révision Trust post-configuration), promotion
  CLI ensuite ; assertions rev-parse sur le ref réel du bare.
- Documentation : `docs/architecture/PAEA-GIT-TRANSACTION.md` §
  « Consommateur CLI gardé de promotion (deux phases interactives) » + miroir
  EN § "Guarded CLI promote consumer".
- Recette `tools/verification/recipes/paea-transfer-call-cli-rust.json`
  étendue à 5 étapes (2 transfer + 3 promote).

## Défauts trouvés et corrigés pendant la vérification (replan)

1. Indice d'opération : la première exécution refusait en Usage car
   `operation()` lisait pos[1] (« promote-call ») au lieu de pos[2] ; établi
   par reproduction directe du binaire (Usage affiché), corrigé.
2. Indices racines : la seconde exécution refusait en `path` NotFound car
   `run()` reprenait les indices du transfer call (sans l'argument OPERATION) ;
   correction service=pos[3], project=pos[4], work/child/pin=pos[5..7],
   déclaration=pos[8]. Les deux défauts sont du même motif que l'incrément 88
   (indices de grammaire) et ont été trouvés avant toute instrumentation :
   reproduction binaire directe d'abord.

## Preuves

- `cargo test -p swarm-cli --test paea_promote -- --ignored` → 3 passed :
  commit une fois (replayed:false, new_oid = candidat, rev-parse du ref réel),
  réconciliation négative avant commit (reconciled_not_committed, ref intact),
  refus frame sans consommation (trust-actions.json inchangé, aucun stdout).
- Recette 5 étapes PASS, preuves
  `evidence/20261028-acquisition81-mvfdts57/glm-resume-20261029T064841Z/
  recipe-paea-transfer-call-cli-89`, verify → integrity VERIFIED (08:45 UTC).
- Help FR/EN et USAGE listent `paea promote-call` (vérifié dans le binaire
  reconstruit).

## Limites restantes

Le raccord web/API de la promotion et du transfer reste ouvert (motif inc
85/86 : CLI d'abord, web ensuite). Aucun commit/push ; installation locale
dans la suite immédiate du même incrément. Séparation UID, enrôlement humain,
approbation indépendante, PA-09 complet, E3, revue indépendante et acceptation
globale restent ouverts ; ils ne sont pas fabriqués ici.
