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
| BLP-04 | Attribution par producteurs, contexte/mémoire/sorties et replays | 03 | PARTIEL — provenance privée v3, contrôle d’objet et diagnostic authentifié des sources API/CLI ; identité producteur, principal d’envoi et héritage ouverts |
| BLP-05 | Médiation réelle fichiers/réseau/fournisseurs/descendants, refus sans repli | 04 | À FAIRE |
| BLP-06 | Identité humaine établie, validation des octets/destinataires, nouvelle copie | 03–05 | À FAIRE |
| BLP-07 | Recettes, revue du candidat, livraison locale, ressources servies et RETEX | 01–06 | EN COURS (incréments ciblés vérifiés ; installé v0.1.1-dev.296 ; recette globale et RETEX transversal ouverts) |

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
