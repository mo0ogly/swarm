# Revue indépendante du delta 60c3dc6

**Verdict : approve. Aucun finding ouvert sur ce delta. Cette approbation du candidat ne vaut pas recette réelle cumulative terminée, acceptation native ou déploiement.**

Candidat exact `60c3dc65edf0f1f3915414e284f58543c6814924`, checkout propre au début et après tous les contrôles. Base de revue `7e9d02490a1da0366cbe1759c2e1c525f9cae298`, dont les corrections précédentes restent inchangées. Revue indépendante autorisée, lecture seule ; aucun source modifié, aucun autre agent, aucune inférence réelle ou état privé de mission native utilisé par ce reviewer. Rapport cache uniquement. `replan` et `code-reviewer` appliqués à la reprise.

## Critères et résultat

| Critère | Vérification indépendante | Résultat |
| --- | --- | --- |
| Kind avant payload | Byte-stream des branches, quatre contextes de réception, 37 branches ; test intégré de l'ordre dans schema généré | PASS |
| Conservation du contrat/droits | Fonctions anciennes/nouvelles exactes exécutées et schémas parsés comparés ; aucune différence après retrait des seules annotations description ; mêmes required/enums/valeurs/bornes/additionalProperties/kinds | PASS |
| JSON sûr | MarshalJSON utilise json.Marshal pour chaque clé et valeur ; exécution avec quotes/HTML/newline et valeur non sérialisable ; erreur propagée | PASS |
| Délégation légitime | Test intégré de root → child → leaf, rejet profondeur puis task/récupération ; branches delegate toujours proposées selon politique existante | PASS local |
| Réception/fraîcheur/capacité | Test admission du contexte complet avant claim ; inputs du schéma, historique, cycle, child/root examinés ; manifeste courant qualifié | PASS local |
| Pas de conversion/fallback/cap ajouté | Diff uniquement représentation ordonnée, annotations, clarification des consignes et docs/manifeste ; validateurs/application/leases/budgets/routes inchangés | PASS de lecture du delta |
| Réelle efficacité et livraison | Probe indépendante de contexte examinée dans le dossier opérateur ; effet natif sur le binaire actuel non encore fourni | PARTIAL, condition de recette T7 conservée |

`planningSchemaProperties.MarshalJSON` sérialise `kind` puis les champs dans l'ordre de `planningOperationSchemaFields` ; il ne décide aucune opération. `planningSchemaForDelivery` garde `anyOf`, les champs autorisés et les gardes de disponibilité. Le moteur continuera à appliquer le kind reçu sans réinterpréter titre/consigne. Les descriptions task/delegate ne changent pas l'autorité, et les clarifications de phase ne suppriment pas les gardes de preuve ou n'ouvrent pas un cycle automatiquement.

La [documentation officielle Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs) référencée par les nouvelles pages FR/EN a été vérifiée le 13 octobre 2026 : elle décrit une sortie suivant l'ordre des clés du schéma. Les docs du candidat attribuent précisément la cause à la probe T7 et ne promettent pas une conformité universelle de tout choix futur.

La probe ordre-seul conservée porte le même prompt beta exact et le même modèle/effort que l'original fautif, avec schémas égaux après parsing. La sortie est task avec payload complet. C'est une exécution fournisseur réelle de l'opérateur, vérifiée en lecture seule par le reviewer, pas un nouvel appel reviewer. Elle isole le mécanisme d'ordre ; le candidat ajoute aussi des clarifications dont l'effet propre n'a pas été démontré et n'est pas revendiqué.

## Contrôles exécutés

1. Programme Go réduit avec les fonctions complètes de planning_schema.go de 7e9d024 et du candidat, quatre états de delivery, comparaison des schémas et examen des premières clés. **Exit 0, 37 branches.** Les seuls champs ignorés pour l'égalité sont les descriptions annotatives ajoutées ; tout champ d'autorité et de validation est conservé. Escaping JSON et valeur non sérialisable PASS. Source/digest et sortie originale ci-dessous.
2. Tests engine ciblés en `-race` : **5 tests de premier niveau / 5 avec sous-cas, exit 0** : délégation récursive/récupération, manifeste, séparation des mutations, ordre du discriminant et enveloppe complète avant réservation. Commande, noms, timestamps et JSONL original ci-dessous. Aucun go test ./... et aucun des 41 contrôles inchangés rejoué.
3. `git diff --check` PASS ; HEAD et absence de dirty diff reconfirmés après les tests.
4. Binaire fourni `swarm-t7-wire-order` inspecté en lecture seule : Go 1.24.3, `vcs.revision=60c3dc65edf0f1f3915414e284f58543c6814924`, `vcs.modified=false`, SHA-256 `2c48fe08c2a5d8cc585e29f573c2c5065f32446a0018cff41426b881b3af0768`.

Le reçu producteur `t7-discriminator-order-green.jsonl` comporte six tests ciblés PASS en -race ; ils restent distingués des contrôles propres au reviewer. Le rouge original échoue sur `first=criteria`, puis les nouvelles vérifications lisent `kind` sur les bytes émis. La qualification du manifeste conserve le statut source-snapshot, sans renouveler l'acceptation des rapports historiques.

Les quatre delivery fixtures du comparateur sont : défaut ; événement frais + root judge/end cycle 3 + exploration + budgets 120/450 ; historical-only avec délégation indisponible ; événement child avec clôture/retries indisponibles + exploration. Ce sont des inputs techniques comparatifs, pas des décisions métier ou changements de quotas de la recette.

## Limites et reprise

La correction est générale au contrat de transport des opérations et n'est liée à aucun nom de mission/erreur. Son application native sur ce binaire doit encore être observée : réception du schema kind-first, décision task valide, Task worker créé dans le scope propriétaire, aucun PlanningScope supplémentaire pour ce changement atomique, lancement et remise vérifiés par les opérations publiques. Le point de cohérence du rapport identifié dans le diagnostic reste à contrôler : le rapport géré doit être `docs/<task-id>.md`, même si une consigne plus ancienne cite un autre nom. Une probe choisissant task ne valide pas cette livraison.

Conserver l'historique, les activations, les refus réels et les interventions ; aucune répétition inchangée, substitution d'opération, nouveau plafond de scopes, reset ou faux résultat. Les coûts/tokens/cache réels et les autres critères cumulatifs T7 devront être rapportés depuis leurs originaux, puis RETEX final et décision de clôture selon l'autorité applicable. Ce verdict ne relance/termine ni mission ni serveur historique.

Sources libérées : aucun check reviewer actif. L'approbation est liée à ce SHA précis ; un changement ultérieur pertinent réclame un contrôle adapté du delta.

## Empreintes

- `internal/engine/planning_schema.go` : `e743a39d1ac6ccb016528f09cb59ed8d4084980d11d54d2d3d0161a9f2541080`.
- `internal/engine/planning_prompt.go` : `c32dbbff1ec7279eabe91437257212b4c2238e0cbdb3fc27c6e1647e94ca1725`.
- `internal/engine/planning_operation_schema_contract_test.go` : `2b97d641edd39ef6624d483086cb870189832dc83c675209657802d54fcf6f62`.
- `docs/engine-cursor-source-manifest.json` : `552cbc716ede0b84f39e9c6351c8c2423c3f79488f8d28b19387098ef43e7bc8`.
- `docs/architecture/AGENT-ENGINE-PROTOCOL.md` : `3d47b29e40df99d5a9c775e50e67e2c450b3d145fee7c9e3ab05b4d057fd66b3`.
- `docs/en/AGENT-ENGINE-PROTOCOL.md` : `2100048c67cf9cc7f69ea311deda33e25bcf3fb0f568d5bdeb8d4dd0e13cd960`.
- `docs/plans/engine-cursor-contract/evidence/t7-20261013/t7-discriminator-order-red.jsonl` : `22dfc5e651c02e080cc7a5b767a0a57700abd2e2fed1c23b164cb896ecba65db`.
- `docs/plans/engine-cursor-contract/evidence/t7-20261013/t7-discriminator-order-green.jsonl` : `db62dcde4a1db27f1468d63a990e80079eee77b07b6be7d630b5debdd2d1510c`.
- `docs/plans/engine-cursor-contract/evidence/t7-20261013/t7-discriminator-first-probe.json` : `00c79fd7fa7f6c421defb6fab15630e744142e8d0ab31ef419950a5e21b5f0c3`.

Finalisé 2026-10-13T07:51:47.988606+00:00.

## Vérification comparative reviewer

```json
{
  "candidate": "60c3dc65edf0f1f3915414e284f58543c6814924",
  "at": "2026-10-13T07:48:58.955023+00:00",
  "method": "exact old/current planning_schema.go functions isolated; 4 delivery contexts and serializer guards; no provider",
  "current_source_sha256": "e743a39d1ac6ccb016528f09cb59ed8d4084980d11d54d2d3d0161a9f2541080",
  "old_source_sha256": "00f7a251ede7d1f25122dcc2e92f5548acb2d965bd69a1384ae1c91c477d2b2b",
  "program_sha256": "61cf062a855b912118cd30434f14db306717535697bb1ed42e28eb8791fd30d2",
  "exit": 0,
  "stdout": "case 0 same schema except non-authorizing descriptions, kind first PASS\ncase 1 same schema except non-authorizing descriptions, kind first PASS\ncase 2 same schema except non-authorizing descriptions, kind first PASS\ncase 3 same schema except non-authorizing descriptions, kind first PASS\n37 branches; JSON escaping and unsupported value error PASS\n",
  "stderr": ""
}
```

## Contrôles intégrés reviewer

```json
{
  "command": [
    "go",
    "test",
    "-race",
    "./internal/engine",
    "-run",
    "^(TestPlanningOperationSchemaSeparatesMutationKinds|TestPlanningSchemaChoosesKindBeforePayload|TestCursorContractRecursiveDelegationHandoffAndAdaptation|TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim|TestGraphDeliveryD03QualificationManifest)$",
    "-count=1",
    "-timeout=90s",
    "-json"
  ],
  "started": "2026-10-13T07:49:20.383922+00:00",
  "finished": "2026-10-13T07:49:28.845669+00:00",
  "exit": 0,
  "top_level": 5,
  "with_subcases": 5,
  "passed": [
    "TestCursorContractRecursiveDelegationHandoffAndAdaptation",
    "TestGraphDeliveryD03QualificationManifest",
    "TestPlanningOperationSchemaSeparatesMutationKinds",
    "TestPlanningSchemaChoosesKindBeforePayload",
    "TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim"
  ],
  "failed": []
}
```

```jsonl
{"Time":"2026-10-13T09:49:25.250222305+02:00","Action":"start","Package":"swarm.local/companion/internal/engine"}
{"Time":"2026-10-13T09:49:25.448450653+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestCursorContractRecursiveDelegationHandoffAndAdaptation"}
{"Time":"2026-10-13T09:49:25.44849686+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorContractRecursiveDelegationHandoffAndAdaptation","Output":"=== RUN   TestCursorContractRecursiveDelegationHandoffAndAdaptation\n"}
{"Time":"2026-10-13T09:49:25.998690194+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestCursorContractRecursiveDelegationHandoffAndAdaptation","Output":"--- PASS: TestCursorContractRecursiveDelegationHandoffAndAdaptation (0.55s)\n"}
{"Time":"2026-10-13T09:49:25.998751249+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestCursorContractRecursiveDelegationHandoffAndAdaptation","Elapsed":0.55}
{"Time":"2026-10-13T09:49:26.001405408+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest"}
{"Time":"2026-10-13T09:49:26.001467976+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"=== RUN   TestGraphDeliveryD03QualificationManifest\n"}
{"Time":"2026-10-13T09:49:27.100367027+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"--- PASS: TestGraphDeliveryD03QualificationManifest (1.10s)\n"}
{"Time":"2026-10-13T09:49:27.100414296+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Elapsed":1.1}
{"Time":"2026-10-13T09:49:27.106785363+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningOperationSchemaSeparatesMutationKinds"}
{"Time":"2026-10-13T09:49:27.10683175+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningOperationSchemaSeparatesMutationKinds","Output":"=== RUN   TestPlanningOperationSchemaSeparatesMutationKinds\n"}
{"Time":"2026-10-13T09:49:27.109298487+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningOperationSchemaSeparatesMutationKinds","Output":"--- PASS: TestPlanningOperationSchemaSeparatesMutationKinds (0.00s)\n"}
{"Time":"2026-10-13T09:49:27.109333493+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningOperationSchemaSeparatesMutationKinds","Elapsed":0}
{"Time":"2026-10-13T09:49:27.109382294+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningSchemaChoosesKindBeforePayload"}
{"Time":"2026-10-13T09:49:27.109387544+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningSchemaChoosesKindBeforePayload","Output":"=== RUN   TestPlanningSchemaChoosesKindBeforePayload\n"}
{"Time":"2026-10-13T09:49:27.115795901+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningSchemaChoosesKindBeforePayload","Output":"--- PASS: TestPlanningSchemaChoosesKindBeforePayload (0.01s)\n"}
{"Time":"2026-10-13T09:49:27.115835676+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningSchemaChoosesKindBeforePayload","Elapsed":0.01}
{"Time":"2026-10-13T09:49:27.116183039+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim"}
{"Time":"2026-10-13T09:49:27.116198087+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim","Output":"=== RUN   TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim\n"}
{"Time":"2026-10-13T09:49:27.760061851+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim","Output":"    planning_prompt_envelope_test.go:81: Complete request=61343 bytes; methods=32084, context=19820; whole events=5/8\n"}
{"Time":"2026-10-13T09:49:27.765242601+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim","Output":"--- PASS: TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim (0.65s)\n"}
{"Time":"2026-10-13T09:49:27.765294448+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestPlanningCompleteEnvelopeAdmitsWholeEventsBeforeClaim","Elapsed":0.65}
{"Time":"2026-10-13T09:49:27.771874308+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"PASS\n"}
{"Time":"2026-10-13T09:49:28.80230197+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"ok  \tswarm.local/companion/internal/engine\t3.552s\n"}
{"Time":"2026-10-13T09:49:28.802353196+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Elapsed":3.552}

```

Stderr :

```text

```
