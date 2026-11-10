# Revue indépendante des preuves RETEX-QW3

Lecture seule des sources et reçus publics retenus le13 octobre2026. Aucun nouveau test/build/provider, autre agent, mutation source, reprise de campagne ou lecture de base/log de mission privée. Binaire figé SHA-256 eeead1e1624ee11b6667a17fa03455c260fd99df907d7be0c0f16cc07625bfa5, base b61694264cbeaf0d72cea8e55201c1fd1f7aaed1 + delta QW2 (metadata modified=true honnêtement conservée). Pas de nouveau moteur spécifique QW3 : configuration publique de recette.

**Verdict : approve pour la qualification QW3 à quatre tâches et niveaux1/4 examinée. Aucun finding concret identifié. Pas de certification de débit général, coût, p95, endurance ou autonomie universelle.**

## Comparabilité

Les deux frozen/execution-inputs lient le même binaire (hash physique revérifié), provider_config (hash des deux fichiers revérifié sans exposer leur contenu), model_policy et harness SHA aee4768e34322565ab7e451d56a7cb977ab5732a0a5a4a7151dc28268a82fba5. La source actuelle du harnais correspond à cette empreinte. L’arbre baseline est identique6fba7fca3663d5ca76cbc4ccda1a92e2ceb90254 ; les SHA des commits baseline diffèrent comme identités de commits, pas comme charge source.

Quatre tâches mêmes ID/titre/exigences/critères/dépendances/livrables et mêmes commandes exactes python3 -B -c, timeout15 ; seule date authorized_at diffère dans les politiques de validation. Paramètres communs4 tâches,16 activations/décisions,8 reviews, observation900s, CLI45s, pause polling2s, route standard gpt-5.6-sol/medium. Slots1 puis4, dossiers/Work/horaires différents. Huit workers possèdent cette route persistée, reported_model absent : on ne substitue pas le modèle demandé à une identité backend rapportée.

Campagnes successives, une observation chacune ; charge locale, état fournisseur/cache et moment diffèrent. La comparaison autorise un ratio observé, pas attribution causale exclusive de tout gain au seul nombre de slots ni une économie de facture.

## Livraison et acceptations réelles

| Parcours | Work / révision | Projet candidat | Validation publique |
| --- | --- | --- | --- |
|1 worker | w-6ec6f9ecb0fb24c96fd9924b / r34 |12c9dc28d4e87124da051405beabfaa7d51f2393 |4 accepted/fresh,0 stale, judge end cycle1 |
|4 workers | w-4cac57c1bf86383814aa3c50 / r30 |b2455c35a4d262a5e2979a1cac443ce4bab0b769 |4 accepted/fresh,0 stale, judge end cycle1 |

Les quatre AutomaticValidation de chaque Work ont candidate_sha égal à ce candidat courant et controles executed/passed/exit0. Les quatre avis de tâche sont passed, candidate_commit courant. Il y a quatre EXÉCUTIONS de revue par campagne, dédupliquées par ID d’événement review.managed.result ; ne pas compter les copies des avis cumulatifs portés par plusieurs tâches. Les dernières revues cumulent le résultat sur le candidat final. Les dix contrôles des événements managed.integrated correspondent à1+2+3+4 contrôles, tous réussis et dix intervalles d’exécution distincts, pas dix copies d’un seul reçu.

Les source originals restent propres sur leurs baseline, leurs quatre modules retournent0. Les deux delivery HEAD correspondent aux projets candidats et inspection des huit modules livrés montre exactement retour1,2,3,4. Empreinte de chaque bundle correspond au résultat. Le harnais a réellement cloné puis exécuté l’assertion exacte des quatre valeurs exit0 ; le reviewer a inspecté les sources et Git/bundles, sans relancer le test fonctionnel demandé interdit dans cette revue. Ces preuves conservent la portée de leur campagne et ne deviennent pas une acceptation de la mission historique.

## Temps et concurrence recomputés indépendamment

À partir des timestamps mission.start et planning.decide operation judge/judge_verdict=end :

| Mesure | Slots1 | Slots4 |
| --- | --- | --- |
|Start mission → juge end |352.434230s |131.814878s |
|Recette totale, bundle/arrêt inclus |356.611392s |134.760361s |
|Pic par intervalles Agent.started/ended |1 |4 |
|Somme contrôles |0.508903s |0.385062s |
|Somme4 revues |46.706353s |39.993212s |

Ratio352.434230/131.814878 =2.6737059985. Toutes les durées de comparaison retrouvent celles recalculées depuis reçus. Les sommes de temps processus/revue ne doivent pas être additionnées à la durée murale : leurs intervalles peuvent se chevaucher. Un intervalle processus inclut travail/outils/attentes, pas uniquement inference modèle.

Pour chaque worker, les métriques de phase ont été recomputées et correspondent : fin processus → premier contrôle de son candidat inclut attente de lane ET préparation ; durée revue = started→finished ; fin revue→managed.integrated inclut publication ET validation de fin, pas temps Git exclusif pur. En parallèle, attente/préparation avant contrôle module2=12.063365s, module3=24.410011s ; module1=0.581581s,module4=0.384565s. Cela matérialise le compromis production parallèle/publication exclusive sans inventer une instrumentation séparant intégralement la queue et la préparation.

Quatorze fenêtres de progression multi-worker recomputées depuis agent list CLI correspondent exactement à la comparaison, aucune en série. La pause est NOMINALEMENT2s ; les écarts réels des14 fenêtres sont2.135929–2.639076s (CLI compris). Ils montrent plusieurs compteurs d’outils progressant dans une fenêtre, pas des appels/inférences simultanés au même instant. Le pic4 est indépendamment établi par chevauchement des intervalles de processus.

## Consommation et interventions

Ledger13 en série /11 en parallèle, dont une réservation initiale manual setup estimated sans usage chacune. Root fini4/2, reviewers4/4, workers4/4. Les nombres planning_calls5/3 incluent setup : ils ne doivent pas devenir5/3 root inferences prouvées. Source publique cli après start contient seulement mission start, lectures, planning bundle, mission spending, mission pause après fin ; aucun retry/édition/acceptation manuel dans ce journal. Le compteur0 interventions après start ne prouve pas à lui seul absence de tout acte extérieur, mais le chemin inspecté est cohérent. Aucun autre appel reviewer effectué.

Sommes champs reçus : serial1574459 input/29968 output, parallel1562170/28860, une entrée sans usage chaque cas. Treize/onze entrées sans coût rapporté : coût réel inconnu. Champs cache restent séparés, pas additionnés aux jetons bruts ou tarifés localement. Ces sommes d’usages reçus ne sont ni facture exhaustive ni comptage des appels internes du client. Pas de ressource/provider hors scénario déduite d’un ledger incomplet.

## Sources, erreurs de méthode et exports

retex_stage_analysis.py examine reçus retenus, n’appelle pas runtime/provider ; sa copie cache retex-stage-analysis.py est identique. L’erreur initiale du propriétaire sur operation verdict au lieu judge_verdict a été corrigée avant production du résultat et sans paid rerun. Le reviewer a également corrigé une inspection utilisant candidate_commit pour AutomaticValidation après lecture de ses clés réelles candidate_sha ; aucun résultat erroné n’est utilisé comme preuve, originaux inchangés.

Original comparaison cache SHA0027cf88e84c547009b6e984872ed1c249a9ab64aa0fa1caec058cee2f632c17. Export dépôt qw3-comparison.json SHAda092ef41e115ea7311610684862c0f83aabca7e4a5a153d1a8a4e4a93d8607d : mêmes chiffres/données, seul texte limits étendu pour expliciter pause nominale et absence de preuve d’inférence simultanée. Il ne remplace pas l’original cache. originals.json décrit copies ou projections sélectionnées et conserve empreintes originales.

CALL-ACCOUNTING FR/EN est cohérent avec QW2 : registre complet de la mission, liste récente200 distincte, null/absence non zéro, cache non additionné, coût reçu non facture, modèle historique non inféré, reprise distincte par tâche/minimum legacy. Pas nouvelle politique ni pouvoir d’acceptation.

## Empreintes des preuves primaires

retex-qw3-serial/frozen.json : 6e7b72858e1318cb395b50bae1b7d24cd76104654415b2cc3d38cd00078c2af5

retex-qw3-serial/execution-inputs.json : a202606a0f75201099f600a3e8d7b6fb80c69f23b013dff767b590aa45358fa1

retex-qw3-serial/result.json : 2701a06bdeba3117888471f9df972689c872cc2e2d7fa5b7f8c82866c84ac54f

retex-qw3-serial/final-public-work.json : 9bd42734bb70718363b383e417f6b25ac4523c4fde77d817a3407875e727ffc7

retex-qw3-serial/spending.json : 87ace9b63e1a64b7ed6d86b129b48276887477d3680d7763d43e3456882eab92

retex-qw3-serial/agents.json : 38cd6a40f9691b8af01af82115df155eec45f19f65191b7a94d9a8ad9939bb43

retex-qw3-serial/cli.jsonl : 7291bbf40a5a0f419eca3ab0db8da8bc091c8a55969cf0ab6558f9f6653f1576

retex-qw3-serial/delivery.bundle : 871792002d12c2b53b5f369b169ff3a7bc0c0a11d1f914f746d8dde970efb85c

retex-qw3-parallel/frozen.json : d7b9519d5fe1df1e8a9b77c8deb5497aaf06b68c3b71fcd7911321161ce70ab9

retex-qw3-parallel/execution-inputs.json : 5d0be78c429dbedef39da0eb534203afe3271d12d811bd32369901483348e244

retex-qw3-parallel/result.json : 3b708ba18a2880cce15960ac67b450c72da7858e736a5c75c83e20f53db1270b

retex-qw3-parallel/final-public-work.json : 7efe1eb69c62923c1b68012140cf47dbe7773791600e717e9124d0f67bf5751d

retex-qw3-parallel/spending.json : 4980cdaeffa41420a03cd655e122d16b8f9b9474a0906539f7d6b8d0c626091c

retex-qw3-parallel/agents.json : 04d5d8750a34e135e9f2a56b570171a87c4e37740f0f906e05c1dad95aa13475

retex-qw3-parallel/cli.jsonl : 8646f1741f49e21053f312c3e66c78aaf93a3ea6d8c8015c89776216a67b850f

retex-qw3-parallel/delivery.bundle : efdc15ffde278d434e9a425958c6cf40f5c0ae85ea447ef8381f5585786059bd

tools/verification/retex_stage_analysis.py : 5d0e1d88729a74ca4cd4f0469a39edf6758ff804391bc014bf7552fdc4fab695

tests/retex_real_campaign.py : aee4768e34322565ab7e451d56a7cb977ab5732a0a5a4a7151dc28268a82fba5

docs/architecture/CALL-ACCOUNTING.md : e5dcd236a63900fdc375923428b515dcd676381bbcf96ffd4155de4a9611ba08

docs/en/CALL-ACCOUNTING.md : 63282327afa84ac2c599fd58424e796f22e511662b3fc5bf8ba1d2211558b886

Sources libérées ; aucun check reviewer actif. RETEX final des suites et livraison versionnée restent les actions du propriétaire. Réutiliser ces preuves sur un commit documentaire seul exige identité source/inputs inchangés ; aucun verdict global ou nouvelle acceptation n’est fabriqué.
