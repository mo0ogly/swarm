# Bell-LaPadula — APEX, 31 octobre 2026

État opérationnel courant : [Bell-LaPadula dans Swarm](../../architecture/BELL-LAPADULA.md). Les sections datées ci-dessous conservent l’historique et les limites des preuves.

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
| BLP-04 | Attribution par producteurs, contexte/mémoire/sorties et replays | 03 | PARTIEL — provenance privée v3, contrôle d’objet et diagnostic authentifié des sources API/CLI ; identité producteur, principal d’envoi et héritage ouverts |
| BLP-05 | Médiation réelle fichiers/réseau/fournisseurs/descendants, refus sans repli | 04 | PARTIEL — quarantaine API/CLI livrée et vérifiée ; envoi classifié, fichiers/réseau/descendants ouverts |
| BLP-06 | Identité humaine établie, validation des octets/destinataires, nouvelle copie | 03–05 | À FAIRE |
| BLP-07 | Recettes, revue du candidat, livraison locale, ressources servies et RETEX | 01–06 | EN COURS (incréments ciblés vérifiés ; installé v0.1.1-dev.299 ; recette globale et RETEX transversal ouverts) |

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
vérifications, 0 erreur console/page/HTTP inattendue**. Preuves et harnais
retenus dans le dépôt :
[verification-blp/20261031-ui-qualification/](verification-blp/20261031-ui-qualification/).
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
pas l'entrée BLP (construction antérieure à la réécriture).

Résolution : le build suivant **v0.1.1-dev.284** (session tierce, commit
`3b5191e2c5` = mon commit de livraison) contient l'entrée BLP — preuve :
`effective-sources.tar.gz` du build (`.cache/local-delivery/rust-e4t0j_65/`),
une entrée BLP réelle. Sonde requalifiée sur l'image active 284 :
`blp-show` sans jeton I3 → 403 `authorization_required` identique.

## Reprise suivante — provenance du prompt

BLP-04 commence par la provenance des octets réellement assemblés, sans étiquette
ni autorisation d’envoi. Voir le [suivi et ses limites](BLP-PROVENANCE-20261031.md).
Le lot reste ouvert ; la migration, le principal authentifié, le contrôle de
destination et l’héritage restent à raccorder.

## Administration BLP positive sur racine fixture — 1er novembre (session Claude)

Expérience suivante du [RETEX](paea/RETEX-BLP-20261031.md), réalisée : le flux
enregistré complet par l'UI réelle, indépendamment de toute décision de
bootstrap de la racine de mission.

- Racine fixture dédiée dans l'aire projet (jamais `/tmp`, jamais la mission),
  initialisée avec le binaire installé (`swarm init`), bootstrap trust par la
  voie CLI opérateur (`trust config-apply`, 3 identités de fixture étiquetées),
  service réel `swarm web` sur 127.0.0.1:38499.
- Chrome réel sur `/trust-actions.html` : `blp-load` affiche « Aucune politique
  de confidentialité enregistrée » ; `blp-example` remplit le catalogue
  4 niveaux sans habilitation ; motif saisi ; `blp-preview` → « Prévisualisation
  validée ; aucun changement appliqué » ; `blp-save` → « Politique enregistrée
  et relue ; protection transversale non active » ; politique relue dans le
  formulaire.
- Relecture serveur : `capability.configured:true`, `runtime_enforced:false`,
  `policy_revision:1`, historique 1 entrée attribuée (actor `admin`, motif
  conservé). **7/7 vérifications, 0 erreur.**
- Preuves retenues :
  [verification-blp/20261101-blp-positive-ui/](verification-blp/20261101-blp-positive-ui/)
  (harnais, `checks.json`, capture, état trust de la racine fixture — données
  de fixture étiquetées uniquement).

Portée et limites : cette preuve établit le **parcours d'administration
enregistré** (persistance, attribution, relecture, garde save-sans-preview) sur
l'image servie ; elle n'établit ni la classification de contenu réel, ni une
exécution protégée (`runtime_enforced:false` inchangé), ni une acceptation
indépendante. La racine de mission reste sans état trust ; la décision de
bootstrap opérateur reste requise avant toute administration réelle.

Reprise du 1er novembre : preuves de provenance (11 tests, CLI 29, HTTP 34) et
installation historique 284 confirmées ; [suivi actualisé](BLP-PROVENANCE-20261031.md).
La version locale observée est désormais 290, conservée sans réinstallation ; ces
preuves ne qualifient pas 290. BLP-04 reste partiel, sans contrôle BLP transversal.

## Reprise du 1er novembre — renouvellement d’attribution et déclassification

Incident découvert en reprenant BLP-04 : `blp-object-attribute` refuse une seconde
attribution dans une même révision, mais permettait une classification plus basse
après renouvellement de politique. La recette
`20261101-attribution-renewal-01-counterexample` reproduit l’admission du même
digest/projet de confidentiel vers public en révision 2 ; le lecteur public reçoit
alors `Allowed`. Le problème concerne le producteur et tous les consommateurs de
l’historique, y compris les aperçus PAEA ; les preuves antérieures restent valables
pour leurs cas exercés, sans couvrir ce changement de révision.

Décision (Codex, périmètre autorisé) : conserver le plan et fermer ce contournement
avant de créer un autre producteur. Deux voies comparées : recalculer une étiquette
plus haute à la lecture seulement, ou refuser la baisse au producteur et dans
l’historique relu. Choix : seconde voie, pour ne pas servir une étiquette différente
de l’attribution enregistrée ni transformer une corruption en succès.

Critères : même digest/projet, niveau conservé ou augmenté et compartiments
conservés ou étendus entre toutes les révisions ; retrait/changement de compartiment
et niveau inférieur refusés sans mutation ; renouvellement égal/plus restrictif,
rejeu et relecture réelle autorisée vérifiés ; historique déjà abaissé refusé sans
divulgation ni réécriture. Une révision ou une habilitation admin ne vaut jamais
accord humain BLP-06. Le rapprochement reste celui du contrat existant digest +
chemin projet ; ce correctif n’établit ni identité physique du fichier source, ni
provenance complète du prompt, ni protection des copies sans filiation.

État : contre-exemple FAIL conservé ; correction et contrôles en cours dans
`tools/verification/recipes/blp-attribution-renewal.json`. Aucun droit, budget,
plafond ou réglage opérateur ajouté. BLP-04 reste partiel ; BLP-06 reste indisponible.

Journal de vérification du correctif :

- `20261101-attribution-renewal-01-counterexample` : FAIL attendu, baisse admise
  et lecteur public autorisé sur le producteur précédent.
- `...-02` : 11/12 passent ; le helper de test n’exposait pas la cause sûre
  derrière la famille `trust-proof-integrity`. Diagnostic corrigé sans assouplir
  le refus ; injections de fixture réutilisant la publication atomique gardée.
- `...-03` : 12/12 passent, mais recette FAIL pour deux sources d’autorité modifiées
  concurremment. Ce résultat ne devient pas une preuve de sources stables.
- `...-04` : compilation sans symboles terminée et sorties 12/12, mais processus
  de recette terminé par SIGTERM avant reçu final (origine non établie) : incomplet.
- `20261101-attribution-renewal-05` : **PASS, intégrité vérifiée**, 12 tests réellement
  exécutés et sources déclarées stables, après réutilisation du candidat de test
  désormais construit. Le changement de précondition est conservé, sans délai ajouté.
- `20261101-attribution-history-cli-01-baseline` : l’image livrée 290 qualifiée
  passe le témoin d’historique égal puis échoue au refus d’un historique abaissé.
  Fixture synthétique uniquement, aucun journal vivant modifié.

La lenteur a conduit à interrompre seulement les exécutions détenues (preuves
`20261101-owned-build-interruption.json`, `20261101-owned-web-profile-replan.json`).
Recettes : symboles de débogage à 0 comme la livraison, une compilation à la fois,
cas fonctionnels conservés et aucun watchdog global. Une annulation n’est pas un PASS.
L’attribution d’objet n’est pas encore exposée par la CLI ; son raccord opérateur
reste au lot BLP-03. Le nouveau parcours CLI vérifie donc le consommateur sur une
histoire de fixture explicitement injectée, sans créer un droit pour le test.

Recadrage du contrôle HTTP : l’extension envisagée de `paea_git_call` a été
retirée du candidat. Ce target exige `SWARM_PAEA_SOURCE_FIXTURE`, une mission
native de fixture préalablement acceptée et modifiable ; aucune fixture d’autrui
n’a été empruntée. Les essais `...-web-01` à `...-web-05` restent conservés :
interruption de compilation, diagnostic d’emprunt du nouveau test, précondition
native absente, puis contrat de sortie du lieur corrigé (JSON compact). Aucun de
ces lots n’est déclaré PASS. La recette compile désormais explicitement avant
liaison pour conserver le diagnostic Cargo complet.

`20261101-attribution-renewal-web-06` : **PASS, intégrité vérifiée**, racine neuve
sans mission, test `confidentiality_http::policy_renewal_and_poisoned_history_refuse_through_http` :
bootstrap/politique HTTP réels, refus au producteur moteur sans mutation,
renouvellement égal, refus HTTP d’un historique abaissé sans matière dans corps
ou en-têtes, reprise exacte. Image de test liée avant/après, octets stables.
Cela ne remplace pas la qualification des deux routes PAEA avec appel préparé :
elle reste à reprendre sur une fixture native fraîche détenue par son vérificateur.

### Résultat local du correctif — 1er novembre 2026

| Contrôle | Résultat et portée |
|---|---|
| Moteur `20261101-attribution-renewal-06` | PASS, 12 tests ; témoin égal autorisé avant injection de l’historique abaissé, cause exacte aux lecteurs, refus sans mutation et reprise |
| HTTP `20261101-attribution-renewal-web-06` | PASS, test autonome sur racine neuve ; image de test liée/stable, aucun prérequis de mission emprunté |
| CLI `20261101-attribution-history-cli-02` | PASS, 15 assertions sur l’image normale qualifiée 292 ; témoin, niveau/compartiment retirés refusés, données absentes de l’erreur, journal inchangé, reprise exacte |
| Instance `20261101-attribution-installed-01` | PASS ; processus chargé, notes compilées et sept sources d’exécution identiques au correctif testé ; 56 assertions navigateur FR/EN, thèmes, version/lien de commit, ressources protégées PAEA conformes à l’inventaire compilé, aucune écriture ni erreur navigateur |
| Inventaire `20261101-flow-inventory-01` | PASS : 24 flux, 45 ancrages, cinq tests de vérificateur ; fraîcheur déclarative, pas médiation complète |

La version **v0.1.1-dev.292**, image
`4164953c841c2b22476bd552a12496aac2b7f95e9b26de04d986c3238e2ed83c`,
a été installée par une livraison parallèle avant cette clôture. Son profil normal
et ses notes FR/EN contiennent déjà le correctif. Comparaison explicite des sept
fichiers moteur/moniteur/consommateurs, puis contrôle de l’image réellement chargée :
aucun redémarrage supplémentaire ni arrêt du service n’a été nécessaire. Ma
reconstruction redondante 294 a été interrompue, sans qualification ni installation
revendiquée. Les autres sources plus récentes restent hors de cette preuve.
Le `status` du lanceur attendait le verrou de cycle de vie tenu par mon build :
l’observation finale a utilisé l’API protégée et l’identité du processus vérifiées
avant/après, sans contourner une opération de mutation. Les profils navigateur
privés restent dans le sous-dossier `evidence/` ignoré par Git.

Revue liée aux octets : [source-review](verification-blp/20261101-attribution-source-review.json).
Aucune politique opérateur ajoutée : la dominance historique est un invariant BLP,
les schémas/identifiants sont des contrats, les valeurs de fixtures sont des entrées
de test ; le tampon de lecture d’image est une allocation technique, pas une borne.
Les contrats des fonctions modifiées/créées sont documentés ; les erreurs ont été
suivies jusqu’aux consommateurs moteur, HTTP et CLI, avec témoins et reprises.
Les anciens diagnostics de compilation et recettes incomplètes restent conservés.

RETEX local : ne pas confondre droit de changer une politique et droit de
déclassifier ; contrôler le producteur **et** la relecture de l’historique. Une
preuve ciblée doit créer ses propres préconditions ; une recette de mission native
retenue n’est pas une fixture HTTP générique. Compiler avec diagnostics conservés
avant de lier l’image ; vérifier le format de sortie réel du lieur. En contexte
concurrent, examiner d’abord l’image déjà qualifiée/installée et ses entrées
pertinentes pour éviter une reconstruction redondante ; ne pas attendre un `status`
exclusif derrière son propre build.

Limites maintenues : auto-revue seulement ; le contrôle PAEA à appel préparé n’a
pas été qualifié ici ; le digest/chemin n’établit pas la filiation physique ou des
copies ; source complète de préparation, principal authentifié, médiation et
héritage restent ouverts. Une ancienne histoire déjà abaissée refuse ; aucun outil
de réparation historique n’est livré. Le journal Trust de l’instance observée est
absent, aucune migration ni initialisation de ses données réelles n’a été faite.
**BLP-04 reste partiel et BLP-06 indisponible ; aucune confidentialité transversale
ni acceptation indépendante n’est déclarée.**

## Reprise après commit/push — producteur public, 1er novembre 2026

Le correctif de monotonie est conservé par `a76fb3ef5`, poussé sur
`origin/codex/clear-product-method-names`. Replan BLP-04 : l'opération moteur
`blp-object-attribute` existe et est protégée I3/admin, mais les listes fixes
HTTP et CLI la refusent avant son consommateur. Cela empêche l'opérateur de
produire l'attribution requise par les lecteurs via ces interfaces.

Deux voies examinées : attendre le registre complet des sources de préparation,
ou raccorder maintenant le contrôle d'objet existant, sans élargir son autorité.
Décision Codex : second chemin, incrément borné aux listes publiques, aide CLI,
témoins de refus/autorisation/rejeu et documentation. Aucun endpoint choisi par
le client, nouvelle politique, principal implicite ou classement automatique.
L'Admin existant configure la politique ; aucun formulaire d'attribution d'objet
n'est livré dans cet incrément. Le contrôle exact digest/chemin ne prouve pas
une source physique ; préparation, héritage et médiation restent ouverts.

Critères avant livraison : reproduire le refus HTTP courant avec une intention
valide ; faire passer la même intention sur HTTP et CLI authentifiés ; vérifier
absence d'autorité pour session seule/jeton faible, révisions périmées, rejeu
exact, conflit sans mutation, déclassement refusé et reprise. Réutiliser les
recettes permanentes avec résultats neufs, puis qualifier/installer le binaire
normal et contrôler version/ressources réellement servies. Propriétaire : Codex.
État : tests en cours d'écriture ; aucun nouveau comportement déclaré vérifié.

Incident de méthode : le contre-exemple HTTP ad hoc n'avait pas de TMPDIR
explicite et sa fixture temporaire a été créée puis détruite par `TempDir` dans
le répertoire système, contraire à la règle du projet. Ses sorties originales
restent conservées, sans qualification. Le contrôle suivant passe par la recette
permanente, qui impose le TMPDIR privé configuré ; ce contrôle HTTP a passé avec
intégrité vérifiée. Le contre-exemple CLI utilise sa propre racine durable privée.
La recette CLI accepte désormais `SWARM_BLP_PROOF_OUTPUT` pour conserver aussi
les requêtes/sorties de la traversée dans le dossier de preuve choisi, plutôt
que seulement un lien vers la zone de fixtures locale. Aucun secret réel.

### Résultat de l’incrément producteur public

- HTTP `20261101-public-producer-web-01` : PASS, intégrité vérifiée ; un test
  autonome exerce l’attribution HTTP, la session seule et le jeton faible refusés,
  la révision périmée, le déclassement, le rejeu, le conflit et la reprise.
- CLI `20261101-public-producer-cli-01` : PASS, intégrité vérifiée ; 25 assertions
  sur l’image de production qualifiée, historique valide produit par opérations
  publiques. Les 78 fichiers de requêtes/sorties et le manifeste final sont liés
  à l’empreinte imprimée dans stdout et revérifiés ; seule l’histoire invalide est
  injectée dans la fixture. Le contre-exemple CLI sur l’ancienne image refuse au
  parseur `command_failed`, avant l’authentification attendue.
- Livraison `20261101-public-producer-installed-01` : PASS ; redémarrage du seul
  service géré avec `--prepared-build`, version **v0.1.1-dev.295**, image
  `e1adea56e4cf53e0787d362f55e14bbd53d40c82a38b3815a081b0a33e203c73`.
  Profil normal conservé, neuf sources d’exécution conformes, 56 assertions
  navigateur FR/EN/thèmes/version/ressources protégées, aide CLI FR/EN contrôlée.
  Commit compilé `a76fb3ef5`, changements non commités annoncés par le dialogue.

Précision de preuve : le champ hérité `service_restarted:false` de la sonde
immuable décrit la sonde de lecture seule, pas l’incrément.
`delivery-scope.json` lie les sorties du redémarrage et le résultat original,
avec `service_restarted_by_increment:true` ; le modèle local de sonde a été
clarifié. Aucun résultat fonctionnel n’est réécrit. Le build unique a terminé
en 5 min 34 s malgré le compactage Git concurrent ; progression CPU/étapes et
pression mémoire sont conservées dans `20261101-public-producer-build-contention.json`.
Aucun processus tiers arrêté, borne arbitraire ni seconde compilation lancée.

Revue politiques/fonctions/erreurs :
[empreintes et limites](verification-blp/20261101-public-producer-source-review.json).
Listes fixes = vocabulaire de protocole ; aucun nouveau choix opérateur ni plafond.
Les commentaires CLI/HTTP décrivent l’autorité réelle et les erreurs restent
celles du producteur, avec fermeture/publication/sortie conservées. La preuve HTTP
reste réutilisée pour ses sources web/moteur inchangées ; une clarification de
l’aide CLI anglaise, non consommée par ce target, a précédé le build normal et
a été vérifiée sur le binaire livré. Les seules modifications suivantes sont
preuves/documentation, sans besoin de reconstruire un comportement inchangé.

RETEX local : vérifier le raccord public d’un producteur avant d’en exiger les
preuves des lecteurs ; ne pas confondre intention Admin et provenance physique.
Retenir les canaux détaillés, pas seulement leur chemin local, et relire les
champs de portée en dérivant une sonde. Utiliser le lanceur de recettes pour le
TMPDIR configuré dès le contre-exemple ; la première exécution ad hoc reste une
erreur de méthode conservée. Auto-revue uniquement ; aucune acceptation indépendante.

Reprise suivante conservée : BF-05 → BF-07 → BF-16, attribution complète des
sources de préparation puis liaison du principal/domaine et héritage du résultat.
Le contrôle d’objet manuel nouvellement public ne remplit aucun de ces contrats.
BLP-04 reste partiel ; BLP-05/06, migration explicite de l’existant et RETEX
transversal restent ouverts. Aucune initialisation Trust des données réelles,
aucun appel fournisseur, aucune déclassification ni conformité globale revendiquée.

## Reprise BLP-04 — contrôle des sources enregistrées, 1 novembre 2026

Codex reprend après le producteur public. Le prompt natif v3 couvre ses octets,
mais aucun consommateur ne combine les attributions de contrôle de ses parties.
Deux voies : activer directement une politique sur les envois existants, ou
construire d’abord un diagnostic authentifié sur le prompt effectivement enregistré.
La seconde évite de prétendre protéger les fournisseurs avant leur médiation.
Incrément décidé : `blp-preparation-check` API/CLI, lecture seule sous réservation
Trust ; identité authentifiée, révision explicite, attribution courante de chaque
partie exacte (framing inclus), combinaison et habilitation. Aucune étiquette
fournie par la requête, aucun défaut public, aucune mutation, aucun appel fournisseur.
Les anciens enregistrements sans provenance refusent ce contrôle. Les erreurs,
la fermeture et la sortie conservent leurs causes. Tests négatifs et reprise,
documentation FR/EN et installation locale requises. Ce diagnostic ne lie pas
encore le principal de l’envoi et n’autorise ni dispatch ni destination.
État : réalisation en cours ; BLP-04 demeure partiel. Propriétaire : Codex.

Incident de recette : `20261101-preparation-check-engine-01` est FAIL intact,
bootstrap avec deux identités alors que `prepare_identities` en exige trois.
Cause : fixture dérivée incomplète, avant le contrôle BLP ; ajout de la troisième
identité de test, sans changer le contrat produit. Un build Rust tiers sur les
mêmes sources est observé ; les écritures Rust attendent sa fin pour préserver
ses entrées de compilation. Python/documentation indépendants avancent.

Incident de coordination : le commit concurrent `b4cbfa2255` a inclus les sources
BLP et des preuves encore ouvertes, sans commit lancé par cette session. Aucun
historique n’est réécrit. Les canaux du run moteur 02 ont ensuite terminé
normalement : PASS, un test nommé, sources stables et intégrité VERIFIED.
Les octets intermédiaires du commit ne constituent pas cette preuve finale.
La recette publique reçoit encore ses contrôles et n’est pas déclarée passée.
Le build normal 296 qualifie ses propres octets et métadonnées Git ; son commit
compilé doit être lu dans l’artefact, jamais déduit du HEAD courant.

Livraison 296 refusée : compilation réussie (1 min 23 s), mais
`profile-effective-input-changed:rust/crates/swarm-web/src/service.rs` : la
modification concurrente de la porte Authority a changé une entrée effective.
Originaux et empreintes conservés dans `20261101-preparation-check-build-refusal-01`,
image non qualifiée/non installée. Options : attendre un état réexaminé et
requalifier les nouvelles entrées, ou créer une copie complète parallèle ;
la première respecte le checkout direct et réutilise le cache sans contourner
le contrôle. Le nouveau prébind doit inclure les octets observés de service.rs.

Critères de cet incrément (pas de clôture transversale) : diagnostic sans mutation
avec 1 test moteur nommé déjà PASS ; parcours public attendu sur image normale
avec 21 assertions CLI/HTTP et émission réelle par double local ; identité de
l’image et ressources/version FR/EN servies après installation. La classification
calculée est celle du contexte observé, pas une attribution persistée à la réponse
du fournisseur. Le principal contrôlé est celui du diagnostic, pas celui de
l’envoi historique. Les documents opérationnels exposent aussi l’absence d’un
écran d’inventaire et la granularité encore agrégée des valeurs JSON.

Contre-exemple public : `20261101-preparation-check-public-before-01` sur l’image
qualifiée 295. Trois témoins passent : véritable envoi public vers double local
inerte, provenance du producteur, couverture complète. Le contrôle nouveau est
ensuite refusé au CLI (opération indisponible), au lieu du refus typé de source
non attribuée attendu. FAIL intact, intégrité VERIFIED ; canaux du parcours liés
par `counterexample.json`. Ce test ne prétend pas que l’ancien envoi était protégé.

Replan des erreurs HTTP : le diagnostic moteur utilise des refus typés Root
qui n’étaient pas mappés par le consommateur HTTP (`trust-command`), donc 500
par défaut. Reproduction `20261101-preparation-http-status-before-01` : contrôle
BLP HTTP 200 valide, puis principal sans habilitation refusé `blp-access-denied`
mais statut 500. Image compilée explicitement non qualifiée, copiée avant son
remplacement ; preuve de défaut seulement, jamais installation/acceptation.
Second build interrompu par SIGINT de son seul groupe vérifié : exit130,
INCOMPLETE, originaux `20261101-preparation-check-build-interrupted-02`.
Décision : mapping des seuls refus métier Root sous contexte trust-command
(400/403/404/409/501), provenance corrompue et nettoyage restent500 ; matrice
typée et parcours public désormais 26 assertions exactes. Les éditions Rust
attendent la fin de la compilation/test tiers qui consomme ce même fichier.

Préconditions de BF-07/BF-16 réexaminées pendant cette reprise : le send public
actuel ne reçoit pas d’identité Trust authentifiée ; `Native` conserve une nonce,
une racine et les digests fournisseur/settings, sans principal. Une nonce de
worker ou le nom de fournisseur ne remplace pas cette liaison. Le prochain
contrat doit lier l’identité issue du transport à l’intention et au rejeu,
revalider l’autorité à l’engagement, puis conserver la classification des résultats
y compris erreurs/résumés. Un hash de configuration fournisseur prouve la
fraîcheur, pas l’identité du compte/destinataire externe. Ne pas ajouter de
principal déclaré dans le JSON pour contourner cette précondition. Ce diagnostic
ne doit jamais être réutilisé comme ticket d’envoi ; une médiation ultérieure
doit recalculer les sources exactes et vérifier l’autorité courante.

### Résultat vérifié — diagnostic des sources, 1er novembre 2026

- Moteur `20261101-preparation-check-engine-02` : PASS, un test nommé, sources
  stables et intégrité VERIFIED. Sept sources/contrats concernés sont encore
  identiques ; ne pas réexécuter ce test pour les seules modifications HTTP.
- Statuts `20261101-preparation-http-status-01` : PASS, un test nommé, sources
  stables, intégrité VERIFIED. Le défaut HTTP500 reproduit auparavant est corrigé
  par mapping des refus métier typés ; provenance corrompue et nettoyage restent500.
- Public `20261101-preparation-check-public-01` : PASS, **26 assertions CLI/HTTP**,
  véritable prompt produit par double local inerte ; toutes les attributions par
  opérations publiques, lecture SQL seulement sur cette fixture. Les refus
  préservent le journal, le résultat est rejouable ; huit cas HTTP ont statuts et
  codes exacts. Les **149 artefacts** et leur manifeste final sont revérifiés ;
  digest `9d5e8d828fb2ce014a8132576c993fe9a63deba4227e6918f9e1e698629d6b99`.
- Livraison `20261101-preparation-check-installed-01` : **installé v0.1.1-dev.296**,
  image `e4a802c4dc4d32d90d036169604168c64db15196695d692885369b748686ca72`, profil
  normal qualifié `.cache/local-delivery/rust-0ukzkap_/build/profile`, sans fixture.
  Compilation terminée en37s après libération mémoire. Redémarrage pris en charge
  avec cette empreinte exacte, zéro entrée compilée plus récente à cet instant.
  Treize sources d’exécution conformes, **56 assertions navigateur** FR/EN/thèmes/
  version/ressources protégées/commit/modifications/focus, deux aides CLI contrôlées.
  Commit compilé `b4cbfa22552140fe9a7bcf46ac91165b0abe120d`, modifié=true.
  `delivery-scope.json` distingue le redémarrage de l’incrément et la sonde readonly.
- Inventaire `20261101-preparation-flow-inventory-01` : PASS, deux étapes et cinq
  tests du vérificateur, intégrité VERIFIED. Trois fichiers revus avant actualisation
  de leurs empreintes/ancrages ; BF-25 et cinq références ajoutés : **25 flux,
  50 ancrages,46 fichiers**, tous OPEN pour le contrôle BLP transversal. L’ancien
  inventaire périmé reste FAIL intact et VERIFIED dans `...-before-01`.

Revue des octets, politiques/fonctions/erreurs et limites :
[registre de revue](verification-blp/20261101-preparation-check-source-review.json).
La dernière tentative d’annuler notre troisième build en attente n’a envoyé
aucun signal : le garde a constaté qu’il venait de commencer. Ce build a ensuite
terminé et fourni le candidat qualifié ci-dessus. Un autre build concurrent
avait refusé ses entrées changées (version_history.json/graph.js), sans transfert
de qualification ni réécriture de ses preuves. La réservation du lanceur sérialise
les builds, pas les écritures de toutes les sessions : observer la fin d’un PID
ne prouve pas l’absence d’un nouveau compilateur. Le compactage Git a disparu
sans intervention de cette session ; son statut de sortie n’a pas été observé.
L’autorisation d’interruption demandée est devenue sans objet ; aucun processus
tiers n’a été interrompu, et cette session n’a lancé ni commit ni push.

RETEX local : suivre chaque refus jusqu’au statut HTTP, à la fermeture et à la
sortie ; un moteur vert ne prouve pas le consommateur public. Reprendre les
fixtures depuis le contrat réel (trois identités), conserver les échecs originaux
et vérifier les manifestes imbriqués. Revoir les inventaires au même incrément.
Le qualificateur a effectivement refusé la dérive et le sélecteur prepared-build
a installé seulement l’image qualifiée. Amélioration de coordination conservée :
origine = builds/écritures concurrents et maintenance Git ; valeur = réduire
les requalifications et l’attente sans affaiblir les gardes ; périmètre = méthode
de développement ; propriétaire = responsable du workflow local. Options :
fenêtre d’écriture unique partagée, ou gel borné des entrées via le chemin de
livraison pris en charge. État = proposition hors incrément BLP ; observable =
qualification de sources stables pendant activité parallèle, sans simple
consigne prise pour preuve. Reprise à la conception du contrat de coordination ;
les sessions déjà démarrées ne reçoivent pas automatiquement cette note.

**BLP-04 reste partiel** : contrôle de classifications administrées pour les
tranches effectivement enregistrées, aucune attribution physique par producteur,
aucun écran d’inventaire, principal de l’envoi et domaine destinataire non liés,
aucun héritage persisté sur les résultats. BLP-05/06, migration de l’existant et
recette/revue/RETEX transversaux restent ouverts. La suite BF-05 → BF-07 → BF-16
conserve ces préconditions ; ne pas traiter ce diagnostic comme un ticket de
dispatch. Aucun bootstrap Trust réel, appel payant, déclassification humaine ou
conformité globale ; auto-revue seulement. Les derniers changements de suivi
et d’inventaire ne modifient pas l’exécutable installé.

## Reprise documentaire — état existant, 1er novembre 2026

Demande utilisateur : documenter l’existant après clarification que l’intégration
BLP n’est pas terminée. Replan : les contrats et preuves existent, mais leurs
limites sont dispersées entre Biba, inventaire et historique du plan. Options :
allonger encore cet historique, ou créer une référence opérationnelle FR/EN
reliée aux entrées existantes. Second choix réalisé dans
[Bell-LaPadula dans Swarm](../../architecture/BELL-LAPADULA.md) et son miroir EN,
avec liens README, positionnement sécurité, Biba et flux. Propriétaire : Codex.

La référence distingue calcul pur, administration, attribution exacte, lecteurs
PAEA ciblés, provenance v3 et diagnostic postérieur à l’enregistrement. Elle
expose les conditions d’autorité, l’absence de filiation physique et d’héritage,
les opérations réelles et les lots restants. Les preuves citées restent celles
de leurs sources/profils et du parcours local owner-key ; aucune qualification
d’Authority/enrôlement ou conformité globale n’est ajoutée. L’API de version a
été relue :296, commit compilé b4cbfa2255, modifié=true (observation datée).

Vérification : liens locaux/ancrages, chiffres des résultats conservés et revue
FR/EN. Aucun code, politique, validation ou traitement d’erreur modifié ; aucune
fonction créée/modifiée, donc aucune Rustdoc/docstring à changer. Les fichiers
documentaires concernés ne sont pas des entrées compilées du profil qualifié :
pas de rebuild, redémarrage ni nouvelle note de livraison. `version_history.json`
conserve les modifications d’autres sessions. Les liens persistants atteignent
les futurs lecteurs du dépôt ; aucun contexte embarqué ou processus déjà démarré
n’est reconstruit par cette documentation. Revue de cet incrément conservée dans
`verification-blp/20261101-blp-documentation-review.json`.

## Reprise exécution — verrou d’émission de préparation, 1er novembre 2026

L’utilisateur autorise la suite BLP-04/05 et l’héritage BF-16, puis la médiation
fichiers/réseau/descendants, migration et déclassification. État réel : le worker
reçoit aujourd’hui une requête réseau après budget/fraîcheur, sans lease BLP
jusqu’à l’émission. Les acquis et changements tiers sont préservés.

Deux voies examinées : un contrôle seulement à la réception du send (fenêtre
d’activation/révocation avant le transport), ou un verrou au transport sérialisé
avec les mutations Trust, plus refus avant admission. Second choix. Première
étape décidée : politique booléenne persistée `preparation_quarantine`, absente/
false = comportement historique, true = refus explicite des nouveaux sends et
des émissions des workers en attente. Aucune activation sur les données réelles.
Édition dans le JSON de politique de l’Admin, mêmes validation et valeur via
API/CLI ; statut FR/EN explicite. Un changement d’état attend la fin d’une
opération déjà engagée sous réservation partagée ; arrêt opérateur et délais
existants restent applicables. Aucun nouveau timeout/plafond ou rôle implicite.

Critères : ancien JSON/digest inchangé quand le champ est absent/false ; rejet
null/type invalide ; aperçu sans mutation, sauvegarde/relecture/reprise ; contrôle
valide avec émetteur inerte avant activation, puis zéro émission après refus ;
garde répétée au transport, activation sérialisée avec une émission en cours ;
causes et effet déjà observé conservés si libération échoue. Recettes permanentes,
revue liée aux octets, documentation FR/EN et installation du candidat qualifié.
Propriétaire : Codex, sans délégation/appel payant. Ce verrou ne fournit pas de
chemin d’envoi classifié autorisé, d’héritage de réponse ou de confinement des
descendants : ces besoins restent ouverts et la suite ne s’arrête pas à ce verrou.

Replan de disponibilité avant livraison : le témoin core-01 validait une
réservation Trust globale pendant l’IO ; cette conception bloquerait aussi les
écritures métier. Options : conserver ce blocage avec arrêt manuel, ou séparer
le verrou d’émission et ne garder Trust que pour le snapshot. Second choix :
`preparation-egress.lock` partagé autour de l’opération, exclusif pour config-apply
et blp-policy-apply, ordre egress → SQL éventuel → Trust court. Le consommateur
enrôlé doit acquérir egress avant sa transaction SQL pour éviter une inversion
contre l’admission. Pré-authentification canonique I3/admin avant attente, puis
authentification autoritative répétée. Core-01 reste historique, remplacé pour
la concurrence ; le modèle pur 11/11 garde ses sources. Le nouveau témoin vérifie
égress exclusif bloqué, Trust exclusif disponible et appel faible refusé sans
attendre. Les parcours publics couvrent aussi l’émission CLI déjà engagée.

Reprise des preuves : locks-02 refuse la qualification malgré 1+15 tests réussis, car les traductions UI ont changé pendant le run. Elles ne sont pas des entrées de compilation des tests moteur ; retrait de ces quatre entrées du seul collecteur moteur, vérification UI conservée dans son parcours public. Le refus original reste conservé.

Contrôle de méthode : le script npm `i18n:check` supposé n’existe pas ; refus conservé dans la session. Le contrat réel de package.json est `i18n:build` et le générateur accepte `--check` ; vérification corrigée par cette commande directe, sans modifier les ressources compilées.

Replan public-01 : les 25 contrôles de transport passent ; le navigateur échoue avant interaction sur une URL de test supposée. La table native_admin.rs sert le fichier trust-native.html sur /trust-actions.html, pas /trust-native.html. Correction du parcours et du mapping route → fichier compilé, diagnostic HTTP enrichi du chemin sans secret. Aucun changement produit, tolérance ou masquage du404 ; reprise avec ces entrées corrigées.

## Qualification et livraison de la quarantaine — 1er novembre 2026

Implémenté, vérifié et installé : v0.1.1-dev.299, SHA256
`75ad879faf4807d33b6b9c3ff5a71e570aa76d252f394da3b309d95ecadda5e9`.
Profil natif normal sans fixture ; helper de tours en attente construit séparément.
Contrôles : modèle pur11 (core-01, sources inchangées), verrou/libération1,
confidentialité15 et enrôlement4 (locks-03), parcours public28 et Admin59
(public-02), inventaire54 ancrages/25 flux OPEN et5 régressions (inventory-01).
Le récepteur inerte observe sockets et requêtes : refus sans émission nouvelle,
worker de production en attente refusé et réservation non engagée libérée ;
activation attend les émissions API et CLI engagées, reprise explicite vérifiée.

Installation par restart --prepared-build exact ; processus/version/notes FR/EN,
ressources protégées et56 assertions navigateur vérifiés (installed-01), aide CLI
FR/EN relue. Commit compilé d4e219eeaf, modifié=true. Une entrée compilée plus
récente non installée est annoncée par le lanceur. Aucun bootstrap Trust, classement
de données réelles ou activation de quarantaine réelle ; aucun appel fournisseur
payant, commit ou push effectué par cet incrément. Les changements tiers sont préservés.

Preuves : dossiers `verification-blp/20261101-preparation-quarantine-*` ; revue
sémantique liée aux octets dans `20261101-preparation-quarantine-source-review.json`.
Locks-01/02 et public-01 restent des refus historiques intacts. Les apprentissages
sont transmis par les contrats FR/EN et les recettes persistantes ; cela ne met
pas à jour les contextes d’agents déjà démarrés. Auto-revue, aucune acceptation
indépendante ni conformité globale déclarée.

Suite conservée : BLP-04 doit lier le principal authentifié de l’admission, les
classifications attribuées par les producteurs et le domaine immuable aux octets
capturés ; BLP-05 doit ensuite lier destinataire/compte/politique au transport ;
BF-16 doit hériter ce domaine sur réponses/erreurs/propositions et lecteurs.
La quarantaine reste un blocage explicite, sans chemin d’envoi classifié autorisé.
La médiation OS des descendants, la migration et BLP-06 restent ouverts.

## Reprise BLP-04 — attribution depuis la source capturée, 1er novembre 2026

Besoin repris après la quarantaine299 : le diagnostic sait résoudre les tranches
mais leur producteur public demande encore un digest et un chemin déclarés par
l’opérateur. Deux voies : enregistrer immédiatement un domaine d’exécution (ses
lecteurs/destinataires ne sont pas encore protégés), ou raccorder d’abord
l’attribution au producteur réel. Seconde voie retenue, sans habilitation implicite.
Codex réalise seul `blp-preparation-attribute` : I3/admin authentifié, habilitation
sur la classification explicitement choisie, révisions Trust/BLP/tour courantes,
parent exact, instance Root et manifeste complet vérifiés ; index d’une tranche
capturée, jamais digest/chemin fournis. La même opération d’attribution conserve
l’unicité, le refus des déclassements et la publication/reprise. Le contexte de
capture participe à l’identité d’événement ; aucun texte source n’est rendu.

Critères à vérifier : CLI/API persistantes, replay identique, index invalide,
parent/révisions erronés, absence d’autorité avant recherche de ligne, entrée
forgée/refus sans mutation, jointure diagnostique après classement des tranches,
profil natif et installation locale identifiés. Sources identiques restent un
seul objet par digest/projet/révision : un autre événement ne les reclasse pas.
Ceci reste une attribution de contrôle postérieure à la capture, sans filiation
physique durable, principal d’admission, domaine immuable, envoi autorisé ni
héritage des résultats. Ces critères et les autres flux restent ouverts.

Replan de recette / recipe recovery: before-01 échoue avant le nouveau parcours
(KeyError revision). Le DTO réel expose source_revision ; correction du consommateur
de test, preuve originale conservée, reprise dans before-02. Aucun comportement
produit ne découle de ce premier échec.

Replan de disponibilité de vérification : compilation encore active sous pression
mémoire observée (14GiB RAM, swap14GiB/15GiB) et pack-objects Git concurrent
(~6GiB, autre parent). Aucun timeout arbitraire ajouté, aucun processus tiers
arrêté. Suivi des sorties/CPU et travaux documentaires indépendants ; cette
lenteur ne constitue pas un résultat de test.

Revue avant livraison : la lecture ajoutée ouvrait une transaction imbriquée
inutile lorsque le consommateur en possède déjà une. L’essai engine-01 est
interrompu volontairement (seul rustc de cette recette), et reste FAIL intact.
Lecture unique de la ligne complète retenue ; régression témoin, transaction
appelante préservée, corruption/root/historique et reprise. Aucun échec en
production observé : le vocabulaire signé d’enrôlement ne propose pas encore
les opérations BLP. Cette correction ne les y ajoute pas et ne qualifie pas
l’authentification enrôlée. Autre constat sur le fichier d’aide modifié : sa
formule héritée « aucun contrôle d’exécution » ignorait la quarantaine ; aide
FR/EN précisée en contrôles partiels, et blp-check séparé de l’exigence Admin.

Amélioration conservée : l’opérateur doit pouvoir distinguer les tranches et les
digests identiques sans lecture SQL. Valeur : classement explicite compréhensible ;
périmètre : futur inventaire protégé des sources, propriétaire BLP/UX, état OPEN.
Critère : identité/habilitation avant métadonnées, sélection du bon producteur,
aucun texte classifié rendu sans droit. La présente API ne livre pas cet écran.

Replan engine-02 : les2 tests réussissent mais le collecteur hérité inclut les
sources web de sa seconde étape ; native_admin.rs change pendant le test moteur,
ce qui refuse correctement le lot entier. Séparation des recettes moteur et
adaptateur HTTP, inventaires conservés selon leurs dépendances réelles. Le moteur
reprend la fermeture de dépendances déjà retenue par locks-03 ; le contrôle web
reste dans sa propre recette et dans le parcours public du binaire qualifié.
Ce défaut de méthode rappelle de dimensionner les preuves par étape compilée,
pas d’hériter un collecteur multi-composants. Original engine-02 intact/FAIL.

Replan de méthode : engine-03 recompilait malgré les2 résultats inchangés. Arrêt
du seul compilateur de cette recette ; refus conservé. Réutilisation explicite
des2 tests nommés engine-02 avec522 entrées moteur conservées, identiques avant/
après/actuellement, dans engine-reuse.json. Le lot initial reste FAIL ; cela ne
prouve ni le graphe compilateur complet ni un profil de production. La recette
moteur corrigée est conservée pour les futures modifications pertinentes.

Le serveur observé est303 (autre livraison, distincte de299 historique).
Build304 compile en38,88s mais refuse la qualification après modifications
concurrentes de locales/en.json, web/i18n-en.js et web/native-authority-admin.js.
Aucun redémarrage ni installation304. Originaux conservés dans build-01.
Reprise : nouvelles empreintes effectives après cette édition Authority/UI,
recette HTTP indépendante puis parcours public sur une image qualifiée.

Replan build-02/adapters-01 : l’unité HTTP nommée réussit1/1, mais son lot
refuse des modifications Authority pendant compilation ; le second build
compile aussi puis refuse audit.rs/server.rs modifiés. Ces fichiers participent
réellement à la fermeture compilée : aucune exclusion ni installation d’un
candidat refusé. Les deux builds et le lot adaptateur restent FAIL intacts.
Coordination demandée à l’utilisateur : finir/suspendre les éditions de la
session Authority pour une fenêtre de qualification. Propriétaire de cette
précondition : session Authority/utilisateur ; pas d’arrêt de ses processus.
En parallèle, nouveau contrôle moteur de l’opération publique : attribution
réelle depuis la tranche, refus/rejeu/conflit, corruption/root/historique et
refus des déclassements après renouvellement. Sources moteur seules modifiées ;
pas de nouvelle fonction produit ni de changement runtime après build-02.

Replan public-01 : attribution CLI et rejeu HTTP réussissent. La première
assertion HTTP de refus échoue parce que la recette reprend des tuples CLI
(code, statut) comme (statut, code). La réponse originale est403 avec
`authorization_required`, conforme au contrat. Conversion explicite de l’ordre
pour le seul consommateur HTTP ; refus original conservé, aucun changement
runtime, reprise sur la même image qualifiée341e71bb… et une fixture neuve.

## Livraison du producteur de sources capturées — 1er novembre 2026

Installé et vérifié en **v0.1.1-dev.304**, image SHA256
`341e71bb2e67d47d0c468c04f8c7ecd38f313fb0686b60af88ba675d50cbade6`,
commit compilé821dbb340ceb99b323fd4bb9a7f2bbdc106b0e9c, modifié=true.
Le troisième build a retrouvé une fenêtre stable et un profil natif qualifié,
sans fixture ; les builds refusés restent intacts, jamais installés. Redémarrage
par sélection exacte du candidat ;0 entrée compilée plus récente à cet instant.
La demande de coordination Authority n’a entraîné aucun arrêt de ses processus.

Preuves : engine-04,2 tests nommés PASS et intégrité VERIFIED ; adapters-02,1 test
HTTP PASS/VERIFIED ; public-02,103 assertions CLI/HTTP PASS/VERIFIED, dont écriture
HTTP puis rejeu CLI et classement sous quarantaine sans la lever ; inventory-02,
56 ancrages/25 flux OPEN et5 régressions ; installed-01,21 entrées runtime
concordantes et56 contrôles navigateur (version, notes FR/EN, commit réel et
modifications locales). Sources fonctionnelles du test moteur inchangées après
seul ajout de sa Rustdoc, preuve séparée test-doc-review ; pas de réexécution
inventée pour cet ajout documentaire. Les échecs de recette, interruptions et
changements concurrents restent conservés et distingués des résultats nommés.

Revue personnelle liée aux octets : `20261101-preparation-source-producer-source-review.json`, politiques/fonctions/
erreurs et documentation. Aucune revue indépendante, acceptation globale ou
conformité transversale ajoutée. Aucun appel payant, classement réel, bootstrap
Trust, activation de politique réelle, commit ou push par ce lot.

BLP-04 progresse par un producteur d’attribution post-capture, pas par un domaine
à l’admission. Suite conservée : identité authentifiée de l’émetteur à l’admission,
classifications héritées des producteurs et domaine fixe ; liaison effective du
compte/destinataire au transport (BLP-05), puis résultats et lecteurs BF-16.
Fichiers/réseau/descendants, migration et déclassification humaine restent ouverts.
`runtime_enforced:false` demeure justifié. Les documents/recettes sont persistants ;
les contextes d’agents déjà démarrés ne sont pas remplacés par ces textes.

Contrôle final : une restauration de stash concurrente crée deux conflits dans
les PAEA-APEX FR/EN. Conservation des originaux compressés et des stages Git dans
documentation-conflict-01 ; fusion des bilans historiques299 puis304 sans perdre
les textes voisins, index résolu sur ces deux seuls documents. Aucun commit ni
changement runtime. Vérifications diff de travail/index et revue documentaire
actualisées.

## Reprise BLP-04 — domaine fixé avant émission, 1er novembre 2026

Autorisation utilisateur : poursuivre le domaine à l’admission. Comparaison :
transformer les tours historiques/pending (lecteurs et dispatch legacy exposés),
ou créer une admission immuable distincte sans tour exécutable. Seconde voie :
réutiliser exactement le producteur de prompt, extraire sa capture sans écriture
SQL/réservation, attribuer explicitement les tranches avant admission par une
opération I3/admin habilitée, puis joindre toutes les attributions courantes sous
le principal authentifié de l’admission. Les sources inconnues refusent sans
capsule ; aucun rôle, label, compte ou destinataire n’est inféré.

Capsule privée create-only, framing natif existant (taille observée, SHA), liée
au Root ouvert, principal, révisions/digest de politique et octets complets du
prompt. Espace distinct des tours legacy : aucun dispatch/worker/lecteur legacy
ne reçoit la capsule. Rejeu immuable, lecture métadonnées sous principal courant
et habilitation, changement de politique/Root/refus avant restitution. Aucun
budget réservé, fournisseur contacté ou transport autorisé par l’admission.
La quarantaine ne bloque pas ce travail local sans IO fournisseur et n’est pas
modifiée. Réutilisation des politiques de contexte et de contention persistées.

Critères : producteur identique au send historique, capture sans mutation,
authentification avant lookup, attribution pré-admission explicite, refus de
source manquante/clearance/révision/déclassement, capsule fixe/replay/reopen,
corruption et publication incertaine conservées, zéro tour/budget/émission,
CLI/API réels, docs FR/EN/revue des octets et installation locale qualifiée.
Propriétaire Codex seul ; aucune délégation, appel payant ou politique réelle.
Le destinataire, les résultats/lecteurs, l’OS et la migration restent ouverts.

Replan engine-01 : compilation normale refuse la référence à AtomicStorageFault
réexportée seulement sous test-fixtures. Consommer le type interne existant,
comme le journal Trust ; aucune feature fixture activée pour contourner le refus.
La capsule retient aussi niveau demandé/cible/mode de contexte, sans clé :
nécessaires au raccord de transport futur, qui reste non autorisé.

Replan engine-02 :6 cas réussissent, dont l’admission ; un ancien test exige un
refus du rejeu actif historique. Le binaire304 reproduit pourtant le retour
readonly/pending, sans modifier la ligne ; la fonction replay_turn est identique
octet pour octet au profil304. Original engine-02 reste FAIL. Correction du
contrat de test : observation readonly, dispatch_if_native=false, claim natif
refusé et budget toujours réservé. Rejouer ce seul cas ; les six corps de tests
réussis et le code produit ne changent pas. Baseline publique conservée dans
legacy-baseline-01. Le compactage Git termine sans interruption ; aucune
suspension n’a été appliquée en l’absence de réponse à la demande d’autorité.

Build-01 compile normalement en40,88s mais refuse la qualification : audit.rs/
server.rs Authority changent pendant la compilation. Originaux conservés, aucune
installation ni exclusion de dépendance. Le build suivant requiert les empreintes
Authority actualisées. L’instance observée avant ce build était308 (autre lot).

Replan public-01 : CLI/admission/jointure/absence de mutations passent ; la
recette HTTP masque son module importé par une fonction locale homonyme http.
Renommage du seul helper en request_http et correction du préfixe de preuve
hérité. Aucun changement runtime, nouvelle fixture et même image309 qualifiée ;
refus original conservé.

Replan livraison réelle :309 est activé dans le moteur interne38434, derrière
Authority38433. Le probe owner-key vérifie version/image/22 sources/notes puis
reçoit403 sur les assets protégés : refus attendu du mode, pas un succès UI.
Aucun gate affaibli, aucune identité/token humain fabriqué. installed-01 conserve
le refus ; installed-02 est explicitement PARTIAL_AUTHORITY_UI_NOT_VERIFIED,
avec22 sources et refus403 sur les3 assets. Le pilote navigateur signale son
hôte indisponible ; contrôle visuel réel demandé à l’utilisateur authentifié.
Une entrée source Authority/assertion.rs plus récente (wire v2) n’est pas dans
l’image309 ; aucune installation de cette évolution n’est déclarée et son
raccord demeure au chantier Authority. Son statut indique actif mais binaire
installé différent du dernier build, sans diagnostic de session humaine fabriqué.

Vérification UI indépendante sur fixture, même image de production : première
recette refuse l’appel Server.request observé pendant capture asynchrone.
Contrat réel : observe=False pour la requête HTTP séparée. Corrigé uniquement
dans le driver ; nouvel espace ui-02, même image, critère Authority réel maintenu
ouvert. Le navigateur partagé distingue désormais explicitement fixture d’image
et instance gérée ; mêmes assertions, marqueur privé obligatoire, aucun contournement.

## État de l’admission pré-transport — 1er novembre 2026

Code implémenté et activé dans le moteur **v0.1.1-dev.309**, image SHA256
`c12004bf64f1ac5371d78ebeb4ab2d852da9b0e83d3e4a692b8fb28268dbc69b`.
Profil natif normal qualifié, sans fixtures ; producteur de prompt extrait sans
changer son corps (preuve producer-parity). Principal Trust réel de l’admission,
jointure de toutes les tranches actuelles et capsule immuable privée ; aucune
émission, réservation, tour exécutable ou droit fournisseur. Paramètres/routage
retenus sans clé, sans autorité de sink implicite. Aucun label/principal du
modèle ne peut remplacer cette admission.

Preuves de code :6 corps de tests réussis engine-02 réutilisés à l’identique,
puis cas legacy corrigé1/1 PASS/VERIFIED (engine-reuse et legacy-01). Le lot
engine-02 reste FAIL original, pas un faux succès global ; son seul échec est
reproduit sur304 et corrigé dans le test selon le contrat readonly réel.
Public-02 :74 assertions CLI/HTTP PASS/VERIFIED, dont jointure différente de
l’habilitation maximale, classement HTTP pré-admission, révocation, corruption
frame/Root/label/JSON avec témoins et reprises, aucune émission/tour/réservation.
Legacy-public-01 :103 assertions anciennes passent sur la même image, dont un
fournisseur CLI inerte. Inventory-01 :60 ancrages/25 flux OPEN et5 régressions.

Livraison par restart --prepared-build exact. Installed-02 confirme image/PID,
version et notes FR/EN, aide CLI et22 fichiers runtime. Une source Authority
plus récente (assertion.rs, v2) n’est pas installée par cette image ; sa livraison
reste au chantier Authority. L’instance a été passée en mode Authority dans
l’autre chantier : démon38433, moteur38434. Les3 assets protégés refusent
l’owner-key403 ; aucune protection n’a été désactivée. Le contrôle UI réel reste
**OPEN**, propriétaire utilisateur/session Authority : connexion réelle et
observation du dialogue Version, pilote navigateur actuellement indisponible.
Installed-02 est donc PARTIAL_AUTHORITY_UI_NOT_VERIFIED, jamais un PASS visuel.

Ui-02 vérifie séparément56 assertions navigateur sur une racine isolée avec
la MÊME image de production (assets réellement servis, notes FR/EN, version et
commit) ; cela ne vaut pas acceptation de la session Authority réelle. Les
recettes/préconditions ont été corrigées sans remplacer les refus originaux.
Les données/politiques réelles n’ont pas été classées ou modifiées ; aucun appel
payant, commit, push ni identité humaine fabriquée. Revue [liée aux octets](verification-blp/20261101-preparation-admission-source-review.json) : politiques, fonctions, erreurs et documents ; revue personnelle uniquement.

Prochain raccord BLP-05 : vérifier au dernier usage le compte/destinataire, la
clé et les droits/budgets actuels avant d’autoriser un transport depuis une
capsule. Puis héritage des résultats/lecteurs et médiation OS/descendants.
Transformations amont, migration, déclassification humaine et retrait explicite
des capsules restent ouverts. `runtime_enforced:false` demeure nécessaire.
Les documents/recettes persistent ; aucun contexte natif déjà lancé n’a été
remplacé par ces textes et aucune surveillance récurrente n’a été créée.

## Reprise BLP-05 — destinataire et transport au dernier usage, 1er novembre 2026

Besoin : BLP-05 — contrôle effectif du compte et du destinataire au dernier
usage, avant émission fournisseur. Choix utilisateur confirmé : **liaison
explicite du sink** — une opération enregistre un label de destination pour le
compte/endpoint exact (base_url+model+digest de configuration) ; l’envoi
classifié revalide capsule+destination+politique/budget au dernier usage sous le
verrou egress ; le chemin d’envoi legacy n’est **pas** modifié.

### Réalisé (implémenté + testé local, non activé côté travailleur)

- `blp-preparation-destination-attribute` : administrateur habilité, écho
  obligatoire `base_url`/`model` de la connexion courante, digest canonique
  recalculé par le Root (jamais appelant), clé jamais en entrée ; passage par le
  producteur `blp-object-attribute` (unicité exact-once, anti-déclassification
  hérités). [`preparation_transport.rs`](../../../rust/crates/swarm-engine/src/root_store/preparation_transport.rs)
- `blp-preparation-transport` : sous verrou egress partagé, re-vérification
  complète de la capsule, fraîcheur connection/provider_source/settings depuis
  l’admission, liaison courante pour ce digest exact, dominance no-write-down
  au dernier usage, observation budget lecture seule ; réponse fixe non émissive
  (`transport-authorized-not-sent`, `runtime_authorized:false`,
  `runtime_enforced:false`), publication `None`.
- Dispatch readonly + egress : `trust_store.rs:69-122` ; mapping HTTP
  400/404/409/403/500 dans `swarm-web/src/service/trust.rs` ; CLI whitelist
  `swarm-cli/src/trust.rs` ; readonly public `blp-preparation-transport`.
- Test d’intégration `rust/crates/swarm-engine/tests/preparation_transport.rs`
  : **PASS 1/1 (2.0s)** — refus non attribué, écho TOCTOU, liaison réservée
  admin, liaison enregistrée sans clé, rejeu exact, décision fixe non émissive,
  rejeu identique, ownership, zéro réservation SQL, fixture auto-nettoyée.
- Parcours public `tests/blp_transport_public.py` + recette
  `tools/verification/recipes/blp-preparation-transport-public.json` :
  **exécutés le 2 novembre 2026 sur le binaire qualifié** sha256
  `41f00bf9…` (profil `rust-zrtk4hyt`, installé localement, v0.1.1-dev.312) :
  **PASS 65/65 assertions** (`BLP_TRANSPORT_JOURNEY_SHA256`
  `0690d1e0…`), recette `recipe.py run` PASS puis `recipe.py verify`
  `VERIFIED/PASS` (preuves `.swarm-fixtures/blp-transport-recipe-20261102/`,
  parcours `.swarm-fixtures/blp-transport-proof-20261102/`). Le parcours
  prouve aussi le rejeu identique, le no-write-down (`blp-access-denied`),
  l’invalidation par renouvellement de politique (3 révisions), la fraîcheur
  des paramètres, la révocation d’identité, la carte HTTP complète
  (200/400/404/404-ownership/403) et l’absence totale de lancement fournisseur
  ou mutation SQL (sentinelle + compteurs). Aucun résidu `/tmp` du parcours.
- Inventaire BF (`tools/paea/check_blp_inventory.py`) : rafraîchi, 65
  références / 25 flux, 0 dérive, +5 références transport (producteur,
  dispatch, HTTP).
- Amélioration portée par l’exécution : `preparation_calls.rs` allouait ses
  fixtures via `std::env::temp_dir()` (interdiction `/tmp` du 31 octobre) ;
  corrigé sur le schéma établi (`TMPDIR` exigé + refus `/tmp`, enregistrement
  de naissance, `Drop` conservant la racine uniquement en panique). Suite
  **7/7 PASS**, preuve `/tmp` avant=après, zéro racine restante.
- Revue liée aux octets : [source-review](verification-blp/20261102-preparation-transport-source-review.json).
- Documentation FR/EN : BELL-LAPADULA (nouvelle section), BIBA-RUST (contrat
  technique complet + diagramme), BLP-FLOWS (paragraphe de révision BF-05/BF-07).

### Limites conservées

La décision n’engage pas le transport réel (BF-07 OPEN) ; héritage des
résultats/lecteurs, confinement OS/descendants, envois legacy, transformations
amont : inchangés. `runtime_enforced:false` reste justifié. Le hook
travailleur (branche Api de `preparation_worker.rs`) est l’incrément **suivant**,
pas celui-ci ; aucun chemin d’envoi existant n’a été touché. Aucune donnée
réelle classée, aucun fournisseur payant, aucune capsule réelle.

### Étape suivante

Incrément suivant : hook travailleur d’émission (branche Api de
`preparation_worker.rs`) appelant la décision au dernier usage avant tout
envoi classifié ; le chemin legacy reste intact. RETEX de l’incrément :
[RETEX-BLP-05-20261102](paea/RETEX-BLP-05-20261102.md).

## Reprise BF-07 — hook travailleur d’émission, 2 novembre 2026

Besoin : BF-07 — le travailleur API (`_prepare_turn`, branche Api de
`swarm-cli/src/preparation_worker.rs`) doit appeler la décision de transport au
dernier usage avant toute émission classifiée ; le chemin legacy (zéro capsule
d’admission) reste intact. Fait suite à BLP-05 (décision
`blp-preparation-transport`), sans toucher cette décision ni ses refus.

### Conception vérifiée avant écriture

- La connexion vient du champ `provider` du tour enregistré (jamais d’un
  argument appelant) ; `base_url`/`model` effectifs du travailleur sont passés
  en écho, exactement comme la décision — le mismatch
  `blp-destination-echo-mismatch` ferme le TOCTOU claim→envoi.
- La garde de tour (`gated_turn`, `pub(super)` dans `preparation_call.rs`)
  relit la ligne sous garde matérielle et refait le contrôle de possession
  token=SHA256(nonce) + PID/stamp/host ; statut exact `running` (l’état laissé
  par la claim). `Native` reste privé ; la projection `TurnCustody` n’expose
  ni prompt, ni provenance, ni nonce.
- Le principal vient du receipt produit par le producteur d’admission
  (auto-authentifié : `admission_id == identity(principal, preparation_id,
  event_id)` — toute altération du principal casse le digest). Le travailleur
  ne détient aucun token de principal ; possession d’un tour réclamé n’est
  jamais une identité.
- Couplage admission↔tour : `request_hash` est canonique sur la struct
  `PreparationSend` **y compris `event_id`** — vérifié par l’exécution du test
  (deux event_ids distincts produisent deux hash distincts) ; un receipt
  correspond donc à (preparation_id, provider, request_hash) du tour pour le
  même event_id.
- Verrous : la gate tourne dans la réservation egress partagée déjà tenue par
  le travailleur (partagé+partagé ne bloque jamais ; la photo de quarantaine
  est reprise, strictement pas plus faible). Ordre egress → Trust → settings
  préservé ; la réservation Trust courte n’est prise qu’après détection
  d’au moins un receipt correspondant.

### Réalisé (implémenté + testé local, non activé en production)

- `RootStore::preparation_worker_transport_gate(id, token, base_url, model)`
  dans `preparation_transport.rs` : zéro receipt correspondant → `Ok(())`
  **avant toute lecture d’état Trust** (chemin legacy inchangé) ; ≥1 receipt →
  re-vérification complète de chaque capsule (principal du receipt, instance,
  révisions, digest de politique, re-jointure de chaque source, reconstruction
  du label de sortie), fraîcheur settings/provider_source, attribution de
  destination pour le digest résolu de la connexion enregistrée, no-write-down
  au dernier usage, observation budget lecture seule. Tout échec refuse avant
  l’envoi ; la gate ne contacte rien, ne réserve rien, n’écrit rien en SQL.
- Balayage fail-closed : seuls les noms de grammaire exacte
  `blp-admission-{64 hex}.receipt` sont candidats ; un receipt correspondant
  illisible/indécodable refuse (`blp-admission-not-found` /
  `blp-admission-record-invalid`), jamais un silence.
- CLI : le token passe de `execute()` à `run()` ;
  `preparation_worker_transport_gate(id, token, &r.base_url, &r.model)` est
  appelée avant la construction de la requête (`swarm_api_client::complete`) —
  aucun socket n’est ouvert avant la décision.
- Test d’intégration
  `rust/crates/swarm-engine/tests/preparation_worker_transport.rs` :
  **PASS 1/1 (1.8s)** — zéro admission → `Ok` immédiat sans réservation ;
  token falsifié → `preparation-worker-registration-mismatch` ; admission sans
  destination → `blp-destination-not-attributed` ; écho faux (base_url et
  model) → `blp-destination-echo-mismatch` ; attribution exacte → la gate
  passe ; aucune réservation de gate. Fixture auto-nettoyée, zone `.swarm-fixtures`.
- Recette permanente `tools/verification/recipes/blp-preparation-worker-gate.json`
  : **PASS 3/3 étapes** (`recipe.py run`, preuves `.cache/bf07-proof/`) —
  gate (refus + chemin legacy), suites voisines inchangées
  (`preparation_calls` 7/7, `preparation_transport` 1/1), compilation
  `swarm-cli` avec threading du token.

### Limites conservées

Gate active uniquement dans la branche Api du travailleur Rust ; le chemin CLI
(`preparation_worker_run_cli`) garde son propre contrôle egress (hors périmètre
BF-07). Aucune donnée réelle classée, aucun fournisseur payant, aucune capsule
réelle. Héritage des résultats/lecteurs (BF-16), confinement OS/descendants :
ouverts. « Implémenté + testé local » n’est ni activé en production ni
accepté ; Bell-LaPadula n’est pas déclaré terminé.

### Étape suivante

BF-16 : héritage du domaine sur les réponses/erreurs/propositions et les
lecteurs. Le commit/push de BF-07 attend l’autorisation explicite habituelle.

## BF-16a — contrôle des lecteurs du dialogue — 5 novembre 2026

Premier sous-incrément de BF-16, livré après le trou reproduit au pré-fix :
un envoi admis à réponse classifiée était servi par la lecture publique du
dialogue à une session nue sans aucun contrôle.

- **Implémenté** : scan-then-décide dans
  `RootStore::preparation_dialogue(id, reader_token)` (lecture SQL d’abord
  comme entrée de décision, puis balayage fail-closed des receipts
  d’admission ; zéro receipt correspondant = chemin legacy, ≥1 = identité
  PA-09 puis récupération trois-passes — re-vérification complète, couverture
  des receipts périmés par équivalent courant de même principal/Root/étiquette/
  lignée, `check_read` sur chaque classification du receipt). Quatre codes
  typés : `blp-dialogue-authentication-required`,
  `blp-dialogue-policy-required`, `blp-dialogue-policy-stale`,
  `blp-dialogue-access-denied`. Web : extracteur `reader_bearer_token`
  (Bearer unique non vide, refus typé avant ouverture de Root) + 403 sur
  les quatre codes. CLI : `SWARM_TRUST_TOKEN` (vide = session nue). Le
  chemin producteur interne (`preparation_call`) reste non contrôlé,
  délibérément. `reader_identity_state`/`ReaderIdentityState` élargis à
  `pub(super)` sans changement logique. La récupération staleness et les
  gardes de corruption stockée sont le travail reçu des sessions
  déléguées des 3–4 novembre 2026 (images 325/331, historique détaillé dans
  `docs/actions-confiance-rust.md`) ; cette entrée documente l’ensemble
  BF-16a.
- **Testé local** : test d’intégration
  `preparation_dialogue_readers.rs` — matrice session nue/jeton inconnu/
  lecteur non habilité (refus, zéro octet servi), récupération par lecteur
  habilité, continuité legacy sans admission ; 2/2 verts. Gate BF-07
  voisine : 1/1 verte. Recette permanente
  `tools/verification/recipes/blp-preparation-dialogue-readers.json` :
  PASS 3/3, preuve
  `.swarm-fixtures/blp-dialogue-readers-proof-20261105b-2479227` (re-run sur
  l'arbre évolué après réception staleness ; integrity VERIFIED ; archive
  `.tgz` + SHA-256 copiée et relue sur
  `/media/fpizzi/Lexar/swarm-preuves/2026-11-05/`, index
  `.swarm-fixtures/PROOF-INDEX.md`). La frontière HTTP publique et la
  récupération staleness sont reçues via
  `tools/verification/recipes/blp-dialogue-reader-public.json` (93
  contrôles, images 325/331, historique dans
  `docs/actions-confiance-rust.md`). La suite complète
  `cargo test -p swarm-engine` n’a pas été rejouée intégralement après
  interruption ; la preuve ciblée porte sur les deux suites concernées.
- **Limites conservées** : lecture présentation seulement —
  `preparation_show`, `preparation_history` et les documents adoptés liés
  par `source_turn` restent sans contrôle lecteur (incrément BF-16
  suivant, clos par BF-16b ci-dessous) ; héritage sur erreurs/propositions,
  médiation fichiers/réseau/descendants, migration des données existantes
  et déclassification humaine (BLP-06) ouverts. Pas d’installation locale
  effectuée (le check workspace reste bloqué par le WIP concurrent de la
  session authority, E0063 `authority_ledger.rs`) ; pas de commit/push
  sans autorisation explicite ; « implémenté + testé local » n’est ni
  activé en production ni accepté.

## BF-16b — lecteurs de show, history et release-review — 6 novembre 2026

Deuxième sous-incrément de BF-16 : le verdict lecteur du dialogue est
étendu aux autres frontières de présentation, pour fermer le contournement
« lire par show/history ce que le dialogue refuse ».

- **Implémenté** : verdict extrait en
  `preparation_dialogue::reader_verdict` (pub(super)) partagé par
  `read_controlled` (dialogue), `preparation_show(id, reader_token)` et
  `preparation_history(id, kind, before, reader_token)` — mêmes codes
  `blp-dialogue-*`, même scan-then-décide, même récupération trois-passes,
  même habilitation ; rien n’est servi avant `Ok(())`. Show/history
  construisent leurs clés de décision via l’aide `recorded_turn_keys`
  (tours enregistrés décodés sous la même capture, prompts effacés,
  incohérence d’identité refusée comme dialogue, sans observation
  d’historique actif). Zéro receipt apparié = chemin legacy conservé.
  `preparation_release_review(id, reader_token)` propage le jeton vers
  show. Web : `reader_bearer_token` extrait sur show/history/
  release-review ; 403 inchangé sur les quatre codes partagés. CLI :
  `SWARM_TRUST_TOKEN` résolu une fois pour show/release-review/dialogue/
  history.
- **Testé local** : `preparation_dialogue_readers.rs` 4/4 verts — les deux
  tests BF-16a conservés plus `show_and_history_inherit_the_dialogue_
  reader_verdict` (session nue + lecteur sous-habilité refusés sur show et
  history pour chaque kind, récupération par le sender habilité, history
  `besoin` révision 1 servie au lecteur habilité) et
  `show_and_history_without_any_matched_admission_keep_legacy_bare_read`
  (continuité session nue sans admission sur les deux frontières).
  Régresseurs web/CLI : `swarm-web --test service` 10/10,
  `--test confidentiality_http` verte, `swarm-cli` lancé avant le WIP
  concurrent `trust/data_policy.rs` (E0432 transitoire observé ensuite,
  périmètre de la session déléguée). `preparation_show` gagne un
  paramètre `reader_token` : appelants adaptés (web, CLI,
  release-review).
- **Limites conservées** : lecture présentation seulement — l’exécution,
  l’adoption et les autres canaux restent hors du verdict ; héritage sur
  erreurs/propositions, médiation fichiers/réseau/descendants, migration
  des données existantes et déclassification humaine (BLP-06) ouverts.
  Pas d’installation locale (même blocage WIP concurrent) ; pas de
  commit/push sans autorisation explicite ; « implémenté + testé local »
  n’est ni activé en production ni accepté. La recette permanente
  `blp-preparation-dialogue-readers.json` couvre la frontière dialogue et
  la gate BF-07 ; une extension de recette couvrant show/history est
  conservée comme suite (les tests d’intégration font foi en attendant).


## Fermeture class-2 — migrations tempdir sous aires gouvernées, 7 novembre 2026

Clôture du constat d'environnement ci-dessus (« les `fixture()` qui passent
par `tempfile::tempdir()` → `/tmp` ») : les 151 sites d'allocation nues sous
`rust/crates` sont migrés au contrat `tempdir_in(<aire>)`/`new_in(<aire>)`
(RAII `TempDir` conservé, garde `tools/check_tmp_interdiction.py` étendu).
Répartition : swarm-engine 14 (aire de dépôt), swarm-web 6 (aire de dépôt),
swarm-cli 86 (aire de dépôt ; `version_no_repository.rs`, les 4 modules
identity `src/identity_{operator,provisioning,service,service_call}/tests.rs`,
les 7 suites d'intégration trust_enrollment + identity_*.rs, cli.rs,
paea_settlement et les 4 suites paea à déclaration (promote, transfer,
approval_web, settlement_delivered) → aire de repli : leur parcours lit
ancres/déclarations via `open_private_directory`, qui refuse les ancêtres
775 du checkout — corrigé après un premier passage à l'aire de dépôt,
7 rouges identity `--lib`, 8 rouges d'intégration (2 trust_enrollment +
6 identity_checkpoint) et 3 rouges cli/paea causés et retirés par les runs
de vérification), crates enrollment + linux-prerequisites
51 (aire de repli inline : le walk d'ancêtres de `open_private_directory`
refuse les ancêtres 775 du checkout, donc `~/.local/state/swarm-fixtures/`
0700 ; pas de dépendance swarm-engine chez les consumers, l'aire est répliquée
inline dans chaque crate). Sécurité git au dépôt établie : tout appel git
moteur épingle cwd+`--git-dir .` et efface `GIT_*` (managed_repository.rs:33) ;
seul `git_metadata.rs` contracte « pas de dépôt ancêtre » (aire de repli, D04).
Le garde passe de « tempfile sans `dir=` » à « toute allocation tempfile Rust
nue » : 0 violation, 10 items de revue (documentés/préexistants).

Vérification : `cargo check --tests` 0 erreur sur les 7 crates touchées
(engine/web/cli en défaut ; config/state/bootstrap/linux-prerequisites et
authority avec `--features test-fixtures`, convention des recettes projet —
les rouges plain `--tests` sur `install_with_hook`/`FixtureFault` sont
préexistants à HEAD, feature-gated, hors parcours de vérification) ;
`cargo test` config/state/bootstrap/linux-prerequisites 110 verts,
authority 207 verts, trust_enrollment 2 verts + identity_*.rs 37 verts
après correction d'aire. Docs : README de vérification FR/EN (forme
gouvernée + aire de repli), doc de module `fixture_area.rs` (D05). Preuves
de travail : `rust/.swarm-fixtures/d04-class2-files.txt`,
`d05-violations.txt`, `d05-guard-final.txt` (garde final : 0 violation),
`d05-cli-files.txt`, `d05-cli-final-noff.txt` (run intégral final). Recette
permanente `tools/verification/recipes/rust-class2-fixture-areas.json` (garde +
identity `--lib` + trust_enrollment + identity_checkpoint) : PASS 4/4,
vérification VERIFIED, preuve
`.swarm-fixtures/class2-recipe-20261108T030000Z` ; section dédiée dans
`tools/verification/README.md` (FR/EN).

Rouges swarm-engine `--lib` qualifiés **hors périmètre class-2** (sources des
tests et de leur chaîne identiques à HEAD, preuves diff vides) :
- `preparation_policy_explicit_defaults_legacy_and_unlimited` et
  `mission_report_settings_absence_is_not_default_adoption` : dérive v52
  (`schema_version()` retourne 52 via `has_web_access_policy`, test attendait
  41) — dette v52 consignée, vague Codex ;
- `captured_source_reuses_caller_transaction…`/`recorded_sources_authentication…`
  : exigent un TMPDIR épinglé (« recipe fixture area required ») — verts avec
  `TMPDIR=/home/fpizzi/swarm-run-tmp` (0700 hors dépôt, à intégrer aux
  recettes permanentes) ;
- `agent_supervision`/`web_session` : verts en isolation, flakes de
  concurrence (port éphémère / écritures process-wide) ;
- **`mission_status_reports_contained_scan_symlinks_refusal_recovery` et
  `current_review_report_and_artifact_staleness_recover` : dette datée du
  lot Biba 2496c031b (28 octobre)** — à l'origine du test (174b89106)
  `artifact_metadata` appelait `local_file` directement (source =
  `LocalPathError`, downcast OK) ; l'interposition `anchored_artifact::with`
  fait de `GateEvaluationError::source()` un `AnchoredArtifactError`, le
  downcast en `LocalPathError` (mission_status_reports.rs:1118) échoue et
  `semantic_artifact` ne reconnaît plus le refus (gate-evidence-invalid).
  Aucun rapport au changement d'aire de fixtures (root `.swarm-fixtures` vs
  `/tmp` n'entre pas dans le containment). Propriétaire : vague Codex Biba ;
  piste : projeter la source `LocalPathError` à travers
  `AnchoredArtifactError::Path` dans `anchored_error` ou downcaster la
  source transitive.

Vérification swarm-cli (TMPDIR gouverné ; cartographie du 7 novembre sans
`--no-fail-fast`, complétée le 8 par un run `--no-fail-fast` intégral) :
suites identity `--lib` 18 verts après correction d'aire (fallback requise —
cf. répartition ci-dessus) ; `agent_launch` 5 verts / 8 rouges = la dette
BT02 Biba cartographiée (workspace==root → `biba-workspace-overlaps-root`,
CLI mort avant READY), propriétaire Codex, dossier
BIBA-TRANSVERSE-APEX.md ; run final intégral : 258 verts / 9 rouges / 20
ignorés — les 9 = les 8 agent_launch + 1 planning
`agent-provider-catalogue-read` (dette « jamais vert », reconstruction au
commit de création 174b89106 ; preuve du rouge : `.swarm-fixtures/d05-cli-final-noff.txt`,
run final intégral ; le diagnostic de reconstruction relève du lot Codex —
aucun fichier de preuve vérifiable au 8 nov., propriétaire Codex) ; les 20 ignorés = gardes/fixtures canoniques des
recettes dédiées ; avec `SWARM_PAEA_SOURCE_FIXTURE` canonique, les 5 suites
paea ajoutent 29 verts. Les rouges cli.rs/paea_settlement du premier
passage intégral étaient introduits par le lot (aire de dépôt sous suites à
déclaration) puis retirés — trust_enrollment était vert à la cartographie
BF-16b du 7 novembre, les rouges d'intégration du premier passage aussi.


## Administration et qualification — 5 novembre 2026

Le plan [Sécurité et preuves](SECURITY-PROOFS-APEX.md), adopté par l’utilisateur,
complète BLP-01–07 : projection moteur/API/CLI, UX, administration, recettes
autorisées et dossier de qualification. Aucun état de conformité, test ou
acceptation BLP n’est changé par ce rattachement.
