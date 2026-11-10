# Matrice de migration : entrées et premiers comportements

> **Instantané historique, non représentatif de l’état actuel.** Relecture du24octobre1454 : ce document contient des états devenus faux (watch/installation) et des comptes de présence qui ne mesurent pas la charge restante. Consulter la [situation actuelle du plan](PLAN.md#situation-actuelle--24-octobre-correction-de-pilotage1454). Les tableaux suivants sont conservés pour leur provenance ; leur régénération doit être requalifiée avant réutilisation de taux ou de délais.

## Actualisation196 — travaux parallèles intégrés

[Matrice intégrée](evidence/20261022-parallel-196/matrix-integrated.json). **83 opérations CLI sur10 familles sont désormais détaillées**, en plus des137 entrées publiques ; ces niveaux se recouvrent. Les93 chemins HTTP ont leurs clauses/consommateurs/écarts et27 originaux Go conservés, contre7 sources de routes empreintées dans194. Aucun pourcentage global ni parité globale acceptée.

Les38 opérations supplémentaires comprennent19 entrées au même nom,5 équivalents renommés/partiels et14 sans opération native identifiée. Leur outil a été requalifié après le seul ajout de queue dans trois chaînes d’aide ; les fonctions de dispatch sont inchangées.

| Référence | État | Consommateur Rust candidat | Reste |
|---|---|---|---|
| `work create` | ENTRY_PRESENT_PARITY_OPEN | work create | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `work list` | ENTRY_PRESENT_PARITY_OPEN | work list | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `work show` | ENTRY_PRESENT_PARITY_OPEN | work show | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `work update` | ENTRY_PRESENT_PARITY_OPEN | work update | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `task add` | ENTRY_PRESENT_PARITY_OPEN | task add | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `task update` | ENTRY_PRESENT_PARITY_OPEN | task update | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `agent attach` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent context-preview` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent preview | Équivalent partiel ; schéma et comportement à rapprocher. |
| `agent history` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent list` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent logs` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent preflight` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent preview (not Go preflight DTO) | Équivalent partiel ; schéma et comportement à rapprocher. |
| `agent prepared` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent reconcile` | ENTRY_PRESENT_PARITY_OPEN | agent reconcile | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `agent resume-dialogue` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent resume-dialogue-preview` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent resume-launch` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `agent show` | ENTRY_PRESENT_PARITY_OPEN | agent show | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `agent start` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | agent prepare + agent run (two distinct operations) | Équivalent partiel ; schéma et comportement à rapprocher. |
| `agent stop` | ENTRY_PRESENT_PARITY_OPEN | agent stop | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `providers cooldown clear` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `providers cooldown show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `providers init` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | providers config init | Équivalent partiel ; schéma et comportement à rapprocher. |
| `providers relay decide` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `providers relay show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `providers show` | RENAMED_PARTIAL_CONTRACT_PARITY_OPEN | providers config show | Équivalent partiel ; schéma et comportement à rapprocher. |
| `pricing estimate` | ENTRY_PRESENT_PARITY_OPEN | pricing estimate | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `pricing list` | ENTRY_PRESENT_PARITY_OPEN | pricing list | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `pricing save` | ENTRY_PRESENT_PARITY_OPEN | pricing save | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `quotas apply` | ENTRY_PRESENT_PARITY_OPEN | quotas apply | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `quotas preview` | ENTRY_PRESENT_PARITY_OPEN | quotas preview | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `quotas show` | ENTRY_PRESENT_PARITY_OPEN | quotas show | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `budget apply` | ENTRY_PRESENT_PARITY_OPEN | budget apply | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `budget preview` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `budget show` | NO_NATIVE_NAMED_OPERATION_REPLACEMENT_OPEN | — | Parcours CLI à raccorder ; composants internes à rapprocher. |
| `task-model apply` | ENTRY_PRESENT_PARITY_OPEN | task-model apply | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `task-model preview` | ENTRY_PRESENT_PARITY_OPEN | task-model preview | Parité des entrées/sorties, effets, refus et reprise à vérifier. |
| `task-model show` | ENTRY_PRESENT_PARITY_OPEN | task-model show | Parité des entrées/sorties, effets, refus et reprise à vérifier. |

P01 progresse :885 sources Go actuellement présentes et mod/sum sont archivés, dont71 fichiers existants modifiés et53 ajoutés depuis553593a. Ce n’est pas encore une référence approuvée ni un inventaire des ressources/graphe/overlays. Source01 initiale est limitée ; source15 est la preuve courante, après reproduction/correction du cas d’ajout concurrent de fichier et liaison du publisher importé.

Le candidat natif196 ajoute mission queue, observation cohérente des tours/blockers, séquences décimales exactes, refus sans mutation et restauration. Six tests Rust, Clippy/Rustdoc moteur/CLI et recette publique PASS ; aucune acquisition/libération/FIFO appliquée n’est déduite. Les13 tests des collecteurs et la garde de dérive d’inventaire sont distincts des tests métier. Les autres45 opérations ci-dessous conservent leur détail ; le JSON matrix12 les réobserve sur la CLI196.

---

22 octobre2026, propriétaire Codex. Référence Git553593a ; changements Go non committés non encore figés. Ce tableau remplace les listes narratives pour ce périmètre, pas les preuves historiques.

[JSON et sources exactes](evidence/20261022-pilotage-195/matrix04/matrix.json) · [Recette permanente](evidence/20261022-pilotage-195/permanent05/result.json).

**137 entrées publiques** :44 noms CLI et93 chemins HTTP. **45 opérations examinées séparément** :11 mission et34 planning. Ne pas additionner ces deux niveaux pour calculer un pourcentage ; ils se recouvrent. Aucun comportement n’est ici accepté en parité globale. Les autres sous-commandes, méthodes HTTP, parcours UI, données, erreurs et reprises restent à détailler.

## États et preuves

- `ENTRY_PRESENT_PARITY_OPEN` : entrée Rust trouvée, pas de parité complète établie.
- `SAME_ENTRY_ABSENT_REPLACEMENT_OPEN` : même entrée absente ; composants internes et remplacements à rapprocher.
- `PARTIAL_CONTRACT_DIFFERENT` : opération native, mais demande/sortie/contrat différent de Go.
- `PARTIAL_RENAMED_VIEW` : remplacement de nom partiel, agrégat historique incomplet.
- `NO_NATIVE_OPERATION` / `NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN` : opération publique non raccordée.
- `EXPLICITLY_UNPORTED` : refus explicite dans le moteur CLI.

Les27 noms CLI absents ont une preuve native de refus en194 ; aucune parité positive n’en est déduite. Les consommateurs CLI/mission ont les mêmes SHA que normal193. Le dispatcher web courant diffère du candidat193 : ses chemins sont une lecture des sources, pas une recette réseau du candidat. Les ressources statiques déléguées et les deux routes OTEL hors référence ne sont pas des équivalents des93 API. Les bilans183–193 restent des preuves locales datées ; leur seule présence ne ferme aucune ligne de cette matrice.

## Mission

| Référence | État | Consommateur Rust candidat | Reste concret |
|---|---|---|---|
| `mission status` | PARTIAL_RENAMED_VIEW | `mission show` | show expose les contrôles ; agrégat missionStatus Go, guidage, changements, dépenses et supervision incomplets |
| `mission changes` | NO_NATIVE_OPERATION | — | Opération publique absente ; rapprocher les composants internes réutilisables avant port du contrat |
| `mission spending` | NO_NATIVE_OPERATION | — | Opération publique absente ; rapprocher les composants internes réutilisables avant port du contrat |
| `mission seen` | NO_NATIVE_OPERATION | — | Opération publique absente ; rapprocher les composants internes réutilisables avant port du contrat |
| `mission recovery` | NO_NATIVE_OPERATION | — | Opération publique absente ; rapprocher les composants internes réutilisables avant port du contrat |
| `mission preview` | NO_NATIVE_OPERATION | — | Opération publique absente ; rapprocher les composants internes réutilisables avant port du contrat |
| `mission start` | PARTIAL_CONTRACT_DIFFERENT | `mission start` | Demande native auditée à événement/révision et sortie Work différentes de LaunchProfile/missionStatus Go ; parité JSON/texte/refus/effets ouverte |
| `mission pause` | PARTIAL_CONTRACT_DIFFERENT | `mission pause` | Demande native auditée à événement/révision et sortie Work différentes de LaunchProfile/missionStatus Go ; parité JSON/texte/refus/effets ouverte |
| `mission resume` | PARTIAL_CONTRACT_DIFFERENT | `mission resume` | Demande native auditée à événement/révision et sortie Work différentes de LaunchProfile/missionStatus Go ; parité JSON/texte/refus/effets ouverte |
| `mission stop` | PARTIAL_CONTRACT_DIFFERENT | `mission stop` | Demande native auditée à événement/révision et sortie Work différentes de LaunchProfile/missionStatus Go ; parité JSON/texte/refus/effets ouverte |
| `mission watch` | EXPLICITLY_UNPORTED | — | Boucle complète de réconciliation/dispatch/watch et sémantique d’interruption absentes |

## Planning

| Référence | État | Consommateur Rust candidat | Reste concret |
|---|---|---|---|
| `planning authorize-recovery` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning bundle` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning claim` | ENTRY_PRESENT_PARITY_OPEN | `planning claim` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning cleanup` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning cleanup-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning configure-reviewer` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning decide` | ENTRY_PRESENT_PARITY_OPEN | `planning decide` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning diagnose-review` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning enable` | ENTRY_PRESENT_PARITY_OPEN | `planning enable` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning extend-attempt` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning fragment-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning handoff` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning history` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning judge` | ENTRY_PRESENT_PARITY_OPEN | `planning judge` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning pause` | ENTRY_PRESENT_PARITY_OPEN | `planning pause` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning post-delivery` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning recovery-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning reject-instructions` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning renew` | ENTRY_PRESENT_PARITY_OPEN | `planning renew` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning requalify` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning restart-task` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning resume` | ENTRY_PRESENT_PARITY_OPEN | `planning resume` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning retry-integration` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning retry-review` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning review-cost` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning review-dossier` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning review-step` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning review-timeout` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning revise-recovered-result` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning scope-patch` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning scope-preview` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning show` | ENTRY_PRESENT_PARITY_OPEN | `planning show` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning step` | ENTRY_PRESENT_PARITY_OPEN | `planning step` | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |
| `planning submit-recovered-result` | NO_NATIVE_CLI_OPERATION_REPLACEMENT_OPEN | — | Rapprocher schéma, branches Root, sorties/refus, effets et reprises ; absence CLI ne prouve pas absence de fonction interne |

Les quatre opérations mission présentes (start/pause/resume/stop) utilisent désormais des demandes auditées à event_id/révision et renvoient un Work ; le Go de référence utilisait ses propres entrées puis le statut agrégé missionStatus. Ne pas appeler cela une parité de sortie. show ne restitue pas toutes les informations de status. order, priority et unpause sont des opérations Rust supplémentaires, pas des ports des cinq opérations manquantes.

## Entrées publiques de référence

| ID stable | État | Source Rust examinée |
|---|---|---|
| `cli:agent` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:budget` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:checkpoint` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:evaluate` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:gate` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:init` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:mission` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:ooda` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:planning` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:pricing` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:providers` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:quotas` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:resume` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:task` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:task-model` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:web` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:work` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:aide` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:automation` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:autonomy` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:connections` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:console` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:control` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:dispatch` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:doctor` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:evidence` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:exchange` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:export` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:help` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:import` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:laboratory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:lifecycle` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:memory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:plan` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:prepare` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:process` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:project-profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:role-model` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:run-limits` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:skills` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:trust` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:validation` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `cli:version` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-cli/src/lib.rs` |
| `http:/api/v1/action` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/quotas` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/works` | ENTRY_PRESENT_PARITY_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/activity` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/agent-detail` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/ask` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/cancel` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/context` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/export` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/meta` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/assist/turns` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/cancel` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/config` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/create` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/external` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/programs` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/automation/state` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/control-diagnostic` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/dialogue-resume` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/dialogue-resume-preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/events` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/exchanges` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-drafts/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/graph-performance/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/lifecycle/missions` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/logs` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-bundle` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-cleanup` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/planning-history` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparation-timeout/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/budget` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/command` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/conversion` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/dialogue` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/files` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/history` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/methods` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/preflight` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/providers` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/send` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/show` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/source` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/stop` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/template-check` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/preparations/templates` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/pricing` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/pricing/estimate` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/processes` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-memory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile/catalog` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/project-profile/select` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/provider-wait/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/admin` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/connections` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/connections/test` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/policies` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/providers/resolve` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/recovery-preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/report` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/retex-evolution` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/effective` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/run-limits/rollback` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/runtime-health` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/skills` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/snapshot` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/task` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/terminal` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/terminal/control` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-actions` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-actions/oidc-backchannel-logout` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-evidence` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-laboratory` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/trust-secrets` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/visit` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/apply` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/preview` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |
| `http:/api/v1/web-requests/script.js` | SAME_ENTRY_ABSENT_REPLACEMENT_OPEN | `rust/crates/swarm-web/src/service.rs` |

## Rejeu et limites

```sh
python3 tools/rust-migration/build_behavior_matrix.py \
  --inventory docs/plans/migration-rust/evidence/20261022-pilotage-194/surface.json \
  --normal-profile-directory docs/plans/migration-rust/evidence/20261022-193/normal04 \
  --output NOUVEAU_REPERTOIRE
```

La recette migration-behavior-matrix utilise SWARM_MATRIX_INVENTORY et SWARM_MATRIX_NORMAL_DIRECTORY. Elle vérifie d’abord source périmée, entrée dupliquée et inventaire vide, puis collecte valide. Source dérivée/dupliquée = refus, aucune matrice PASS publiée. Trois tests nommés et deux étapes PASS, source stable et intégrité VERIFIED ; pas de watchdog, compilation, Go, provider ni mutation applicative. Le collecteur est borné aux syntaxes relues : usage/branches planning et whitelist mission ; il n’est pas un parseur général. Une évolution de syntaxe impose une nouvelle revue sémantique, pas un compteur zéro interprété comme réussite.

Suite : figer le périmètre complet P01, approfondir P02 autres sous-commandes et P03 méthodes/corps/effets HTTP, relier les preuves par comportement, puis reprendre P05/P04/P06. Les améliorations de diagnostic restent séparées du port fonctionnel.


## Actualisation197 : FIFO native appliquée, preuve locale bornée

Deux agents autorisés ont livré transitions de file, sélection des candidats et
revues bornées ; la racine a intégré vrai bail/consentement/readiness/cycle Agent.
Normal39 :292 entrées effectives par défaut, sans fixture, non installé.
final43–46 (quatre recettes/six étapes) PASS ; quality21 conserve17 tests Rust nommés,
Clippy strict et Rustdoc privé. Intégrité finale vérifiée ; SDK40/41 liés au normal39.
P05 couvre désormais inscription debout, acquisition Binding3, libération terminale
sûre et historique des générations. Le défaut de nettoyage après pause est reproduit
avant correction. P05 reste partiel : toutes variantes de permission, reprise
incertaine/crash et acceptation complète d’équité ne sont pas établies. P04/P06
dispatch/watch restent non raccordés ; ces API SDK n’ajoutent aucune opération CLI
ou parité HTTP. Les inventaires137/83/93 restent des niveaux de référence qui se
recouvrent, jamais un pourcentage d’achèvement. Les pannes cleanup/annulation, owner/réservations exacts, progression même
workspace et exécution historique v2 passent. Inconnus/crash/panne de completion,
rejeu terminal après nouvelle génération de même tâche et4 permissions restent ouverts.
Sources et preuves : 20261022-fifo-197 dans le plan.

Scellé197 : normal65 SHA8e5af7aa /292 entrées effectives par défaut, SDK66/67/68,
quality69 18 tests Rust nommés/Clippy strict/Rustdoc privé. Cinq recettes finales
(completion-after70 et final71–74) passent7 étapes, intégrité vérifiée. Pause et
preuve de release terminale absente ont leurs témoins avant/après ; la régression
autocommit60 est diagnostiquée/corrigée, jamais dispensée. Sorties imbriquées
archivées/vérifiées séparément dans retained76. Retry de production61 PASS sur
normal55 pour le code d’observation inchangé. Aucune acceptation P05/globale,
installation, parité HTTP supplémentaire ou conducteur/dispatch/watch complet.

### Qualification198 — sélection/admission suivante

P04 partiel : API Rust dispatch sous vrai bail, original avant sync, ordre partagé et candidat disjoint, capacité mission globale au passage, refus permission explicite. Normal24 ab349179/293entrées ; SDK26 mêmes200entrées moteur, vingt tests10 réutilisés sur corps exécutables identiques, Clippy/Rustdoc27, SDK27 Clippy. Public28 :4scénarios +3conflits originaux (manuel/autre Work/révision), sans SQL supplémentaire au rejeu/refus. Une recette/une étape PASS/intégrité vérifiée,116sorties retained29. Compteurs non additifs à la matrice globale. Avant04/13 garde négative namespace :1FAIL attendu puis correction et test positif parmi les20tests ; pas vrai BUSY. P04/P05/P06 et les18livrables restent ouverts, pas de watch/installation/parité branches non portées.

### Point199 — prérequis d'indépendance vérifié, watch produit ouvert

Public23 : deux étapes PASS/intégrité VERIFIED, douze gardes et deux parcours
inertes réels (sortie gracieuse puis SIGTERM du conducteur). Même worker survivant,
vrai waitpid CLI, originaux de fin liés au résultat public, réservation réglée et
FIFO libérée, rejeu sans nouveau provider ni acceptation. SDK10 default qualifié
sur les mêmes 200 entrées moteur de normal198/24 ; trois tests Rust08 et Clippy11
PASS. Retained24 conserve 115 sorties sans base Root. P06 progresse par ce
prérequis B1 ; aucun des 18 livrables globaux n'est déclaré clos. B2 reste à
raccorder, notamment une route session sûre compatible avec forbid unsafe.
L'analyse P07 conserve quatre axes manquants distincts : budget de chaîne,
éligibilité temporelle/multiplicateur, marge d'horloge rapport, relais
nombre/cadence ; le seuil de coût est déjà porté. Aucune nouvelle politique
opérateur, installation ou qualification combinée Live déduite.

### Point200 — bootstrap produit sûr vérifié, conducteur encore ouvert

Normal05 c734f0c4 : 294 inputs default sans fixture ; SDK10 aux mêmes 200 sources
moteur. Cinq tests CLI06, trois SDK11, Clippy/Rustdoc12 PASS. Public14 : deux étapes
PASS et intégrité VERIFIED, quatorze gardes, quatre refus publics avant Root puis
gracieux/SIGTERM réels via le helper produit sans unsafe. Retained15 : 129 sorties
avec originaux de fin et statuts noyau. Le code du driver watch et ses canaux
Root-bound ne sont pas livrés par ce prérequis ; aucun P01–P18 clos ni pourcentage
inféré. Vérification du remplacement de .swarm à chemin constant requise avant
raccordement, hypothèse non reproduite au présent bilan. Pas d'installation,
qualification combinée Live ou acceptation globale.


## Point201 — instance Root, refus historiques et reprise mesurés

Normal65 `8bd7ebc0` (301 entrées effectives, default sans fixture) et SDK77
partagent 201 entrées moteur et la config Cargo. Les 67 tests Rust nommés,
Clippy/Rustdoc85, injections publiques répertoire88/DB seule89, cycle normal/
SIGTERM98 et compatibilité v1/103–v2/104 passent dans leurs portées. Les recettes
ont leur vérification d’intégrité et rétention imbriquée90/99/105 ; aucune base
utilisateur ni modèle payé. Les compteurs ne closent aucun des 18 livrables.

Sous verrou, le refus CLI92 expose Prepare/SQLite5 en 4440712ns, avec reprise et
SQL/DB/settings inchangés ; l’ancien61 mesurait5017838602ns. Aucun seuil ni
engagement de performance ; l’ordre du handler zéro et la conservation du réglage
persisté sont testés74. L’initialisation de témoin et les anciens producteurs
v1/v2 sont explicites. Leur nouveau grant refuse sans mutation ; lecture,
annulation/reconcile/replay, fermeture du pair et réouverture du terminal annulé
sont vérifiés. Binding3 et un ancien completed restent des critères distincts.

La revue96 couvre 62 fichiers avec106 axes ouverts ; elle ne clôt ni les ajouts
Live de Claude, ni le timeout zéro durable de TelemetryStore, ni la continuité
planning/reviewer. Cette dernière et le conducteur produit sont les suites
concrètes (plans100/101). Les binaires sont non installés ; le transport de
binding en développement n’est pas qualifié par normal65. Baseline199 conservée.

Preuves : [synthèse fonctionnelle94](evidence/20261022-root-instance-201/functional-handoff94/result.json), [revue96](evidence/20261022-root-instance-201/review96/README.md).


Actualisation201, preuves118–125 : la compatibilité historique v3 est maintenant
vérifiée par `legacy-v3-118` (12 gardes et parcours réel), avec 169 sorties
conservées dans `legacy-v3-retained119`. Cela complète les preuves v1/v2 ; cela
ne vérifie ni ancien completed, ni replay public `mission.start` v3, ni nouvelle
admission. Le composant d'identité par descripteurs passe six tests116 mais reste
réservé aux tests Linux, sans raccord aux rôles. Après quatre lints de tests
conservés en FAIL121, les corrections122/123 et le gel124 passent Clippy strict125
(lib/tests engine, sans exécuter le reproducteur reviewer). Ce dernier reste
non exécuté et bloqué par l'outil115. `continuation-plan120.md` est adopté et en
implémentation pour observer les intentions avant dispatch ; pas encore qualifié.
Ces résultats n'avancent pas la baseline199 et ne clôturent aucun lot global par
simple décompte de tests. Les sources/profils65 et124 restent distincts.


## Audit Biba Rust du 24 octobre 2026 — R09/T2 ouvert

Le service Rust installé dev.50 ne porte pas le pilote Biba : huit commandes
`trust` en FR/EN refusent `command_not_ported` (sortie2, Root de fixture absent).
Après témoin HTTP authentifié200, les API trust-actions/trust-evidence et la page
trust-actions répondent404.22observations,531entrées Rust/manifests/lock stables ;
aucune compilation, mutation métier, mission ou nouvelle délégation.
[Preuve, périmètre et critères de reprise](evidence/20261024-biba-audit/AUDIT.md).
Les anciens reçus Go ne constituent pas une qualification Rust. Le moniteur par
action, les niveaux dérivés, admission/mandats/accords/effets et leur suite de refus
restent à porter et vérifier avant de fermer R09/T2. Propriétaire : Codex racine,
migration Rust. Audit disponible, implémentation non réalisée par cette revue.
Les pages T2 FR/EN signalent maintenant leur statut historique ; fichiers Rust,
profils, politiques et service inchangés. Aucun rebuild/restart requis pour ces
seuls documents non consommés par les embeds Rust inspectés.

The installed Rust Biba pilot is not ported. Authenticated public probes confirm
missing routes, rather than a session denial. Historical Go evidence is retained
but does not qualify Rust. The linked audit preserves resumption criteria;
this review changes documentation only and grants no extra execution authority.

### BIBA-RUST local, 25 octobre 2026
Le noyau R09/T2 est installé v0.1.1-dev.53 : moniteur et parcours local
CLI/API/Admin/actions, deux effets simulés, preuves compatibles exécutées dans
les deux sens. 58 tests Rust,56 assertions publiques,52 navigateur ; aucune
acceptation globale ou extension automatique aux autres missions. Contrat
[FR](../../actions-confiance-rust.md) / [EN](../../en/actions-confiance-rust.md),
[preuve](evidence/20261024-biba-rust/delivery.json), [RETEX](evidence/20261024-biba-rust/RETEX.md).
L’audit antérieur reste exact pour son ancien binaire et n’est pas réécrit.

### Budgets préparation,26 octobre2026

Enveloppe CLI/API/modale installée125 ; lecture historique CLI sans input et GET `/api/v1/preparations/budget` implémentée dans l’incrément suivant, qualification conservée dans [rapport](PREPARATION-BUDGET-READ-20261026.md). Ce delta remplace la ligne ancienne d’entrée absente dans ce périmètre précis : aucun appel IA préparation ni parité globale déduite.
