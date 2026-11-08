# Articles Swarm et état réel — audit du 3 novembre 2026

Demande : comparer les articles lus sur LinkedIn à l'implémentation et aux
preuves réellement disponibles. Méthodes `audit-pdca` en lecture seule et
`replan`. Aucun changement produit, compte, mission, politique ou service.
Ce rapport français est une observation datée, pas un changement de contrat ;
aucune traduction de contrat ni nouvelle note de livraison n'est nécessaire.

## Conclusion et périmètre

Le positionnement des articles est cohérent avec les composants examinés,
surtout lorsqu'il décrit un objectif et conserve les limites. Le moteur Rust
et plusieurs contrôles d'effet sont réels. La continuité globale identité →
mémoire → délégation → outil → effet → preuve n'est pas établie. Les articles
ne revendiquent pas cette qualification globale : il faut préserver leurs
réserves, y compris lorsque les figures sont réutilisées seules.

Comparaison actuelle, pas reconstitution du code exact au jour de publication.
Les PDF PAEA joints aux posts n'ont pas été lus intégralement pendant cet audit :
les annonces de ces pièces ne valent pas audit de leur contenu ni des 17 invariants.
Les références externes citées par les articles n'ont pas été réauditées.

Articles lus dans le navigateur lors du tour précédent :

- A1, [5 octobre : processus critiques](https://www.linkedin.com/pulse/agents-ia-et-processus-critiques-pourquoi-un-moteur-fabrice-pizzi-vtj6e/).
- A2, [9 octobre : moteur déterministe](https://www.linkedin.com/pulse/agents-ia-pourquoi-jai-plac%C3%A9-un-moteur-d%C3%A9terministe-au-fabrice-pizzi-elb1c/).
- A3, [23 octobre : identité et mémoire](https://www.linkedin.com/pulse/identit%C3%A9-et-m%C3%A9moire-des-agents-ia-dans-swarm-fabrice-pizzi-5ieee/).
- A4, [25 octobre : délégation et PAEA](https://www.linkedin.com/pulse/ia-agentique-jusquo%C3%B9-peut-on-d%C3%A9l%C3%A9guer-sans-perdre-le-contr%C3%B4le-pizzi-vluye/).

## Base observée directement

- Checkout : `/home/fpizzi/Documents/ChatGPT/swarm`, HEAD
  `562953dba3f8d6eaabca7beda35535b31a9390fa`, arbre modifié et fichiers non suivis.
- `./swarm.sh status` : moteur Rust prêt, PID 1426548, port interne 38434,
  version `v0.1.1-dev.316`, `modified:true`, build `2026-11-02T19:35:26Z`.
- `sha256sum bin/swarm /proc/1426548/exe` : même SHA256
  `e2f0039a1f3a34957c161e770c4e5efdd46f4e3a16fc82fe9cf5d9761dc94da3`.
- Démon `swarm-authority` actif PID 3795, UID 993, moteur UID 1001.
  Image installée : SHA256
  `5f17747c55bd97d6420cc8a9ed6255289db73285885fbd7dc31e510866db2aa7`.
  Cette observation ne relie pas tout le code Authority courant à cette image.
- GET façade `http://127.0.0.1:38433/login` : 200 HTML ; GET direct moteur
  `/api/v1/native-settings` sans authentification : 403 `web-access-refused`.
  Cela prouve ce refus précis, pas toutes les autorisations ni un parcours connecté.

## Matrice des écarts

| ID / présentation | Implémentation examinée | Preuve disponible et limite | Verdict |
|---|---|---|---|
| C01 — A1/A2/A3 : moteur déterministe, dépendances et validations | `swarm-engine`, `RootStore`, validation de candidat et gardes ; `task_validation/gate.rs` traite les contrôles et préserve le mode humain | Moteur Rust effectivement actif ; preuves ciblées de migration et PAEA, pas nouvelle recette exhaustive de toutes les transitions | Réel, qualification par parcours ; aucune fiabilité universelle déduite |
| C02 — A1/A4 : contenu exact, révocation et absence de doublon | Registre SQLite séparé, facture unique, intention métier, empreinte, état observé ; `paea_settlement.rs` et `paea_settlement/call.rs` ; CLI `paea settlement-call` | Recette livrée settlement : 7 étapes PASS, intégrité vérifiée à nouveau ; accord incorrect, révocation puis reprise, concurrence, sortie perdue, fractionnement/re-clé compris | Vérifié localement sur paiements fictifs ; aucun virement bancaire ni connecteur bancaire réel qualifié |
| C03 — A4 : extension à des processus complets | Parcours Git, transfert, promotion conditionnelle et rapprochement ; `swarm-cli/tests/paea_promote.rs` | Recette chaîne : 4 étapes PASS, intégrité vérifiée à nouveau ; composition Git/règlement et refus d'image différente | Plus avancé qu'une proposition documentaire, mais fixtures et autorité simulée ; pas tout processus métier |
| C04 — A3/A4 : identité, principal, instance et mandat | Contrats d'identité, enrôlement, journal, contrôle ; comptes/sessions et assertions dans `swarm-authority` | Démon réel sous UID distinct observé. Les harnais d'effet PAEA utilisent toujours `authenticated_fixture.rs` ; leur commentaire exclut présence humaine, démon protégé et séparation UID | Identité cockpit et autorité des effets PAEA sont deux périmètres. La première ne clôt pas la seconde |
| C05 — A3/A4 : mémoire gouvernée, provenance et usages | `memory_files.rs` contrôle références, appartenance, alias et sources gérées ; `project_memory_source.rs` protège les sources de profil | `PROJECT-MEMORY.md` et FPA02A distinguent références Rust et contrat historique Go. Le code de références ne lit pas les contenus et ne prouve pas leur fraîcheur | Écart majeur de couverture : cycle Rust complet d'admission/retrait/sélection et droits lire/résumer/transmettre non établi |
| C06 — A3/A4 : délégation et échanges A2A | `trust/operations.rs` refuse les mandats délégués et l'admission avancée non portés ; T14-A2A-DESIGN est explicitement une conception | Codes `trust-delegated-mandate-not-ported` et `trust-advanced-admission-not-ported` présents ; aucune recette d'A2A protégé complet établie ici | La hiérarchie de tâches ne constitue pas une délégation sécurisée des capacités |
| C07 — A3/A4 : gardien des clés et barrières non contournables | Barrière Linux Landlock/seccomp dans `swarm-biba-linux`, appliquée aux helpers natifs ; `.swarm` exclu des lectures et workspace d'écriture attribué | `kernel.rs` crée `Ruleset{fs:ALL,net:0,scoped:3}` ; lectures générales hors `.swarm` autorisées par `read_leaves`. Preuves historiques de gates citées dans la migration, non rejouées ici | Confinement partiel réel ; pas de confinement général des lectures/secrets, d'egress ou de toutes les voies alternatives |
| C08 — A3/A4 : validation humaine et séparation des rôles | Mode `human` conserve les contrôles manuels et pose `human-validation-required` ; formulaires/accords exacts existent | Source de `gate.rs` relue ; registres locaux de navigateur et E2 disponibles. Pas de nouvelle approbation humaine authentifiée d'un effet réel exercée | Le verrou de workflow existe ; attention humaine effective et chaîne d'effet réelle restent à qualifier |
| C09 — A1/A2 : mesure des coûts, latence et actions inutiles | Journaux, métriques et bancs disponibles ; mesures sur effets fictifs et campagnes historiques | Données Go d'octobre présentes, analyse des exclusions et rejeu F4e ; E3 Rust reste brouillon. Pas de campagne comparative actuelle agents réels/B2/seconde implémentation établie | Résultats historiques utilisables dans leur périmètre ; aucun gain actuel global de sûreté, coût ou productivité démontré |
| C10 — A2/A3 : supervision, diagnostic et reprise | Mécanismes de supervision et réception/décision, journaux et contrôles existent | Preuves de fixtures et rapports de migration ; aucun nouveau collectif autonome avec fournisseur réel exercé par cet audit | Implémentation réelle, autonomie prolongée de diagnostic/réparation non établie par les seules règles ou fixtures |
| C11 — A4/annonces PAEA : cadre vérifiable et limites de qualification | Schémas, canonicalisation, outils/recettes, protocole E3 et registre PA-00…13 | Déclaration E2 du 31 octobre bornée ; tableau T01…T22 consigne 0/22 campagnes formelles E3 et 0/17 acceptations indépendantes | La réserve publique « expérimental, non certifié » est justifiée et doit rester attachée aux résultats |

## Preuves réexaminées dans ce tour

`python3 tools/verification/recipe.py verify --evidence CHEMIN` a retourné
`integrity:VERIFIED`, `recorded_status:PASS` pour :

- `.swarm-fixtures/i17-rerun/settlement-run-16` : 7 étapes nommées, sorties
  des tests conservées, témoins de l'image livrée à chaque lancement métier.
- `.swarm-fixtures/i17-rerun/chain-run-9` : 4 étapes nommées ; dont transfert /
  promotion / rejeu / rapprochement, verrou partagé, composition et mauvais SHA.

Les deux résultats portent `source_stable:true`. Comparaison SHA256 de chaque
entrée déclarée avec le checkout pendant cet audit : settlement 566/566
identiques, chaîne 568/568 identiques, aucun absent. Ce sont des inventaires
déclarés, pas une qualification indépendante de tout le graphe de compilation.
Les nombres 7 et 4 sont des étapes de recette, pas des invariants PAEA qualifiés.

`verify` contrôle la cohérence des preuves conservées ; il ne réexécute pas
les scénarios. Aucun nouveau build, Cargo test, appel de modèle, lancement
de mission ou redémarrage n'a été effectué. Aucun succès de cette vérification
ne clôt une acceptation E3. L'interface connectée, Version, FR/EN, thèmes et
assets servis n'ont pas été revérifiés ici.

## Écarts de documentation et de temporalité

- A1 dit que la campagne n'a pas encore tourné. Le dépôt conserve ensuite
  des campagnes et analyses d'octobre : c'est un état daté devenu ancien,
  pas une preuve que la phrase était fausse le 5 octobre. Ces résultats Go
  ne qualifient pas le moteur Rust actuel. Le rejeu du 10 octobre couvre
  31/31 anciennes combinaisons F4e, avec responsable/reviewer scriptés,
  aucun appel LLM et aucune tentative de paiement ; il ne démontre pas
  l'achèvement autonome d'une mission. F7 reste non testé dans ce rejeu.
- A3 annonçait le confinement après migration. Il existe maintenant une
  barrière Linux partielle ; annoncer simplement « aucun confinement » serait
  trop pessimiste. Annoncer « confinement complet » serait non étayé.
- Le tableau de bord PAEA conserve « aucune séparation UID distincte » pour
  la chaîne E2. Le démon cockpit sous UID 993 existe pourtant. La limite E2
  doit rester rattachée aux services d'effet et au modèle de menace concernés.
- `PHASE-B-AUTORITE.md` commence par « plan, non exécuté » puis décrit des
  réalisations et installation ; `swarm-authority/src/lib.rs` annonce encore
  l'absence de socket/session alors qu'il exporte `server` et `sessions`.
  Ces introductions sont périmées ; elles ne doivent pas servir seules au bilan.
- Le nouveau plan Admin marque T1…T8 vérifiés localement, T9 installation
  restant à faire. Ne pas attribuer automatiquement ses groupes, matrices
  et diagnostics enrichis au démon installé dont le hash est observé ici.
- Le tableau de bord relate la suppression d'un ancien FAIL run-13 puis
  recréation du dossier. C'est une réserve de conservation des preuves à
  examiner, pas une preuve que les PASS actuels sont faux. Le vérificateur
  actuel n'atteste pas l'existence historique d'un dossier supprimé.

## Suites concrètes, sans exécution implicite

| Priorité / propriétaire | Écart à traiter | Critère observable de reprise/clôture |
|---|---|---|
| Haute — intégrateur identité/effets et opérateur | Chaîne humaine et contrôle protégé PAEA | Identité réelle, mandat et accord exacts consommés par le service d'effet ; refus/révocation/reprise prouvés avec les privilèges déployés |
| Haute — propriétaire mémoire/composition | Mémoire Rust et A2A | Admission, sélection, retrait, résumé, transmission et reprise testés aux consommateurs ; délégation bornée réelle ; aucune élévation de droit par contenu |
| Haute — propriétaire confinement | Secrets et voies de contournement | Inventaire des accès et témoins valides puis refus effectifs fichiers/processus/réseau/connecteurs ; pas seulement logs après effet |
| Haute — commanditaire, équipe tierce, évaluateur | Qualification comparative | Désignation indépendante, seconde frontière d'exécution, protocole préenregistré puis T01…T22 ; pas de remplacement par le nombre de tests locaux |
| Moyenne — mainteneur documentation/qualification | Statuts anciens et conservation des échecs | Résumés courants cohérents avec les preuves et frontières ; origine du FAIL supprimé retrouvée ou perte explicitement conservée |
| Moyenne — auteur des publications | Actualisation publique | Ajouter un état daté liant résultats historiques Go, preuves locales Rust et critères ouverts, sans transformer les schémas cibles en garanties |

Ces constats sont transmis par ce rapport au demandeur ; aucun propriétaire
tiers n'a été contacté et aucune réception ou acceptation externe n'est revendiquée.

## Sources locales principales

- [Plan PAEA](../PAEA-APEX.md), [tableau T01…T22](DASHBOARD-T20261101.md),
  [déclaration E2](evidence/claude-e2-arbitrage-20261031T135859Z/DECLARATION-E2.md),
  [protocole E3](E3-PROTOCOL.md), [conception A2A](T14-A2A-DESIGN.md).
- [Mémoire](../../../PROJECT-MEMORY.md),
  [plan migration](../../migration-rust/PLAN.md),
  [positionnement sécurité](../../../SECURITY-POSITIONING.md).
- [Phase B](../../acces-cockpit/PHASE-B-AUTORITE.md),
  [Admin](../../acces-cockpit/ADMIN-APEX.md).
- [Bilan facturation du 10 octobre](../../../benchmarks/billing/reprise-20261010/BILAN.md),
  [analyse des exclusions](../../../benchmarks/billing/analyse-32-exclusions-20261008.md).

Revue de l'incrément : seul ce rapport est ajouté. Aucune fonction, gestion
d'erreur ou politique exécutable n'est créée/modifiée. Les valeurs datées et
comptages ci-dessus sont des observations, pas des limites opérateur. La revue
est celle de l'auteur de l'audit, pas une acceptation indépendante.

Suite préparée à la demande de l'utilisateur :
[plan priorisé G0…G7](GAP-CLOSURE-PLAN-20261103.md), avec rattachements PA,
critères de sortie, dépendances humaines et indépendantes. Cette suite ne
modifie pas les observations historiques ci-dessus et ne lance aucun lot.
