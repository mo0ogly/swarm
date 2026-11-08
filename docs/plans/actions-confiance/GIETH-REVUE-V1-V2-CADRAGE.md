# Gi.eth, lot 14 — applicabilité d'une revue après changement effectif

**État courant : GI-T1 à GI-T4 réalisés dans la portée bornée ci-dessous.
Défaut reproduit avant correction ; V2/R1 refusé après correction ; reprise sur
variante V2 sûre avec nouveaux contrôles et R2, puis publication/rejeu vérifiés.
Correction locale active v0.1.1-dev.347. Pas d’audit de sécurité indépendant ni de
fermeture universelle des dépendances. RETEX et correction du focus GI-UI-1 en fin de note.
Les sections de cadrage et incidents antérieurs restent historiques.**

Cadrage initial du 8 novembre 2026, Codex seul, méthode replan. **À cette étape seulement :
aucun développement, test, build, recette, appel fournisseur, service ou mission
exécuté.** Les commandes utilisées servent à lire les sources et le suivi ; les
contrôles documentaires finaux ne sont pas une recette comportementale.

Base lue : `main`, HEAD `bd6927b75823e8c4b68cdf30e98a8a7613af2c29`.
`/home/fpizzi/workspace/swarm` résout vers `/home/fpizzi/Documents/ChatGPT/swarm`.
Les notes PoC FR/EN et le rapport Peter déjà modifiés sont préservés. L'instance
active et son binaire ne sont pas inspectés : aucune conclusion sur leur état.

## Conclusion bornée et besoin repris

**Une revue V1 ne doit pas suffire à accepter V2 si une dépendance pertinente
pour ses conclusions a changé. Le refus effectif pour toute dépendance n'est pas
établi.** Le code contrôle plusieurs liens candidat/preuve ; il ne démontre pas
la découverte exhaustive du graphe effectif. Pour une dépendance hors inventaire,
la réutilisation de l'ancien avis n'est donc pas exclue par cette lecture. C'est
une hypothèse à reproduire, pas une acceptation indue observée.

V1/V2 désignent ici deux états effectifs ; leur commit Git peut être identique.
L'ancien avis peut rester historiquement valide pour V1 sans être applicable à V2.
Un changement sans incidence sur les conclusions peut permettre une réutilisation
motivée sur un périmètre inchangé ; cette décision demande ses propres preuves.
Ni l'égalité de la commande ni une durée de validité ne suffisent à l'établir.

Origine : [X-IDEE-01/02 et complément Gi.eth](../../communication/x-swarm-20261107/IDEES-DISCUSSIONS.md),
rubriques initiales et lot 14 ; [POC-02](../AGENT-CONTROL-POCS.md).
Le registre rapporte l'échange Gi.eth/Ammy sur revue spécialisée et conserve
les liens X. Aucune consultation nouvelle de X ni capacité TermiX/AACP attribuée.

## Exigence documentaire, distincte de l'implémentation

`AGENTS.md` impose candidat exact, dépendances effectives, profils et binaire
exercé ; un changement du contrat consommé exige de réexaminer les preuves.
`tools/agent-workflows/CONTRACT.md`, §2/4, impose contexte indépendant,
liaison des contrôles/revue à un candidat et invalidation lors de changements
code/méthodes/entrées. Il exige des originaux complets dans le contexte, pas de
déduction depuis une citation ou un rôle. POC-02 exige témoin valide, détection
du changement avant mutation, conservation de cause et reprise applicable.
Ces exigences ne prouvent ni leur couverture native ni une exécution de sécurité.

## Parcours réel lu : correspondance affirmation → source et portée

Les chemins Rust ci-dessous sont relatifs à `rust/crates/swarm-engine/src/`.
Les noms de fonctions permettent de retrouver les passages sans supposer une
stabilité des numéros de ligne. Toutes ces observations sont **statiques**.

| Étape | Producteur, vérificateur et consommateur lus | Ce que le passage établit / limite |
| --- | --- | --- |
| Constitution | `managed_repository_candidate.rs::merge_candidate` ; `managed_repository_candidate_store.rs::coordinates`, `ManagedCandidate`, `binding_digest` | Fusion du résultat worker dans candidat cumulatif, commit mono-parent avant contrôles ; checkpoint privé v2 liant coordonnées, commits, plan complet, sources suivies et identités physiques. Un OID seul ne constitue pas tout ce lien. |
| Inventaire | `managed_repository_source.rs::tracked_sources` ; `root_store/task_validation/managed_execution.rs::capture`, `prepare_for_review`, `bound_source` ; `source_origin.rs::verify_base_source` | Arbre Git complet suivi, octets/modes des fichiers et cible littérale des liens ; gitlinks refusés. Inputs déclarés des contrôles et répertoires observés séparément ; source liée au candidat, producteur, soumission, autorité et projection de politique. Ce n'est pas un inventaire automatique de dépendances transitives. |
| Contrat et réglages | `review_contract.rs::review_contract`, `managed_review_contract` ; `managed_repository_controls.rs::plan` ; `root_store/task_validation/source_policy.rs::projection`, `generation` | Contrat tâche : titre/livrable/critères/IDs de dépendances/brief/mémoires. Cohorte : tâche courante + acceptées, dernière tentative et politique. Le plan normalisé dépend aussi des réglages ; le seul contrat métier ne suffit pas. Projection explicite des réglages d'exécution, avec génération préservant retrait/restauration ; aucun graphe de paquets n'est calculé ici. |
| Exécution des contrôles | `root_store/task_validation/process.rs::source`, `gate_command` ; `biba.rs::Prepared::prepare` | Source revérifiée et commande issue de politique ; environnement vidé puis valeurs des noms autorisés copiées depuis le processus hôte ; PATH doit être autorisé pour un nom simple. Le chemin de programme et l'allowlist ne sont pas à eux seuls l'empreinte de ses octets, de ses bibliothèques ou des valeurs héritées. Biba reste un contrôle distinct, pas une preuve de fraîcheur. |
| Contexte reviewer | `root_store/review_preview/inputs.rs::assemble_managed_using` ; `managed_context.rs::build` | Rapports de cohorte, chaque source suivie en texte ou cible littérale de lien, inventaire, politiques et observations des contrôles ; protocole/catalogue et prompt liés par empreintes. Les limites ou formats non pris en charge refusent ; le reviewer n'acquiert pas les dépendances externes par simple présence d'un chemin/lockfile. |
| Avis et stockage | `root_store/reviewer_native/guard.rs::inputs_match_source`, `fresh_using` ; `publication.rs::prepare_using` ; `root_store/native_review_context.rs::recorded_native_review` | Garde de l'appel/résultat sur sources/prompt/contrat/provider/workflow et génération de réglages ; journal privé, ligne `native_review_jobs`, événement `review.result`, contexte et reçu public rapprochés. Un verdict enregistré n'accepte pas la tâche ; `verdict_applicable` historique n'est pas une garantie universelle de fraîcheur future. |
| Péremption | `review_evidence.rs::check_current_review_evidence`, `check_review_evidence_policy` ; `root_store/reviewer_native/stale.rs::changed`, `prepare` | Vérifications de tentative/contrat, contexte projet et artefacts selon parcours. Le retrait explicite d'un original sain devenu périmé produit `stale`, garde l'original et ne crée pas un avis favorable. Ce parcours de récupération d'un job non terminal n'est pas la preuve d'un retrait automatique de tout avis déjà publié. IO/absence/illisibilité ne sont pas assimilées à un changement démontré. |
| Intégration et acceptation | `root_store/managed_publication.rs::observe`, `verify_recorded_cohort` ; `managed_publication/writer.rs::publish_managed_candidate`, `current` ; `task_lifecycle.rs::transition_managed_publication` ; `root_store/native_review_context.rs::verify_current_native_review` | Candidat détenu/rafraîchi, originaux et cohorte, politiques, rapports, gate, prérequis, Work/réglages comparés ; transition atomique pointeur candidat + tâche acceptée + règlement worker + événement canonique. Rejeu distinct, sans nouvel appel. La preuve opaque ne vient pas du JSON client. Cette chaîne ne découvre pas une entrée extérieure absente des projections. |

Point à qualifier séparément : `ReviewCandidate::preview` recalcule une source,
alors que `recorded_evidence_for_review_result` sélectionne les contrôles par
empreinte de reçu historique (`Origin::ReviewedReceipt`). Les gardes d'égalité
avant application d'un résultat reviewer et celles après enregistrement/publication
ne sont pas interchangeables. La portée des changements de réglages après avis
déjà enregistré doit être vérifiée au consommateur final ; le cas stale existant
ne la clôt pas. Aucun contournement ni échec de ces gardes n'a été exécuté ici.

Parcours public Rust retrouvé dans `rust/crates/swarm-cli/src/task_validation.rs`
et `lib.rs` : `task validation-candidate`, `validation-preview`, `validate`,
`review-inputs`, `publication-preview`, `publish` ; invocation reviewer via
`task review`, telle qu'utilisée par `tests/rust_native_managed_reviewer.py`.
Ce sont des opérations distinctes ; `review-check` vérifie une réponse fournie
et ne crée ni original natif, ni autorité, ni acceptation.

Comparaison Go, sans déduire de parité avec Rust :
`internal/engine/managed_integration.go::integrateManagedAttempt`,
`prepareManagedCandidate` figent le résultat, constituent le candidat, exécutent
les contrôles de la tâche et des acceptées, demandent la revue puis publient.
`managed_review.go::preparedManagedCandidate` réutilise un checkpoint sur
previous/result/contrat ; `managedReviewsForPublication` rapproche candidat,
reçu, contexte reconstruit, politiques et critères. `saveManagedReview` conserve
l'avis. La mutation d'intégration installe les preuves et statuts acceptés dans
la transaction ; la référence Git est consultative. Le contrat de cohorte n'est
pas davantage un résolveur de dépendances effectives. Aucun parcours Go exécuté.

## Fichiers apparents identiques : cas discriminants proposés

| Changement isolé | Couverture lue / inconnue à conserver |
| --- | --- |
| Dépendance versionnée modifiée avec le candidat | Sources suivies et candidat changent ; liens anciens doivent refuser. Vérifier au chemin d'acceptation, sans se contenter d'un refus de préparation. |
| Valeur par défaut de contrôle modifiée, fichiers identiques | Plan normalisé comparé au checkpoint ; contrat métier seul insuffisant. Choisir une valeur qui change réellement le plan, conserver décision/settings et résultat. |
| Réglage pertinent change après enregistrement d'un avis | Gardes de settings pendant le job observées ; effet après avis enregistré à qualifier séparément, ainsi que restauration de politique et réutilisation historique. |
| Même chemin absolu de programme, octets de dépendance externe remplacés | Pas de résolution transitive/empreinte universelle trouvée dans les producteurs lus. Hypothèse prioritaire : l'inventaire et le contexte peuvent rester identiques ; vérifier d'abord que la fixture autorise réellement cette dépendance au point d'effet. |
| Même allowlist, valeur héritée différente ; bibliothèque/cache/résolution externe différente | L'allowlist n'est pas la valeur effective. Couverture de bout en bout non établie ; aucune promesse sur dépendance réseau ou environnement arbitraire. |
| Lien suivi identique, contenu de sa cible changé | L'inventaire suivi lie le texte du lien. Un input déclaré distinct peut couvrir la cible ; ne pas affirmer un contournement sans lire son résolveur et établir que le consommateur la lit réellement. |

## Scénario GI-V1V2 proposé — NOT TESTED pour chaque étape

Fixture isolée, sans mission réelle, fournisseur payant, secret ou réseau requis.
Réutiliser les producteurs publics et programmes inertes existants ; ils prouvent
les liens de protocole, pas la qualité d'un audit de sécurité par un spécialiste.
Une vraie revue indépendante exige un contexte séparé, les originaux complets,
et des critères de sécurité explicites. Le worker ne décide pas son acceptation.

1. **GI-01 / témoin V1** : déclarer la dépendance et sa pertinence pour un critère
   de sécurité observable ; conserver ses vrais octets/valeurs résolus, les
   empreintes source/plan/candidat/binaire/profil, contexte réellement reçu et
   original d'avis R1. Dans une fixture témoin distincte, publication/acceptation
   réussie sous R1 ; dans la fixture de mutation, V1 est prêt mais non publié.
   Un `passed` seul ou un refus commun de Biba/permissions n'est pas ce témoin.
2. **GI-02 / V2** : après R1, changer exactement une dépendance effective et
   constater le nouvel effet par l'observateur détenu. Pour la variante invisible,
   conserver les hashes égaux des sources suivies, rapport et commande ; relever
   séparément ce qui diffère dans le graphe effectif. Ne pas modifier le reçu R1,
   la base ou le verdict pour fabriquer V2.
3. **GI-03 / ancien avis insuffisant** : tenter le parcours normal de publication
   et son chemin d'acceptation applicable. Attendre un refus motivé avant
   intégration, pointeur/état métier inchangés, R1 original conservé et inapplicable
   à V2, aucune émission interdite ni nouveau fournisseur implicite. Si V2 est
   accepté, retenir ce résultat en FAIL avec ses preuves, sans le corriger par
   effacement/restauration silencieuse. Un rejet pour mauvaise révision de requête
   ne démontre pas le garde de dépendance.
4. **GI-04 / revue fraîche** : reconstituer inventaire, contrôles affectés et
   contexte V2 ; obtenir un nouvel avis R2 réellement lié à V2. Si le moteur ne
   fournit pas de reprise publique après avis `passed` devenu inapplicable,
   enregistrer ce prérequis et cadrer son contrat général ; ne pas supposer que
   `review-retry` ou `review-retire-stale` accepte ce cas. Conserver tentative,
   consommation et histoire conformément au parcours pris en charge.
5. **GI-05 / reprise** : nouvelle décision autorisée et publication/acceptation
   sur V2 avec R2 ; observer candidat, statut, événement, effet et rejeu sans
   deuxième appel/débit. R1 reste consultable pour V1. Répéter après réouverture
   du Root ; toute dépendance changée entre R2 et application impose un nouveau
   contrôle. Absence d'atomicité sur un état externe : limite explicite, pas PASS.

Avant chaque variante, établir son propre témoin sous mêmes réglages et droits.
Variantes nécessaires : dépendance déclarée, dépendance extérieure non inventoriée,
valeur effective modifiée sans édition de fichier, retour V2→V1, entrée manquante
ou illisible, changement pendant la fenêtre revue→application. Historique non
réécrit ; raison, propriétaire et condition de reprise conservés pour tout refus.

## Réemploi des recettes et preuves historiques

| Recette maintenue / sources lues | Réemploi proposé et limite |
| --- | --- |
| `native-managed-review-protocol.json` ; `rust_native_managed_review_inputs.py`, `rust_native_managed_protocol.py` | Matérialisation/inventaire/catalogue, refus des entrées non supportées ; inspection seule, aucune acceptation démontrée. |
| `native-managed-first-publication.json` ; `rust_native_first_publication.py` | Témoin worker/contrôles/reviewer inerte, publication atomique, preuves corrompues/restaurées et rejeu. Restaurer des octets V1 n'est pas obtenir une revue R2 sur V2. |
| `native-managed-publication-current.json` ; `rust_native_publication_current.py` | Trois altérations binding/cohorte/result et restauration ; contrôle de lecture courante, pas changement réel de dépendance. Ses corruptions SQL sont des fautes de fixtures, pas une manière de constituer V2. |
| `native-reviewer-stale-original.json` ; `rust_review_stale_original_cases.py`, `reviewer_native/stale_sdk.rs` | Rapport/settings/pause/autorité changés, original sain et sortie non verdict, sans nouvel appel. Ne prouve pas R2 puis acceptation, ni le cas après avis favorable déjà enregistré. |
| `native-reviewer-task-acceptance.json` ; `rust_native_review_acceptance.py` | Acceptation ordinaire et corruption/refus/restauration ; socle pour assertions de pureté, distinct de la publication Git gérée. |

Ces manifestes sont lus, **aucun n'est lancé ni modifié**. Leur liste `inputs`
ne vaut pas graphe de compilation ni inventaire de l'environnement hérité :
`tools/verification/README.md`, début FR/EN, le précise explicitement.

Preuves d'exécution antérieures : `docs/plans/migration-rust/REPLAYS.md`, lot159,
rapporte quatre variantes stale PASS avec limites ; `PLAN.md`, rubrique S2 première
publication, et le [rapport du 26 octobre](../migration-rust/evidence/20261025-scaling-s2-oi9dgo_e/native-first-publication-ytg974j3/RESULT.md)
rapportent publication/acceptation et altérations sur fixtures, profil et binaire
installé de l'époque. Le rapport distingue reviewer inerte et revue indépendante
non obtenue. **Ce sont ici des résultats historiques rapportés** : le répertoire
`recipe-final/` examiné est vide et `activation/public-flow/` ne contient pas le
`result.json` annoncé. Les originaux archivés ne sont pas récupérés ou réauthentifiés
dans cette analyse. Aucun PASS ancien transféré au HEAD courant ou à GI-V1V2.

## Proposition minimale, propriétaires et clôture de ce cadrage

Option A : traiter toute modification Git comme invalidation suffisante. Elle
couvre les octets suivis mais manque précisément la variante invisible demandée.
Option B retenue comme **proposition** : prolonger le parcours/recette de première
publication avec une dépendance effective contrôlée et un changement à la fois,
en réutilisant inventaire, journaux, contexte, garde et publication existants.
Représenter la dépendance par un input existant lorsque celui-ci décrit et lie
réellement le consommateur ; sinon qualifier le manque avant tout nouveau schéma.
Pas de TTL, plafond, second système de verdict ou audit universel proposé.

Ordre de reprise : GI-T1 inventorier le consommateur et ses résolutions réelles
→ GI-T2 établir témoin et reproduire V2/R1 par parcours public
→ GI-T3 seulement si défaut reproduit, proposer correction au contrat commun
→ GI-T4 nouvelle revue applicable, reprise et recette/refus/réouverture.
Codex porte diagnostic/cadrage ; reviewer séparé porte l'avis ; moteur applique
les gardes ; autorité compétente porte décision et acceptation. Fabrice décide
l'autorisation d'exécution/développement : elle manque volontairement ici.

Améliorations conservées : GI-A inventaire effectif au consommateur (valeur attendue :
éviter un faux « mêmes fichiers ») ; GI-B distinguer péremption pendant job et après
avis enregistré ; GI-C retrouver les originaux historiques avant réemploi. Owner
Codex ; toutes proposées/non vérifiées, reprise sous autorisation correspondante.
GI-C exige localisation et lecture des vrais membres archivés, pas un nouveau
PASS déduit d'un pointeur ou d'un rapport. Aucun archivage volumineux créé ici.

Revue documentaire de cet incrément : aucune fonction, politique, valeur par
défaut ou issue d'erreur produit modifiée ; pas de docstring à modifier, pas de
version livrée ou `version_history.json` à annoncer. Le plan PoC FR/EN et le
registre d'idées reçoivent le lien/état courant. Fichiers persistants seulement ;
aucun contexte embarqué ou processus existant n'est actualisé par ces notes.
Le cadrage est terminé ; GI-01 à GI-05 restent tous NOT TESTED.

Contrôle documentaire final : `git diff --check` sans erreur. Le registre
`docs/communication/` est ignoré par `.gitignore` : sa mise à jour est locale,
sans ajout forcé à Git ; le présent cadrage conserve aussi sa provenance et ses
suites dans `docs/plans/`. Aucun commit ni publication effectué.

## GI-T1/GI-T2 — reprise autorisée, protocole avant exécution

Fabrice autorise maintenant la qualification du consommateur et la reproduction
en fixture isolée, avec témoin V1 et preuves avant correction. Base de reprise :
HEAD `3b347399ee08e997295b28267d24c80bf375d364`. Les paragraphes précédents
restent le cadrage historique ; aucun résultat nouveau n'y est rétroattribué.
Méthodes debug/replan, Codex seul ; aucun correctif du moteur n'est autorisé par
ce lot de reproduction. Les modifications existantes des notes PoC et de l'étude
TechOnlineNews restent préservées.

Consommateur retenu : commande de contrôle persistée `python3 security-consumer.py`,
admise par le moteur natif dans son vrai workspace candidat sous la frontière
Biba existante. Le script est suivi et figé avant les workers. Il lit réellement
`effective-security.json`, non suivi, dans ce workspace. Les inputs déclarés sont
`tracked.txt` et le script ; l'omission volontaire de la configuration constitue
précisément l'hypothèse de défaut, sans prétendre à une découverte exhaustive.
La politique V1 autorise seulement fixture-A ; V2 autorise aussi fixture-B.
Le même contrôle demande que B soit refusé : V1 retourne deny/exit0, V2 doit
retourner allow/exit42. Ces destinations, trois contrôles et exit42 sont des
données/observables de fixture ; aucune politique produit nouvelle.

Recette dérivée minimale : `gieth-effective-dependency-v1-v2.json`, utilisant
`tests/rust_gieth_effective_dependency.py` et les helpers maintenus de workers,
inspection, reviewer et première publication. Le premier Root démontre V1 accepté
par la recette de première publication existante. Le second constitue ses propres
contrôles et R1 valides, obtient publication-preview applicable, puis change la
configuration seule. L'observation V2 exécute la même commande/cwd comme propriétaire
de fixture : diagnostic du consommateur, pas un second contrôle natif ni une preuve
Biba sur V2. L'acceptation se tente uniquement par `task publish`, à bonne révision.
Comparer tous les octets suivis, les originaux R1 et les compteurs fournisseur.

Le reviewer est un programme inerte générant un vrai original de protocole,
pas un expert de sécurité ni une revue indépendante humaine/modèle établie.
Un ancien avis ne doit pas suffire après ce changement pertinent : une acceptation
V2 produit **FAIL/exit1**, conservé comme reproduction et jamais comme PASS global.
Un refus sera analysé par sa cause ; une précondition étrangère ne clôt pas GI-T2.
Pas de SQL écrit, réseau métier, service actif, installation ou correctif produit.

Binaire réutilisable identifié : qualification normale
`.cache/local-delivery/rust-z1haax4r/build/profile/result.json`, SHA-256
`84b1e78c8dbd9f38c5ebebb7fd48c097f6e7de76805249fd06592730aa5211a8`.
Les 777 sources effectives et configurations Cargo ont été comparées au dépôt,
sans différence ; le helper doit répéter et conserver ce contrôle avant/après.
Aucun nouveau build requis. Watchdog global explicitement absent ; les gardes
produit demeurent. Fixtures sur l'aire Unix configurée, preuves destinées à Lexar
après vérification du montage et archivage préservant les métadonnées.
État de cette rubrique : protocole enregistré avant lancement, résultats à ajouter.

Incident de méthode GI-M1 : `run-01` FAIL, source stable, 49,84 s ; V1 a ses
trois vrais contrôles favorables et R1 natif inerte, mais la première publication
n'est pas tentée. `rust_native_first_publication.py` exige le fichier de naissance
dans le contexte d'inspection fourni ; l'inspection réutilisée ne transmet que
`setup.json`, alors que l'original de naissance reste sous les preuves workers.
Cause : contexte de réemploi incomplet dans le nouveau scénario, pas un refus de
sécurité V2. Original traceback et canaux conservés dans `flow-01/failure.json`
et `run-01/`. Replan : fournir les octets exacts résultat/requête/naissance dans
un **nouveau** handoff avec chemins/hashes de provenance, sans modifier les preuves
précédentes, le helper maintenu ou la base. `--reuse-control` permet de reprendre
le même V1 sans nouvel appel worker/reviewer ; nouvelle mutation V2 sur Root neuf.
Cette précondition corrigée justifie `run-02`, pas une relance inchangée.

GI-M2 : `run-02` a effectivement accepté V1 (`v1-publication/result.json`, huit
obligations du parcours maintenu), puis construit le second V1 avec trois contrôles
et R1 valides. Arrêt avant injection : mon assertion exigeait à tort
`publication_ready=true`. Source relue : `root_store/managed_publication.rs:194–205`
retourne toujours false avec `managed-publication-write-admission-required` ;
l'aperçu n'accorde jamais l'écriture. Sortie conservée et assertion corrigée pour
vérifier originaux/candidat/support et ce blocage exact, sans supprimer de garde.
`--resume-from flow-02` reprend ces deux témoins, par lecture de leurs originaux
et contrôle public de V1 accepté ; aucune relance de worker/contrôle/reviewer.
`run-01` et `run-02` ont chacun intégrité VERIFIED et statut historique FAIL.
GI-T2 reste incomplet à ce stade ; aucune modification V2 encore effectuée.

## GI-T1/GI-T2 — résultats observés, avant toute correction

**Reproduction obtenue**, sur le binaire normal SHA-256
`84b1e78c8dbd9f38c5ebebb7fd48c097f6e7de76805249fd06592730aa5211a8`,
777 entrées effectives comparées avant/après, zéro différence, aucune compilation.
La preuve est bornée à une dépendance effective volontairement non déclarée dans
la politique de contrôles de cette fixture ; elle n'établit pas une vulnérabilité
sur toutes les politiques ni une absence de fraîcheur pour les inputs déclarés.

| Critère | Résultat réel | Preuve et portée |
| --- | --- | --- |
| GI-T1 : vrai consommateur | ÉTABLI | Trois commandes natives `python3 security-consumer.py` par Root lisent la configuration V1 dans le candidat ; sorties originales deny et hash réel. La configuration n'est ni suivie, ni dans les inputs déclarés, ni fournie comme matériau au reviewer. |
| Témoin V1 valide et accepté | PASS | `flow-02/v1-publication/result.json`, huit obligations du helper maintenu : vraie publication/acceptation, refus/restaurations des preuves, rejeu ; aucun avis positif injecté en SQL. |
| Second V1, avant changement | PASS | `flow-02/mutation/review/result.json`, cinq assertions reviewer ; inspection existante 12 assertions, trois contrôles V1 favorables ; original/candidat vérifiés par `flow-03/v1-preflight.stdout`. |
| Changement effectif V2 | ÉTABLI | Politique A seule → A+B ; même commande et programme, diagnostic V2 allow/exit42. Ce diagnostic est lancé par le propriétaire de fixture, sans prétendre être une seconde exécution native/Biba. Aucune transmission externe. |
| Refus de V2 sous R1 | **FAIL** | `flow-03/v2-publish.stdout` : `task_accepted=true`, état `submitted` révision39 → `accepted` révision40. Onze entrées suivies identiques ; avis et validation automatique entiers inchangés ; zéro nouvel appel reviewer. |
| Intégrité de la preuve | VERIFIED / FAIL conservé | `recipe.py verify` sur `run-01`, `run-02`, `run-03` ; chaque statut enregistré reste FAIL. Les deux premiers sont des incidents de méthode ; le troisième est le résultat attendu d'une reproduction qui révèle l'acceptation interdite par le scénario. |
| Nouvelle revue applicable R2 et reprise | NOT TESTED | GI-T3/GI-T4 et GI-04/05 non autorisés dans cette demande ; aucune correction ni acceptation V2 sous preuve fraîche fabriquée. |

Les contrôles de fixture portent explicitement l'observable « fixture-B doit être
refusé » dans leurs commandes/justifications, reliées au critère1. Le helper garde
son libellé générique de tâche `Owned handoff`. Le reviewer inerte renvoie un avis
au format valide et produit de vrais originaux ; **ce n'est pas une expertise de
sécurité indépendante**. Le test établit la réutilisation du mécanisme de verdict,
pas la qualité d'un spécialiste ou le raisonnement d'un modèle.

Origine causale corroborée : la configuration est consommée mais absente de la
fermeture des inputs déclarés/suivis. `managed_repository_source::tracked_sources`
énumère l'arbre Git ; `managed_execution::capture` observe les inputs de politique ;
`prepare_for_review` lie ces observations. La publication rafraîchit et vérifie
ces mêmes objets. Elle peut donc valider leurs empreintes inchangées tout en
ignorant le changement de cette dépendance non représentée. Aucun garde supprimé,
jeton, valeur `allow` ou revue fabriquée ajouté à la requête d'acceptation.
Le candidat Git accepté ne contient pas la configuration non suivie : le test
ne prétend pas démontrer son déploiement ni un export de données.

### Preuves conservées et réemploi

Aire locale unique : `.cache/gieth-v1-v2-20261108/`.
`consumer-qualification.json` rapproche commandes natives V1, matériau reviewer,
politiques/empreintes V1/V2, égalité complète des avis/validations et décision.
`source-review.json` lie l'auto-revue aux octets du test et de la recette :
neuf fonctions documentées ; constantes = entrées de fixture ou observables,
aucune politique produit nouvelle ; causes/canaux conservés, aucun refus rendu
succès vide. Les sources des helpers et le profil normal sont conservés séparément
du statut de la recette. Les Roots restent sur l'aire Unix configurée, jamais
une base de mission réelle ou `/tmp`.

Le test/recette permanents sont dans le dépôt ; pas seulement dans `.cache`.
Le manifeste dérive de première publication, réutilise les helpers existants et
attend un refus : il sort1 pour le cas reproduit. `--reuse-control` transmet une
naissance originale manquante dans un nouveau handoff ; `--resume-from` réemploie
le V1 accepté et le second V1 revu, avec vérification publique, sans relancer leurs
producteurs. Les trois exécutions et leurs sources exactes restent distinctes.
GI-M3, amélioration de méthode après reproduction : une branche hypothétique de
refus ne doit pas devenir PASS sans qualification de sa cause. Le helper final
classe désormais ce cas PARTIAL/exit1 et marque explicitement le besoin de
qualification ; un refus étranger ne peut satisfaire le marqueur positif attendu
par la recette. Ce durcissement de classement n'est pas une correction moteur.
La branche d'acceptation indue exercée par `run-03` est inchangée ; ses octets
exacts sont dans l'archive d'inputs de cette exécution. La nouvelle branche de
refus reste non exercée ; aucun parcours produit relancé pour cette clarification.

### Suite minimale proposée, non engagée

GI-T1 et GI-T2 sont terminés sur ces preuves. GI-T3 peut comparer une déclaration
complète d'entrées imposée/vérifiée au consommateur, et une fermeture d'exécution
empêchant les dépendances non représentées ; un simple renouvellement de R1 ne
détecte pas une entrée toujours absente. Retenir la solution générale la plus
petite après définition de cette frontière, puis tester refus et R2 applicable
sans modifier les originaux. Aucun ajout automatique d'un TTL ou plafond proposé.
Codex porte ce diagnostic ; Fabrice décide du périmètre de correction GI-T3.

Apprentissage de méthode : réutiliser un helper exige de lui transmettre les vrais
originaux attendus ; relire le contrat final des champs de preview avant assertion.
GI-M1/M2 ont été diagnostiqués et repris sans recréer leurs témoins, preuves
originales gardées. Le suivi FR/EN est actualisé ; aucun contexte embarqué ou
processus actif n'est modifié. Aucune version locale produit à installer : seules
la reproduction, sa recette et leurs documents sont ajoutés.

### Conservation vérifiée et clôture GI-T1/GI-T2

Archive sur Lexar réellement montée `/dev/sda1`, vfat, écriture disponible :
`/media/fpizzi/Lexar/swarm-evidence/gieth-v1-v2-20261108/gieth-v1-v2-20261108-evidence-01.tar.gz`.
35 131 479 octets ; SHA-256
`bd184066fded75299112ea8d461dd0dd372e30680b172f545735924f840c2e31`.
2 276 fichiers/liens relus contre leurs sources, copie USB entièrement relue et
empreinte concordante. Archive : trois runs/flows, deux Roots de fixture complets,
38 sources de helpers, binaire exact et qualification/archives effectives.
Aucun original supprimé ou remplacé. Métadonnées Unix dans l'archive, pas de
fixture exécutable directement sur FAT32. Index local :
`.cache/gieth-v1-v2-20261108/LOCATION.json` et `archive-inventory.json`.

Les trois dossiers de recette ont réellement été restaurés depuis l'archive USB
dans `.cache/gieth-v1-v2-restored-20261108/`, puis `recipe.py verify --evidence`
a rendu VERIFIED / statut enregistré FAIL pour chacun. Ce contrôle ne relance
aucun test ni appel. Restaurer après contrôle du SHA dans un dossier Unix neuf,
jamais sur un Root actif. Les chemins absolus et identités physiques historiques
ne sont pas une permission transférable de réexécution de ces Roots.

Durées observées, sans en faire des politiques : run01 49,84 s, run02 57,77 s,
run03 5,79 s. Sources déclarées stables dans chaque exécution. Aucun test rejoué
après la reproduction positive : la clarification conservatrice GI-M3 est revue
statiquement dans `source-review-final.json`, distincte des octets testés de run03.
`git diff --check` ne signale pas d'erreur ; liens documentaires contrôlés.

Changement de HEAD observé pendant la session : `3b347399…` → `aaf1a7b8…`, commit
documentaire Peter portant uniquement les deux notes PoC. Aucun commit créé par
ce travail. Rapprochement final des 777 entrées effectives : toujours zéro
différence ; les preuves du binaire restent donc dans leur portée. Les notes
préexistantes et l'étude TechOnlineNews sont préservées, sans appropriation de
leurs résultats. Le rapport final et ses empreintes de clôture sont aussi copiés
à côté de l'archive ; leur écriture n'est pas une nouvelle exécution.

## GI-T3/GI-T4 — plan autorisé et frontière de correction

Autorisation suivante de Fabrice : correction, refus V2/R1, R2 et reprise,
docs FR/EN, installation locale et RETEX. Base `aaf1a7b8…`, Codex seul,
APEX/replan ; originaux GI-T1/T2 inchangés. Critères : GI-F1 lier les fichiers
réguliers présents dans le workspace avant contrôles, même non déclarés ; GI-F2
refuser un R1 dont ces entrées ont changé ; GI-F3 renouvellement public explicite,
histoire et consommation conservées ; GI-F4 nouveaux contrôles et R2 applicables,
publication/rejeu ; GI-F5 profil livré, version/notes FR/EN, installation et RETEX.

Deux options examinées : confinement des lectures à une fermeture déclarée, ou
inventaire conservateur des entrées de copie candidate réutilisant source/reçus.
La première exige un contrat de lecture système/interpréteur/sorties absent et
ne constitue pas un petit correctif. Retenue : inventorier les fichiers réguliers
présents avant les contrôles, enregistrer la sélection initiale dans la préimage
privée et le reçu, relire les mêmes octets jusqu'à publication. Les fichiers
créés ensuite par les contrôles restent des sorties, pas des entrées initiales.
Aucune exception par mission, nom de configuration, date ou erreur. Pas de lecture
automatique du contenu privé dans le prompt ; empreintes et contrôles restent
distincts des originaux nécessaires au jugement. Dépendances hors workspace,
environnement et entrées générées pendant un contrôle restent une frontière
explicitement non hermétique, sans garantie universelle inventée.

Ordre : nouvelle régression attendue et cas invalides → inventaire/préimage/source
et garde de publication → opération publique de renouvellement sur changement
prouvé (archive R1, nouveaux contrôles/R2, sans remise de budget) → recette complète
→ revue de source/erreurs/docs → build normal/livraison locale → preuve réelle,
conservation et RETEX. Le V2 dangereux initial (B autorisé malgré contrôle qui
exige son refus) ne doit pas obtenir de R2 favorable ; la reprise favorable sera
également exercée sur un changement de configuration conservant le critère de
sécurité, avec ses nouveaux contrôles. Aucun succès fabriqué sur contrôle échoué.


## GI-T3/GI-T4 — résultat vérifié et installation

Cause générale corrigée : la sélection d’entrées provenait des seules déclarations.
`input_inventory.rs` sélectionne les fichiers réguliers initiaux de la copie candidate,
y compris non déclarés ; la préimage privée d’admission schéma 2 retient cette sélection.
`managed_execution.rs` et `review_candidate.rs` relisent ces entrées avant revue/publication.
Ce schéma privé est distinct du checkpoint candidat. Une ancienne préimage schéma 1
ne fabrique pas de liaison rétroactive. Fichier modifié, absent ou illisible : refus
avec cause, sans transformer une erreur d’observation en succès vide.

`task review-refresh` est l’opération CLI publique de reprise : requête stricte,
révision attendue, tâche soumise sans travail natif actif, R1/contrôles authentiques,
changement observé ou ancien schéma sans liaison. Elle archive R1 dans
`previous_reviews`, invalide les preuves courantes et exige nouveaux contrôles/revue.
Historique, jobs, tentatives, appels et consommation sont conservés. Rejeu sans
nouvel appel ; transaction annulée sans fausse revendication d’état appliqué.
Aucun nouveau plafond opérateur, droit de fournisseur, route HTTP ou bouton ajouté.
Contrat FR/EN : [FR](../../architecture/NATIVE-REVIEWER.md),
[EN](../../en/NATIVE-REVIEWER.md). Implémentation et recettes restent distinctes
comme preuves : les programmes de reviewer sont inertes, pas un jugement spécialisé.

Preuves locales `.cache/gieth-fix-20261108/` :

| Preuve | Résultat et portée |
| --- | --- |
| `unit-final.stdout` | Test nommé d’inventaire : entrées non déclarées, sorties générées, changement/restauration, absence, symlink et chemin hors copie ; PASS |
| `run-03`, `flow-03/result.json` | PASS, 176,55 s, sources déclarées stables ; recette revérifiée VERIFIED/PASS |
| `flow-03` | Témoin V1 réellement publié ; 11 entrées suivies inchangées ; V2 dangereux autorise B (exit42), publication sous R1 refusée ; R1/état/consommation conservés |
| `flow-03` | Variante V2 sûre différente (autorise C, refuse B) encore refusée sous R1 ; refresh explicite/rejeu, nouveaux contrôles, R2 distinct, publication/rejeu et réouverture acceptés |
| `flow-03` / `refresh-sdk` | Test SDK ciblé d’annulation SQL : erreur originale et absence de faux WorkRecorded ; binaire de test séparé de la livraison |
| `api-refusals-maintained/result.json` | Témoin de rejeu valide puis mauvaise révision et champ inconnu : causes typées, aucune mutation ni consommation |
| `legacy-run`, `legacy-flow` | PASS, 66,03 s, VERIFIED/PASS ; R1 produit par l’ancien vrai binaire, refus sous nouveau contrat, refresh puis nouveaux contrôles, originaux et consommation conservés |
| `rustdoc.stdout/stderr` | Documentation moteur compilée ; avertissement préexistant de lien privé dans memory_usage_decision.rs, aucune assertion de compilation sans avertissement |

Le V2 dangereux n’est pas accepté sous R2 : ses contrôles échouent. La reprise
favorable concerne explicitement la variante sûre, avec nouvelle preuve applicable.
Le run03 conserve son snapshot propre : l’ajout ultérieur des cas API et legacy
est vérifié séparément, sans prétendre à un nouveau parcours complet sur ces octets.
Le SDK rollback précède le dernier renforcement lexical de l’inventaire ; le test
unitaire final et la recette publique utilisent le contrat final concerné.

Livraison normale par `swarm.sh build`, puis `restart --prepared-build` :
**v0.1.1-dev.347**, SHA-256
`3512fdaed47a9f111959de2cfd8186276d1d32d073da510e0dd8e86913b0735f`.
Qualification `.cache/local-delivery/rust-t6j1_qkl/build/profile/result.json` :
779 entrées effectives archivées, profil sans feature de fixture et artefacts core
non test. Identité du processus relue, cinq Works/revisions conservés ; aucune
base de mission active utilisée comme fixture. Le dialogue protégé réel, après
connexion par Fabrice, affiche version, note GI FR/EN, commit de base exact
`9d64cb33d09a0285abae8975aa6fcdd2268e8e4d` et modifications non commitées.
Aucun commit ni push par ce travail. Notes compilées concordantes au registre.

## RETEX — cause, incidents de méthode et suites

- Cause : un inventaire déclaratif de candidat ne suffit pas à prouver la fraîcheur
  d’une configuration réellement consommée. Prévention bornée : retenir la sélection
  initiale et ses octets jusqu’au dernier consommateur, puis renouveler explicitement
  les preuves. Les contextes natifs n’ont pas été réécrits ; c’est le moteur livré
  qui applique cette garde et la CLI qui expose la reprise.
- Run01 FAIL conservé (167,55 s) : le moteur refusait déjà V2, mais un helper hérité
  exigeait zéro appel malgré R1 existant. Corrigé en conservation des appels/avis ;
  aucune remise à zéro pour réussir. Run02 FAIL (1,04 s) : chemin temporaire de
  candidat déplacé par le build ; aucun démarrage de fixture. Reprise avec binaire
  gelé adressé par empreinte. Les FAIL originaux restent revérifiables.
- Les témoins précèdent chaque défaut : V1 publié, refresh frais refusé, entrée
  restaurée après absence, rejeu valide avant requêtes invalides, vrai R1 historique.
  Les recettes existantes de workers/reviewer/publication sont réutilisées.
- Intervention humaine : autorisations successives et connexion au navigateur par
  Fabrice ; implémentation/diagnostic/vérifications par Codex seul. Aucun appel
  fournisseur payant ni audit indépendant. Pas d’acceptation globale inférée.
- Limites ouvertes, propriétaire contrat natif : dépendances externes, environnement,
  programmes/interpréteurs, métadonnées Git, IPC et entrées créées pendant contrôles.
  Cette correction n’est pas hermétique ; reprendre avec un contrat explicite de
  fermeture des lectures et une fixture discriminante avant toute garantie plus large.
- GI-UI-1, propriétaire interface : Échap ferme Version, mais le focus observé revient
  au document au lieu du bouton. `web/runtime-health.js` tente déjà opener.focus ;
  cause non établie (visibilité de navigation à vérifier). Notes FR/EN, lien et version
  restent vérifiés ; conformité complète de focus non acquise. Hors modification
  moteur de cet incrément ; reprise : témoin navigateur isolé navigation ouverte /
  fermée, diagnostic du cycle close, correction commune et régression clavier.
- Amélioration de méthode : vérifier le chemin gelé après le producteur de build et
  traiter les compteurs comme historique à préserver, non comme état frais implicite.
  Capitalisation dans le helper maintenu, les recettes et cette note, sans réécrire
  l’archive historique ni attribuer une supervision autonome aux instructions.

Conservation : index local `.cache/gieth-fix-20261108/LOCATION.json`, archive USB
`/media/fpizzi/Lexar/swarm-evidence/gieth-fix-20261108/` avec empreinte et commande
restauration dans cet index. Voir aussi l’archive GI-T1/T2 ci-dessus. La conservation
ne constitue aucune nouvelle exécution ni permission de rejouer les Roots historiques.


### Conservation finale relue

Archive GI-T3/GI-T4 : `/media/fpizzi/Lexar/swarm-evidence/gieth-fix-20261108/gieth-fix-20261108-evidence-01.tar.gz` ; 67359525 octets, SHA-256
`b612a9b90c8f7cb9229a7f63d0b9247176b4b603addcdf6c2068c5e853a2d860`. 6010 fichiers/liens comparés aux originaux, copie USB relue intégralement.
Quatre dossiers de recette restaurés en répertoire Unix neuf et revérifiés :
run01/run02 VERIFIED/FAIL historiques ; run03 et legacy-run VERIFIED/PASS.
Aucun original supprimé. Le rapport final et les résultats de restauration sont
copiés à côté de l’archive (postérieurs à sa constitution), avec empreintes propres.
`git diff --check` et liens locaux ajoutés contrôlés ; aucune modification de
source compilée depuis le candidat installé. GI-F1 à GI-F5 vérifiés dans la portée
décrite ; GI-UI-1 et fermeture universelle des dépendances restent explicitement ouverts.


## GI-UI-1 — reprise autorisée, focus Version

Demande suivante : corriger le retour du focus Échap, parcours réel FR/EN, preuves,
docs et installation. Méthodes replan/verify-fix, Codex seul. Témoin livré 347 :
navigation seule + Échap ferme le menu et focalise nav-toggle ; dialogue Version
+ Échap ferme aussi le menu, cache version-open (zéro rectangle), focus BODY.
Cause lue : gestionnaire document keydown de web/swarm-shell.js ne vérifie pas
le dialogue modal, tandis que runtime-health.js tente déjà le retour à l’ouvreur.
Options : rouvrir la navigation lors du retour du focus, ou empêcher le gestionnaire
arrière-plan de consommer Échap sous modal. Retenue : seconde, commune aux dialogues
modaux, sans restaurer artificiellement des préférences de navigation. Les événements
déjà consommés restent également ignorés. Les dialogues non modaux ne monopolisent
pas ce contrat. Aucun moteur, droit, délai, budget ni traduction nouvelle.

Critères : témoin sans modal conservé ; modal actif ferme seul, menu reste visible,
retour version-open ; seconde touche Échap ferme menu et revient nav-toggle ;
réouverture et bouton Fermer ; FR/EN, thèmes clair/sombre, largeur réelle du navigateur ;
notes/version/commit/modifié du nouveau binaire. Régression maintenue avec double DOM
explicitement distinct du navigateur réel, FAIL avant correction conservé.
Preuves nouvelles : .cache/version-focus-20261108 ; historique GI intact.


### GI-UI-1 — correction et vérification locale

Gestionnaire commun corrigé : un modal natif (`dialog:modal`) garde Échap ;
un événement déjà consommé ne ferme pas la navigation arrière-plan. Le dialogue
Version conserve son gestionnaire close et le bouton reste visible pour son retour
de focus. Aucun nom de dialogue ni langue codé en exception.

Preuves `.cache/version-focus-20261108` : recette version-focus-escape, FAIL avant
correction (témoin navigation seule PASS), puis cinq assertions nommées PASS ;
les deux dossiers revérifiés VERIFIED, sans watchdog. Ce double DOM exécute le vrai
gestionnaire mais n’établit pas le comportement natif du navigateur. L’ancienne
recette version_ui ne suffit pas : sa surface de fixture ne charge pas le gestionnaire
de navigation et ne cache pas l’ouvreur ; elle ne couvrait donc pas cette cause.

Le parcours CUA réel, session authentifiée par Fabrice précédemment, vérifie FR/EN
× clair/sombre à la largeur réelle 609 px : titre focalisé à l’ouverture ; Échap
ferme Version, navigation toujours ouverte, version-open visible et focalisé.
Réouverture par Entrée + bouton Fermer vérifiées FR/EN ; seconde touche Échap ferme
la navigation et rend le focus à nav-toggle. Retour final en français/clair.
Aucune erreur ni avertissement dans la console capturée. Les réponses réseau
échouées ne sont pas inventoriées par ce canal ; sondes statiques séparées refusées
401/403, sans contournement d’authentification. Pas de comparaison indépendante des
octets HTTP protégés revendiquée ; archive des ressources compilées vérifiée et
comportement du vrai navigateur constaté. Pas de qualification visuelle à toutes
les largeurs, ni d’acceptation indépendante déduite.

Installation par build puis restart --prepared-build : **v0.1.1-dev.348**, SHA-256
`edea3b37f360d7adad75d6736c124aaa67847e0bfb41647ff3bd6c035ef886be`.
Profil normal sans feature de fixture, 779 entrées effectives archivées ;
notes FR/EN visibles, commit de base `1fec07d4874cc469c5a3905abb9bb64f855b2d26`,
modifications non commitées divulguées. Identité réelle relue via swarm.sh status.
Aucune entrée compilée modifiée depuis le candidat sélectionné. Des commits
extérieurs documentaires sont apparus pendant le travail ; aucun commit/push
créé ici et aucune appropriation de leurs changements.

RETEX complémentaire : l’ouvreur connecté n’est pas nécessairement visible ; la
responsabilité des événements modaux doit être conservée jusqu’au consommateur
arrière-plan. Prévention dans le gestionnaire commun et la régression maintenue,
transmission dans les docs FR/EN et notes embarquées reconstruites. Incidents de
méthode conservés : liaison CUA reprise après reset ; argument recette corrigé
avant exécution ; tentatives anglaises sous sous-menu replié refusées, puis
expansion réelle et nouveau témoin ; changement de langue suivi trop vite d’un
clic menu corrigé après relecture de son état. Aucun échec masqué en PASS.
GI-UI-1 clos pour le retour de focus demandé ; les limites externes du contrat GI
restent inchangées. Conservation/empreinte/restauration : index local
`.cache/version-focus-20261108/LOCATION.json` et archive Lexar dédiée.


Conservation GI-UI-1 terminée : archive `/media/fpizzi/Lexar/swarm-evidence/version-focus-20261108/version-focus-20261108-evidence-01.tar.gz`,
28103147 octets, SHA-256 `41dda0ec0c25a164e615e2eb0ef6768e97eb5960a9f071122b97538d8c808623`.
55 fichiers relus contre leurs sources, copie USB relue intégralement ; deux
dossiers de recette restaurés, VERIFIED/FAIL puis VERIFIED/PASS. Aucun original
supprimé. Rapport final postérieur à l’archive copié séparément avec empreinte.
`git diff --check` ne signale aucune erreur.
