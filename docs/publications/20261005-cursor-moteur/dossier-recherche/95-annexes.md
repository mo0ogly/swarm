# Annexe A. Invariants et tests du moteur

État au 5 octobre 2026, arbre de travail de `racine du dépôt swarm`. État git :
`commité` = fichier inchangé depuis le dernier commit ; `M` = fichier commité
mais modifié localement ; `??` = fichier non suivi. Seuls les tests d'un commit
figé pourront être cités dans la version soumise.

**Tableau A.1 — Tests associés aux invariants (état au 5 octobre 2026, à figer sur un commit).**

| Invariant | Test | Fichier | État git |
| --- | --- | --- | --- |
| I1 | `TestIdempotencyAndRevision` | `companion_test.go` | M |
| I1 | `TestConductorReplayHasNoDoubleEffect` | `conductor_test.go` | commité |
| I1 | `TestExchangeHostConcurrentSendHasOneEffect` | `agent_exchange_host_test.go` | ?? |
| I2 | `TestConcurrentWriters` | `companion_test.go` | M |
| I2 | `TestGatePreviewRevisionConflictVisible` | `gate_dialog_test.go` | commité |
| I3 | `TestAcceptanceFreshnessAndAttempts` | `companion_test.go` | M |
| I3 | `TestAutomaticValidationStaleEvidenceBlocksDependency` | `automatic_validation_test.go` | ?? |
| I3 | `TestExchangeChangedArtifactBecomesStale` | `agent_exchange_test.go` | ?? |
| I4 | `TestLaunchAtomicIdempotentAndWorkspaceExclusive` | `agents_test.go` | M |
| I4 | `TestDurableCoordinatorFencesExpiredOwnerAtLaunchTransaction` | `durable_coordinator_test.go` | ?? |
| I5 | `TestBudgetReservationReplayAndExhaustion` | `budgets_test.go` | commité |
| I5 | `TestCostReadFailureStopsDispatchAndDecisions` | `dispatcher_test.go` | M |
| I6 | `TestEnvironmentRetryRequiresNewEvidenceAndKeepsLimits` | `environment_retry_test.go` | ?? |
| I7 | `TestAutomaticValidationNeverExecutesCommandsFromAITextAndHonorsPause` | `automatic_validation_test.go` | ?? |
| I7 | `TestValidationPolicyRejectsShellAndUnboundedControls` | `automatic_validation_test.go` | ?? |
| I8 | `TestOperatorPreview` | `activity_description_test.go` | commité |
| I8 | `TestConductorExactPreviewAndIdempotencyIsolation` | `assist_conductor_test.go` | commité |
| Atomicité (support de I1–I4) | `TestCrashRollsBack` | `companion_test.go` | M |

E1, E2 et E3 ne sont pas des propriétés de Swarm mais du composant d'exécution
du banc (`benchmarks/billing/bench/settle.py` et `payment_api.py`), couvertes
par ses tests : jeton exigé (`test_token_is_required_when_configured`), clé
métier et rejeu (`test_lost_response_business_key_one_payment`), empreinte
(`test_digest_mismatch_pays_nothing`).

# Annexe B. Protocole d'attribution des blocages (QR3)

## B.1 Question

QR3 : quand une tentative échoue ou qu'une tâche se bloque, le moteur attribue-t-il la faute
au bon responsable : l'agent, l'environnement, la règle, ou le moteur lui-même ? Une
attribution fausse a un coût direct sur un flux financier : une indisponibilité prise pour une
faute de l'agent déclenche une régénération du livrable (défaut D3) au lieu d'une attente.

## B.2 Taxonomie

Cinq classes proposées pour l’attribution opérationnelle, en regard de MAST (Cemri et al.,
2025), qui inclut déjà les défauts de conception du système. Elles ne remplacent pas MAST :

| Classe | Définition |
|---|---|
| Agent | L'agent a produit un livrable ou un comportement fautif (candidat modifié, rapport ancien relayé, sortie invalide). |
| Environnement | Une cause extérieure à l'agent et au moteur : service indisponible, processus tué, réponse perdue, écriture concurrente d'un tiers. |
| Règle | Une règle juste mais mal calibrée a arrêté un travail correct (budget trop bas, délai trop court). |
| Moteur | Un défaut du moteur a causé ou aggravé l'échec. |
| Inconnu | Les traces ne permettent pas de trancher. Classe assumée, jamais utilisée par défaut. |

Une unité peut recevoir une classe principale et, au plus, une classe aggravante (exemple :
environnement, aggravée par le moteur).

À part la cause, chaque unité reçoit un résultat : arrêt correct (la faute n'a produit aucun
effet faux) ou non. *Amendement du 6 octobre 2026 :* ce résultat remplace la classe « faute
injectée correctement arrêtée » de la version initiale du protocole, qui mêlait cause et résultat.

## B.3 Corpus

- **Source.** Un lot S séparé de la campagne principale, dont les racines sont conservées
  (la campagne principale supprime celles des exécutions OK) : clé métier, les dix cas
  (aucune faute, F1 à F8, F4e), 10 graines (1000 à 1009), soit 100 exécutions.
- **Unité d'annotation.** Chaque tentative terminée autrement que `completed` avec validation
  acceptée, chaque tâche passée `blocked`, chaque `planning.failure`. Les exécutions sans
  aucune de ces unités ne contribuent pas au corpus.
- **Pièces fournies aux annotateurs.** Pour chaque unité : journal de la tentative, diagnostic
  de tentative et motif de blocage enregistrés par le moteur, événements adressés au
  responsable, contrôles exécutés et leurs codes de sortie. Le nom de la faute injectée, la
  graine et la condition sont retirés.
- **Limite de l'insu.** Certaines traces révèlent la faute (un 503 dans la sortie d'un
  contrôle, un code 137). L'insu est donc partiel ; nous le signalons sans le corriger.

## B.4 Vérité de référence

*Précision méthodologique du 8 octobre 2026, avant constitution du corpus :* l’injection
identifie une perturbation contrôlée, pas nécessairement la cause effective de chaque unité.
La table suivante donne des hypothèses d’attribution, à vérifier sur les traces avant annotation.
Une faute injectée par le harnais ne prouve pas qu’un agent réel l’aurait produite. Les causes
initiales, aggravantes et indémontrables doivent être distinguées ; les corrections de référence
sont publiées, sans réécrire les résultats historiques.

| Faute injectée | Classe principale attendue | Remarque |
|---|---|---|
| F1 réponse perdue | Environnement | |
| F2 lanceurs concurrents | Environnement | écriture concurrente d'un tiers |
| F3 arrêt brutal (137) | Environnement | |
| F4, F4e candidat modifié | Agent | |
| F5 service indisponible | Environnement | aggravée par le moteur si la tâche est régénérée (D3) |
| F6 budget épuisé | Règle | budget volontairement trop bas |
| F7 propriétaire expiré | Environnement | |
| F8 rapport ancien relayé | Agent | |
| tout cas, `planning.failure` sur SQLITE_BUSY | Moteur | défaut D5 |
| aucune faute | sans objet | toute unité produite est examinée comme faux positif |

## B.5 Diagnostic automatique comparé

Le moteur classe chaque échec de reprise dans une catégorie (`recovery.go`) : `transient`,
`environment`, `conflict`, `business`, `unknown`. Correspondance fixée d'avance :

| Catégorie du moteur | Classe |
|---|---|
| `transient`, `environment`, `conflict` | Environnement |
| `business` | Agent |
| `unknown` | Inconnu |

Le moteur n'a pas de catégorie pour la règle ni pour lui-même. Son rappel sur ces deux classes
est donc nul par construction ; nous le rapportons comme un constat, au même titre que
la nécessité de confronter cette attribution aux défauts de conception décrits par MAST.

## B.6 Annotation

*Amendement du 6 octobre 2026, avant constitution du corpus : aucun second annotateur humain
n'est disponible.*

- **Annotateur humain unique** : l'auteur étiquette toutes les unités avec la taxonomie de 7.2,
  après une séance d'étalonnage sur dix unités tirées hors corpus, exclues des mesures.
  L'accord entre annotateurs humains n'est pas mesuré ; l'article le déclare. Si un second
  annotateur humain devient disponible avant l'analyse, il étiquette le même corpus selon la
  même procédure et la mesure 1 est rétablie.
- **Annotateur LLM, mesure secondaire** : un modèle différent de celui des agents mesurés
  (`claude-opus-5-5`, contre `claude-sonnet-5` pour les agents du lot réel), sans outils, reçoit
  pour chaque unité les mêmes pièces que l'annotateur humain (7.3) et la même consigne écrite
  (définitions de 7.2, une classe principale, au plus une classe aggravante, et « inconnu »
  autorisé). Consigne, modèle, version du client et réponses brutes sont publiés. Il étiquette
  indépendamment : ni l'auteur ne voit ses étiquettes avant d'avoir fini, ni le modèle ne voit
  celles de l'auteur.
- Le modèle n'est pas un second annotateur humain : son accord avec l'auteur est rapporté comme
  une mesure distincte (sur le modèle de MAST, qui rapporte séparément l'accord humain et celui
  d'un juge LLM), jamais comme κ inter-annotateurs.
- La vérité par construction (7.4) reste la référence principale ; aucune étiquette n'est
  corrigée après comparaison.

## B.7 Mesures

1. Accord entre annotateurs humains : κ de Cohen sur la classe principale, avec intervalle à
   95 % par bootstrap (1 000 rééchantillonnages, graine fixée). *Non mesuré tant qu'un seul
   annotateur humain est disponible.*
2. Diagnostic du moteur contre la vérité de référence : précision et rappel par classe,
   matrice de confusion, taux d'`unknown`.
3. Auteur contre la vérité de référence, et annotateur LLM contre la vérité de référence :
   exactitude, séparément, pour vérifier que la taxonomie est applicable à partir des seules
   traces.
4. Accord auteur et annotateur LLM : κ de Cohen avec le même intervalle, rapporté comme accord
   humain–modèle, et liste des désaccords.
5. Cas où le moteur est fautif ou aggravant : nombre, défaut en cause (D1 à D5), et part de
   ces cas que le diagnostic du moteur attribue à l'agent.

Aucune mesure n'est agrégée en score unique. Avec environ une centaine d'unités, les
intervalles seront larges ; QR3 est une étude exploratoire, pas un test d'hypothèse.
