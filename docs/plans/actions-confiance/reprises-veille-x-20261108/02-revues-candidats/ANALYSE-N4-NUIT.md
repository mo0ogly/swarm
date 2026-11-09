# N4 — rapprochement des revues et remise des changements propres

Lecture datée 2026-11-08T22:47:43.363221+00:00. Propriétaire : chat `01a1bbe3-cea2-7413-998e-6a27656acd69` ; arbitrage/intégration : coordinateur `01a1b540-a5ad-76c2-b0e5-ac4187f04fd4`, sous autorité de Fabrice.

Analyse et remise uniquement. Seul ce document est écrit atomiquement. Aucun code, test, build, installation, fournisseur, agent, index Git, commit, push ou autre plan modifié dans cet incrément. Les résultats d’exécution ci-dessous sont les preuves préexistantes, pas des vérifications relancées cette nuit. Méthode replan ; contrat commun relu. Facturation différée, aucun lot moteur nouveau derrière le push bloqué.

## Décision proposée au coordinateur

Ne pas recréer GI-T1–T4 ni Go364. **Aucun nouvel écart borné démontré par cette lecture.** Le besoin Gi.eth est déjà couvert dans le périmètre local défini ci-dessous ; Jackson/Efi ajoute la question de l’autorité sur une modification de CI, dont le raccord réel n’est pas établi ici. Ce manque de preuve ne démontre pas un défaut, ne justifie pas un nouveau lot automatique et ne permet pas d’annoncer une couverture complète.

L’intégration Git reste une décision du coordinateur : les chemins dirty partagés ne sont pas des unités de commit sûres. Aucun avis indépendant ni acceptation de notre propre travail n’est produit par cette remise.

## Sources, exigences et couverture

Sources historiques lues : `docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md` lignes656–673 (Gi.eth) et1025 (Jackson/Efi), `docs/plans/AGENT-CONTROL-POCS.md` POC-02 et complément2433, `tools/agent-workflows/CONTRACT.md` sections1–4. Sources X : [Gi.eth](https://x.com/Girlgym67/status/2119440940641476747), [Jackson](https://x.com/lyjackson/status/2119512430552228324), [Efi](https://x.com/eforus_overseer/status/2119513961821315375). Les liens sont des provenances conservées ; aucune nouvelle consultation X ni validation de leurs affirmations n’est revendiquée.

| Besoin documentaire | Code observé maintenant | Preuve d’exécution existante / portée |
| --- | --- | --- |
| Candidat constitué et dépendance effective inventoriée | `input_inventory.rs` collecte les fichiers réguliers initiaux de la copie, y compris non déclarés ; préimage privée schéma2 dans `managed_execution.rs`. Relecture de la sélection retenue, distincte des sorties générées. | GI-T1/T2 : consommateur réel `python3 security-consumer.py`, configuration initialement non suivie/non déclarée/non fournie au reviewer. Témoin V1 publié ; V2 dangereux A+B accepté à tort sous R1 avant correction ; onze entrées suivies identiques. FAIL original conservé. |
| Contexte indépendant, verdict et preuve applicables au candidat intégré | `review_candidate.rs::preview` rattache la source observée au job original ; relecture avant revue/publication ; erreurs d’observation conservées. | GI-T3/T4 `run-03` et `flow-03` PASS : V2 dangereux refuse sous R1 ; variante sûre A+C encore refusée sous R1. La fixture utilise un reviewer inerte, pas une expertise sécurité indépendante. |
| Ancien verdict conservé, mais insuffisant ; reprise | `review_refresh.rs::refresh_task_review` : révision attendue, conditions strictes, archivage dans `previous_reviews`, invalidation des contrôles/revue courants, rejeu sans nouvel appel. | Variante sûre : nouveaux contrôles/R2 distinct, publication/rejeu et réouverture. Le V2 dangereux échoue aux contrôles et n’est pas accepté sous R2. API mauvaise révision/champ inconnu et annulation SQL conservées ; legacy schéma1 renouvelé explicitement. |
| Jackson : arbre/test/workflow changé après verdict | L’inventaire local couvre les octets initiaux concernés lorsqu’ils appartiennent à cette copie ; il ne constitue pas une autorité administrative CI. | Aucune campagne Jackson spécifique ni preuve d’interdiction de modifier une CI externe. Ne pas déduire droits de branche, identité d’un service CI ou contrôle d’un workflow externe d’un simple hash. |
| Anciennes preuves Go lisibles sans fausse qualification | `work_projection.rs` laisse un avis non favorable non porté explicitement non qualifié par tâche, sans consommer son reçu de remplacement ; avis favorable batch/fragment toujours refusé. `managed_review_git.rs` utilise la même association physique privée et lane que361 pour les alias historiques. | Go364 : 10 cas nommés PASS ; altération/absence/refus/reprise ; alias du même stockage courant, autre datastore même candidat refusé, lane occupée/alias/Git absents refusés puis restauration. Aucune opinion inconnue promue en PASS. |

Limite fondamentale GI : inventaire initial local, **pas graphe hermétique** des programmes externes, environnement, IPC, services réseau ou dépendances créées ultérieurement. Un changement effectif sans octet local inventorié changé n’est donc pas universellement couvert. Une recette future demanderait un consommateur concret et un changement discriminant documenté ; pas un nouveau test générique sur simple rapprochement lexical.

## Candidats et preuve finale réutilisable

- GI-T1/T2 : `docs/plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md` section284 ; `.cache/gieth-v1-v2-20261108`, originaux FAIL préservés. Binaire de reproduction `84b1e78c8dbd9f38c5ebebb7fd48c097f6e7de76805249fd06592730aa5211a8`.
- GI-T3/T4 : même suivi section431 ; `.cache/gieth-fix-20261108/run-03/result.json` PASS/source_stable=true et `flow-03/result.json` PASS relus ; `legacy-run/result.json` PASS/source_stable=true. Recettes maintenues `gieth-review-refresh.json` et `gieth-legacy-refresh.json`. Livraison347 `3512fdaed47a9f111959de2cfd8186276d1d32d073da510e0dd8e86913b0735f`, historique distinct de364.
- Lecture historique361 : `docs/reports/historical-mission-read-20261108.md`, recette `historical-mission-recorded-read.json`, `.cache/historical-mission-read-20261108.HANDOFF.json` (remise361 seulement). SHA361 `98d32027dae6d285f9607767e806471125a4838392c72723d119ce626f72b3c9`. Cette livraison ne réparait pas la qualification Go.
- Final364 : `docs/reports/historical-go-qualification-20261108.md` section finale ; `.cache/historical-go-qualification-20261108/source-review-final-364.json` : **16/16 empreintes concordent à cette lecture**. `recipe-sdk-03/result.json` relu : PASS, source_stable=true, failure=null. Dix cas nommés selon recette maintenue `historical-go-qualification.json` (groupe existant7 + trois témoins). Les SDK sont distincts du profil livré ; revue de même session seulement.
- Record final : `.cache/historical-go-qualification-20261108/delivery-record-03.json` ; profil `.cache/local-delivery/rust-s4i7pgmg/build/profile/result.json` relu `VERIFIED_DEFAULT_NATIVE_PROFILE`, inventaire initial787 et effectif782 entrées. Les fichiers GI `input_inventory.rs` et `review_refresh.rs` sont présents dans l’inventaire effectif. Leur présence et leurs octets ne constituent pas un nouveau rejeu fonctionnel GI sur364.
- Image relue `/proc/2171843/exe` → `/home/fpizzi/Documents/ChatGPT/swarm/bin/swarm` ; SHA `aed1422210080ceae550cdc7133f242134bfa4eb0b7088a44c40b37d3c9414f8`, version364 selon record. HEAD `d084c1487726f0d240de84d521d19d14d7938250`, branche `main`, checkout dirty. Aucun nouveau commit de notre travail. Le commit de base ne contient pas implicitement nos changements non commités.
- Résultats réels déjà conservés : `final-show-fr.stdout`, `final-show-en.stdout`, `browser-observations-final.json` : résumé calculé Reprise/OODA FR/EN, une acceptation historique courante T1, T2 unknown non qualifiée, huit tâches, mission blocked. Aucun lancement/reprise d’agent ou nouvelle acceptation ; 49 tables et Work/manifestes inchangés. T1 périmée sous362 était encore l’erreur lexicale d’alias, pas une conclusion finale à reconduire.

Les étapes362 (installée) et363 (préparée non installée) restent historiques. Le statut opérationnel global et les protocoles favorables batch/fragment non portés restent refusés. `protocol_verified` et `authority_verified` restent faux dans l’observation Git ; concordance de données ≠ authentification indépendante. La double erreur callback/fermeture est revue dans le code via Root Cleanup, sans injection noyau revendiquée.

## Conservation et ressources

Remise finale `.cache/historical-go-qualification-20261108.HANDOFF.json`, index `.cache/historical-go-qualification-20261108.LOCATION-final.json`. Archive finale USB conservée `/media/fpizzi/Lexar/swarm-evidence/historical-go-qualification-20261108-evidence-02.tar.gz`, SHA `6c49f44f8cb33b8eee780fab97cd8b317504996259aa9b4f6c6183617d4ada0b` ; 721 fichiers réguliers relus et trois recettes restaurées/vérifiées selon la preuve de conservation antérieure. Pas de nouvelle recopie, lecture intégrale d’archive ou vérification de recette dans cette analyse. Les index des lots GI/badges/361 et les rapports donnent leurs propres archives et commandes de restauration ; ne pas remplacer leurs preuves par l’archive finale364.

Nos compilations, SDK et serveurs de fixture sont libérés selon la remise finale ; aucun n’est lancé par cet incrément. Seul notre serveur local installé364 reste actif, image relue ci-dessus ; aucun arrêt demandé. Fixtures, SDK de reproduction et originaux d’échec restent conservés sur disque/USB. Aucun verrou de build pris ou retenu par notre analyse. Cela ne qualifie **ni l’activité ni les ressources d’autres chats/processus** ; aucune annonce d’inactivité globale.

## Découpage de remise proposé, sans index ni commit

1. **GI** : modules inventaire/refresh + branchement CLI/exports + producteurs/consommateurs + tests/recettes + contrats FR/EN, suivi et notes GI. Dépendance atomique du schéma2 et de ses lecteurs : ne pas intégrer un lecteur sans le producteur ni le renouvellement explicite des anciennes preuves.
2. **Focus348** : interception Échap modal dans `swarm-shell.js`, garde nommée, recette et documentation/notes FR/EN. Indépendant des badges ; ne pas réintroduire un ancien shell complet pour extraire ces quatre lignes.
3. **Badges349/357/358** : CSS sémantique KPI puis helper `statusBadge` partagé des tâches, helper agents distinct et consommateurs. Préserver les composants/design préexistants. Tests tâches précèdent/régressent ceux des agents ; fixture et ressources associées font partie du lot agents. Ce lot ne répare ni lecture ni qualification.
4. **Lecture361** : association physique de stockage + lecteurs managed + double protocole de verrous Go/Rust + tests ; UI historique enregistrée explicite + rendu original texte + styles/traductions + recette/docs. Les trois fichiers Rust se composent ; l’UI distingue historique et statut actuel. Aucune réécriture des locators stockés ni nouvel état opérationnel.
5. **Qualification364** : projection par tâche et Git sur alias + tests/recette/docs/notes. Dépend du garde commun361. Dans `managed_repository.rs`, seul le re-export `with_bound_storage` est nouveau pour364 ; dans `managed_repository_worker.rs`, visibilité `pub(crate)` et contrat commenté de `with_storage`, corps provenant de361. Dans `root_store.rs`, Rustdoc de `inspect_work_validation` pour364, export de `ReviewRefreshOutcome` pour GI. Dans UI361, seule correction du bandeau « statut opérationnel non évalué » et traduction correspondante relève de364.

Le coordinateur choisit regroupement/ordre des commits selon candidat et preuves applicables. Les recettes historiques n’autorisent pas à qualifier un assemblage différent sans vérifier sa pertinence. Après tout changement de ces octets, la concordance calculée ci-dessous devient historique ; aucun commit/push n’est effectué ici.

## Sélection des hunks partagés et exclusions

- `web/swarm-workbench.js` : `signal`, `statusBadge`, `agentStatusBadge`, usages dans inspecteur/couloirs et exports = badges. La structure du workbench préexiste ; ne pas l’attribuer intégralement à ce chat.
- `web/cockpit-native-read.js` : `taskRow` appelle `statusBadge` = tâches ; branche `readMode`, `renderConduite`, `.history-task`/JSON `textContent`, compteur non observé et propagation argument = lecture361.
- `web/cockpit-native.js` : sélection `read=history/current`, bouton `mission-read-mode`, absence d’appel opérationnel en historique, `showView` =361 ; libellé du bandeau seulement =364.
- `web/i18n-en.js` : libellés agents =358 ; cinq clés de lecture historique =361 ; remplacer la clé de qualification par « statut opérationnel » et traduction =364. Sélectionner les entrées, pas une ancienne copie du catalogue.
- `web/swarm-design.css` : règles de pastilles `.metric strong/.mission-kpi strong` et paires sémantiques + variantes `.workbench-badge` = badges. Ne pas reprendre les deux tokens de design déjà intégrés à HEAD comme modification propre.
- `docs/UI-DESIGN.md` / `docs/en/WINDOW-IDENTITY.md` : paragraphes focus et badges seulement ; préserver liens/design préexistants et tout autre propriétaire.
- `docs/plans/AGENT-CONTROL-POCS.md` : le diff courant à ligne2009 (EN1677) est notre correction historique de la limite Échap347 vers348. **Exclure** du lot de ce chat les premières lignes de qualification native et les longs ajouts finaux lot15/BF-L0-A/lot16 : ils sont partagés et n’appartiennent pas à ce correctif. La section GI antérieure existe dans la base ; la citer ne veut pas dire la recommitter.
- `version_history.json` : sélectionner nos notes GI/focus/KPI/tâches/agents/lecture/qualification FR/EN ; conserver les notes des autres propriétaires. Un numéro d’image préparée n’est pas un tag publié. Ne pas annoncer un changement exclu du candidat.
- **Exclure** les suivis DESIGN-INTEGRATION, TECHONLINENEWS, FILE-APEX, REPRISES-PAR-LOTS, dossiers mémoire/deepagents, contrôles agents natifs/réels, PAEA, providers, site-publication, outils BUILD-STORAGE/BUILD-VOLUME/BF-L0-A/codex_ipc et leurs recettes. Leur présence dirty ne prouve pas notre propriété. Aucun fichier de coordination partagé n’est modifié par cette remise.

## Inventaire exact au moment de la remise

Les SHA ci-dessous portent le **fichier complet courant**, pas un certificat de propriété de tous ses octets. Les sections/hunks sélectionnables sont délimités ci-dessus ; les en-têtes de diff sont des localisateurs contre HEAD actuel, pas des patches à appliquer seuls. Une même entrée apparaît dans plusieurs groupes lorsqu’elle porte leurs dépendances. « effectif364=oui » signifie hash égal au record effectif livré, pas exécution du test correspondant. Les autres fichiers sont sources de test/docs/recettes non prouvées embarquées par ce champ.

### GI — candidat, inventaire et renouvellement

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `rust/crates/swarm-cli/src/lib.rs` | `f81df335d6576900f8adb18f38f89cb01c2b2b3ec8a8494c5d2b3c127452803b` | oui |
| `rust/crates/swarm-cli/src/task_validation.rs` | `8d5182b0d9fc9c870480a7ea353f4ad2cd5024b6cc256d07f30d8b2976c32546` | oui |
| `rust/crates/swarm-engine/src/lib.rs` | `b0b69f24dbd63654c1d204e27da707547ecb32d274492ba9464ef7ea87db5210` | oui |
| `rust/crates/swarm-engine/src/root_store.rs` | `49ba554065187ac1d097c6fd19fe005c8f3f05b6dcfdb73d58ae4d927a31cbcc` | oui |
| `rust/crates/swarm-engine/src/root_store/task_validation.rs` | `f4a1dce53a5cbd9314cb676a17b09635754a78648970d9ca5e0925ce6afa6cae` | oui |
| `rust/crates/swarm-engine/src/root_store/task_validation/managed_execution.rs` | `68af46019492648d54e4b0e6291980e01ef925dc87a43330cba45d34d369bbcb` | oui |
| `rust/crates/swarm-engine/src/root_store/task_validation/review_candidate.rs` | `d805fac095ee2b84142939560cc961ebf0132bccb8c6e5d8ca5f18352b8463d2` | oui |
| `rust/crates/swarm-engine/src/root_store/task_validation/input_inventory.rs` | `ec5a5e9ff93502ed5ca9c8a02c60e6cddaaa60bbfb30155339324b14f160e0c5` | oui |
| `rust/crates/swarm-engine/src/root_store/task_validation/review_refresh.rs` | `bc96e376b9e7e504d50aae9a2a9c08044650d6bb6b843d94a5e6241e5095fbf2` | oui |
| `tests/rust_native_managed_review_inputs.py` | `352b02df62ee69b8f4479f8b9940ef0ca31c40bbf9caa6acdad706670386a142` | hors champ |
| `tests/rust_gieth_review_refresh.py` | `29f2e638844a23c1c21af34df548e767d97f41601b9037f2865b4186d6f5f768` | hors champ |
| `tools/verification/recipes/gieth-review-refresh.json` | `e1d31850388a8f9c700f18961955b867ffebfb51402e9ae0b412e13496e74c50` | hors champ |
| `tools/verification/recipes/gieth-legacy-refresh.json` | `670105b673a69d89b8263d234adfe93e2dca5ed372278648e514baa84de160d7` | hors champ |
| `docs/architecture/NATIVE-REVIEWER.md` | `671353bfd4e34ee10496fff69dfaf4ba9f2fe80559a5fff286445d3f8e4ea733` | hors champ |
| `docs/en/NATIVE-REVIEWER.md` | `89c9f9b493c3fd40597eef2529477eff792c406316a28eb75cb8299ce4815d4c` | hors champ |
| `docs/plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md` | `13065b01239967b80faafb3a1b527dc48bd51ab2e83bf49b5d808a5396bfea1c` | hors champ |

### Version348 — retour du focus

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `web/swarm-shell.js` | `593f179e1521ae53867a7a5771e4ffeedf17bcafdbe78a1a794ff6d28b0b3b44` | oui |
| `tests/swarm_shell_escape_test.cjs` | `a2c68b258d25bfa797b3d5b6621d591b6246761209249535194b39d230f5655f` | hors champ |
| `tools/verification/recipes/version-focus-escape.json` | `4d88175520e2bfe3269c39a28084810d7c633dde71179560458132d3b22661ba` | hors champ |

### Badges349/357/358 — avant les corrections historiques

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `web/swarm-design.css` | `2db8b9b99e97c28706dee90870d102e6914e423926bb55ab4854aa69a60d43b8` | oui |
| `web/swarm-workbench.js` | `fc75e69292044e414f29fbbbefa30e3d683c02ce6a420aee17ff09bb8c1c4a15` | oui |
| `web/cockpit-native-read.js` | `a42912994d18b036509e975463f54488e7511f43859f6c4f58c42967e8ef8f10` | oui |
| `web/cockpit-native-panels.js` | `1cb55f37c7647aa975d15a3b980e37ebcf168744ee35d9f9c43bc5c40e854848` | oui |
| `web/i18n-en.js` | `88e0d631fe67fa6ce016c43ec2dc7365d47b930645c241fb96f22fae8e9b2d8b` | oui |
| `tests/task_status_badges_test.cjs` | `427f695a7fc5a2720a21080104b12219e7152f970f4244f5028ebe94939c724f` | hors champ |
| `tests/agent_status_badges_test.cjs` | `83e221d543e1e384b0eef46aeadc301c079b5909be357df4217c12c76b3e2f70` | hors champ |
| `tests/agent_status_badges_fixture.py` | `2494b7061a594b7000bcda88bc6d7d9e74e29bfa30af51ae927c437d78bc8f39` | hors champ |
| `tools/verification/recipes/task-status-badges.json` | `34643b0cdb521b7cb2c15d8c1832e96bb23d063168712e673dec58e1b7d8d6db` | hors champ |
| `tools/verification/recipes/agent-status-badges.json` | `6cb40b4c7e4f1fefbe1357909aa57b5bd7f906ce0e2150b8f0252e01abe9a029` | hors champ |
| `docs/reports/kpi-badges-20261108.md` | `01358e45b828a7411fe6f3ed45a37425571d48d54cb99f6dd9997f2ddd867faf` | hors champ |
| `docs/reports/task-status-badges-20261108.md` | `f195dc2d93006ca0efdca4a0895aee080b81b5e5ec07419877933587e7cf9516` | hors champ |
| `docs/reports/agent-status-badges-20261108.md` | `fc21d06ba747d1d9ada968c8dcfdb08b5683e42fcfb139ccf9d26b5664166e21` | hors champ |
| `tests/fixtures/agent-status-badges/fixture.js` | `7d7ff666ac37a4d2eac4ecfd77103897ad3b9df7b9bc021dfe0c8e5b30b5c419` | hors champ |
| `tests/fixtures/agent-status-badges/index.html` | `471b575e2a344e7cff296bce6f6da852a71e22809affb378a4ad9cea8a623dc7` | hors champ |

### Lecture361 — stockage physique et historique enregistré

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `rust/crates/swarm-engine/src/local_storage.rs` | `d1d19a830612dc78bb140877a110a99826a69025ec8ac309cb54f90687510c1b` | oui |
| `rust/crates/swarm-engine/src/managed_repository.rs` | `942a233dbe07fe2019dba9bb618299952172addb250d6690110ffecb08a1a95c` | oui |
| `rust/crates/swarm-engine/src/managed_repository_worker.rs` | `e6390bcf15c5f07e3629c98c54a5fbe3e6180f87149254e51fcf38cddd87e61c` | oui |
| `web/cockpit-native.js` | `5f399ee00f0a19ec322c888d07e08e2cae3c85635fb2ed750fae95dbb2deba6c` | oui |
| `web/cockpit-native-read.js` | `a42912994d18b036509e975463f54488e7511f43859f6c4f58c42967e8ef8f10` | oui |
| `web/swarm-workbench.css` | `f49ba198cc7861a2f4cf604fe46d036c7f98dfdafe5568cd109e12885689247f` | oui |
| `web/i18n-en.js` | `88e0d631fe67fa6ce016c43ec2dc7365d47b930645c241fb96f22fae8e9b2d8b` | oui |
| `tests/historical_mission_read_ui.cjs` | `2c1e54efe29b1753e7c5e5bd2e82883909a5efb5385f75fd92d3aa08d5437ca9` | hors champ |
| `tools/verification/recipes/historical-mission-recorded-read.json` | `fe721f1c056400de488ab8518cd535c3700566c71f8b146148735f3cde4e20b2` | hors champ |
| `docs/architecture/NATIVE-WORKER-RECOVERY.md` | `252a3c1b3eba783a8a8debf1af32d6ca71d35790a0056079790acfc842aa5af8` | hors champ |
| `docs/en/NATIVE-WORKER-RECOVERY.md` | `699bb12b76b972c1b931ae5d19804ac3ecdf4b9e905f7f6c0e1c4cc5b6905097` | hors champ |
| `docs/reports/historical-mission-read-20261108.md` | `23df77c85cca8e38a28947304111e512946e2f1046a37ddd3a567769c192c6aa` | hors champ |

### Qualification364 — portée du refus et Git sur alias

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `rust/crates/swarm-engine/src/work_projection.rs` | `0db26b3e5ff8851bc5099523904a5cadd3b47330bc9b1346a6b0ab5be293cf55` | oui |
| `rust/crates/swarm-engine/src/root_store.rs` | `49ba554065187ac1d097c6fd19fe005c8f3f05b6dcfdb73d58ae4d927a31cbcc` | oui |
| `rust/crates/swarm-engine/tests/work_projection.rs` | `234fad4e03ef482b3db6d8c9352de600df1ef3ccb9df3eb4ed86849e3b110b65` | hors champ |
| `rust/crates/swarm-engine/tests/root_review_evidence.rs` | `253ab1b44d5bb47d42ea887f96043bbd30319023442333e09c8c492b5ffdc27a` | hors champ |
| `docs/architecture/RUST-MIGRATION.md` | `9b170f677c36cb877175f2990df45010565dc432b745346422698fa37b8b69fb` | hors champ |
| `docs/en/RUST-MIGRATION.md` | `e2de0b1c40a341b2726ee29a04c9d03c90025b40bce5fd8624e71bb4cc0fc28f` | hors champ |
| `docs/architecture/NATIVE-WORKER-RECOVERY.md` | `252a3c1b3eba783a8a8debf1af32d6ca71d35790a0056079790acfc842aa5af8` | hors champ |
| `docs/en/NATIVE-WORKER-RECOVERY.md` | `699bb12b76b972c1b931ae5d19804ac3ecdf4b9e905f7f6c0e1c4cc5b6905097` | hors champ |
| `version_history.json` | `419fa54e5d9cb9b8b04d329267cf699791f6f3137fce17a33cfbdaf778c78d18` | oui |
| `tools/verification/recipes/historical-go-qualification.json` | `a750c6350952f0ba4dfd4ea3deebd45e038d995a3a7ca472ba411c93bcb01d70` | hors champ |
| `docs/reports/historical-go-qualification-20261108.md` | `ec4773979baef83a4d49484ab05ac56d5af4cfbd531f7abe90cdca3af098b977` | hors champ |
| `rust/crates/swarm-engine/src/managed_review_git.rs` | `d585fc360783a26f07f35966250a3d2496e565edadce3d4426b7bff24075025e` | oui |
| `rust/crates/swarm-engine/src/managed_repository.rs` | `942a233dbe07fe2019dba9bb618299952172addb250d6690110ffecb08a1a95c` | oui |
| `rust/crates/swarm-engine/src/managed_repository_worker.rs` | `e6390bcf15c5f07e3629c98c54a5fbe3e6180f87149254e51fcf38cddd87e61c` | oui |
| `web/cockpit-native.js` | `5f399ee00f0a19ec322c888d07e08e2cae3c85635fb2ed750fae95dbb2deba6c` | oui |
| `web/i18n-en.js` | `88e0d631fe67fa6ce016c43ec2dc7365d47b930645c241fb96f22fae8e9b2d8b` | oui |

### Documentation et registre partagés — sélection de sections seulement

| Chemin | SHA-256 courant | Effectif364 |
| --- | --- | --- |
| `docs/UI-DESIGN.md` | `5754b02b24e742e4a38069180c2c000cf8222e296fc4116a2b0d4272e924a604` | hors champ |
| `docs/en/WINDOW-IDENTITY.md` | `e335248923cc4c69d402d997af991dc7f98550bfc538eba57fbb27a98d3a1532` | hors champ |
| `docs/plans/AGENT-CONTROL-POCS.md` | `4b2484434c03b4f45ed425e8b7440159a151c6038b90ca381d0731b09b06286e` | hors champ |
| `docs/en/plans/AGENT-CONTROL-POCS.md` | `266687de8e5da979c801f5b9511e5af22bad46344dc48f51a837a714185f54e5` | hors champ |
| `version_history.json` | `419fa54e5d9cb9b8b04d329267cf699791f6f3137fce17a33cfbdaf778c78d18` | oui |

### Localisateurs des diffs suivis

```text
rust/crates/swarm-cli/src/lib.rs
@@ -35 +35 @@ const USAGE:&str="swarm-rust init --root ROOT | work list --json --root ROOT | w
@@ -892 +892 @@ fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
rust/crates/swarm-cli/src/task_validation.rs
@@ -12,0 +13,2 @@ use std::io::{self, Write};
@@ -36 +38 @@ pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError
@@ -58,0 +61,5 @@ pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError
rust/crates/swarm-engine/src/lib.rs
@@ -106,0 +107 @@ pub use root_store::{
rust/crates/swarm-engine/src/root_store.rs
@@ -67 +67 @@ pub use planning_step::PlanningStepOutcome;
@@ -2486 +2486,4 @@ impl RootStore {
rust/crates/swarm-engine/src/root_store/task_validation.rs
@@ -9,0 +10,3 @@ mod managed_execution;
rust/crates/swarm-engine/src/root_store/task_validation/managed_execution.rs
@@ -65,0 +66 @@ impl Session {
@@ -85 +86,2 @@ impl Session {
@@ -92,0 +95,2 @@ struct Frame {
@@ -201 +205,3 @@ fn relative(root: &Path, path: &Path) -> Result<String, E> {
@@ -209,0 +216 @@ fn capture(
@@ -224 +231,9 @@ fn capture(
@@ -270,0 +286 @@ fn acquire(
@@ -318,0 +335 @@ fn acquire(
@@ -336 +353 @@ pub(super) fn acquire_fresh(
@@ -354 +371,2 @@ pub(super) fn acquire_job(
@@ -363 +381,2 @@ pub(super) fn acquire_for_review(store:&RootStore,job:&store::Job,candidate:&Man
@@ -372 +391 @@ pub(super) fn persist(store: &RootStore, id: &str, session: &Session) -> Result<
@@ -374,0 +394 @@ pub(super) fn persist(store: &RootStore, id: &str, session: &Session) -> Result<
@@ -391 +411 @@ pub(super) fn for_helper(store: &RootStore, job: &store::Job) -> Result<Option<S
@@ -400,0 +421,17 @@ pub(super) fn for_helper(store: &RootStore, job: &store::Job) -> Result<Option<S
rust/crates/swarm-engine/src/root_store/task_validation/review_candidate.rs
@@ -16,2 +16,3 @@ impl ReviewCandidate {
@@ -21 +22,4 @@ impl ReviewCandidate {
tests/rust_native_managed_review_inputs.py
@@ -29 +29,2 @@ def run(binary,profile,out,*,reuse=None,before=False,binary_source=False,context
@@ -120 +121 @@ def run(binary,profile,out,*,reuse=None,before=False,binary_source=False,context
docs/architecture/NATIVE-REVIEWER.md
@@ -1305,0 +1306,35 @@ une autorisation PAEA courante ou un effet Git protégé.
docs/en/NATIVE-REVIEWER.md
@@ -1243,0 +1244,31 @@ current PAEA authority or protected Git effect.
docs/plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md
@@ -3,7 +3,8 @@
@@ -395,0 +397,214 @@ leurs résultats. Le rapport final et ses empreintes de clôture sont aussi copi
web/swarm-shell.js
@@ -55,0 +56,3 @@
@@ -56,0 +60 @@
web/swarm-design.css
@@ -188,0 +189,9 @@ button:disabled { opacity: .6; }
@@ -194,0 +204,3 @@ button:disabled { opacity: .6; }
web/swarm-workbench.js
@@ -11 +11 @@
@@ -15 +15,15 @@
@@ -21 +35 @@
@@ -30 +44 @@
@@ -44 +58 @@
web/cockpit-native-read.js
@@ -37 +37,3 @@
@@ -51 +53 @@
@@ -53 +55,2 @@
@@ -61 +64,4 @@
@@ -63 +69 @@
@@ -68 +74,17 @@
@@ -88 +110,3 @@
@@ -98 +122,4 @@
@@ -135 +162 @@
web/cockpit-native-panels.js
@@ -160,2 +160,3 @@
@@ -174 +175,3 @@
web/i18n-en.js
@@ -7009 +7009,6 @@ globalThis.SwarmEnglish = {
rust/crates/swarm-engine/src/local_storage.rs
@@ -778,0 +779,46 @@ impl LocalDirectory {
@@ -781 +827 @@ impl LocalDirectory {
@@ -959,0 +1006,18 @@ pub(crate) struct LocalReservation {
@@ -986,0 +1051,37 @@ mod binding_tests {
rust/crates/swarm-engine/src/managed_repository.rs
@@ -19,0 +20 @@ pub(crate) use worker::{candidate_worker, clean_lane_contention, prepare_worker,
@@ -131,0 +133,54 @@ fn inside<T>(
@@ -274 +329 @@ pub(crate) fn prepare(
@@ -448,3 +502,0 @@ pub(crate) fn observe(
@@ -464 +516 @@ pub(crate) fn observe(
@@ -466 +518 @@ pub(crate) fn observe(
@@ -498 +550 @@ pub(crate) fn observe(
@@ -509,0 +562,58 @@ mod tests {
rust/crates/swarm-engine/src/managed_repository_worker.rs
@@ -34,3 +34,9 @@ struct Proof {
@@ -49,3 +55 @@ fn with_storage<T>(
@@ -67 +71 @@ fn with_storage<T>(
@@ -69 +73 @@ fn with_storage<T>(
@@ -76 +80 @@ fn with_storage<T>(
web/cockpit-native.js
@@ -34 +34 @@
@@ -54 +54 @@
@@ -56,0 +57,6 @@
@@ -59 +65 @@
@@ -62 +68 @@
@@ -121,0 +128,2 @@
web/swarm-workbench.css
@@ -19,0 +20,3 @@
docs/architecture/NATIVE-WORKER-RECOVERY.md
@@ -344,0 +345,74 @@ revue indépendante ou acceptation globale :
docs/en/NATIVE-WORKER-RECOVERY.md
@@ -327,0 +328,67 @@ verification grants no installation, independent approval or global acceptance:
rust/crates/swarm-engine/src/work_projection.rs
@@ -179,0 +180,15 @@ struct EvidenceContext<'a> {
@@ -243 +258,6 @@ fn evidence(
@@ -462 +482,3 @@ fn dependency_freshness<'a>(
@@ -487,0 +510 @@ pub fn project_work_validation(
rust/crates/swarm-engine/tests/work_projection.rs
@@ -429,0 +430,49 @@ fn stored_human_control_freshness_rechecks_saved_policy_and_real_artifacts() {
rust/crates/swarm-engine/tests/root_review_evidence.rs
@@ -980,0 +981,73 @@ fn root_managed_projection_tracks_current_review_and_preserves_io_refusal() {
docs/architecture/RUST-MIGRATION.md
@@ -1322,2 +1322,5 @@ répare pas le garde de fraîcheur, qui exige toujours l'enregistrement original
@@ -1331 +1334,3 @@ flowchart LR
docs/en/RUST-MIGRATION.md
@@ -1259,3 +1259,5 @@ This fallback is display data only: it does not change Work or repair freshness,
@@ -1269 +1271,3 @@ flowchart LR
version_history.json
@@ -8,0 +9,20 @@
@@ -20,0 +41,12 @@
rust/crates/swarm-engine/src/managed_review_git.rs
@@ -7,0 +8,32 @@ use std::path::Path;
@@ -26 +58,5 @@ pub struct ManagedReviewGitObservation {
@@ -74,6 +110,26 @@ pub fn inspect_managed_review_git(
@@ -81,15 +137,10 @@ pub fn inspect_managed_review_git(
@@ -97,9 +147,0 @@ pub fn inspect_managed_review_git(
docs/UI-DESIGN.md
@@ -60,0 +61,6 @@ sur leur saisie ; Échap et les fermetures rendent le focus au déclencheur.
@@ -279,0 +286,16 @@ et [en anglais](en/RUST-MIGRATION.md#native-cockpit--ux-journey24october2026).
@@ -285,0 +308,20 @@ les références visuelles ; leurs valeurs et verdicts d’exemple ne sont pas l
docs/en/WINDOW-IDENTITY.md
@@ -30,0 +31,6 @@ Forms keep native input focus. Escape and close actions restore the opener focus
@@ -57,0 +64,31 @@ existing unrelated edits; it never renews historical acceptance.
docs/plans/AGENT-CONTROL-POCS.md
@@ -5,3 +5,6 @@ Qualification avec vrai modèle demandée le 8 novembre 2026 :
@@ -2006 +2009,2 @@ Profil normal, identité active et dialogue Version FR/EN vérifiés. Limite ouv
@@ -2424,0 +2429,415 @@ leur résultat courant et la décision de fusion sont consultables dans PR #20.
docs/en/plans/AGENT-CONTROL-POCS.md
@@ -5,3 +5,6 @@ Real-model qualification requested on 8 November 2026:
@@ -1674 +1677,2 @@ profile, active binary identity and actual FR/EN Version dialog verified. Escape
@@ -2053,0 +2058,391 @@ retains their current outcome and the merge decision.
```

## Fin de la remise

Options comparées : refaire un lot générique GI/Jackson, ou réutiliser les témoins et isoler seulement un consommateur manquant démontré. La seconde évite le doublon et préserve les limites : aucune variante d’autorité CI effective n’est démontrée manquante par cette analyse, donc pas de développement proposé maintenant. Si le coordinateur retient cette question plus tard : désigner le vrai producteur/validateur/consommateur CI et son propriétaire, puis témoin V1 autorisé, modification V2 sans autorité refusée sans mutation, restauration et nouvelle décision applicable. Ces étapes restent proposées, non exécutées.

Les critères GI dans leur portée et Go364 ne sont pas rouverts par proximité de vocabulaire. Intégration/publication indépendante, port des protocoles favorables et statut global restent des objets distincts avec leur propriétaire migration. Ressources remises ; arrêt de ce lot après ce document et la réponse dans notre chat, sans continuation implicite.
