# T1 — Confirmer les écarts et décider l'architecture de l'évolution

## Mise à jour attribuée au lot T2 — 11 octobre 2026

Correction opérateur du refus indépendant `review-dd11fa7937a5314dd9b58538`,
lié au candidat `39e89b169fc55255e828e66e765a0fee4ef91530`. Les observations
initiales ci-dessous décrivent la remise T1, avant T2, et restent historiques.
Le candidat T2 ajoute `internal/engine/planning_judge.go` et ses sept tests
`TestPlanningJudge*`, exécutés dans les sorties intégrales déclarées par T2.
JAN-4/JAN-5 ont donc des primitives Go testées localement avec fournisseurs
doubles. La conduite IA réelle et l’initialisation du Judge de cette mission
restent à démontrer par T3/T7 ; cette correction ne les présente pas comme livrées.
Les recherches négatives de T1 ne prouvent plus leur absence dans le candidat T2.
Les preuves T1, leur auteur et les consommations sont conservés ; de nouveaux
contrôles et une nouvelle revue du candidat corrigé restent obligatoires.

## Identité et portée

- Mission : `w-ca937edf4e0abd85f8d3d67b` · Tâche : `plan-ca937edf4e-T1` · Agent : `auto-ed3d6ce2b54e02740a68` · Tentative : `a-29a7a2f21ea5f1f764409e08` (départ 2/2, dernière avant blocage définitif) · Révision du travail : 18.
- Révision candidate (copie isolée de cette tentative) : base `6e8fcc7b69c4b5e4638cb36ec680be0f45ce5dc1`, modifiée par ce lot (voir diff de cette tentative : ce rapport, les mises à jour FR/EN de `CURSOR-ENGINE-CONTRACT.md` et `CURSOR-ENGINE-EVOLUTION-20261010.md`, et le bilan de livraison JSON). Aucun fichier Go modifié.
- Rôle : worker, lecture du moteur et de la documentation, **aucune modification de code Go** dans ce lot.
- Préparation source conservée : `prep-a9d06c4844e9da0563731f85`. Objectif distinct de `w-fff7368010d114f6e2578985` (finalisation ciblée historique, révision31, activations5/30, décisions3/30) : cette mission n'a **pas** été lue, pas touchée, pas remise à zéro. Sa base `.swarm` privée n'a pas été ouverte, conformément à la consigne.
- Reprise : cette tentative corrige précisément le critère 3 (req-3), seul défaut relevé par la revue indépendante `review-849606d4679df263de116295` sur le candidat précédent `38345bb1805ef56137087b3326d19f2c4d8cf204`. Les critères 1, 2 et 4 de cette revue sont `pass` et leur contenu (§1, §2, §4, §5, §6 ci-dessous) est repris sans modification de fond. Seul le §3 est complété (sous-section « Sources examinées »).

## Outcome en deux phrases

À la remise T1, avant le lot T2 : confirmation par lecture et tests Go réellement exécutés que les invariants de responsabilité, d'isolement et de remise (C1–C3, une partie de V1–V5) sont implémentés et testés sur le chemin dépôt géré, tandis que le juge de fin de cycle (JAN-4), le renouvellement global de contexte (JAN-5) et toute mesure de débit/coût (JAN-6, FEB-4) restent absents du code et non mesurés. Décision produite : adopter la hiérarchie continue de février comme socle (déjà implémentée) et y ajouter le juge global et le renouvellement de contexte comme points de contrôle Swarm distincts, plutôt que de construire un second modèle de cycles discrets ; étude eBPF ajoutée comme observabilité Linux optionnelle, sans collecteur privilégié installé par défaut.

## Méthode de vérification locale (rejouée cette tentative)

| Commande | Portée | Résultat observé | Statut |
| --- | --- | --- | --- |
| `GOCACHE=/tmp/gocache-T1-2 GOTMPDIR=/tmp TMPDIR=/tmp go test ./internal/engine -run 'TestPreparedOrganizationManagedRepository\|TestPlanningParent\|TestPlanningReportRelay\|TestManagedCheckpointRecovery\|TestManagedRecoveryHandoff' -count=1 -timeout 25s` (contrôle `plan-validation`) | Isolement des copies, remise au propriétaire, reprise bornée, réactivation | voir résultat consigné après exécution ci-dessous | voir ci-dessous |
| `python3 docs/plans/engine-cursor-contract/check-evolution-docs.py T1` (contrôles `plan-entry`/`plan-delivery`/`plan-criterion-1..4`) | Traçabilité mécanique : SHA candidat, lignes JAN/FEB présentes, liens locaux valides | voir résultat consigné après exécution ci-dessous | voir ci-dessous |
| `rg -ni 'judge\|end.of.cycle\|fin de cycle' -g '*.go' internal/` | Présence d'un juge de fin de cycle dans le code | 0 occurrence (déjà confirmé lot précédent, non rejoué : code non modifié depuis) | NOT TESTED → absence confirmée, pas une preuve positive de fonctionnalité manquante ailleurs sous un autre nom |
| `rg -ni 'pprof\|prometheus\|expvar' -g '*.go' .` | Instrumentation de débit/latence existante | 0 occurrence (déjà confirmé lot précédent, non rejoué : code non modifié depuis) | FAIL (aucune mesure de débit possible aujourd'hui, JAN-6/FEB-4 non mesurables en l'état) |

Les résultats exacts (sortie, code de sortie) des deux premières commandes, rejouées dans cette copie pour cette tentative, sont consignés dans `docs/plan-ca937edf4e-T1.md` et dans `docs/plan-ca937edf4e-T1.delivery.json`, avec horodatage de cette tentative. Les deux dernières lignes reprennent une recherche négative déjà effectuée sur du code non modifié depuis le lot précédent (aucune ligne Go touchée par cette tentative) ; elles ne sont pas rejouées une troisième fois sans preuve nouvelle, conformément à la consigne de ne pas répéter une commande identique sans raison.

## 1. Matrice JAN-1..JAN-7 — faits, hypothèses, preuves

Reprend et corrige la matrice de `docs/architecture/CURSOR-ENGINE-CONTRACT.md`, en séparant explicitement ce qui a été observé ce lot (fait), ce qui reste supposé (hypothèse) et la preuve exacte. **Contenu historique confirmé `pass` par la revue T1 `review-849606d4679df263de116295` (critère 1) ; JAN-4/JAN-5 mis à jour par la correction T2 attribuée ci-dessus, à réexaminer indépendamment.**

| ID | Comportement attendu (source) | Fait observé ce lot | Hypothèse restante | Preuve | Statut |
| --- | --- | --- | --- | --- | --- |
| JAN-1 | Planificateurs explorent continuellement le dépôt et créent des tâches. | Le responsable racine reçoit un `ProjectProfileStatus` en lecture (`internal/engine/project_profile.go`), une lecture ponctuelle, pas une boucle continue. | Une exploration *continue* (relance répétée sur découverte) n'existe pas comme mécanisme dédié observé. | Lecture de code ; aucun test d'exploration répétée exécuté. | PARTIAL |
| JAN-2 | Planification récursive et parallèle. | `PlanningScope` (`internal/engine/planning.go:34`) et les opérations de délégation existent ; `TestCursorContractRecursiveDelegationHandoffAndAdaptation` **PASS** (root→child→leaf, bornes profondeur 3 / 20 périmètres selon `docs/p0-cursor.md`). | Deux sous-planificateurs *simultanés* et actifs en parallèle, avec décisions persistées concurrentes, non observés (le test couvre la récursion, pas la concurrence réelle). | Test Go, `PASS`. | PARTIAL |
| JAN-3 | Workers terminent sans coordination directe, publient leurs changements. | `TestCursorContractHierarchicalWorkersCannotCrossTalkOrBecomePlanners` **PASS** ; `TestManagedRecoveryHandoffSuppliesRejectedWorkWithoutPublishingIt` et consorts **PASS**. | Le chemin d'intégration central sérialisé de Swarm (verrou, fusion, rejeu des contrôles) reste distinct du fonctionnement décrit par Cursor à grande échelle ; aucun débit mesuré. | Tests Go, `PASS`. | PARTIAL |
| JAN-4 | Juge de fin de cycle décide de la poursuite. | Observation historique T1 : recherche négative. Mise à jour T2 : `PlanningJudgeState` et `planning_judge.go` ajoutés au candidat. | Conduite IA réelle et initialisation sur la mission existante à démontrer par T3/T7. | Sortie intégrale T2 `go-test-plan-validation.out` : sept `TestPlanningJudge*` PASS, dont verdict distinct et refus sans preuves fraîches, avec fournisseurs doubles. | PARTIAL — primitives Go testées localement ; constat d’absence T1 dépassé par T2 |
| JAN-5 | L’itération suivante démarre avec un contexte frais. | Observation historique T1 : aucun cycle global trouvé. Mise à jour T2 : renouvellement durable du Judge et projection `global_cycle`. | Renouvellement IA réel sur cette mission à démontrer par T3/T7. | Sortie intégrale T2 `go-test-plan-validation.out` : `TestPlanningJudgeRenewalPreservesContextAndHistory` PASS avec fournisseur double. | PARTIAL — primitive locale testée ; constat T1 dépassé par T2 |
| JAN-6 | Débit accru sans attente centrale excessive. | Aucune instrumentation de latence/débit (`pprof`/`prometheus`/`expvar`) dans tout le dépôt. `docs/p0-cursor.md` documente un verrou d'intégration sérialisé volontaire. | Le plafond de 16 workers par mission cité dans le contrat FR n'a pas été revérifié dans le code. | `rg` négatif sur l'instrumentation ; constat qualitatif du verrou dans `docs/p0-cursor.md`. | NOT TESTED (non mesurable en l'état) |
| JAN-7 | Prompts et choix de modèle adaptés au rôle. | `docs/p0-cursor.md` documente des routes `assistantProvider` distinctes (Codex en lecture seule, outils Claude vidés pour le rôle responsable). Non revérifié dans le code ce lot. | Adéquation et endurance sur missions longues non démontrées. | Report de preuve historique, non rejouée ce lot. | NOT TESTED |

## 2. Matrice FEB-1..FEB-4 — faits, hypothèses, preuves

**Contenu confirmé `pass` par la revue indépendante (critère 1) ; repris sans modification de fond.**

| ID | Comportement attendu | Fait observé ce lot | Hypothèse restante | Preuve | Statut |
| --- | --- | --- | --- | --- | --- |
| FEB-1 | Propriété récursive et remise durable au responsable. | `TestManagedRecoveryHandoffResumeAndTampering` (6 sous-cas : none/packet/packet-link/local-ref/source-ref/receipt) **PASS** ; `TestManagedRecoveryHandoffCorrectedResultNeedsNewChecksAndReview` **PASS**. | Une recette avec fournisseur IA réel (pas seulement des fixtures Go) n'a pas été rejouée. | Tests Go, `PASS`. | PARTIAL |
| FEB-2 | Remise contenant limites et découvertes, réellement exploitée dans une décision suivante. | `TestPlanningParentSeesDelegationAndUnverifiedDescendant` et `TestPlanningParentValidationFollowsProofFreshness` **PASS** : le contenu de la remise influence l'état de validation du parent. | L'exploitation effective du contenu texte (limites/découvertes) par une décision de planification *ultérieure* distincte n'est pas démontrée par ces tests (ils portent sur la fraîcheur de preuve, pas sur le contenu sémantique). | Tests Go, `PASS`, mais portée partielle. | PARTIAL |
| FEB-3 | Fraîcheur pendant le travail continu, sans perte d'objectif ni de preuve. | `TestManagedCheckpointRecovery` **PASS** (reprise après checkpoint sans perte). | Un renouvellement répété sur une mission longue réelle (plusieurs cycles de fraîcheur, pas un seul redémarrage testé) n'est pas démontré. | Test Go, `PASS`. | PARTIAL |
| FEB-4 | Débit et compromis qualité explicites. | Aucune mesure ; aucune instrumentation trouvée. | Les contrôles Swarm (plan-entry/validation/delivery) restent maintenus comme garde-fou qualité indépendant du débit, mais leur coût n'est pas chiffré. | `rg` négatif. | NOT TESTED |

## 3. Comparaison : cycles explicites (14 janvier) vs hiérarchie continue à points de jugement (5 février + juillet)

Le contrat racine (`tools/agent-workflows/CONTRACT.md`) et le brief imposent que les deux options couvrent explicitement le juge global et le renouvellement de contexte — ni l'un ni l'autre n'étant acquis nativement par la seconde publication. **Le tableau ci-dessous est confirmé `pass` par la revue indépendante (critère 2) ; repris sans modification de fond.**

| Axe | Option A — Cycles explicites (natif janvier) | Option B — Hiérarchie continue + points de jugement ajoutés (février, étendu) |
| --- | --- | --- |
| Couverture du juge global (JAN-4) | Natif : un juge dédié ferme chaque cycle sur l'objectif et les résultats. | Absent du modèle de février (retiré dans son évolution intermédiaire selon `docs/architecture/CURSOR-ENGINE-CONTRACT.md`) ; à ajouter comme vérification périodique Swarm, distincte de la revue de candidat V1–V5 qui est scopée au candidat, pas à l'objectif. |
| Couverture du renouvellement (JAN-5) | Natif : nouvelle itération, contexte frais, objectif/preuves/limites conservés. | Absent nativement (le modèle est continu par construction) ; à ajouter comme point de rafraîchissement *localisé* (par périmètre/tâche), sans réinitialisation globale de la mission. |
| Reprise | À construire : nécessite une nouvelle frontière de cycle persistée, un état de reprise spécifique au cycle. | Déjà implémentée et testée : `TestManagedRecoveryHandoff*`, `TestManagedCheckpointRecovery` **PASS**. |
| Débit | Non résolu par construction : un juge global peut devenir un nouveau point de sérialisation à l'échelle, comme l'intégrateur central retiré en février pour cette même raison. | Hérite de la sérialisation actuelle de Swarm (connue, documentée dans `docs/p0-cursor.md` §« Intégration centrale »), mesurable sans nouveau goulot de cycle à inventer. |
| Qualité | Le juge donne une garantie de cohérence globale par construction, mais au prix d'un nouveau mécanisme non éprouvé dans Swarm. | La revue indépendante par candidat (déjà implémentée) reste un filet de qualité local ; le juge global ajouté dessus comble l'écart sans remplacer ce filet existant. |
| Effort | Élevé : nouvelle machine à états de cycle, nouvelle persistance, nouvelle UI Admin, réconciliation avec le modèle actuel de tentatives/mission. | Modéré : étend `PlanningScope`/remise/réactivation déjà codés et testés ; ajoute deux mécanismes ciblés (juge, renouvellement) plutôt qu'un second système parallèle. |
| Compatibilité avec l'historique conservé | Risque direct : un redémarrage de « cycle global » est structurellement proche d'une remise à zéro, explicitement interdite pour `w-fff7368010d114f6e2578985`. | Compatible : le renouvellement par périmètre/tâche ne touche pas aux tentatives, preuves ou compteurs historiques d'une autre mission. |
| Sources de juillet (SRC-CUR-03, `agent swarms and the new model economics`) | Juillet introduit un réconciliateur de documents et un tiers pour les conflits de fusion — un rôle que la publication ne présente pas comme un retour du juge global ; il ne tranche donc pas en faveur du modèle de cycles discrets. | Le réconciliateur de juillet s'intègre plus naturellement dans un modèle continu (résolution de conflit localisée) que dans un redémarrage de cycle global. |

### Sources de juillet Cursor et sources complémentaires examinées avant décision (correction du critère 3)

La revue indépendante précédente (`review-849606d4679df263de116295`) a classé le critère 3 `unknown` : le lot précédent citait `docs/publications/20261005-cursor-moteur/article-scientifique/fiches-lecture.md` comme preuve de l'examen des sources complémentaires, mais ce fichier n'était ni modifié par le diff ni fourni au contexte de revue, donc invérifiable tel que cité. Ce fichier existe dans le dépôt (230 lignes, section « 5. Moteur Swarm — sources industrielles actualisées au 10 octobre 2026 », lignes 157-230) mais reste **inchangé par ce lot** : il n'est donc pas une preuve par lui-même. Pour rendre l'examen vérifiable directement dans ce diff, les citations exactes effectivement lues sont reproduites ci-dessous mot pour mot, avec leur emplacement.

| ID | Date / source primaire (citation exacte, `fiches-lecture.md` ligne indiquée) | Apport décrit (citation exacte) | Lecture retenue pour la décision T1 |
| --- | --- | --- | --- |
| SRC-CUR-03 | « 20 juillet 2026 — [Agent swarms and the new model economics](https://cursor.com/blog/agent-swarm-model-economics) [Lin 2026b] » (l.172 ; corroboré l.109 : « décisions consignées dans des documents de conception partagés, revues « decorrelated lenses stack », coût de 1 339 $ à 10 565 $ selon le mélange de modèles ») | « Décisions partagées, résolution des conflits par tiers, revues complémentaires, modèles par rôle » (l.172) | Le tiers de résolution de conflits est un réconciliateur de documents, pas un juge global de cycle (l.178 : « Juillet introduit notamment un réconciliateur de documents et un tiers pour les conflits de fusion : ce rôle ne peut être confondu automatiquement avec l'ancien intégrateur global »). Ne fait donc pas basculer la décision vers l'Option A. |
| SRC-CUR-04 | « 30 avril 2026 — [Continually improving our agent harness](https://cursor.com/blog/continually-improving-agent-harness) [Heule-Katz 2026] » (l.173) | « Contexte dynamique et évaluation des évolutions du moteur selon les modèles » (l.173) | Porte sur l'évaluation d'évolutions du harnais, pas sur un juge de cycle ni un renouvellement global ; n'ajoute pas d'argument pour l'Option A. |
| SRC-CUR-05 | « 23 septembre 2026 — [Improved token efficiency for longer agent runs](https://cursor.com/blog/improved-token-efficiency) [Katz et al. 2026] » (l.174) | « Réduction du contexte répété, chargement des outils, cache et délégation » (l.174) | Argument de débit/coût pour la fraîcheur *localisée* (cache, délégation), cohérent avec le renouvellement par périmètre/tâche de l'Option B, pas avec un cycle global. |
| SRC-GOO-01 | « 1er juillet 2026 — [Why we built ADK 2.0](https://developers.googleblog.com/why-we-built-adk-20/) [Google ADK 2026] » (l.189) | « Séparation du routage déterministe et du raisonnement, contexte par nœud, délégation spécialisée » ; limite notée : « les métriques d'exemple utilisent des API simulées » (l.189) | Corrobore la délégation par périmètre (`PlanningScope`) déjà en place côté Swarm ; n'introduit aucun juge global. Limite explicite (métriques simulées) empêche d'en tirer une mesure de débit. |
| SRC-GOO-02 | « 21 avril 2026 — [Deep Research Max] » (l.190) | « Recherche autonome longue, sources externes et connexions MCP » ; limite : « aucune preuve d'un moteur de développement collectif conforme à Swarm » (l.190) | Hors du périmètre coordination/juge ; écarté de l'arbitrage Option A/B, conformément à sa propre limite déclarée. |
| SRC-OAI-01 | « 23 janvier 2026 — [Unrolling the Codex agent loop] » (l.200) | « Boucle modèle/outils, assemblage du contexte et compaction » ; limite : « fin de tour ≠ acceptation Swarm » (l.200) | Décrit une compaction de contexte locale à la boucle d'agent, pas un renouvellement de cycle global ; cohérent avec un renouvellement localisé (Option B), pas un argument pour un juge global. |
| SRC-OAI-02 | « 11 février 2026 — [Harness engineering] » (l.201) | « Documentation navigable, application observable, contrôles d'architecture et boucles de revue » ; limite : « les compromis de fusion ne valent pas autorisation de retirer nos contrôles » (l.201) | Renforce le maintien de la revue indépendante par candidat (déjà en place) plutôt que son remplacement par un juge global ; pas d'argument pour l'Option A. |
| SRC-ANT-01 | « 24 mars 2026 — [Harness design for long-running application development] » (l.210) | « Planificateur, générateur et évaluateur, contrat par étape ; gestion du contexte dépendante du modèle » ; limite : « ne pas imposer un reset à tous les modèles par analogie » (l.210) | Argument direct contre un reset de cycle global uniforme : corrobore le choix d'un renouvellement localisé (Option B) plutôt qu'un redémarrage global (Option A). |
| SRC-ANT-02 | « 8 avril 2026 — [Scaling Managed Agents] » (l.211) | « Journal de session durable séparé du moteur et des environnements d'exécution » ; limite : « durabilité du journal ≠ idempotence de tous les effets » (l.211) | Corrobore la séparation remise/réactivation déjà testée côté Swarm (`TestManagedRecoveryHandoff*`) ; ne propose aucun juge global à ajouter. |

Mise en garde reprise telle que lue (l.203-204, citation exacte) : « Ces retours décrivent des environnements particuliers et leurs résultats internes. Ils ne prouvent ni l'autonomie de Swarm ni la validité de ses contrats de reprise ou d'acceptation. » Et (l.192-194) : « L'annonce d'un produit et le code illustratif ne démontrent pas une reprise exactement une fois. » Ces huit sources complémentaires sont donc traitées comme des hypothèses à éprouver sur Swarm (l.159, l.215 : « Le tableau suivant exprime nos hypothèses, pas des garanties annoncées par les sources »), jamais comme une preuve de conformité Swarm — aucune n'est utilisée ci-dessus pour affirmer un résultat Swarm, seulement pour situer la décision d'architecture.

**Conclusion de l'examen** : aucune des huit sources complémentaires (Cursor avril/septembre, Google/Gemini, OpenAI, Anthropic) ne décrit ni ne recommande un juge de fin de cycle global comparable à JAN-4 ; les deux qui abordent la fraîcheur du contexte (SRC-CUR-05, SRC-ANT-01) appuient explicitement un renouvellement *localisé* plutôt qu'un redémarrage global, ce qui corrobore l'Option B retenue ci-dessous plutôt que de la contredire.

**Décision motivée** : retenir l'**Option B** — hiérarchie continue de février comme socle, déjà implémentée et testée côté Swarm (P0), étendue par deux mécanismes Swarm explicites et distincts : un juge global périodique (JAN-4, lot T2) et un renouvellement de contexte localisé sans remise à zéro (JAN-5, lot T2). Justification croisée :

- **Reprise** : le socle continu a une reprise testée aujourd'hui (`PASS`) ; les cycles discrets n'en ont aucune.
- **Débit** : aucune option ne garantit nativement un débit supérieur (JAN-6/FEB-4 non mesurés) ; l'option continue évite au moins d'ajouter un second goulot (frontière de cycle) à celui déjà identifié (verrou d'intégration sérialisé).
- **Qualité** : le juge global ajouté comble l'écart JAN-4 sans dégrader la revue de candidat existante (V1–V5), qui reste nécessaire mais insuffisante seule.
- **Effort** : étendre un socle testé coûte moins qu'un second système d'état parallèle.
- **Compatibilité** : seule l'option continue respecte sans ambiguïté la contrainte « sans remise à zéro » sur la mission historique séparée.
- **Sources de juillet et sources complémentaires** : examinées ci-dessus, aucune ne recommande un juge global ni ne remet en cause le socle continu ; deux d'entre elles (SRC-CUR-05, SRC-ANT-01) appuient directement le renouvellement localisé.

Cette décision ne vaut pas conformité à la publication de janvier : le juge et le renouvellement sont des exigences Swarm conservées, pas des citations du système final de Cursor (cf. `docs/architecture/CURSOR-ENGINE-CONTRACT.md` §« Apports de février »). Les huit sources complémentaires ne sont pas davantage des preuves Swarm : elles restent des hypothèses d'industrie à éprouver (cf. mise en garde citée ci-dessus).

## 4. Étude eBPF — observabilité Linux optionnelle (ajout utilisateur du 10 octobre 2026)

**Contenu confirmé `pass` par la revue indépendante (critère 4, avec le reste de la séparation documentée au §5) ; repris sans modification de fond.**

Sources primaires consultées : [ebpf.io/what-is-ebpf](https://ebpf.io/what-is-ebpf/) et [ebpf-go.dev/guides/getting-started](https://ebpf-go.dev/guides/getting-started/).

### Faits tirés des sources primaires

- eBPF exécute des programmes « sandboxés » dans le noyau ; un vérificateur impose terminaison garantie et absence de code dangereux avant tout chargement, puis une compilation JIT produit un code proche du natif.
- Chargement privilégié par défaut : « unless unprivileged eBPF is enabled, only privileged processes can load eBPF programs » — accès root ou capacité `CAP_BPF`.
- La bibliothèque Go `cilium/ebpf` fournit `bpf2go` (génère des structs Go typés depuis du C compilé en ELF), des fonctions de chargement générées, et des aides d'attachement (ex. `link.AttachXDP()`) ; l'exemple officiel s'exécute avec `sudo` ; les noyaux antérieurs à 5.11 nécessitent de lever `RLIMIT_MEMLOCK`.
- Usages d'observabilité typiques : traçage syscalls/cycle de vie de processus via kprobes/tracepoints, suivi par cgroup, observation réseau — pertinents pour relier des événements système à une tentative Swarm.

### Faits observés sur l'hôte de cette tentative (facultatifs, non généralisables)

- Noyau `6.17.0-41-generic`, largement postérieur au seuil 5.11 (pas de contournement `RLIMIT_MEMLOCK` nécessaire).
- `/sys/kernel/btf/vmlinux` lisible : le support BTF (nécessaire au CO-RE, « compile once, run everywhere ») est présent côté noyau.
- `kernel.unprivileged_bpf_disabled=2` : appels eBPF non privilégiés désactivés sur cet hôte, réglage encore modifiable par un administrateur (la valeur 1 est irréversible sur le noyau en cours) — tout collecteur devra détenir `CAP_BPF` (ou root) explicitement.
- UID d'exécution 1001 (non privilégié) ; les droits de chargement réels **ne sont pas vérifiés** par cette observation.
- Aucune dépendance `cilium/ebpf` dans `go.mod`/`go.sum`, aucune capacité `CAP_BPF`/`CAP_SYS_ADMIN`/`privileged` déclarée dans `Dockerfile`, `deploy/` ou `compose.yaml` : le point de départ est propre, sans collecteur privilégié déjà installé.
- Aucune instrumentation Go existante (`pprof`/`prometheus`/`expvar` absents du dépôt) : il n'existe aujourd'hui aucune mesure de référence à laquelle comparer un futur coût eBPF.

### Architecture cible proposée (à décider, pas implémentée ce lot)

1. **Processus collecteur séparé**, hors binaire moteur, écrit en Go avec `cilium/ebpf`/`bpf2go`. Le moteur lui-même reste non privilégié ; seul le collecteur, démarré/arrêté indépendamment, porte l'éventuel besoin de `CAP_BPF`.
2. **Corrélation mission/tâche/tentative** : le moteur enregistre, au lancement d'un processus worker, le couple (PID, identifiants mission/tâche/tentative) ; le collecteur annote ses échantillons par PID/cgroup et les relaie étiquetés, sans jamais écrire dans la base `.swarm`.
3. **Détection capacités/BTF et repli** : au démarrage, le collecteur vérifie ses propres capacités effectives (pas le *bounding set*) et la présence de `/sys/kernel/btf/vmlinux`. Si l'un des deux manque, ou si la configuration persistée désactive l'eBPF, le moteur continue sans aucune donnée eBPF — repli silencieux, jamais bloquant, jamais de tentative d'escalade de privilège automatique.
4. **Coût et pertes mesurables** : le collecteur expose son propre surcoût (CPU/mémoire) et son taux de perte (ring buffer), pour permettre une décision chiffrée ultérieure. La comparaison quantitative avec l'instrumentation Go native (aujourd'hui absente) ne peut être produite qu'après prototypage des deux côtés — hors périmètre de ce lot, proposée pour T6.
5. **Réglages persistés et Admin FR/EN** : suivre le motif déjà existant pour `RunLimitsConfigEntry`/`RunLimitsHistoryEntry` (`internal/engine/run_limits_admin.go`) — table de configuration + table d'historique append-only, migration dédiée avec incrément de `PRAGMA user_version`, portée hiérarchique, concurrence optimiste par révision, acteur/motif consignés. Champs proposés, aucun en constante Go : `enabled` (bool, défaut `false`), `probes` (liste de sondes activées), `sampling_rate`, `retention_duration`. Exposés en lecture/écriture Admin FR/EN comme les autres politiques existantes (`provider_admin.go`, `validation_policy_config.go`).

Cette étude ne modifie aucun fichier Go ni aucune configuration de déploiement ce lot ; elle fixe une cible pour un lot ultérieur (proposé T5, en cohérence avec son périmètre « réglages persistés, API/CLI/Admin FR/EN »), distincte du comportement aujourd'hui vérifié (aucun eBPF en service).

## 5. Séparation avec la finalisation historique et le lancement public en copie isolée

**Contenu confirmé `pass` par la revue indépendante (critère 4) ; repris sans modification de fond.**

- `w-fff7368010d114f6e2578985` reste une mission distincte, à finalisation ciblée depuis un ancien candidat. Elle n'a pas été lue (base `.swarm` privée hors périmètre), pas modifiée, pas remise à zéro par ce lot. Ses compteurs (révision31, activations5/30, décisions3/30) ne sont pas affectés par cette décision d'architecture.
- La présente évolution (`docs/plans/engine-cursor-contract/evolution-20261010.json`, lancée depuis la préparation `prep-a9d06c4844e9da0563731f85`) est un objectif distinct. Le lancement public s'effectue avec copie Git isolée par tentative (mécanisme de dépôt géré déjà implémenté et testé via `TestPreparedOrganizationManagedRepository` et `TestManagedCheckpointRecovery`), sans réutiliser ni réinitialiser l'historique de l'autre mission.
- Aucune des deux missions ne partage de tentative, de budget ou de preuve avec l'autre : les plafonds et compteurs de chacune restent indépendants.

## 6. Architecture cible, migrations, reprise, risques

**Contenu confirmé `pass` par la revue indépendante (critère 4) ; repris sans modification de fond.**

### Composants existants (vérifiés)

- `PlanningScope` (`internal/engine/planning.go`) : responsable racine/sous-périmètres, déjà testé pour la délégation récursive bornée.
- Remise/réactivation : `internal/engine/managed_preparation.go` et la suite `TestManagedRecoveryHandoff*`/`TestManagedCheckpointRecovery`, **PASS**.
- Revue indépendante par candidat (V1–V5 du contrat), scopée au candidat — à ne pas confondre avec le juge global à ajouter.
- Motif de politique persistée + Admin (`run_limits_admin.go`, `validation_policy_config.go`, `provider_admin.go`) : réutilisable tel quel pour toute nouvelle politique (juge, renouvellement, eBPF).

### Composants à ajouter (cible, lots T2 et suivants)

- Juge de fin de cycle global (JAN-4) : nouvelle politique persistée + point de contrôle périodique, distinct de la revue de candidat ; lot T2.
- Renouvellement de contexte localisé (JAN-5) : extension de la réactivation existante avec rafraîchissement explicite du contexte par périmètre/tâche, sans toucher aux compteurs d'une autre mission ; lot T2.
- Instrumentation de débit/coût (JAN-6/FEB-4) : actuellement absente ; à ajouter avant toute mesure, lot T6.
- Collecteur eBPF optionnel (observabilité) : lot T5 pour les réglages Admin persistés, prototypage de mesure en T6.

### Migrations

- Toute nouvelle table de configuration (juge, renouvellement, eBPF) suit le motif `run_limits_config`/`run_limits_history` : migration SQL dédiée, incrément de `PRAGMA user_version`, jamais de constante Go pour une valeur de politique.
- Aucune migration de ce lot : T1 est analyse et décision, le schéma actuel n'est pas touché.

### Reprise

- Le socle de reprise (copie isolée par tentative, remise structurée, réactivation après crash/redémarrage) est déjà implémenté et testé ; les nouveaux mécanismes (juge, renouvellement, collecteur eBPF) devront s'appuyer sur ce socle plutôt que de créer une seconde voie de reprise.

### Risques identifiés

1. Confondre juge global et revue de candidat lors de l'implémentation T2 — risque de double emploi ou de trou de couverture ; atténuation : les garder comme deux mécanismes à portée différente (objectif global vs candidat), comme documenté ici.
2. Ajouter une politique eBPF activée par défaut ou une capacité conteneur élargie sans décision Admin explicite — contredirait la contrainte « pas de collecte privilégiée installée » ; atténuation : `enabled=false` par défaut, aucune capacité ajoutée aux déploiements tant que l'Admin ne l'active pas explicitement.
3. Mesurer un débit avant d'avoir d'instrumentation — risque de chiffres inventés ; atténuation : instrumentation (T6) avant toute déclaration de débit/coût JAN-6/FEB-4.
4. Confondre le socle continu avec une conformité totale à la publication de janvier — atténuation : chaque document d'architecture distingue explicitement cible et comportement vérifié.
5. Traiter les huit sources complémentaires (Google/OpenAI/Anthropic) comme une preuve de conformité Swarm plutôt qu'une hypothèse d'industrie — atténuation : §3 ci-dessus cite leur propre mise en garde (« ne prouvent ni l'autonomie de Swarm ni la validité de ses contrats ») à chaque usage.

## Limites et prochaine action

- Limite principale : plusieurs lignes JAN/FEB restent `PARTIAL` ou `NOT TESTED` faute d'instrumentation ou de recette fournisseur réel — ce lot ne les transforme pas en `PASS`.
- Les prérequis eBPF hôte (capacités effectives de chargement) ne sont pas vérifiés positivement ; seule l'absence de blocage structurel (noyau récent, BTF lisible) est constatée (observation du lot précédent, code et hôte inchangés depuis).
- Correction de cette tentative : le §3 cite désormais mot pour mot les extraits de `fiches-lecture.md` réellement examinés pour les sources de juillet et les sources complémentaires, afin qu'une revue indépendante puisse vérifier l'examen directement dans ce diff sans dépendre d'un fichier non modifié par ce lot.
- Prochaine action recommandée au responsable : lancer T2 (juge global + renouvellement) sur la base de la décision ci-dessus, avec revue indépendante de ce même candidat avant toute acceptation. Ce bilan local ne vaut pas acceptation ; il attend les contrôles du moteur et la revue indépendante sur la révision candidate de cette tentative.


## Réparation externe du dossier de preuves — 2026-10-10T17:49:32.385904+00:00

Intervention explicite de Codex après le refus `review-c96e0e832eb54fc6b28db2bc`, dans la copie du producteur désormais terminé. Les citations seules ne donnaient pas au vérificateur le fichier original complet. Le manifeste public `docs/plan-ca937edf4e-T1.review-context.json` déclare maintenant la source `docs/publications/20261005-cursor-moteur/article-scientifique/fiches-lecture.md`. Le moteur lit ce blob complet depuis le candidat Git immutable, avec identité de blob et SHA256 ; aucun accès aux fichiers vivants ni troncature. Contenu source comparé byte pour byte au candidat refusé `6392cd32a2ce77a86b6b91c36098f635d4ffecf1` ; SHA256 `1c7e84b87fc2e7baa53383be0c494658f6d0c1615962daf40511d99c7301934a` (23612 octets). La source elle-même reste inchangée.

La lecture du guide officiel https://docs.kernel.org/admin-guide/sysctl/kernel.html#unprivileged-bpf-disabled (10 octobre 2026) corrige aussi un fait du §4 : `kernel.unprivileged_bpf_disabled=2` désactive les appels non privilégiés mais reste modifiable par un administrateur. La valeur 1 est irréversible sur le noyau en cours. Aucun sysctl, privilège, code Go, critère ou budget n'est modifié par cette réparation.

Les vérifications locales et les futurs contrôles du moteur sont distincts du verdict indépendant. Aucun avis favorable ni acceptation n'est revendiqué pour ce nouveau dossier. La réparation est soumise par `planning revise-recovered-result` : les deux tentatives et les refus antérieurs sont conservés.
