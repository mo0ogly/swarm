# Bell-LaPadula — APEX, 31 octobre 2026

Propriétaire : Codex racine. Autorisation : demande « Implement the plan ».
Travail direct, sans délégation ni consommation fournisseur supplémentaire.
Préserver les travaux PAEA, préparation et migration présents à la reprise.

[English](../../en/plans/actions-confiance/BLP-APEX.md).

## Rattachement au programme PAEA

Ce plan décompose techniquement la confidentialité du [plan PAEA](PAEA-APEX.md).
La section « Rattachement Bell-LaPadula » de ce dernier relie BLP-01–07 aux lots
PA-00–13, invariants I01–I17 et tests T01–T22 existants, sans les renuméroter ni
créer de second registre d'assurance. PAEA porte les critères de qualification
et leur état ; ce suivi conserve les actions techniques, preuves et reprises BLP.
La reprise GLM/SKYNET doit recevoir les deux plans et restituer les références
PAEA correspondantes. Modifier ces fichiers ne met pas à jour une session déjà
démarrée ni les instructions embarquées du moteur.

Ce rattachement documentaire ne reprend pas le développement arrêté, ne démarre
pas AC-02 complet et n'établit aucun nouvel invariant vérifié ou accepté. Les
limites du WIP ci-dessous restent historiques ; ne pas les traiter comme un
diagnostic courant de sources modifiées par une autre session sans vérification.

Revue de cet incrément : documentation seulement, sans modification des politiques
exécutées, schémas, fonctions ou chemins d'erreur. Les scénarios ajoutés sont des
critères à vérifier, pas des résultats. Les notes de version et l'installation
ne changent pas : aucun comportement produit n'est livré par ce rattachement.

## Reprise des acquis PAEA — obligation explicite

Les actions PAEA déjà réalisées, y compris livrées ou déclarées terminées,
doivent être examinées à travers les sept lots BLP-01–07. La section « Reprise
obligatoire des actions déjà réalisées » du [plan PAEA](PAEA-APEX.md#reprise-obligatoire-des-actions-déjà-réalisées)
définit le relevé requis par action : applicabilité justifiée, producteurs et
consommateurs, candidat/profil, preuves réutilisables, écarts, propriétaire et
condition de reprise/clôture. Ce relevé reste dans le suivi existant.

Conserver les preuves et résultats historiques ; ne pas leur attribuer une
garantie de confidentialité non établie. Réouvrir les critères concernés et
reprendre les seules vérifications affectées, en réutilisant les preuves dont
les entrées et la portée restent valides. BLP-07 ne clôt pas sans réexamen des
acquis et traitement des écarts applicables.

État actualisé : [réexamen d'applicabilité réalisé](paea/BLP-REASSESSMENT-20261031.md),
115 actions et 805 décisions dans le [relevé](paea/BLP-REASSESSMENT-20261031-ANNEX.md).
Les huit écarts BLP-RET-01–08 ont un propriétaire et une condition de reprise ;
ils rouvrent ou maintiennent ouverts les critères BLP applicables, sans effacer
les acquis historiques. Les 150 intrants initiaux de revue sont archivés sous
empreintes ; les preuves de livraison/UI ajoutées concurremment sont relevées
séparément dans le rapport. Cette revue ne réinstalle rien et ne réécrit pas les
preuves de livraison du socle de l’autre session. BLP-07 transversal ne peut clôturer avant les
raccords, contrôles et écarts applicables, notamment BLP-01/04/05/06, même si le
socle d'administration est livré et son RETEX rédigé. Réception de la revue par
la session de reprise, corrections et vérifications de bout en bout restent ouvertes.

## BLP-01 — inventaire statique des flux, 31 octobre

[Contrat et menaces FR/EN](../../architecture/BLP-FLOWS.md),
[24 flux sous empreintes](../../architecture/blp-flow-inventory.json).
43 ancrages dans 41 fichiers lient les vrais assembleurs, transports CLI/HTTP,
sonde manuelle, validations, télémétrie, historiques, effets Git/règlement et
exports. Chaque flux conserve ses acquis, son propriétaire, ses lots manquants
et son observable ; les 24 contrôles BLP effectifs restent ouverts.

Recette permanente `blp-flow-inventory.json` : deux étapes PASS ; cinq tests
du vérificateur exécutés (témoin, modification/refus/reprise, ancrage, graphe et
échappement de chemin). Le résultat prouve la fraîcheur des déclarations, pas
la prévention d'une divulgation ou l'exhaustivité des canaux dynamiques.
Preuves neuves : `verification-blp/20261031-flow-inventory-01/`.

Suite retenue : BF-05 → BF-07 → BF-16, attribution du contexte complet de
préparation HTTP, émission puis héritage du résultat, avec récepteur local de
fixture. Aucune classification publique implicite, étiquette client autorisante
ou chaîne libre ne doit remplacer le producteur protégé. Le moniteur existant,
les contrôles Biba et les gardes de budget/autorité restent cumulatifs.
BLP-01 est partiel tant que les accès SDK, caches, outils, proxys éventuels et
descendants ne sont pas inventoriés/qualifiés dynamiquement ; BLP-RET-01/02/03
restent ouverts. Cet incrément de suivi et d'outillage n'a changé aucun
comportement produit ; aucune installation ou note de version n'est requise.

## Contrat adopté et ordre

Confidentialité transversale Projets/Processus, complémentaire de Biba ; migration
explicite de l'existant, déclassification humaine exacte, sous-ensemble borné des
barrières fichiers/réseau AC-02. Le reste d'AC-02 reste différé. Aucun rôle client,
label désérialisé ou champ de provenance ne constitue une autorité.

| Lot | Livrable et critère observable | Dépendance | État |
|---|---|---|---|
| BLP-01 | Inventaire sources, consommateurs, sorties et frontières, modèle de menace | — | EN COURS |
| BLP-02 | Moniteur de dominance/combinaison, domaines fixes, erreurs sûres, indépendance Biba | 01 | VÉRIFIÉ (contrats purs ; 10/10 tests du 31/10/2026, voir « Reprise ») |
| BLP-03 | Configuration persistée révisionnée, migration explicite, Admin/API/CLI | 02 | VÉRIFIÉ partiellement (historique, opérations, tests 5/5 du 31/10/2026 ; migration des objets existants non faite) |
| BLP-04 | Attribution par producteurs, contexte/mémoire/sorties et replays | 03 | À FAIRE |
| BLP-05 | Médiation réelle fichiers/réseau/fournisseurs/descendants, refus sans repli | 04 | À FAIRE |
| BLP-06 | Identité humaine établie, validation des octets/destinataires, nouvelle copie | 03–05 | À FAIRE |
| BLP-07 | Recettes, revue du candidat, livraison locale, ressources servies et RETEX | 01–06 | EN COURS (recette permanente PASS ; installé v0.1.1-dev.282 ; HTTP+UI qualifiés ; RETEX restant) |

Chaque lot inclut documentation FR/EN, contrats des fonctions et revue explicite
politiques/fonctions/erreurs liée aux octets. Aucun critère critique non vérifié
ne devient conforme. La revue dans ce contexte n'est pas indépendante.

## Analyse et décisions

Deux voies : contrôle des seuls prompts/outils déclarés, ou moniteur commun et
médiation des accès réels. Choix : seconde voie ; la première permettrait les
lectures et sorties directes par le fournisseur et ses descendants.

Classification = niveau ordonné + compartiments. Dominance = niveau supérieur
ou égal et sur-ensemble des compartiments. Combinaison = maximum des niveaux
et union des compartiments. Aucun lien implicite avec les niveaux Biba.
Un domaine d'exécution est fixe ; aucune baisse après exposition au secret.
Une source non classifiée refuse les nouveaux usages protégés. Un résumé,
une sortie vide ou un filtrage LLM n'est pas une déclassification.

Catalogue initial proposé : Public, Interne, Confidentiel, Restreint ; son
édition est une politique persistée, jamais une constante d'autorisation.
Les destinataires effectifs sont liés au service/compte/endpoint, pas au seul
nom du modèle. Toute décision doit être revalidée à l'engagement.

## Reprise observée

- Branche `codex/clear-product-method-names`, changements tiers conservés.
- `swarm.sh status` : aucun serveur géré actif ; aucune version installée
  déduite d'un ancien rapport.
- Disque : environ 14 Gio disponibles sur le volume du dépôt ; aucun nettoyage
  ou copie complète autorisé implicitement. Préserver les originaux, écrire
  atomiquement ; aucune donnée de projet dans `/tmp`.
- Le contrat `swarm-protection-contract::SourceRef` représente des labels,
  sans établir provenance ni contrôle effectif.
- Le moniteur Biba pur ne gère pas la confidentialité. La frontière Linux
  autorise des lectures hors racine privée ; elle ne confine pas Internet.

## Inventaire initial des frontières

| Frontière | Producteur/consommateur inspecté | Preuve ou travail restant |
|---|---|---|
| Étiquettes et propositions | `swarm-protection-contract` | Représentation non autorisante ; adaptateur typé à établir |
| Autorité/configuration | `root_store/trust_store`, `trust/authentication` | Réservation Trust, racine ouverte, identité courante ; habilitations BLP absentes |
| Fournisseurs CLI | `native_provider_exec` et `native_provider_biba` | stdin scellé, environnement sélectionné, frontière Biba ; médiation BLP absente |
| API, SSE, préparation | `root_store/preparation_call`, `preparation_call_cli` | Inventorier les transmissions effectives et reprises avant raccord |
| Mémoire et fichiers | `memory_files`, `project_memory_source`, sources de préparation | Attribution, versions, imports et lecteurs à raccorder |
| HTTP et opérateur | `service/trust`, CLI `trust` | Session/CSRF distincts de l'identité métier ; pas d'habilitation BLP actuelle |
| Déclassification humaine | Enrôlement/approbations PAEA existants | Un rôle I3 seul ne prouve pas une personne ; raccord exact à établir |

## Preuves et clôture

Recette permanente sous `tools/verification/recipes/`, preuves neuves dans
`docs/plans/actions-confiance/verification-blp/`. Témoins valides avant chaque
injection, refus sans divulgation observé côté récepteur, reprise et persistance.
Conserver empreintes, argv, sortie originale, profil/features et image exercée.
Séparer tests de contrat, consommateurs publics, confinement, installation et
acceptation indépendante. Les lots ouverts ci-dessus interdisent une clôture globale.

## Arrêt demandé et sauvegarde WIP — 31 octobre

L'utilisateur a demandé l'arrêt du développement, un prompt de reprise GLM/SKYNET,
puis un commit. Ce commit sauvegarde un **travail incomplet**, sans qualification,
installation, activation transversale ni acceptation indépendante.

- BLP-02 : crate `swarm-confidentiality` écrite ; dix tests ont passé **avant**
  l'ajout ultérieur de la liaison de révision du domaine. Aucun PASS sur les octets
  finaux n'est établi.
- BLP-03 : historique persisté, opérations `blp-show`, `blp-policy-preview`,
  `blp-policy-apply`, `blp-check`, tests d'intégration et section Admin écrits,
  non qualifiés. Le diagnostic est explicitement non autorisant et l'interface
  annonce que la protection transversale n'est pas active.
- Une compilation moteur a échoué (E0308, String au lieu de &str pour
  `CompactJson::new`). Correction écrite, non recompilée.
- La revue du diff avant commit relève un autre défaut ouvert : les noms
  d'opérations BLP ont été insérés comme une expression `|` dans la branche
  GET/HEAD de `service/trust.rs`, au lieu de la liste `matches!` des opérations.
  Le raccord HTTP n'est donc pas compilable et les opérations POST BLP restent
  absentes de la liste autorisée. Corriger puis vérifier au prochain incrément.
- BLP-01 reste partiel ; BLP-04 à BLP-07 restent ouverts. Migration des objets,
  attribution réelle, médiation fichiers/réseau, déclassification humaine,
  documentation technique FR/EN, traductions, aide CLI, évolutions, recette et
  revues restent à réaliser/vérifier. Aucune conformité globale n'est revendiquée.
- Propriétaire de reprise : session outillée GLM/SKYNET choisie par l'utilisateur.
  Relire ce suivi, les sources et le prompt transmis ; préserver les changements
  tiers. Aucune délégation n'a été lancée par Codex.

La commande de commit est une sauvegarde Git demandée ; elle ne reprend pas le
développement arrêté et ne déclenche ni build, ni installation, ni publication.

## Reprise de vérification — 31 octobre (session Claude)

Relance du travail depuis le prompt transmis ; commit WIP `0aa7335e1` relu,
changements tiers préservés (graphe/migration, PAEA, recettes : 11 fichiers
modifiés restés intacts). Disque au relevé : ~11–12 Gio libres.

Correctifs écrits puis vérifiés :
- Défaut de routage HTTP corrigé : branche GET/HEAD de `service/trust.rs`
  rétablie à `"show"` seul ; opérations BLP ajoutées à la liste `matches!` des
  POST autorisés. Compilation swarm-web : Finished.
- Défaut E0308 (`CompactJson::new`) : la correction `from_utf8` héritée est
  vérifiée par compilation (`--test trust_biba --no-run` puis suites).
- Test d'intégration `tests/confidentiality.rs` corrigé : le fixture fournissait
  2 identités alors que `prepare_identities` exige au moins 3 (invariant métier
  existant, échec initial `"format"` conservé) ; troisième identité `approver`
  ajoutée, motif `trust_biba.rs`.

Preuves neuves (`docs/plans/actions-confiance/verification-blp/
20261031-confidentiality-tests/` et `20261031-recipe-run/`) :
- `swarm-confidentiality --lib` : **10/10 PASS** sur les octets actuels (liaison
  de révision incluse) — lève la limite du PASS hérité.
- `swarm-engine --test confidentiality` : **5/5 PASS** (TMPDIR aire projet et
  repli privé opérateur).
- Coexistence Biba : `trust_biba` **14/14**, `trust_governance` **11/11**,
  `trust_enrollment` **3/3** (repli privé ; échec sous aire projet directe
  causé par le contrat de répertoire privé du fixture tiers, non modifié).
- Recette permanente `tools/verification/recipes/blp-confidentiality-rust.json`
  créée : **PASS 4 étapes**, preuve revérifiée (`verify` → VERIFIED).
- Traductions EN : 13 clés BLP ajoutées à `locales/en.json` (aucune clé
  existante modifiée), artefact `web/i18n-en.js` régénéré (`--check` pass).
- Aide CLI FR/EN complétée (opérations blp-*), compilée.
- Documentation FR/EN : section opérateur dans `actions-confiance-rust.md` et
  miroir EN ; section architecture avec graphe Mermaid dans
  `architecture/BIBA-RUST.md` et `en/BIBA-RUST.md` (arêtes pleines = vérifié,
  pointillées = cible étiquetée). Notes d'évolution `version_history.json`.

Limites inchangées et ouvertes :
- `runtime_enforced:false` et `runtime_authorized:false` restent la vérité du
  runtime ; aucun contenu réel classé, aucune exécution protégée.
- Le parcours HTTP réel (serveur installé, POST `action=blp-*`, CSRF, UI)
  n'est pas encore qualifié ; l'interface n'a pas été inspectée dans un
  navigateur sur l'image servie.
- Installation locale par `swarm.sh` non effectuée ; aucune acceptation
  indépendante. BLP-01 partiel ; BLP-04 à BLP-06 ouverts ; BLP-07 en cours.

## Livraison locale et qualification réelle — 31 octobre (même session)

Installation : disque remonté à ~72 Gio libres avant build (relevé `df -h`).
`./swarm.sh build --no-open` : candidat **v0.1.1-dev.282** vérifié par le
collecteur (`VERIFIED_NATIVE_CANDIDATE`, profil `VERIFIED_DEFAULT_NATIVE_PROFILE`,
commit 164915f73, modified:true, sha256 ea072ccc…, preuves dans
`.cache/local-delivery/rust-7639180g/build/`). `./swarm.sh restart --no-open`
a installé le candidat sur le seul serveur géré : pid 2158179,
`/proc/<pid>/exe` = `bin/swarm`, sha256 de l'exécutable actif = sha du candidat,
`server.json` et `swarm.sh status` concordent.

Qualification HTTP réelle sur le serveur installé (curl, session propriétaire
persistée + dérivation CSRF d'adresse, aucune donnée projet dans `/tmp`) :
- POST `action=blp-show` sans jeton I3 → **403 `authorization_required`** :
  routage BLP, admission Host/session/Origin/CSRF et authentification trust
  atteignent la vérification métier (refus typé, pas 404/500).
- POST `action=blp-unknown` → **400 `trust-action-unknown`** (liste blanche).
- POST sans CSRF → 403 `web-access-refused` ; GET `trust-confidentiality.js`
  sans session → 403 (asset protégé, embarqué par `include_bytes`).
- État trust de la racine réelle : `trust-actions.json` absent (protocole
  non configuré) ; l'administration BLP positive exige le bootstrap trust,
  décision opérateur non prise dans cet incrément.

Qualification UI par navigateur réel (Chrome headless, script
`.swarm-fixtures/blp-ui-qualification.cjs`, preuves
`.swarm-fixtures/blp-ui-evidence/` : 3 captures + `checks.json`) — **9/9
vérifications, 0 erreur console/page/HTTP inattendue** :
- page `/trust-actions.html` servie 200, asset `trust-confidentiality.js`
  référencé ; dialog Admin ouvert ; `blp-load` affiche le refus typé FR
  « Identité ou autorité insuffisante… » avec un seul 403 classé.
- `blp-example` remplit une politique 4 niveaux sans habilitation ; save
  désactivé sans prévisualisation validée ; légende EN « Bell-LaPadula
  confidentiality » après navigation `?lang=en` ; refus typé EN affiché ;
  thème sombre→etat vérifié par attribut ; aucun débordement horizontal à
  390 px.
- Deux défauts de qualification corrigés dans le script (inertie des
  éléments hors dialog modal : ouvrir `<details>` et fermer le dialog avant
  les gestes header) ; aucune modification produit nécessaire.

Restant pour BLP-07 : RETEX ; aucune acceptation indépendante. BLP-01
partiel ; BLP-04 à BLP-06 ouverts. `runtime_enforced:false` reste la
vérité du runtime.

RETEX écrit : [paea/RETEX-BLP-20261031.md](paea/RETEX-BLP-20261031.md)
(miroir EN `.en.md`). BLP-07 reste ouvert pour l'acceptation indépendante ;
les lots BLP-01 (partiel) et BLP-04–06 restent ouverts.

Collision concurrente constatée en clôture : une session tierce a reconstruit
et installé **v0.1.1-dev.283** (même commit, sha e8961b39…) pendant la
qualification ; son entrée `version_history.json` a été réinsérée par-dessus
l'état tiers et l'entrée BLP réécrite atomiquement (les deux coexistent).
Le serveur 283 exécute bien le routage BLP corrigé (sonde `blp-show` →
403 `authorization_required` identique) ; sa note d'évolution ne contient
pas l'entrée BLP (construction antérieure à la réécriture) — elle sera
incluse au prochain build.
