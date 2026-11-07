# Plan priorisé — écarts entre articles, produit et preuves

**P1/G1 autorisé puis première tranche locale livrée le 3 novembre 2026** :
[accord exact du règlement, dev.322 et preuves](P1-G1-20261103.md).
G1 reste partiel aux frontières humaines/OS ; G2…G7 ne sont pas lancés par cette tranche.

**P0/G0 autorisé le 3 novembre 2026** : réalisation et preuves suivies dans
[P0-G0-20261103.md](P0-G0-20261103.md). La proposition et les mentions « non exécuté »
ci-dessous restent le plan initial historique ; G1…G7 ne sont pas démarrés par ce lot.

3 novembre 2026 · [English](../../../en/plans/actions-confiance/paea/GAP-CLOSURE-PLAN-20261103.md)
· [Audit C01…C11](ARTICLES-REALITY-20261103.md)
· [Plan PAEA canonique](../PAEA-APEX.md)

**Proposition de séquencement, non exécutée et non adoptée dans le moteur.**
Méthodes : spec-builder et replan ; revue de cohérence spec-audit par le même
auteur. Ce plan ne lance ni développement, délégation, campagne, appel fournisseur,
opération privilégiée ou publication. Les autorisations antérieures restent dans
leur périmètre ; cette proposition ne les élargit pas.

## Objectif et base conservée

Fermer la chaîne principal → mandat → contexte → proposition → accord exact →
engagement → observation → reprise, pour logiciel et facturation fictive.
Distinguer implémentation, preuve locale, installation, preuve humaine et verdict
indépendant. Aucun paiement réel ou processus critique de production inclus.

HEAD repris : 277e337cb400b81ed3d0beb620006a6ee7bdbcb9 ; arbre modifié.
Le fichier bin/swarm et l'image /proc/1426548/exe ont été comparés à la préparation :
SHA256 e2f0039a1f3a34957c161e770c4e5efdd46f4e3a16fc82fe9cf5d9761dc94da3,
dev.316 identifié par l'audit. Le démon cockpit sous UID distinct existe déjà.
newuidmap/newgidmap restent absents du PATH observé ; ce constat doit être refait
avant installation. Les PASS antérieurs restent historiques ; aucun rejeu fonctionnel
nouveau n'est revendiqué par ce plan.

Choix : terminer les raccords existants plutôt que reconstruire une plateforme.
Attendre tous les intervenants avant de coder bloquerait inutilement les travaux
sur fixtures ; développer seulement des fixtures ne permettrait pas la clôture
protégée. La stratégie combine préparation locale et portes humaines explicites.

## Lots ordonnés

« Maintenant » signifie techniquement réalisable par Codex sur sources et fixtures,
dès le développement de ce lot autorisé ; aucun lot n'est exécuté par cette demande.
Priorités fondées sur risque et dépendances, sans date ou durée inventée.

| Lot / priorité | Écarts et plan existant | Ce que Codex peut implémenter et vérifier localement | Porte humaine ou indépendante | Critère de sortie |
|---|---|---|---|---|
| G0 — P0 Base et diagnostic | C01/C09/C11 ; PA-00/09 | Matrice source/preuve/image/limite ; résumés et docstrings périmés ; conservation des FAIL ; statut concurrent explicite | Session humaine si nécessaire pour l'interface connectée | Deux images identifiées séparément ; statut sans attente silencieuse ; historique conservé, pertes déclarées |
| G1 — P1 Autorité et effets | C02/03/04/08 ; PA-02/03/08/09 | Raccord autorité courante → accord exact → engagement des deux effets ; tests identité/projet/mandat/révocation/UNKNOWN sur fixtures | Opérateur : identités réelles et services protégés ; approbateur distinct si exigé | Le service d'effet consomme l'autorité exacte au dernier usage ; refus sans effet ou issue incertaine explicitement conservée |
| G2 — P1 Mémoire Rust | C05 ; PA-06, FPA02A, MEM-P2…P4/M20, BLP-04 | Admission/version, sélection/contexte, retrait/remplacement puis droits lire/résumer/transmettre/compacter ; tous chargeurs recensés | G1 pour identité réelle ; décision opérateur seulement pour une portée non définie | Retrait bloque les nouveaux usages, contextes déjà reçus conservés ; aucune élévation de droit ou budget par mémoire |
| G3 — P1 Confinement/secrets | C07 ; PA-04/05/07, AC-02, BLP-05 | Inventaire des voies ; tests synthétiques ; barrières fichiers/FD/processus/réseau/descendants et politiques partagées ; respecter les prérequis AC-02 | Administrateur : helpers/mappings/comptes ; propriétaire métier : accès nécessaires ; services protégés déployés | Témoin valide puis contournements refusés ; aucun octet synthétique sensible reçu à destination interdite ; pas de repli non confiné |
| G4 — P2 Délégation/A2A | C06 ; PA-06/07, T14/I02/I14 | Porter T14 par sous-ensembles : mandats enfants, audience, provenance, révocation et rejeu ; garder les refus pour le non porté | Intégration réelle après G1 ; composition qualifiée après G2/G3 ; droits délégables fixés par politique | L'enfant ne dépasse jamais mandat/audience du parent ; message et UID ne créent aucun droit |
| G5 — P2 Supervision/E2 composée | C01/02/03/08/10 ; PA-07…10 | Incident reçu → décision → application → effet ; deux domaines sur un candidat ; reprise, preuve fraîche, CLI/API/browser | Humains pour approbations/vigilance ; fournisseur réel seulement avec moyens et droits établis ; revue autre contexte à organiser | Reprise causale sans doublon ni acceptation périmée ; mode human bloque la suite ; preuves fixture et agents réels séparées |
| G6 — P1 organisationnel, dès G0 | C09/11 ; PA-11 | Paquet de revue, contrats/vecteurs, collecte et comparateur B2 avec mêmes protections métier | Commanditaire désigne évaluateur et équipe tierce ; ils acceptent mission, corpus réservé et protocole | Préenregistrement avant campagnes ; seconde frontière d'exécution sans dépendre du moteur Swarm |
| G7 — P3 Évaluation/RETEX | C09/11 ; PA-12/13 | Reproduire les défauts reçus, corriger le contrat commun, livrer, préparer RETEX | Évaluateur mène T01…T22 et réévalue les corrections ; participants habilités | Verdict indépendant borné, défauts et exclusions conservés, puis RETEX final |

## Dépendances de qualification

```mermaid
flowchart TD
  G0["G0 Base et preuves"] --> G1["G1 Autorité et effets"]
  G0 --> G3a["G3 Inventaire et préparation"]
  G0 --> G6["G6 Préparer E3 / désigner les tiers"]
  G1 --> G2["G2 Mémoire"]
  G1 --> G3["G3 Barrières qualifiées"]
  G3a --> G3
  G2 --> G4["G4 Délégation / A2A"]
  G3 --> G4
  G4 --> G5["G5 Composition / E2"]
  G5 --> G7["G7 Campagnes / corrections / RETEX"]
  G6 --> G7
```

Le graphe porte sur la qualification, pas sur la préparation indépendante du code
ou des tests. PA-06 conserve PA-02 + PA-05 + MEM-P2…P4 : G2 ne devient pas une
garantie globale avant G3. PA-07 (outillage/baux) et PA-09 (preuves) accompagnent
chaque incrément. PA-10 historique conserve sa clôture E2 bornée ; G5 étend la
qualification à un nouveau périmètre sans réécrire cette décision.

Un intégrateur Codex ; aucune délégation ni exécution parallèle implicite.
Préparer G6 pendant les lots locaux n'autorise pas à contacter les tiers.

## Exigences et recette de sortie

| ID / lots | Précondition et déclencheur | Observable, refus et reprise |
|---|---|---|
| R0 / G0 | Statut demandé pendant opération détenant le verrou de cycle de vie | État concurrent explicite, sans bloquer sur le verrou exclusif des mutations ni inventer « prêt » ; lecture normale après libération |
| R1 / G1 | Témoin admis puis montant/bénéficiaire/dépôt/candidat/identité/politique modifié après accord | Accord incompatible refusé au dernier usage ; registre/référence avant-après inchangé ; nouvel accord exact requis |
| R2 / G1/G5 | Révocation concurrente, crash ou sortie perdue après engagement | Ordre établi ; effet unique ou UNKNOWN ; réconciliation avant répétition ; consommation et historique conservés |
| R3 / G2 | Version retirée/remplacée ; alias ou résumé utilisé pour contourner | Nouveau contexte contrôlé refuse l'ancien usage ; contexte déjà reçu reste attribué ; erreur d'inventaire jamais transformée en catalogue vide |
| R4 / G3 | Accès par descendant/FD/env/connexion existante, secret ou destination interdite | Refus préventif mesuré à la ressource et octets reçus observés ; panne refuse ; politique légitime rétablit le témoin |
| R5 / G4 | Mandat enfant trop large, audience différente, message rejoué/révoqué | Refus avant capacité/effet ; cas légitime reste utilisable ; aucun droit dérivé du seul contenu |
| R6 / G5 | Service indisponible, budget épuisé, preuve périmée ou superviseur repris | Réception attribuée, décision courante appliquée et effet constaté ; aucun retry inchangé ni reset de consommation |
| R7 / G1/G5 | Mode human et revue IA favorable | Suite bloquée jusqu'à décision humaine authentifiée liée au contenu exact ; session/droit révoqué ou contenu modifié refusé |
| R8 / G6/G7 | Candidat/protocole gelés, Swarm/B2/seconde implémentation | Mesures complètes et inconnus conservés ; mêmes protections métier B2 ; un invariant requis en échec reste bloquant |

Avant toute faute, témoin valide sous mêmes préconditions. Vérifier échec sans
mutation, persistance/réouverture, annulation, fin des processus et effet réel.
Toute nouvelle politique : source persistée → validation commune → Admin/API/CLI
→ consommateur effectif, sans plafond caché. Toute modification inclut fonctions
documentées, erreurs propagées, docs FR/EN et schémas concernés dans le même lot.

Recettes permanentes en dossier neuf, tests ciblés effectivement exécutés, sources
et sorties originales conservées puis verify. Sans borne justifiée, watchdog global
désactivé et progression suivie ; aucune limite arbitraire pour masquer un blocage.
Profil produit distinct des fixtures, image et entrées identifiées. Après
développement autorisé, livraison locale gérée par défaut, vérification image
exécutée/Version/notes FR/EN/assets protégés et parcours public. L'installation
système du démon garde son intervention privilégiée opérateur ; pas de redémarrage
de services d'autrui. Une auto-revue ou un build vert ne vaut pas verdict indépendant.

## Interventions à organiser

- H1 — **Opérateur système, G1/G3** : relever les prérequis, préparer procédure
  réversible, installer les helpers et services effectivement nécessaires.
  Installer uidmap seul ne prouve pas les UID externes distinctes, mappings,
  groupes/capacités et droits des services. Aucun secret dans le chat.
- H2 — **Commanditaire/approbateur, G1/G5** : enrôlement et approbation réels,
  mandats et séparation des rôles selon politique. Les fixtures ne les remplacent pas.
- H3 — **Commanditaire puis tiers, G6** : désignation et acceptation de mission
  de l'évaluateur et de l'équipe de seconde implémentation. Codex prépare le dossier
  mais ne tient pas leur rôle et ne fabrique pas leur réception.
- H4 — **Responsable métier/participants, G6/G7** : critères de risque, réussite,
  faux refus et essais humains ; l'évaluateur fixe volumes, ordre et arrêts avant
  campagne. Aucun coût, budget ou seuil de performance inventé.

Ces dépendances ne bloquent pas les travaux locaux indépendants. La première
tranche proposée est G0, puis un raccord G1 sur règlement fictif, réemploi du contrat
commun côté Git et requalification des dépendances. G6 commence en préparation dès
G0 ; les contrats/tests G2/G3 peuvent être préparés pendant l'attente humaine.

## Découverte D01 et continuité

Origine : préparation du plan. ./swarm.sh status sans sortie ; notre lecteur
PID3641689 observé en locks_lock_inode_wait, FD3 vers lifecycle.lock.
SIGINT de ce seul lecteur : traceback sur fcntl.flock(lock, LOCK_EX), ligne155
de tools/swarm_local.py. Aucun verrou supprimé, service arrêté ou détenteur
interrompu. Le détenteur n'a pas été attribué ; ce n'est pas une preuve de
deadlock moteur. Le lecteur n'existe plus à la reprise.

Valeur : statut disponible pendant build/restart. Propriétaire : mainteneur du
lanceur ; G0/R0 ; état découvert, correction proposée, non réalisée.
Comparer lecture cohérente sans verrou exclusif et acquisition non bloquante avec
statut « opération concurrente ». Préserver l'exclusion des mutations ; pas de
timeout arbitraire. Clôture : verrou détenu, diagnostic explicite puis lecture
normale après libération, sans agir sur le détenteur.

Conservation des preuves : examiner le FAIL run-13 rapporté supprimé dans l'audit,
retrouver un original si disponible, sinon déclarer la perte sans reconstituer de
fausse preuve. Les PASS actuels ne deviennent pas faux par cette seule réserve.

L'enregistrement initial de ce plan a refusé un patch mal formé avant création.
À la reprise, absence du fichier vérifiée ; nouvelle génération du patch avec
préfixage systématique de chaque ligne. Ce défaut de méthode n'est pas un défaut
produit et n'invalide pas l'audit conservé.

## Découverte D02 — garde de source mémoire déjà porté, appelants obsolètes

Origine : tranche G2 « mémoire versionnée — moteur » (5 novembre 2026). En
câblant le cycle `memory.*` dans le moteur Rust, constat que le garde de source
versionnée attend par `project_context.rs:294` est déjà porté :
`rust/crates/swarm-engine/src/project_memory_source.rs` (commit `2496c031be`,
vague Biba) refuse `project-memory-source-inventory` /
`project-source-memory-managed`. Preuve : `cargo test -p swarm-engine --test
project_context` — `review_document_cap_and_managed_memory_alias_refusals_are_explicit`
échoue en attendant l'ancien code `versioned-memory-source-guard-not-ported`
(`tests/project_context.rs:134,153`) ; le mapping HTTP 501 de
`rust/crates/swarm-web/src/service.rs:401-402` filtre ce même code devenu
inobservable. Valeur : la bascule 501 devient une simplification (le parcours
est porté) et deux obsolences appellent une mise à jour. Propriétaire : vague
BF-16b active sur `service.rs` au moment du constat (diff concurrent visible) ;
test concerné modifiable après accord de la vague. **Corrigé le 6 novembre 2026**
(ordre utilisateur « corrigés ») : attentes du test mises à jour vers les codes
réels du garde porté (`project-memory-source-inventory`, refus causé explicite
sur fixture nue — commentaire prouvant le portage, pas un affaiblissement des
assertions), filtre 501 retiré au profit de `caller_invalid` (400 pour les
entrées d'appelant invalides : `project-source-memory-managed` inclus ;
les défaillances d'inventaire racine/SQL restent des erreurs serveur par défaut).
Preuves : `cargo test -p swarm-engine --test project_context` 4/4 vert ;
`cargo test -p swarm-web --test service` 10/10 ; `--lib -- project` 31/31 ;
`--test root_project_context` 3/3 ; `cargo check -p swarm-web` 0 erreur.
Clé UX morte remplacée dans `web/project-profile.js` (motifs français réels).
Doc FR/EN REFERENCE.md mise à jour (garde porté, frontière 501 retirée).

Tranche G2 moteur (même date) : cycle complet porté et vérifié — module
`project_memory.rs` (10 tests), câblage `work_engine.rs` + aperçu
`root_store.rs`, intégration `tests/project_memory_engine.rs` (6 tests),
recette permanente `tools/verification/recipes/versioned-memory-engine-rust.json`
(PASS, preuve `.swarm-fixtures/versioned-memory-engine-evidence/run-20261105-1`),
doc FR/EN `docs/actions-confiance-rust.md`. Restent fermés : surface HTTP/CLI
dédiée, résumé extractif M20, droits lire/résumer/transmettre/compacter.

Tranche G2 filiation (même date) : `agentMemoryReferences`
(`project_memory_dialogue.go`) porté — `memoryUses` parcourt la chaîne
`dialogue_restore` (reçus valides, source recontrôlée contre son empreinte,
racine et tours strictement décroissants), alimente l'inventaire des usages
actifs avec les sélections d'ancêtres, refuse les filiations brisées par codes
stables `project-memory-lineage-*` ; le garde cyclique est documenté comme
défense en profondeur sans cas constructible (empreinte couvrant le reçu
suivant à chaque hop, tours strictement décroissants), aligné sur le test Go
qui n'affirme que le refus. Preuve : test
`lineage_walk_reads_ancestors_and_refuses_broken_chains` (11 tests lib au
total), recette mise à jour 10→11 et
rejouée, preuve `.swarm-fixtures/versioned-memory-engine-evidence/run-20261105-3`.
Restent fermés : surface HTTP/CLI dédiée, résumé extractif M20, droits
lire/résumer/transmettre/compacter.

## Découverte D03 — fixtures de tests Rust sous /tmp (interdiction opérateur)

Origine : reprise tranche G2 filiation (6 novembre 2026). Après l'interdiction
opérateur du 31 octobre 2026 (aucune écriture projet sous /tmp, tmpfs effacé au
redémarrage), deux producteurs de tests Rust du dépôt créent encore leurs
fixtures sous `std::env::temp_dir()` : `project_context.rs:593`
(`swarm-profile-document-{label}-{pid}-{nanos}`, module
`supplied_document_tests`) et `root_store/mission_status_review.rs:369`
(`swarm-status-review-{pid}-{nanos}`, fixture diagnostique retenue volontairement
par `current_review_report_and_artifact_staleness_recover` en échec). Un
chacun laisse des répertoires résiduels sous /tmp après chaque exécution de la
suite lib. Valeur : conformité à l'interdiction et survie des fixtures de
diagnostic à un redémarrage (tmpfs) — la fixture retenue d'un échec est perdue
au reboot. Propriétaire : vague BF-16b (fichiers en modification concurrente au
moment du constat). **Corrigé le 6 novembre 2026** (ordre utilisateur
« corrigés », avec accord implicite des propriétaires vagues puisque l'utilisateur
a ordonné la correction) : les deux producteurs déplacés vers
`.swarm-fixtures/` avec l'idiome du dépôt (`CARGO_MANIFEST_DIR` + base 0700
récursive + répertoire de fixture 0700) ; canonisation de la base pour les
chemins de fixture (le test exerce `clean()` qui refuse les `..` lexicaux —
même piège qu'un TMPDIR non canonique, précédent : `runtime_temporary.rs:10`) ;
le `Fixture` de `tests/project_context.rs:5-7` déplacé aussi (troisième
producteur repéré à la reprise, même classe). Preuves : producteur 1 — test
`supplied_document_matches_reader_without_profile_writes` vert, répertoires
vérifiés sous `.swarm-fixtures/swarm-profile-document-*`, zéro sous /tmp ;
producteur 2 — fixture créée sous `.swarm-fixtures/swarm-status-review-*`
(le test reste rouge uniquement à cause de l'échec préexistant de la vague
BF-16b `gate-evidence-invalid` à `mission_status_review.rs:412`, non une
régression du déplacement — diff purement localisateur) ; producteur 3 —
`cargo test --test project_context` 4/4 vert après canonisation. `check_tmp_interdiction.py` :
0 violation (le garde ne scanne pas les sources Rust — l'exclusion Rust est
couverte par le filet TMPDIR de recipe.py et la règle source documentée ;
extension du garde au Rust non retenue ici : ~130 sites `env::temp_dir()`
passeraient en rouge sans traiter la cause, découverte consignée ci-dessous D04).
Reste sous /tmp : la fixture diagnostique de la vague BF-16b
`swarm-status-review-536028-1793974459868644663` est préservée telle quelle
(suppression refusée à raison par le classificateur : preuve de la vague, non
propriété de cette correction) — décision de suppression à son propriétaire.

## Découverte D04 — allocations de fixtures Rust via `env::temp_dir()` (systémique)

Découverte de reprise (6 novembre 2026) pendant la correction de D03 : environ
130 sites de tests Rust du dépôt allouent leurs fixtures via
`std::env::temp_dir()` (grep `env::temp_dir` sous `rust/crates`). Ce n'est pas
une violation des octets sous /tmp tant que l'exécution passe par
`tools/verification/recipe.py`, qui épingle TMPDIR vers la zone de fixtures —
le filet documenté du garde `check_tmp_interdiction.py` (qui scanne
volontairement `.py/.cjs/.js` seulement). Mais toute exécution cargo nue
(TMPDIR par défaut) écrit sous /tmp : la conformité à l'interdiction du
31 octobre repose alors sur la discipline de lancement, pas sur les octets.
Valeur : déplacer ces sites vers l'idiome `.swarm-fixtures/` rendrait la
conformité structurelle. Non corrigé ici : massive (≈130 sites), hors
périmètre de l'ordre « corrigés » (D02+D03 ciblés), et un garde Rust rouge
immédiat bloquerait toute la vague sans corriger la cause. Propriétaire :
ouvert — à rattacher à un lot de migration ou à un accord de vague ; condition
de reprise : décision d'étendre le garde `check_tmp_interdiction.py` aux
sources Rust (classe `env::temp_dir()` sans TMPDIR explicite) en même temps
que la migration des sites, producteur connu identifié : `tests/trust_biba.rs:7`
(`biba-*`).

### Leçons vérifiées de la reprise (6 novembre 2026, portée D03)

1. **TMPDIR à la main ≠ filet de recipe.py** : épingler TMPDIR vers un
   répertoire sous le dépôt (ex. `.swarm-fixtures/runner-tmp`) casse les tests
   de contrats ancêtres-privés (`open_private_directory`,
   swarm-enrollment-config/src/store.rs:105) — les ancêtres `ChatGPT`/`swarm`
   sont 775 inscriptibles au groupe. Le filet de `tools/verification/recipe.py`
   (repli `~/.local/state/swarm-fixtures/runner-tmp`, chaîne 700 vérifiée par
   `namei`) est le mode contractuel : 11/11 suites swarm-enrollment-config au
   vert sous ce repli. Ne pas réinventer ce TMPDIR à la main.
2. **Tests mutateurs BLP sur fixture partagée** : les tests `paea_git_call`
   mutent la révision BLP d'une racine commune (chaque appel `owned()` →
   config-apply → blp-policy-apply) ; une exécution groupée perd des courses de
   révision (`blp-revision-conflict` en 2625/3677). Le mode contractuel
   délivré (recettes) est un test par pas avec `--exact` — 3/3 au vert
   individuellement, y compris avec la fixture restaurée depuis l'archive USB.
3. **Concurrence cargo, deux flocons observés** (non reproductibles
   individuellement, fichiers jamais touchés par D02/D03) :
   `agent_supervision.rs:662` (assertion heartbeat de passage — re-flou sous
   charge) et `managed_repository_candidate_store.rs:1257`
   (`ExecutableFileBusy` « Text file busy » sur la fixture de garde-fou).
   Ré-exécution complète sous le même TMPDIR : 736/4 (ensemble exact
   préexistant BF-16b), soit la ligne de base d'avant corrections — aucun
   élément de D02/D03 ne rougit. Le diagnostic a coûté 2 minutes ; la re-run
   complète valant la dissection, qualifier par re-run avant d'ouvrir un sujet.
4. **Rupture d'archive USB → recettes déclarant des fixtures déplacées** :
   l'archivage USB du 4 novembre a déplacé
   `source-fixture-writer78b-regen/result.json` (input déclaré ligne 19 de
   `tools/verification/recipes/paea-git-call-web-rust.json`), ce qui aurait
   fait refuser la recette (exit 2, input vide). Le 7 novembre, les deux
   chemins déclarés sont restaurés depuis l'archive Lexar (extraction
   `tar -xOf` d'un seul membre, JSON validé, dir 0700), et la recette
   contractuelle complète repasse : PASS 3/3 pas, `verify` integrity
   VERIFIED. Preuve sur clé : `preuves/swarm/20261107-paea-git-call-web-rust/`
   (index local `.swarm-fixtures/INDEX-USB-PREUVES.md`). Leçon : après tout
   déplacement de preuves vers la clé, relancer les recettes touchées — un
   input déclaré manquant est un refus au lancement, pas un échec de test ;
   ne pas déduire la santé d'une recette d'une fixture restaurée à moitié.
5. **Portée réelle de la restauration après archivage USB** : un balayage des
   391 recettes a trouvé 163 chemins déclarés absents (dont beaucoup
   préexistants hors du déplacement USB : globs `tests/rust_*.py`, preuves
   migration-rust 20261022) ; 15 déclarations relevaient du déplacement USB,
   réparties sur 8 recettes et 3 fixtures supplémentaires
   (`consent-fixture-regen-20261031c`, `chain-fixture-regen-20261031b`,
   `source-fixture-current-pub-regen-20261031`). Contrat de restauration
   établi par échec : `result.json` seul ne suffit pas — les tests
   `born()` exigent aussi le témoin `native/inspection/workers/planning/
   setup/owned-root-*.json` (extraction de l'arborescence `native/`
   complète, 572-932 fichiers par fixture). Les 8 recettes réparées ont
   toutes repassé (promote 2/2, payment-agreement 4/4, effect-reconcile 5/5,
   consent-ui 10/10 avec navigateur réel, transfer-call-cli 8/8,
   transfer-reconcile 4/4, source-acquisition 5/5, source-context 6/6,
   current-managed-git-publication 4/4 — celle-ci attend son fixture par env
   explicite au lancement, README:2341, non déclaré dans le JSON).
   Par ailleurs la recette `paea-delivered-chain-cli-rust` épingle le SHA du
   binaire livré : après l'installation v341 (8704ab44…), re-pin des 3
   occurrences (env + témoins required_stdout), motif du commit 8a946aafd.
   Enfin la rédaction D02 avait introduit deux liens intra-doc vers
   `MemoryError::Refused`/`Row` dans une docstring publique
   (`decode_project_memory_request`) qui échouaient au pas rustdoc
   `-D warnings` de 18 recettes — liens convertis en texte (project_memory.rs),
   rustdoc à nouveau vert, prouvé par les pas rustdoc des recettes ci-dessus.

## Revue et état de clôture

Revue de cohérence par le même auteur : C01…C11 rattachés, PA existants conservés,
graphe acyclique, prérequis de composition et propriétaires H1…H4 explicites.
Plan prêt à sélectionner pour travail local ; garantie protégée conditionnée à
H1/H2/G3 ; E3 à H3/H4/G5. Aucune validation moteur ou revue indépendante prétendue.
État de tous les lots : proposés, non exécutés par cette demande.
Cette modification est documentaire ; aucune fonction ou politique exécutable
modifiée, aucun build/redémarrage nécessaire pour livrer un plan.

## Suivi du 7 novembre — G1 séparation avancée exécutée localement

Le lot G1 (séparation avancée au règlement fictif, suivi `P1-G1-20261103.md`)
a été exécuté localement par la session Claude de relais : conflits
admin/requester/approver↔effect vérifiés au point d'engagement via le moniteur
Trust existant, refus typés sans insertion, rétablissement prouvé. Critère
« matrice avec require_separation=true » couvert ; « matrice seule » reste
explicitement refusée. Recette `paea-payment-agreement` étendue, PASS 5/5
VERIFIED. G1 global reste partiel : enrôlement humain, services d'effet
protégés, parcours opérateur et revue indépendante restent ouverts.

Livraison locale (même jour) : `./swarm.sh restart` a requalifié et installé
le candidat dev.342 (SHA `9f268e0e…`, profil normal vérifié
`VERIFIED_DEFAULT_NATIVE_PROFILE`) portant le changement moteur G1, avec
identité vérifiée en chaîne (build record = disque = `/proc/<pid>/exe`,
digest effectif = octets modifiés de `policy.rs`). Un avertissement
compilation dans le code nouveau du lot, corrigé pendant un run de recette
(éditer une source déclarée pendant un run : run invalidé, conservé comme
témoin — défaut de méthode reconnu, leçon : geler les sources avant run),
a exigé une re-livraison. La re-livraison dev.343 a d'abord été **refusée
par le lanceur** (`profile-effective-input-changed`) : la session
concurrente swarm-3d éditait ses fichiers fixture_area pendant le build ;
serveur resté dev.342, échec conservé. Reprise à 13:55 sur demande
opérateur : arbre moteur stable, restart réussi — **dev.343 installée et
vérifiée en chaîne** (SHA `a0721124…` = build record `VERIFIED_DEFAULT_
NATIVE_PROFILE` = disque = `/proc/<pid>/exe`), embarquant les deux lots
(G1 + fixture_area). Re-épinglage des recettes livrées depuis le hash
réel (chain 3 occ., settlement 8 occ.) puis rejeu sur le livré réel :
**settlement-run-19 PASS 7/7** (23 spawns témoins) et **chain-run-11 PASS
4/4** (10 spawns témoins, refus de mauvais SHA exercé), les deux `verify`
VERIFIED. `paea-payment-agreement` inchangé (archive historique fixe).
Le lot G1 reste non committé ; commit/push attendent l'instruction
explicite de l'opérateur.
