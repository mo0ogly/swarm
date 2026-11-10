# Revue cumulative T7 — candidat 6bfd100

**Verdict candidat : approve. F1/F2/F2b, l’ordre discriminant et F4 (compteur de contrôles gérés) sont corrigés et vérifiés. Livraison réelle de la fixture confirmée. Aucun finding ouvert dans ce périmètre ; la clôture du projet et le déploiement natif historique restent des décisions distinctes.**

Moteur courant propre : `6bfd1005b663319021be60745a38c66c4d5e20c5`. Le dernier delta depuis `60c3dc65edf0f1f3915414e284f58543c6814924` ne change que le comptage public des contrôles, ses tests, les notes FR/EN et le manifeste. Les rôles, pipeline de production/revue/publication, budgets et schéma d’opérations restent identiques au candidat 60c3dc6 approuvé indépendamment et utilisé pour la phase finale de recette réelle.

Fixture isolée : `w-6a2c13c498cba8780192b44d`, révision127 conservée. Commit du **projet de recette** livré : `b569bb35fb5816bb806826000f99be32b973108c`. Un SHA moteur et un SHA projet ne désignent pas le même objet : l’avis code porte sur 6bfd100, les contrôles/revues/acceptations du projet portent sur b569. Cette distinction est essentielle pour “même candidat”.

Revue en lecture seule : CLI publiques sur racine isolée, états et snapshots publics fournis, lecture du bundle, tests locaux ciblés. Aucun appel fournisseur, aucun autre agent, aucun accès direct à la base mission ou log privé, aucune acceptation/mutation de mission ou modification de source par le reviewer. Aucun processus partagé interrompu. Les méthodes replan/code-reviewer et les limites de rôle restent appliquées.

## Critères cumulatifs et observations

| Critère T7 | Observable confirmé | Qualification |
| --- | --- | --- |
| Fournisseur disponible/politique figée | Refus réel initial gpt-6.1-sol/xhigh conservé ; sélection explicite du standard configuré gpt-5.6-sol/medium ; appels réels de production, planification et revue observés sous cette route ; fichier frozen60 lie binaire/politique de la phase finale | PASS pour disponibilité de la route finale ; aucun fallback silencieux ou santé universelle inférés |
| Découverte/délégation/concurrence/remise/jugement/reprise | Exploration de SPEC.md attribuée dans l’inbox ; snapshots de deux scopes simultanément réservés, dont r88 sous la phase60 ; deux workers réels se chevauchent53,148968s ; rapports et sidecars livrés ; deux tâches accepted/fresh, neuf scopes closed, root judge end cycle5 ; redémarrage propre r17→r18 conserve activations et candidat | PASS du parcours réel assisté observé ; neuf scopes historiques, pas une topologie minimale neuve de trois scopes |
| Corrections/reprise | Pannes fournisseur et harness, répétition cycles/sous-plans, erreur de diagnostic et choix de branche traités par l’opérateur avec public pause/resume/config, correctifs source et reviews ; l’effet du kind-first produit désormais les tâches, puis livraisons acceptées | Correction réellement observée, assistée ; aucune autonomie générale de diagnostic/correction revendiquée |
| Revue et contrôles sur même candidat | Deux appels natifs de revue au total ; première revue sur candidat partiel2d127…, puis revue cumulative finale sur b569 couvrant les deux tâches. Les deux avis par tâche finaux portent le même review ID `review-c6999b3aaea9111ef7ada1b4`, leurs producteurs sont distincts du reviewer ; gates/hash/criteria et acceptedFresh publics restent valides sur 6bfd100 | PASS ; deux tâches couvertes par une revue cumulative finale, pas deux appels indépendants distincts sur le commit final |
| Rapport cumulatif/échecs/usages/limites | Échecs originaux, interventions, versions, source/binaire et reçus conservés.40 activations,30 décisions,2 appels de revue ; usages incomplets et coût inconnu explicités ; F4 rouge puis correction vérifiée sans répétition payante | PASS de qualification ; pas une facture exacte ou preuve d’échelle de nombreuses missions |

La fixture a traversé plusieurs moteurs et préconditions :44a86e6 initial ; corrections61f82bd/7e9d024 ; ordre de schéma60c3dc6 pour créer les workers et terminer le cycle ;6bfd100 pour restituer fidèlement la télémétrie et recontrôler la fraîcheur. Les étapes antérieures restent de leur révision, elles ne sont pas redatées comme une exécution neuve entièrement sous 6bfd100. La phase finale pertinente pipeline/source est inchangée depuis60c3dc6 ; seul le lecteur de métriques a changé, puis a été exercé publiquement sur les mêmes événements immuables. Ce périmètre justifie de réutiliser les preuves sans relancer des modèles pour une correction de lecture.

## Livraison et fraîcheur vérifiées indépendamment

Le reviewer a cloné `delivery.bundle` dans un répertoire temporaire. HEAD estb569, les sources inspectées sont deux fonctions retournant respectivement42 et7. L’assertion réelle `python3 -B -c 'import alpha,beta;assert alpha.answer()==42 and beta.answer()==7;print("alpha=42 beta=7")'` passe exit0. Les modifications sont alpha.py, beta.py, les deux rapports canoniques docs/<task-id>.md et leurs deux sidecars delivery.json. Le repo source original reste propre au commit12237f3, alpha/beta y retournent encore0/0. Cela établit l’effet livré isolément et préserve l’original.

Les getters publics `work show` ont recalculé les gates et la validation :2 validées,0 périmées, `fresh=true` pour les deux tâches, à la même révision127. Les deux contrôles finaux sont exécutés/PASS/exit0 sur b569 et les preuves de revue sont liées à ce commit. Les projections indiquent une révision d’exécution/acceptation inconnue dans certains anciens champs ; on n’en fabrique pas une. Les événements d’intégration identifient cependant les révisions96 et106, candidats, commandes, timestamps et résultats. L’attestation Git, les checks de fraîcheur et les objets immuables apportent la preuve précise utilisée ici.

Le bundle SHA-256 est `e64199d24f1f8e34469cfde061715cb989bfb83e6997bc69add9384e88830ed4`. Les détails et stdout original du contrôle figurent dans le rapport historique `t7-cumulative-review-60c3dc6.md`, conservé.

## F4 métrique — résolu

À 60c3dc6, `mission_insights.go:634` ne sélectionnait que les événements task.auto-validation. Le registre public contient pourtant managed.integrated r96 avec un alpha, puis r106 avec alpha+beta : **trois exécutions enregistrées**, toutes PASS et timestamps distincts. Le getter renvoyait0. Ce rouge était un filtre incomplet, pas un registre absent, et la somme des deux AutomaticValidation actuels aurait encore omis le premier contrôle alpha.

Le delta 6bfd100 lit kind/payload des deux types d’événement, décode la liste ordinaire ou la map par tâche du chemin géré, compte Executed=true une fois par événement et propage les erreurs de décodage. Aucun changement de registre, de consommation, de state accepté, de rôle ou de politique ; aucune reconstitution arbitraire de succès. Test mixte : contrôle ordinaire, contrôles gérés réussi/refusé, non-exécutés ignorés, autre work exclu, controls malformé refusé.

Le reviewer a exécuté2 tests ciblés en -race : `TestMissionSpendingCountsManagedAndAutomaticControlEvents` et `TestGraphDeliveryD03QualificationManifest`, exit0, noms et JSONL originaux ci-dessous. Le producteur a fourni trois tests MissionSpending PASS et un rouge valide expected3/got1 sur la source précédente ; son premier fixture invalide (collision unique de révision) est conservé mais n’est pas une preuve du produit. Pas de suite entière ni paid run répété.

Relecture publique indépendante avec `swarm-t7-delivery` : `recorded_control_executions=3`, `worker_retries=0`, work r127 et les deux acceptations fraîches inchangées. Le binaire est lié à6bfd100 par vcs.revision et vcs.modified=false. `git diff --check` passe et le checkout est propre après les tests.

## Usages, interventions et limites

Les lignes publiques donnent44 réservations/tentatives/appels enregistrés, dont40 de planification et2 de revue ;8 n’ont pas d’usage reçu. Somme des champs de jetons rapportés :1 392 836 entrée /29 750 sortie ; ces champs ne sont pas une facture ou un total exhaustif des inférences internes.20 appels d’outils workers observés ; les tests ne sont pas identifiables de façon fiable dans ce compteur.0 coûts rapportés sur44 lignes : **coût réel inconnu**, jamais0 dollar connu. Les probes opérateur hors mission, le coût historique et les caches non exposés ne sont pas inclus dans une prétendue mesure exhaustive. Le nombre worker_retries=0 n’efface ni pauses/reprises de supervision ni interventions humaines.

Interventions conservées : erreur de harness mission start/Work, refus de capacité et configuration publique de route, restart propre, instruction FIRST qui réouvrait des cycles, pauses/reprises et instructions de phase, sur-délégation corrigée par contrat d’ordre de schema et probe isolée, ENOSPC traité par cache Go reconstructible, correction des compteurs. Le rapport du harness PASS hardcode scopes3 ; la source publique r127 établit9. L’original reste intact et le reçu distinct corrige cette présentation. L’état conservé ne démontre pas une efficacité optimale ni une récupération totalement autonome.

Le scénario ne provoque pas un worker défectueux puis une nouvelle tentative de production ni une revue sémantique changes_requested avec réparation autonome : les workers réussissent leur seule tentative et les refus corrigés sont surtout fournisseur/planification/méthode/transport. Ces chemins gardent leurs preuves de fixtures ciblées précédentes, sans les transformer en nouvelles observations réelles. Il n’y a pas non plus de mesure de coût de plusieurs niveaux de concurrence, de requalification browser globale ou de preuve de montée en charge à partir de ces deux modules. Ces limites sont conservées ; aucune autre campagne n’est inventée pour les masquer.

## Correction additive de provenance du premier rapport

Le rapport original 44a86e6 décrivait une réception current.json r5 avec erreur fournisseur et associait ensuite un hash d’un fichier courant évoluant pendant la recette. L’opérateur a retrouvé l’original exact du hash `391f524be24e62048513b4310b33c49204b33c255e0de5f4d93332932c98e85d` : `t7-review-received-snapshot-original.json`, **r15, cycle2, sans failure**. Le lien hash→r5 n’était donc pas valide. Cette précision est ajoutée ici ; ni le premier rapport ni les échecs historiques ne sont réécrits. F2 reste indépendamment établi par les événements structurés de capacité intacts et le chemin source de parseur/runner.

Leçon de méthode appliquée dans cette revue : lire une fois les bytes, parser et hasher ce même buffer ; conserver les reçus originaux nommés immuablement. Ne pas prendre une nouvelle empreinte de current.json après avoir décrit une version précédente. Les réponses publiques de cette revue sont parsées/hashées depuis le même stdout conservé en mémoire ; les projections variables et les réservations ne sont pas confondues avec un original figé.

## Clôture de cette revue, prochaine frontière

Avis code 6bfd100 favorable et recette fonctionnelle isolée attestée, avec ces limites explicites. Aucun finding ouvert de cette revue. Le root peut consolider les documents/RETEX et traiter la mise à jour du service partagé selon l’autorité et la situation des autres travaux. Ce reviewer n’accepte, ne relance ni ne clôt la mission historique. La présence d’un autre travail enabled/nonpaused dans le précontrôle opérateur signifie que la maintenance est une action distincte à organiser ; elle n’est pas démontrée par le bundle de cette fixture.

Aucun check reviewer actif ; sources libérées. Une modification pertinente ultérieure demande le contrôle de son delta. Ce rapport ne remplace pas la décision métier finale ou les permissions de maintenance/déploiement.

## Empreintes du dossier final

- `t7-cumulative-public-receipt.json` : `ae9785d5c41a56ee6123cfe0f70cdeb6cc2d9f5a6300835a86ce4e7044d4f45c`.
- `t7-real-codex-44a86e6/final-public-work.json` : `399a5253be29bb069f9bca337336cc68bb8958783ffba56dca64dcdcbfd65f09`.
- `t7-real-codex-44a86e6/delivery.bundle` : `e64199d24f1f8e34469cfde061715cb989bfb83e6997bc69add9384e88830ed4`.
- `t7-spending-public-final.json` : `da3331b180dcc3d656db0a8c934076cc0c7dd070155efb70be7085a2229bbeaa`.
- `t7-spending-public-corrected.json` : `d790f8d74c14172eb5a70a6f385a21d6b12d6bc353a4b77223b9fce3ebe18baa`.
- `t7-real-codex-44a86e6/frozen-60c3dc6-1791877705.json` : `ed2415efe7ced3eac1e31d39bb6d31b9e5bf77f846dbbb1f36b73e15a86d99d2`.
- `t7-real-codex-44a86e6/snapshots/88-acf8cb1483d3.json` : `acf8cb1483d320d2bad471254c634bdefcec16c468b453c474ea614304b2bc77`.
- `t7-review-received-snapshot-original.json` : `391f524be24e62048513b4310b33c49204b33c255e0de5f4d93332932c98e85d`.
- `swarm-t7-delivery` : `e070ac46a9bce4bbef8d6e91299c24b9a3ab51280a8f2d72adb80df07d06b60e`.

Finalisé 2026-10-13T08:20:28.554380+00:00.

## Tests ciblés reviewer

```json
{
  "command": [
    "go",
    "test",
    "-race",
    "./internal/engine",
    "-run",
    "^(TestMissionSpendingCountsManagedAndAutomaticControlEvents|TestGraphDeliveryD03QualificationManifest)$",
    "-count=1",
    "-timeout=90s",
    "-json"
  ],
  "started": "2026-10-13T08:07:53.324908+00:00",
  "finished": "2026-10-13T08:08:33.159001+00:00",
  "exit": 0,
  "passed": [
    "TestGraphDeliveryD03QualificationManifest",
    "TestMissionSpendingCountsManagedAndAutomaticControlEvents"
  ],
  "failed": []
}
```

```jsonl
{"Time":"2026-10-13T10:08:30.293088819+02:00","Action":"start","Package":"swarm.local/companion/internal/engine"}
{"Time":"2026-10-13T10:08:30.47305837+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest"}
{"Time":"2026-10-13T10:08:30.47310553+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"=== RUN   TestGraphDeliveryD03QualificationManifest\n"}
{"Time":"2026-10-13T10:08:31.462068739+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"--- PASS: TestGraphDeliveryD03QualificationManifest (0.99s)\n"}
{"Time":"2026-10-13T10:08:31.462136008+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Elapsed":0.99}
{"Time":"2026-10-13T10:08:31.464692971+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestMissionSpendingCountsManagedAndAutomaticControlEvents"}
{"Time":"2026-10-13T10:08:31.464753677+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestMissionSpendingCountsManagedAndAutomaticControlEvents","Output":"=== RUN   TestMissionSpendingCountsManagedAndAutomaticControlEvents\n"}
{"Time":"2026-10-13T10:08:32.056325436+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestMissionSpendingCountsManagedAndAutomaticControlEvents","Output":"--- PASS: TestMissionSpendingCountsManagedAndAutomaticControlEvents (0.59s)\n"}
{"Time":"2026-10-13T10:08:32.056374038+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestMissionSpendingCountsManagedAndAutomaticControlEvents","Elapsed":0.59}
{"Time":"2026-10-13T10:08:32.058580163+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"PASS\n"}
{"Time":"2026-10-13T10:08:33.079017065+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"ok  \tswarm.local/companion/internal/engine\t2.786s\n"}
{"Time":"2026-10-13T10:08:33.079060338+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Elapsed":2.786}

```

```text

```

## Relecture publique sur binaire corrigé

```json
{
  "mission": {
    "command": [
      "/home/fpizzi/.cache/swarm-operator-replan-20261011/swarm-t7-delivery",
      "--root",
      "/tmp/swarm-t7-codex-2pbm7fl4",
      "--json",
      "mission",
      "spending",
      "w-6a2c13c498cba8780192b44d"
    ],
    "exit": 0,
    "sha256": "a73c0ee1623ee3c740258fd84f6f278c2d88a37a6d36f70aa79b663f9a2b56b4",
    "controls": 3,
    "retries": 0
  },
  "work": {
    "command": [
      "/home/fpizzi/.cache/swarm-operator-replan-20261011/swarm-t7-delivery",
      "--root",
      "/tmp/swarm-t7-codex-2pbm7fl4",
      "--json",
      "work",
      "show",
      "w-6a2c13c498cba8780192b44d"
    ],
    "exit": 0,
    "sha256": "440367a576f89c7991442a5db63c4b42e922ae6dfaee0f646d0b71a16d92f09f",
    "revision": 127,
    "validated": 2,
    "stale": 0,
    "fresh": {
      "alpha-answer-42-code-3c8e1a": true,
      "impl-beta-answer-7-4c91a2": true
    }
  },
  "received_original": {
    "file": "t7-review-received-snapshot-original.json",
    "sha256": "391f524be24e62048513b4310b33c49204b33c255e0de5f4d93332932c98e85d",
    "revision": 15,
    "failure": null,
    "cycle": 2
  }
}
```
