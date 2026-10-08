# Brouillon — section 4 : modèle

Statut : brouillon du 5 octobre 2026. Les noms de tests cités existent dans
l'arbre de travail de `racine du dépôt swarm` à cette date ; plusieurs sont dans
des fichiers non commités (colonne « État git » du tableau 3). La correspondance
invariant → test est établie par le nom du test et la conception ; elle doit être
confirmée par lecture du corps de chaque test au gel de la version mesurée, et
peut changer avec le passage au modèle hiérarchique de Swarm.

---

## 4. Modèle

Cette section définit les acteurs, l'état manipulé par le moteur, les fautes
considérées et les propriétés que le moteur doit garantir. Nous distinguons les
invariants du moteur, qui portent sur son propre état, des propriétés
d'exécution de l'effet, qui portent sur l'action irréversible elle-même : les
confondre conduirait à attribuer au moteur une protection qui relève en réalité
de la clé d'idempotence.

### 4.1 Acteurs et base de confiance

Nous considérons quatre acteurs.

- **Les agents** proposent : plans, livrables, lots de paiements, explications.
  Ils ne sont pas dignes de confiance : leur sortie peut être fausse,
  incomplète, répétée ou périmée. Ils ne détiennent aucune capacité sur
  l'effet irréversible.
- **Le moteur** est un programme déterministe. Il est le seul à transformer une
  proposition en action autorisée (lancement d'une tentative) et un processus
  terminé en résultat accepté.
- **Le composant d'exécution de l'effet** (dans notre scénario, le règlement)
  est déterministe et non LLM. Il est le seul détenteur de la capacité de
  paiement, matérialisée par un jeton que l'API de paiement exige.
- **L'humain** fixe l'objectif, le périmètre et la politique de validation, et
  arbitre les exceptions que le moteur lui renvoie.

La base de confiance comprend le moteur, le composant d'exécution, le stockage
transactionnel du moteur et l'API de paiement, dont on suppose qu'elle exécute
correctement une demande et déduplique par clé d'idempotence. Cette séparation
reprend l'idée du moniteur de référence [Anderson 1972] : une médiation
complète, inviolable par les sujets qu'elle contrôle, et assez petite pour être
vérifiée. Elle relève aussi du motif Plan-Then-Execute [Beurer-Kellner 2025],
appliqué à un effet financier.

### 4.2 État

L'état du moteur pour un travail comprend :

- un ensemble de **tâches** reliées par un graphe de dépendances acyclique ;
- pour chaque tâche, une suite de **tentatives**, chacune avec un identifiant,
  un état (en attente, en cours, terminée, échouée, interrompue) et un espace de
  travail réservé ;
- des **candidats** : les livrables produits par une tentative, identifiés par
  leur empreinte cryptographique ;
- des **preuves** : résultats de contrôles, chacun lié à une tentative et à
  l'empreinte du candidat examiné ;
- des **budgets** par tâche (tentatives, appels d'outils, coût lorsque le
  fournisseur le rapporte) ;
- une **révision** globale, incrémentée à chaque mutation, et un **journal**
  d'événements identifiés.

Toute mutation est une requête portant un identifiant d'événement `event_id`
et la révision attendue `expected_revision`. Une tâche est **acceptée** lorsque
le moteur constate que tous ses critères sont couverts par des preuves valides.

### 4.3 Modèle de fautes

Nous considérons des fautes **non adversariales** : celles que produit
l'exploitation ordinaire d'un système distribué et d'agents faillibles.

| Faute | Description |
| --- | --- |
| F1 | Réponse perdue après un effet, suivie d'un nouvel essai |
| F2 | Deux processus de conduite actifs sur le même travail |
| F3 | Arrêt brutal entre l'exécution d'un effet et son enregistrement |
| F4 | Candidat modifié après sa validation, une fois l'exécution de l'effet lancée |
| F4e | Candidat modifié après sa validation, avant le lancement de l'exécution de l'effet |
| F5 | Indisponibilité persistante (plusieurs appels consécutifs) d'un service nécessaire au contrôle |
| F6 | Agent qui boucle et épuise son budget |
| F7 | Processus de conduite figé au-delà de son bail pendant une tentative en cours, puis réveillé |
| F8 | Remise d'une tentative ancienne et rejetée coexistant avec celle d'une tentative plus récente |

Les attaques par injection de prompt et les agents délibérément malveillants
sont hors du périmètre de cet article ; les défenses par construction comme
CaMeL [Debenedetti 2025] leur sont dédiées et sont compatibles avec notre
séparation des rôles. Les attaques contre la reprise étudiées par Zheng et al.
[Zheng 2026] et Wu et al. [Wu 2026] recoupent F1, F3 et F7 lorsqu'elles sont
déclenchées volontairement ; nous les traitons ici comme des fautes.

### 4.4 Invariants du moteur

Le moteur doit maintenir les huit invariants suivants, quelles que soient les
fautes F1 à F8 et F4e.

- **I1 — Effet unique par intention.** Deux requêtes portant le même `event_id`
  produisent au plus une mutation ; la seconde renvoie le résultat de la
  première.
- **I2 — Pas de mutation sur un état périmé.** Une requête dont
  `expected_revision` diffère de la révision courante est refusée sans effet.
- **I3 — Acceptation liée au candidat.** Une tâche n'est acceptée que si chacun
  de ses critères est couvert par une preuve produite par la tentative
  courante, sur le candidat dont l'empreinte est la sienne au moment de
  l'acceptation. Une preuve devient périmée dès que le candidat change.
- **I4 — Lancement atomique et exclusif.** Le lancement d'une tentative est
  atomique, idempotent, et réserve un espace de travail qu'aucune autre
  tentative active ne recouvre.
- **I5 — Limite atteinte, jamais succès.** L'épuisement d'un budget, ou
  l'impossibilité de lire la consommation, bloque la tâche ; il ne peut pas
  produire une acceptation.
- **I6 — Reprise justifiée.** Après un échec attribué à l'environnement, une
  nouvelle tentative exige une preuve nouvelle que la cause est levée, et
  conserve les limites déjà consommées.
- **I7 — Aucun texte de modèle exécuté.** Les contrôles sont des commandes
  structurées, tirées d'une liste de programmes autorisés et exécutées sans
  interpréteur de commandes ; aucun texte produit par un modèle (plan,
  rapport, réponse) n'est interprété comme une commande.
- **I8 — Confirmation de ce qui a été montré.** Une modification soumise à
  confirmation humaine n'est appliquée que si la confirmation porte sur
  l'aperçu exact présenté ; tout changement intervenu entre-temps impose un
  nouvel aperçu.

### 4.5 Propriétés d'exécution de l'effet

Les invariants du moteur ne suffisent pas à garantir qu'un virement part une
seule fois : I1 porte sur les mutations du moteur, pas sur les requêtes que le
composant d'exécution adresse à la banque. Nous exigeons donc trois propriétés
supplémentaires du composant d'exécution.

- **E1 — Capacité exclusive.** Seul le composant d'exécution détient la
  capacité d'effet ; une requête de paiement sans le jeton correspondant est
  refusée par l'API.
- **E2 — Clé d'intention métier.** La clé d'idempotence de chaque effet est
  calculée à partir de l'identité métier de l'intention (ici, fournisseur et
  numéro de facture), jamais à partir de la tentative ni du texte de la
  requête. Une même intention rejouée, par la même tentative ou par une autre,
  porte donc la même clé.
- **E3 — Exécution du candidat validé.** Le composant d'exécution n'exécute
  que le candidat dont l'empreinte est celle qui a été validée et remise par
  le moteur ; une différence d'empreinte arrête l'exécution sans effet.

E2 répond au constat de Zheng et al. : une clé dérivée de la requête rejouée
n'est pas stable lorsque l'émetteur est un LLM [Zheng 2026]. Ici, l'émetteur
n'est pas un LLM, et la clé ne dépend pas de la requête. E3 prolonge I3
jusqu'à l'effet : sans elle, un candidat modifié après validation (F4) pourrait
être exécuté sous le couvert d'une preuve qui ne le concerne pas.

### 4.6 Correspondance avec les fautes

Le tableau 2 indique, pour chaque faute, les propriétés censées l'arrêter. Il
exprime une hypothèse de conception ; la section 6 la confronte aux mesures.

**Tableau 2 — Fautes et propriétés censées les arrêter (hypothèses).**

| Faute | Propriétés | Équivalent dans un processus de paiement |
| --- | --- | --- |
| F1 Réponse perdue + nouvel essai | E2 (et I1 pour les mutations du moteur) | Clé d'idempotence de paiement |
| F2 Deux conducteurs | I4, I2 | Verrou sur l'instruction de paiement |
| F3 Arrêt entre effet et enregistrement | E2, I1 | Rapprochement avant relance |
| F4 Candidat modifié après lancement de l'effet | E3 | Contrôle de l'ordre au moment de l'exécution |
| F4e Candidat modifié avant lancement de l'effet | I3 | Validation liée à la version de l'ordre |
| F5 Service indisponible | I6 | Reprise après incident documentée |
| F6 Budget épuisé | I5 | Plafond, arrêt en position sûre |
| F7 Conducteur figé puis réveillé | I4, I2 | Mise à l'écart d'un opérateur dont le mandat a expiré |
| F8 Résultat ancien présenté | I3 | Rejet d'une instruction supplantée |
| — | I7 | Séparation entre instruction et exécution |
| — | I8 | Contrôle à quatre yeux |

### 4.7 Ce que le modèle ne garantit pas

Le modèle ne garantit pas la vérité métier des preuves : un contrôle mal écrit
validera un lot faux avec régularité. Il ne garantit pas l'exhaustivité des
critères déclarés, ni la justesse des règles : déterministe ne signifie pas
correct. Il ne garantit pas non plus la conformité d'une suite d'actions au-delà
des dépendances déclarées, au sens des violations compositionnelles de Kurady
et al. [Kurady 2026]. Enfin, E2 suppose que l'API d'effet déduplique
correctement par clé ; un fournisseur qui ne le fait pas exige une
réconciliation que ce modèle ne couvre pas.

Ces limites définissent la question QR3 : lorsqu'un échec survient, il faut
pouvoir distinguer une faute de l'agent, de l'environnement, du moteur ou d'une
règle mal calibrée.

### 4.8 Spécification formelle (option)

I1, I2 et I4 se prêtent à une vérification par modèle sur de petites instances
(TLA+ ou Alloy) : requêtes concurrentes, rejeu, révisions. Cette vérification
n'est pas réalisée à ce stade ; elle renforcerait l'article et constitue une
décision ouverte.

---

## Annexe B (provisoire) — invariants et tests de Swarm

État au 5 octobre 2026, arbre de travail de `racine du dépôt swarm`. État git :
`commité` = fichier inchangé depuis le dernier commit ; `M` = fichier commité
mais modifié localement ; `??` = fichier non suivi. Seuls les tests d'un commit
figé pourront être cités dans la version soumise.

**Tableau 3 — Tests associés aux invariants.**

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

## Points à vérifier avant soumission

1. Lire le corps de chaque test du tableau 3 et confirmer qu'il exerce bien
   l'invariant annoncé, au commit figé de Swarm.
2. Après le passage au modèle hiérarchique, vérifier I3 et E3 sur le chemin de
   remise réel (passation au responsable de mission plutôt qu'échange).
3. Référence exacte du rapport d'Anderson (ESD-TR-73-51, 1972) à vérifier.
4. Vérifier les noms des tests du banc cités pour E1–E3 au gel du banc.
