# Revue indépendante du candidat 7e9d024

**Verdict : approve. Aucun finding ouvert dans le périmètre examiné. Les findings F1, F2 et F2b sont résolus. Cette approbation du candidat ne vaut ni recette réelle cumulative terminée, ni acceptation native, ni déploiement.**

Candidat exact : `7e9d02490a1da0366cbe1759c2e1c525f9cae298`, checkout `/home/fpizzi/Documents/ChatGPT/swarm` propre au début et à la fin. Delta examiné depuis `61f82bda2b4990c3552ec6bf7d76ab25304c02fe`, continuité avec les revues indépendantes de `44a86e6` et `61f82bd` conservées. Revue autorisée en lecture seule : aucun source modifié, aucun autre agent, provider réel, état privé de mission native ou service partagé utilisé. Les méthodes `replan` et `code-reviewer`, déjà lues, restent appliquées à la reprise.

## Résultats

| Critère | Observation | Résultat |
| --- | --- | --- |
| F1 aperçu/application navigateur | Validation commune de la représentation avant !preview, `len(raw)+1` couvre le saut de ligne ; refus sans écriture et relecture couverts par rouge/vert de la correction 61f82bd, source inchangée depuis | Résolu |
| F2 cause sûre | Champs directs/nested, libellés constants capacité/schéma/generic ; propagation du résultat err même si exit non nul ; aucun texte privé reflété par cette classification | Résolu |
| F2b non-classification outil | Prédicat à assist_run.go:162 limité à error, turn.failed ou result/is_error=true ; tool_result direct et nested ignorés ; sources et tests examinés, contrôle intégré reviewer passé | Résolu |
| Données privées | Tests PRIVATE_SECRET/URL/generic et schéma rejeté ne reflètent pas le texte ; la correction ne recopie aucun payload libre | PASS local sur chemin modifié |
| Repli/retry implicite | Le delta modifie seulement le filtre, ses régressions, les docs FR/EN et manifeste ; aucune mutation de route/budget/réservation ni nouvelle boucle de retry | Aucun nouveau comportement de ce type |
| Documentation FR/EN | PROVIDER-LIVENESS décrit les trois enveloppes exactes ; WEB-REQUEST-POLICY garde la validation commune et l'absence de mutation | Cohérent avec les sources |
| Manifeste | Qualification du manifeste courant, hashes des parties/inputs, stale/missing/incomplete guards exécutés ; statut source snapshot, jamais acceptation indépendante | PASS |

Le défaut F2b était reproduit sur 61f82bd par le reviewer et par le rouge producteur conservé. Le nouveau rouge `t7-review-tool-envelope-red.jsonl` contient les quatre causes fournisseur déjà PASS puis FAIL sur l'événement outil. Le reçu vert `t7-review-tool-envelope-green.jsonl` contient **16 tests de premier niveau / 23 avec sous-cas**, tous PASS en -race, incluant les contrats de cooldown, conservation consommation et refus de lancement/reprise. Ces runs sont ceux du producteur et restent identifiés ainsi.

Le reviewer a exécuté le contrôle ciblé intégré suivant sur le candidat courant : **3 tests de premier niveau / 7 avec sous-cas, -race, exit 0** : `TestStructuredProviderFailurePreservesSafeCause`, `TestStructuredSchemaRejectionDiagnostic`, `TestGraphDeliveryD03QualificationManifest`. Les originaux JSONL et timestamps sont ci-dessous. Les variantes outil direct/nested sont des assertions du premier test sans noms de sous-cas supplémentaires ; le dénominateur 7 n'en invente pas. Aucune suite entière et aucun des 41/77 contrôles critiques inchangés n'a été relancé. Ces précédentes vérifications restent preuves historiques des contrats inchangés, distinguées des nouveaux contrôles de ce delta.

`git diff --check` passe ; HEAD exact et absence de dirty diff confirmés après les tests. Le binaire fourni `swarm-t7-final` a été inspecté en lecture seule par `go version -m` : Go 1.24.3, `vcs.revision=7e9d02490a1da0366cbe1759c2e1c525f9cae298`, `vcs.modified=false`. Son SHA-256 est `2ccdb420a4296af40c1ae9143ceaefd25816cf8d64370e1183f6308641385924`. Cela lie le binaire à ce candidat ; cela ne prouve pas qu'un serveur partagé le sert ou que la recette réelle est terminée.

## Limites et prochaine frontière

L'approbation porte sur le candidat et les corrections examinées, avec le contexte des deux revues antérieures. Les lacunes T7 de recette réelle cumulative, coût/tokens/cache et effets natifs ne sont pas remplacées par un PASS de tests. Les résumés opérateur signalent des cycles, exploration et deux sous-planificateurs réels ; cette revue finale de delta n'a pas encore reçu/examiné leur dossier de livraison cumulatif complet. L'opérateur/root doit poursuivre la recette sur le binaire identifié, préserver les consommations/échecs/interventions et transmettre les originaux publics pour examiner les critères restant non démontrés. Le serveur et la mission historiques ne sont ni acceptés ni relancés par cet avis. Le RETEX final reste dépendant de cette frontière réelle, sous l'autorité applicable.

Sources libérées : aucun check reviewer actif. Une modification pertinente ultérieure demande une vérification/revue adaptée au delta ; cet avis appartient à ce SHA précis.

## Empreintes examinées

- `internal/engine/assist_run.go` : `e9823ae05045a8717dbc967fccba2874e9b4885c8b404aaa8bc2011d7775aa4e`.
- `internal/engine/planning_runner.go` : `c22af213b08cd084c9a29c255bf1284347f3bfe707d121ce4cf6601505c2c834`.
- `internal/engine/web_requests_config.go` : `7e06c63b5c2739eeb2e6b7d3823368c7e484e436803411bfed5de7a5424751eb`.
- `internal/engine/structured_provider_failure_test.go` : `cad4eb6ba476b3befd579c1efa88dbef4bfe5d30b5a7edeba96c6201a25e01d3`.
- `docs/engine-cursor-source-manifest.json` : `51482879e78c9dfbd9804e63f802ea9a5040bc0f1c27238f764d51244c4d48b9`.
- `docs/plans/engine-cursor-contract/evidence/t7-20261013/t7-review-tool-envelope-red.jsonl` : `9a38d04ecaf3d06f1811bd5b0ea3833e1ec8c2ae97b26426103b9cf68857feab`.
- `docs/plans/engine-cursor-contract/evidence/t7-20261013/t7-review-tool-envelope-green.jsonl` : `8ba144cec44fb2f27d0f759fe2a4d0436cf2d048bfa4a2fb6278b071798a3949`.

Rapport finalisé 2026-10-13T07:28:57.913883+00:00.

## Contrôles originaux reviewer

```json
{
  "candidate": "7e9d02490a1da0366cbe1759c2e1c525f9cae298",
  "started": "2026-10-13T07:27:11.484270+00:00",
  "finished": "2026-10-13T07:27:26.696325+00:00",
  "command": [
    "go",
    "test",
    "-race",
    "./internal/engine",
    "-run",
    "^(TestStructuredProviderFailurePreservesSafeCause|TestStructuredSchemaRejectionDiagnostic|TestGraphDeliveryD03QualificationManifest)$",
    "-count=1",
    "-timeout=90s",
    "-json"
  ],
  "exit": 0,
  "passed_top_level": 3,
  "passed_with_subcases": 7,
  "passed_names": [
    "TestGraphDeliveryD03QualificationManifest",
    "TestStructuredProviderFailurePreservesSafeCause/message",
    "TestStructuredProviderFailurePreservesSafeCause/nested",
    "TestStructuredProviderFailurePreservesSafeCause/code",
    "TestStructuredProviderFailurePreservesSafeCause/unknown",
    "TestStructuredProviderFailurePreservesSafeCause",
    "TestStructuredSchemaRejectionDiagnostic"
  ],
  "failed": []
}
```

```jsonl
{"Time":"2026-10-13T09:27:22.930129293+02:00","Action":"start","Package":"swarm.local/companion/internal/engine"}
{"Time":"2026-10-13T09:27:23.140587103+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest"}
{"Time":"2026-10-13T09:27:23.140645112+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"=== RUN   TestGraphDeliveryD03QualificationManifest\n"}
{"Time":"2026-10-13T09:27:25.617187915+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Output":"--- PASS: TestGraphDeliveryD03QualificationManifest (2.48s)\n"}
{"Time":"2026-10-13T09:27:25.617234773+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestGraphDeliveryD03QualificationManifest","Elapsed":2.48}
{"Time":"2026-10-13T09:27:25.620573079+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause"}
{"Time":"2026-10-13T09:27:25.620615859+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause","Output":"=== RUN   TestStructuredProviderFailurePreservesSafeCause\n"}
{"Time":"2026-10-13T09:27:25.620704035+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/message"}
{"Time":"2026-10-13T09:27:25.620712962+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/message","Output":"=== RUN   TestStructuredProviderFailurePreservesSafeCause/message\n"}
{"Time":"2026-10-13T09:27:25.624743739+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/message","Output":"--- PASS: TestStructuredProviderFailurePreservesSafeCause/message (0.00s)\n"}
{"Time":"2026-10-13T09:27:25.624806137+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/message","Elapsed":0}
{"Time":"2026-10-13T09:27:25.624818139+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/nested"}
{"Time":"2026-10-13T09:27:25.624822938+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/nested","Output":"=== RUN   TestStructuredProviderFailurePreservesSafeCause/nested\n"}
{"Time":"2026-10-13T09:27:25.62766406+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/nested","Output":"--- PASS: TestStructuredProviderFailurePreservesSafeCause/nested (0.00s)\n"}
{"Time":"2026-10-13T09:27:25.627704225+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/nested","Elapsed":0}
{"Time":"2026-10-13T09:27:25.627737588+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/code"}
{"Time":"2026-10-13T09:27:25.627743759+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/code","Output":"=== RUN   TestStructuredProviderFailurePreservesSafeCause/code\n"}
{"Time":"2026-10-13T09:27:25.630379284+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/code","Output":"--- PASS: TestStructuredProviderFailurePreservesSafeCause/code (0.00s)\n"}
{"Time":"2026-10-13T09:27:25.630441571+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/code","Elapsed":0}
{"Time":"2026-10-13T09:27:25.630564552+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/unknown"}
{"Time":"2026-10-13T09:27:25.630576425+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/unknown","Output":"=== RUN   TestStructuredProviderFailurePreservesSafeCause/unknown\n"}
{"Time":"2026-10-13T09:27:25.633378001+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/unknown","Output":"--- PASS: TestStructuredProviderFailurePreservesSafeCause/unknown (0.00s)\n"}
{"Time":"2026-10-13T09:27:25.633427474+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause/unknown","Elapsed":0}
{"Time":"2026-10-13T09:27:25.63379767+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause","Output":"--- PASS: TestStructuredProviderFailurePreservesSafeCause (0.01s)\n"}
{"Time":"2026-10-13T09:27:25.633822597+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredProviderFailurePreservesSafeCause","Elapsed":0.01}
{"Time":"2026-10-13T09:27:25.633994911+02:00","Action":"run","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredSchemaRejectionDiagnostic"}
{"Time":"2026-10-13T09:27:25.634010571+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredSchemaRejectionDiagnostic","Output":"=== RUN   TestStructuredSchemaRejectionDiagnostic\n"}
{"Time":"2026-10-13T09:27:25.637113805+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredSchemaRejectionDiagnostic","Output":"--- PASS: TestStructuredSchemaRejectionDiagnostic (0.00s)\n"}
{"Time":"2026-10-13T09:27:25.637157206+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Test":"TestStructuredSchemaRejectionDiagnostic","Elapsed":0}
{"Time":"2026-10-13T09:27:25.637732147+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"PASS\n"}
{"Time":"2026-10-13T09:27:26.655978948+02:00","Action":"output","Package":"swarm.local/companion/internal/engine","Output":"ok  \tswarm.local/companion/internal/engine\t3.725s\n"}
{"Time":"2026-10-13T09:27:26.656020807+02:00","Action":"pass","Package":"swarm.local/companion/internal/engine","Elapsed":3.726}

```

Stderr :

```text

```
