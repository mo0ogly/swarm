# Examen cumulatif T7 — reçu historique du candidat 60c3dc6

**Livraison réelle de la fixture confirmée ; anomalie métrique P2 confirmée sur 60c3dc6. Cette anomalie est corrigée et requalifiée séparément sur 6bfd100 ; voir `t7-cumulative-review-6bfd100.md`.**

Moteur observé `60c3dc65edf0f1f3915414e284f58543c6814924`, fixture `w-6a2c13c498cba8780192b44d`, r127. État public : deux tâches accepted/fresh, gates valides, neuf scopes clos, root judge end cycle5. Commit projet livré `b569bb35fb5816bb806826000f99be32b973108c`. Ce commit projet et le commit moteur ont des rôles distincts ; ils ne sont jamais confondus.

Contrôles reviewer sans provider ni base privée : CLI `work show`/`mission spending` public, lecture des snapshots fournis, bundle cloné dans temporaire, modules inspectés puis assertion Python exécutée. Le bundle contient alpha42/beta7 ; le repo projet d'origine est propre au commit12237f3 et retourne encore0/0. Le dépôt Swarm n'est pas modifié par le reviewer. Les reproductions et sorties projet/public sont conservées ci-dessous.

## Finding [P2] — exécutions de contrôles gérés omises du compteur

`internal/engine/mission_insights.go:634` à 60c3dc6 filtre seulement `task.auto-validation`. `managed_integration.go:69,100` persiste le reçu de contrôle dans `managed.integrated`, sous une map par tâche. Les deux événements publics observés enregistrent trois contrôles exécutés : r96 un alpha, r106 alpha et beta. Tous PASS/exit0, timestamps distincts. Le getter mission spending renvoie pourtant0. Il s'agit d'un filtre incomplet ; le registre d'événements n'est pas absent.

Ne pas corriger par somme des derniers AutomaticValidation : ces états ne gardent que deux contrôles du candidat final et perdraient le premier contrôle alpha. Compter le résultat typé de chaque événement une fois, avec liste du chemin ordinaire/map du chemin géré, Executed=true y compris contrôle refusé ; refuser une représentation malformée et rester dans work_id. Ce défaut de mesure ne retire pas les preuves de réussite des contrôles et acceptations existantes, mais empêchait de déclarer une télémétrie cumulative exacte. Le correctif et son recontrôle sont dans le rapport6bfd100, sans effacer ce rouge.

## Limites héritées

La recette est cumulative et assistée : changement explicite de modèle, pauses/reprises, corrections de harness, du parseur d'erreurs, du schéma et du compteur ; elle a commencé sous44a86e6 et fini sous60c3dc6. Ses réservations et échecs sont conservés. Le harness original hardcode scopes3 ; les snapshots publics établissent9. Ce chiffre original n'est pas réécrit ni repris comme preuve de topologie. Les deux tâches finales portent le même ID de revue cumulative ; deux appels de revue ont été faits en tout, une première sur candidat partiel, puis une revue finale couvrant les deux tâches surb569. Un provider/role nouveau n'est pas inventé.

Ce rapport n'accepte aucune mission historique ni déploiement du service partagé. L'absence de coût rapporté reste inconnue, pas0. Les opérations correctrices humaines ne prouvent pas une récupération autonome générale. Le bilan et verdict du candidat de clôture sont repris séparément après le correctif métrique.

Finalisé 2026-10-13T08:13:54.708181+00:00.

## Contrôle public reviewer

```json
{
  "candidate": "60c3dc65edf0f1f3915414e284f58543c6814924",
  "at": "2026-10-13T07:58:23.096063+00:00",
  "command": [
    "/home/fpizzi/.cache/swarm-operator-replan-20261011/swarm-t7-wire-order",
    "--root",
    "/tmp/swarm-t7-codex-2pbm7fl4",
    "--json",
    "work",
    "show",
    "w-6a2c13c498cba8780192b44d"
  ],
  "exit": 0,
  "public_output_sha256": "440367a576f89c7991442a5db63c4b42e922ae6dfaee0f646d0b71a16d92f09f",
  "current_work_revision": 127,
  "current_gates": {
    "alpha-answer-42-code-3c8e1a": {
      "evaluation": {
        "method_version": "2",
        "scope_id": "alpha-answer-42-code-3c8e1a",
        "phase": "delivery",
        "config_digest": "2e166b58eb22c1baa4ef037bdc2a400c1f64ae4cc9d74dce82e5655c92a04f2a",
        "applicability_digest": "2e249f33c9a612f9537d4c7b872ee954805427f837ada8a972fc359b86ea031e",
        "artifacts": {
          ".swarm/managed/w-6a2c13c498cba8780192b44d/proofs/auto-9295b10d2edc3aba2613/receipt.json": "5a03de97e264f50d245e2ff62d15ae449b989c5ef7343f5226f1f1669d65f611",
          ".swarm/managed/w-6a2c13c498cba8780192b44d/proofs/auto-9295b10d2edc3aba2613/report.md": "862c8f9bbc7c6faa7ce9a758b4e51208ad6bff1cf942afd5d6c625b298d00ed2"
        },
        "domain_scores": {
          "automation": 100
        },
        "quality": 100,
        "provisional": false,
        "quality_band": "OK",
        "progress": {
          "passed": 1,
          "applicable": 1,
          "percent": 100
        },
        "max_severity": "none",
        "blockers": [],
        "allowed": true,
        "ship_allowed": true
      },
      "valid": true
    },
    "impl-beta-answer-7-4c91a2": {
      "evaluation": {
        "method_version": "2",
        "scope_id": "impl-beta-answer-7-4c91a2",
        "phase": "delivery",
        "config_digest": "24466eb876e379aaaaa9db6d7f0122db3d4121d463f3d5a2730bfdba03cedf58",
        "applicability_digest": "6cd6b87d7180ef2db1d0d635dcd798918984ec5619eaa92cbea191cb01296549",
        "artifacts": {
          ".swarm/managed/w-6a2c13c498cba8780192b44d/proofs/auto-9295b10d2edc3aba2613/receipt.json": "5a03de97e264f50d245e2ff62d15ae449b989c5ef7343f5226f1f1669d65f611",
          ".swarm/managed/w-6a2c13c498cba8780192b44d/proofs/auto-9295b10d2edc3aba2613/report.md": "862c8f9bbc7c6faa7ce9a758b4e51208ad6bff1cf942afd5d6c625b298d00ed2"
        },
        "domain_scores": {
          "automation": 100
        },
        "quality": 100,
        "provisional": false,
        "quality_band": "OK",
        "progress": {
          "passed": 1,
          "applicable": 1,
          "percent": 100
        },
        "max_severity": "none",
        "blockers": [],
        "allowed": true,
        "ship_allowed": true
      },
      "valid": true
    }
  },
  "validation": {
    "state": "validated",
    "label": "VALID\u00c9",
    "validated": 2,
    "historical": 2,
    "stale": 0,
    "tasks": {
      "alpha-answer-42-code-3c8e1a": {
        "state": "accepted",
        "fresh": true,
        "blockers": [],
        "owner": "codex-standard",
        "next": "",
        "evidence": {
          "attempt_id": "a-41226ea6356ec8ff0ebad9eb",
          "revision": 127,
          "freshness": "fresh",
          "observed_at": "2026-10-13T07:50:25.393728005Z",
          "report_review": {
            "state": "passed",
            "attempt_id": "a-41226ea6356ec8ff0ebad9eb",
            "reviewer": "reviewer://codex-standard",
            "at": "2026-10-13T07:50:25.227202315Z",
            "limits": [
              "La revue examine le candidat, les sources fournies et les re\u00e7us de contr\u00f4les ; elle n\u2019ex\u00e9cute pas elle-m\u00eame les commandes et ne vaut pas acceptation."
            ]
          },
          "controls": {
            "state": "passed",
            "items": [
              {
                "id": "check-1ec957ce50ea947be88f4e01",
                "attempt_id": "a-41226ea6356ec8ff0ebad9eb",
                "execution": "executed",
                "revision": "unknown",
                "candidate_sha": "b569bb35fb5816bb806826000f99be32b973108c",
                "command": [
                  "python3",
                  "-B",
                  "-c",
                  "import alpha; assert alpha.answer()==42"
                ],
                "exit_code": 0,
                "started_at": "2026-10-13T07:50:10.022411615Z",
                "finished_at": "2026-10-13T07:50:10.04982034Z",
                "wall_duration_ms": 27,
                "cpu_duration_ms": 26,
                "cost": "unknown",
                "tokens": "unknown",
                "freshness": "fresh",
                "result": "passed",
                "limits": [
                  "Sortie non expos\u00e9e ; seule son empreinte SHA-256 est conserv\u00e9e. Plafond 65536 octets."
                ]
              }
            ],
            "history": [],
            "longest_measured": [
              {
                "id": "check-1ec957ce50ea947be88f4e01",
                "attempt_id": "a-41226ea6356ec8ff0ebad9eb",
                "execution": "executed",
                "revision": "unknown",
                "candidate_sha": "b569bb35fb5816bb806826000f99be32b973108c",
                "command": [
                  "python3",
                  "-B",
                  "-c",
                  "import alpha; assert alpha.answer()==42"
                ],
                "exit_code": 0,
                "started_at": "2026-10-13T07:50:10.022411615Z",
                "finished_at": "2026-10-13T07:50:10.04982034Z",
                "wall_duration_ms": 27,
                "cpu_duration_ms": 26,
                "cost": "unknown",
                "tokens": "unknown",
                "freshness": "fresh",
                "result": "passed",
                "limits": [
                  "Sortie non expos\u00e9e ; seule son empreinte SHA-256 est conserv\u00e9e. Plafond 65536 octets."
                ]
              }
            ]
          },
          "acceptance": {
            "state": "accepted",
            "revision": "unknown",
            "at": "unknown"
          },
          "limits": [
            "Les valeurs unknown ne constituent pas un succ\u00e8s.",
            "R\u00e9vision d\u2019ex\u00e9cution inconnue pour cet ancien re\u00e7u."
          ]
        }
      },
      "impl-beta-answer-7-4c91a2": {
        "state": "accepted",
        "fresh": true,
        "blockers": [],
        "owner": "codex-standard",
        "next": "",
        "evidence": {
          "attempt_id": "a-98610e7d814f52ebbdef106a",
          "revision": 127,
          "freshness": "fresh",
          "observed_at": "2026-10-13T07:50:25.394077101Z",
          "report_review": {
            "state": "passed",
            "attempt_id": "a-98610e7d814f52ebbdef106a",
            "reviewer": "reviewer://codex-standard",
            "at": "2026-10-13T07:50:25.227202315Z",
            "limits": [
              "La revue examine le candidat, les sources fournies et les re\u00e7us de contr\u00f4les ; elle n\u2019ex\u00e9cute pas elle-m\u00eame les commandes et ne vaut pas acceptation."
            ]
          },
          "controls": {
            "state": "passed",
            "items": [
              {
                "id": "check-f5c6abf044d8389671435662",
                "attempt_id": "a-98610e7d814f52ebbdef106a",
                "execution": "executed",
                "revision": "unknown",
                "candidate_sha": "b569bb35fb5816bb806826000f99be32b973108c",
                "command": [
                  "python3",
                  "-B",
                  "-c",
                  "import beta; assert beta.answer()==7"
                ],
                "exit_code": 0,
                "started_at": "2026-10-13T07:50:10.050484228Z",
                "finished_at": "2026-10-13T07:50:10.077411609Z",
                "wall_duration_ms": 26,
                "cpu_duration_ms": 26,
                "cost": "unknown",
                "tokens": "unknown",
                "freshness": "fresh",
                "result": "passed",
                "limits": [
                  "Sortie non expos\u00e9e ; seule son empreinte SHA-256 est conserv\u00e9e. Plafond 65536 octets."
                ]
              }
            ],
            "history": [],
            "longest_measured": [
              {
                "id": "check-f5c6abf044d8389671435662",
                "attempt_id": "a-98610e7d814f52ebbdef106a",
                "execution": "executed",
                "revision": "unknown",
                "candidate_sha": "b569bb35fb5816bb806826000f99be32b973108c",
                "command": [
                  "python3",
                  "-B",
                  "-c",
                  "import beta; assert beta.answer()==7"
                ],
                "exit_code": 0,
                "started_at": "2026-10-13T07:50:10.050484228Z",
                "finished_at": "2026-10-13T07:50:10.077411609Z",
                "wall_duration_ms": 26,
                "cpu_duration_ms": 26,
                "cost": "unknown",
                "tokens": "unknown",
                "freshness": "fresh",
                "result": "passed",
                "limits": [
                  "Sortie non expos\u00e9e ; seule son empreinte SHA-256 est conserv\u00e9e. Plafond 65536 octets."
                ]
              }
            ]
          },
          "acceptance": {
            "state": "accepted",
            "revision": "unknown",
            "at": "unknown"
          },
          "limits": [
            "Les valeurs unknown ne constituent pas un succ\u00e8s.",
            "R\u00e9vision d\u2019ex\u00e9cution inconnue pour cet ancien re\u00e7u."
          ]
        }
      }
    }
  },
  "control_events": [
    {
      "event": "integrated-auto-3c659d24398001a7ef3a",
      "revision": 96,
      "kind": "managed.integrated",
      "candidate": "2d12762c794a5402b482dff1cc2bf21f359bf4e1",
      "controls": {
        "alpha-answer-42-code-3c8e1a": [
          {
            "id": "check-1ec957ce50ea947be88f4e01",
            "command": [
              "python3",
              "-B",
              "-c",
              "import alpha; assert alpha.answer()==42"
            ],
            "executed": true,
            "passed": true,
            "exit_code": 0,
            "output_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "summary": "contr\u00f4le r\u00e9ussi",
            "started_at": "2026-10-13T07:49:41.337029055Z",
            "finished_at": "2026-10-13T07:49:41.357790104Z",
            "wall_duration_ms": 20,
            "cpu_duration_ms": 20
          }
        ]
      }
    },
    {
      "event": "integrated-auto-9295b10d2edc3aba2613",
      "revision": 106,
      "kind": "managed.integrated",
      "candidate": "b569bb35fb5816bb806826000f99be32b973108c",
      "controls": {
        "alpha-answer-42-code-3c8e1a": [
          {
            "id": "check-1ec957ce50ea947be88f4e01",
            "command": [
              "python3",
              "-B",
              "-c",
              "import alpha; assert alpha.answer()==42"
            ],
            "executed": true,
            "passed": true,
            "exit_code": 0,
            "output_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "summary": "contr\u00f4le r\u00e9ussi",
            "started_at": "2026-10-13T07:50:10.022411615Z",
            "finished_at": "2026-10-13T07:50:10.04982034Z",
            "wall_duration_ms": 27,
            "cpu_duration_ms": 26
          }
        ],
        "impl-beta-answer-7-4c91a2": [
          {
            "id": "check-f5c6abf044d8389671435662",
            "command": [
              "python3",
              "-B",
              "-c",
              "import beta; assert beta.answer()==7"
            ],
            "executed": true,
            "passed": true,
            "exit_code": 0,
            "output_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "summary": "contr\u00f4le r\u00e9ussi",
            "started_at": "2026-10-13T07:50:10.050484228Z",
            "finished_at": "2026-10-13T07:50:10.077411609Z",
            "wall_duration_ms": 26,
            "cpu_duration_ms": 26
          }
        ]
      }
    }
  ],
  "spending_controls": 0,
  "spending_output_sha256": "15e3a2f0cc5dbdc5c8d0fe6e77d77d026b6249eea7fdae3a6e7e02b4add1aa7f"
}
```

## Bundle et effet observé par reviewer

```json
{
  "at": "2026-10-13T07:59:27.430617+00:00",
  "bundle_sha256": "e64199d24f1f8e34469cfde061715cb989bfb83e6997bc69add9384e88830ed4",
  "bundle_heads_original": "b569bb35fb5816bb806826000f99be32b973108c refs/heads/swarm-result\nb569bb35fb5816bb806826000f99be32b973108c HEAD\n",
  "delivered_candidate": "b569bb35fb5816bb806826000f99be32b973108c",
  "changed_files": [
    "alpha.py",
    "beta.py",
    "docs/alpha-answer-42-code-3c8e1a.delivery.json",
    "docs/alpha-answer-42-code-3c8e1a.md",
    "docs/impl-beta-answer-7-4c91a2.delivery.json",
    "docs/impl-beta-answer-7-4c91a2.md"
  ],
  "sources": {
    "alpha.py": "def answer():\n    return 42\n",
    "beta.py": "def answer():\n    return 7\n"
  },
  "report_sha256": {
    "docs/alpha-answer-42-code-3c8e1a.delivery.json": "0374168df2095d0c564fefbd8498fa1a499193c92134664cb85b59b368ee3675",
    "docs/alpha-answer-42-code-3c8e1a.md": "b21794e58ee4df5997cae875f57d50091fa450dc6e8633dd71539bc7e67cf75b",
    "docs/impl-beta-answer-7-4c91a2.delivery.json": "6f2dd6903fa76207011675d4f7b00cd6db7c2f0b8691a1a999a1defacceeb268",
    "docs/impl-beta-answer-7-4c91a2.md": "862c8f9bbc7c6faa7ce9a758b4e51208ad6bff1cf942afd5d6c625b298d00ed2"
  },
  "independent_python_exit": 0,
  "independent_python_stdout": "alpha=42 beta=7\n",
  "independent_python_stderr": "",
  "source_repository": {
    "head": "12237f3054868454d7c1ed415f49a84217245b9b",
    "status": "",
    "alpha": "def answer():\n    return 0\n",
    "beta": "def answer():\n    return 0\n"
  }
}
```

## Parallélisme et usages publics

```json
{
  "parallel_planner_snapshot": [
    {
      "file": "/home/fpizzi/.cache/swarm-operator-replan-20261011/t7-real-codex-44a86e6/snapshots/52-72deb6d051a3.json",
      "sha256": "72deb6d051a37c71e19516555a737ab966a827b6736127c0774fea9a2ff509c9",
      "revision": 52,
      "simultaneous_held_scopes": [
        {
          "id": "alpha-owner",
          "generation": 7,
          "lease_until": "2026-10-13T07:29:02.533040961Z",
          "last_model": {
            "level": "standard",
            "model": "gpt-5.6-sol",
            "effort": "medium",
            "billing": "external",
            "policy_hash": "cc05fc883f7a7ab1768743a76794fb504005839a9b3addbb2e4230e69172703f",
            "reason": "Politique du fournisseur \u00b7 planning \u00b7 sans mont\u00e9e en gamme automatique"
          }
        },
        {
          "id": "beta-answer-7",
          "generation": 8,
          "lease_until": "2026-10-13T07:29:03.761753139Z",
          "last_model": {
            "level": "standard",
            "model": "gpt-5.6-sol",
            "effort": "medium",
            "billing": "external",
            "policy_hash": "cc05fc883f7a7ab1768743a76794fb504005839a9b3addbb2e4230e69172703f",
            "reason": "Politique du fournisseur \u00b7 planning \u00b7 sans mont\u00e9e en gamme automatique"
          }
        }
      ]
    }
  ],
  "worker_overlap_seconds": 53.148968,
  "public_spending_qualified_totals": {
    "recorded_calls": 44,
    "calls_without_usage": 8,
    "reported_input_fields_sum": 1392836,
    "reported_output_fields_sum": 29750,
    "observed_worker_tools": 20,
    "reported_cost_rows": 0,
    "unreported_cost_rows": 44,
    "worker_retries": 0
  }
}
```
