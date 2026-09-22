# Méthodes de travail pour Codex, Claude et Swarm

[English](en/AGENT-METHODS.md) · [Guide utilisateur](../GUIDE-UTILISATEUR.md)

Ce dépôt fournit **huit méthodes** pour cadrer, réaliser, vérifier et améliorer
Swarm. Elles reprennent les pratiques utiles des méthodes APEX et audit PDCA de
LIA : critères explicites, décisions tracées, preuves observables, reprise bornée
et retour d'expérience. Elles sont réécrites pour ce dépôt, sans dépendance à
l'installation LIA, ses hooks, ses scripts de déploiement ou ses permissions.

## Démarrer

Ouvrez Codex ou Claude **dans ce dépôt**, puis utilisez la commande adaptée :

| Besoin | Codex | Claude |
| --- | --- | --- |
| Analyser, planifier et réaliser un changement | `$apex` | `/apex` |
| Auditer avec des preuves reproductibles | `$audit-pdca` | `/audit-pdca` |
| Rédiger un besoin et ses critères vérifiables | `$spec-builder` | `/spec-builder` |
| Trouver les trous d'un plan | `$spec-audit` | `/spec-audit` |
| Examiner le code et ses risques | `$code-reviewer` | `/code-reviewer` |
| Vérifier le parcours après une correction | `$verify-fix` | `/verify-fix` |
| Reprendre un plan bloqué | `$replan` | `/replan` |
| Comprendre les résultats et prioriser la suite | `$retex-analyzer` | `/retex-analyzer` |

Exemples à saisir dans la conversation :

```text
$apex --plan-only Définir une reprise après quota, avec les mêmes règles web et CLI.
$audit-pdca Auditer l'acceptation des résultats ; rapport seulement, sans correction.
$audit-pdca --fix Corriger les défauts confirmés dans ce périmètre et rejouer les contrôles.
$verify-fix Vérifier que le refus de lancement ne crée ni agent ni réservation.
$replan Reprendre le plan existant à partir de l'échec observé, en conservant ses limites.
```

Dans Claude, remplacez `$` par `/`. Les indications `--plan-only`, `--resume`,
`--fix` et `--score-only` sont des consignes pour la méthode, pas de nouvelles
options de l'exécutable `swarm`. L'audit est en lecture seule par défaut.
Les méthodes répondent dans la langue de la demande.

## Depuis l'interface Swarm

Dans **Préparer avec l'IA**, la liste des méthodes existante utilise :

| Méthode de préparation | Contenu fourni à l'IA |
| --- | --- |
| APEX | Analyse et planification du besoin |
| KS — cadrer une fonctionnalité | Cadrage, critères et décomposition en tâches |
| Audit PDCA | Définition du périmètre, des risques et du plan de contrôle |

Le fichier de méthode et le contrat commun sont transmis intégralement au modèle.
Les phases d'exécution des méthodes restent interdites dans cette préparation :
elle propose un brief ou un plan, sans lancer ni accepter des agents.
Les cinq autres méthodes s'utilisent dans les sessions natives Codex/Claude ;
elles ne sont pas cinq nouveaux boutons dans le menu de préparation.

Le CLI expose le même catalogue :

```sh
swarm --root "$PWD" --json prepare methods
```

Chaque entrée indique `available` et une empreinte `sha256`. `audit_pdca` reste
un alias de `audit-pdca` dans Swarm et `/audit_pdca` dans Claude. Les anciennes
commandes `/ks-feature` et `/ks-plan` sont conservées.

La méthode de préparation est lue dans **la racine de projet utilisée par Swarm**. Installer le
binaire seul ne copie pas cette configuration dans d'autres projets. En Docker,
le projet monté comme racine doit contenir ces fichiers, liens compris. Ne pointez
pas par erreur vers une ancienne copie du dépôt.

## Cadrage automatique des agents par le moteur

Pour les nouvelles tentatives, le moteur inclut les méthodes adaptées au rôle dans
le contexte envoyé au fournisseur. Il ne dépend pas de la découverte des commandes
par Codex ou Claude pour fournir ce cadrage.

| Rôle | Méthodes incluses | Limites |
| --- | --- | --- |
| Planificateur et sous-planificateur | APEX, audit-pdca, spec-builder, spec-audit, replan | PLAN et ACT ; sans outils ni modification de code |
| Exécutant | APEX, audit-pdca, verify-fix, modèles de rapport et de suivi | DO et CHECK dans le périmètre autorisé ; audit seul si demandé |
| Vérificateur indépendant | audit-pdca, code-reviewer | CHECK sur les preuves fournies ; sans outils ni modification du candidat |

Le champ `workflow` conserve la version, le rôle, la liste des méthodes et leur
empreinte SHA-256. Le moteur refuse un rôle inconnu ou un cadrage dépassant sa
limite. Les méthodes sont **embarquées à la compilation** : modifier celles d'un
espace exécutant ne remplace pas les consignes du moteur. Une modification de ce
pack exige une reconstruction du binaire pour les futures tentatives.

Les préparations continuent à lire leurs méthodes dans le projet. Les anciennes
tentatives ne reçoivent pas rétroactivement ces métadonnées. Le champ prouve le
cadrage construit, pas l'obéissance du modèle. Les contrôles exécutables restent
nécessaires.

Le [protocole de communication](AGENT-COMMUNICATION.md) détaille les rapports
complets transmis aux planificateurs, les reçus de contexte et les décisions liées
aux retours, ainsi que leurs limites.

## Une source commune, deux points d'entrée

```text
AGENTS.md                         règles chargées pour Codex
CLAUDE.md                         point d'entrée pour Claude
.claude/skills/<nom>/SKILL.md      source canonique des huit méthodes
.agents/skills/<nom>              lien relatif vers la même méthode
.claude/skills/<nom>/agents/      noms et exemples du sélecteur Codex
.claude/commands/                 trois commandes de compatibilité
tools/agent-workflows/CONTRACT.md règles communes, également lues par Swarm
```

Les liens restent internes au dépôt et se transportent avec Git. Les copies qui
ne préservent pas les liens symboliques doivent les rétablir avant utilisation.
Modifiez la source canonique, puis lancez :

```sh
python3 tools/agent-workflows/check.py
go test ./... -run 'TestRepositoryPreparation|TestPreparationMethodCatalogue|TestPreparationDialogueLateProposalAndMethodDrift' -count=1
git diff --check
```

Une modification des méthodes ou du contrat change leur empreinte. Une préparation
ancienne doit appliquer la nouvelle version et refaire sa validation ; les preuves
précédentes ne deviennent pas actuelles par simple changement de fichier.
Si une méthode n'apparaît pas dans une session native, vérifiez la racine, les liens
et les éventuelles méthodes personnelles du même nom, puis rouvrez la session.

## Ce que cette configuration garantit — et ses limites

Les méthodes demandent des responsabilités distinctes : le planificateur organise,
l'exécutant réalise, le vérificateur indépendant examine, le moteur accepte selon
ses contrôles. Une auto-revue dans la conversation de l'auteur reste une auto-revue.
Le contrat interdit de transformer une limite de tentatives, un quota ou une preuve
manquante en réussite en changeant les règles.

Ces instructions guident l'IA ; **elles ne remplacent pas les verrous du moteur**.
Un test avec un faux fournisseur ne démontre pas une mission autonome réelle.
Une méthode n'accorde pas d'autorisation supplémentaire de délégation, publication
ou déploiement. Le modèle et les identifiants restent ceux du fournisseur/session
configuré ; aucun modèle ni mode de permissions n'est imposé par ce pack.

L'organisation planificateurs/exécutants s'inspire des travaux de Cursor. La revue
indépendante et les conditions d'acceptation sont le contrat propre de Swarm ;
ce pack ne constitue pas une certification Cursor.

## Formats et sources

- Adaptation locale : méthodes APEX, audit-pdca, spec-builder, spec-audit,
  code-reviewer, verify-fix, replan et retex-analyzer de la configuration LIA.
  Les chemins machine, règles produit et automatismes de publication sont exclus.
- [Documentation Codex : découverte des skills](https://learn.chatgpt.com/docs/build-skills).
- [Documentation Claude : skills de projet et commandes](https://code.claude.com/docs/en/skills).
- [Contrat partagé de Swarm](../tools/agent-workflows/CONTRACT.md).

### Budget local et poursuite des autres périmètres

Un sous-planificateur qui atteint son plafond d'activations conserve ses retours
non traités et son historique. Le conducteur ne le rappelle plus et poursuit les
périmètres dont le budget, ainsi que celui de leurs parents, reste disponible.
Ce plafond local ne doit pas créer une panne globale de planification. La
réservation transactionnelle vérifie de nouveau les limites avant chaque appel.
Si aucun périmètre ne peut agir, les sondages ne consomment aucun appel et ne
modifient pas la mission. Un budget global épuisé, un fournisseur indisponible ou
une véritable panne de planification restent des conditions distinctes ; cette
règle ne les efface pas et n'ajoute aucune tentative.

### Ordre des retours en attente

Le conducteur choisit le propriétaire du plus ancien événement non traité dans
l’ordre durable de la boîte de réception. Un message de reprise récent au
coordinateur ne dépasse donc plus les résultats déjà reçus par un sous-planificateur.
Les périmètres fermés, réservés, au plafond ou en attente d’intégration restent
exclus du départ ; leurs événements sont conservés. La réservation revérifie les
limites avant l’appel. Cet ordre ne clôture aucun périmètre à la place du
planificateur et ne rend pas de budget aux tentatives précédentes.

### Retour au coordinateur après clôture d’un enfant

Le contexte du coordinateur distingue les exigences encore possédées des exigences
déléguées. `task_capacity_remaining` indique des places de création disponibles,
pas des tâches restant à terminer. `descendant_validation` transmet les identités
des tâches descendantes, leur état, la fraîcheur de leur acceptation calculée par
le moteur et l’identité/révision de leur revue éventuelle. Un statut « accepted »
sans preuves actuelles produit `accepted_fresh: false`. Ce résumé ne remplace pas
une revue indépendante : il permet au coordinateur de proposer la clôture ; le
moteur contrôle de nouveau toutes les preuves avant de l’appliquer.

### Reprise exceptionnelle après livraison incomplète

Après trois tentatives consommées, `planning authorize-recovery` permet une seule
reprise explicitement confirmée par l’opérateur. Pour un résultat refusé avant la
revue payante, fournir `result_commit` (SHA du résultat immuable) à la place de
`review_id`, avec `attempt_id`, `confirm_recovery: true`, motif et nouvelle consigne.
Le moteur vérifie l’attribution Git, la dernière tentative arrêtée, le refus de
complétude réel et la disponibilité du vérificateur. Il conserve les trois essais,
ajoute au plus une tentative, ne lance aucun agent lors de l’autorisation et
n’accorde aucune validation. Les contrôles et la revue du résultat corrigé restent
obligatoires. Une seconde dérogation, un résultat remplacé ou une revue déjà rendue
sur ce résultat sont refusés par cette voie. L’opération est disponible par le CLI
et l’API publique ; le formulaire existant de refus de revue reste inchangé.

### Budget d’un lancement préparé

La reprise explicite par `agent resume-launch` conserve l’identité et la copie préparées. Si le nombre d’appels d’outils demandé dépasse le plafond du fournisseur inchangé, le moteur réduit ce nombre au plafond autorisé. Un budget plus petit reste inchangé. Le manifeste initial conserve le budget demandé ; la tentative enregistre le budget réellement appliqué. Les autres paramètres, la révision du travail, les preuves et l’identité du fournisseur restent contrôlés. Une double reprise retourne la même tentative.

### Consulter un stockage créé par un ancien moteur

Les commandes courantes de consultation (`work show/list`, `planning show`, `mission status/preview`, `agent show/list/logs/prepared`, listes et historiques de préparation, état des espaces et des échanges) refusent désormais de migrer implicitement un stockage plus ancien. Elles retournent `storage_upgrade_required`, sans changer sa version. Utiliser le CLI correspondant au serveur actif pour consulter cette mission. Pour une mise à niveau explicite, arrêter d’abord les anciens processus, puis lancer `swarm init` avec le nouveau CLI. Les commandes de mutation et le démarrage du serveur conservent leur mécanisme de migration existant : ne pas mélanger leurs versions sur un même stockage.
