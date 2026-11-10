# Traçabilité : de l’analyse au développement du moteur

## Décision du 10 octobre 2026

L’utilisateur demande de corriger le manque de traçabilité, de documenter son
utilité, ses contraintes, les responsabilités des superviseurs et le travail
simultané des agents, avec Mermaid. Il précise que cette analyse doit servir à
construire le développement. Le [contrat FR](../../../../architecture/TRACEABILITY.md)
et sa [version EN](../../../../en/TRACEABILITY.md) deviennent les références de
travail des lots existants, avec six schémas chacun et repères TR-1 à TR-6.

Ces repères précisent les critères adoptés ; ils ne créent pas de nouvelles gates,
ne changent pas le graphe ni les budgets. Les consignes publiques des lots à venir
doivent porter ces attentes, leur provenance et les chemins des références.
Le relevé JSON adjacent conserve les événements effectivement appliqués.
Les consignes T3 à T7 sont appliquées par `task update`, révisions 65 → 70,
avec chemins et empreintes des références. Une remise complémentaire à T2,
révision 70 → 71, donne les originaux de la reproduction et ses limites pour
le diagnostic après revue. Ce sont des interventions opérateur enregistrées ;
leur application ne prouve pas encore leur lecture ou usage par un responsable.

## Construction des lots et preuves demandées

| Lot | Utilisation de l’analyse | Résultat à vérifier |
| --- | --- | --- |
| T2 | Distinguer jugement d’objectif, revue de candidat et contrôle exécuté | Le juge ne prend pas une déclaration pour une preuve fraîche |
| T3 | TR-2/TR-3/TR-4 : contexte utile des responsables et périmètres concurrents | Propriétaire exact, événements distincts, décision persistée, refus de révision périmée |
| T4 | TR-1/TR-3/TR-4/TR-5 : remises, exécutions et reprise | Originaux attribués, même candidat contrôlé/publié, conservation du refus et des consommations |
| T5 | TR-2/TR-5 : consignes et parcours publics des politiques | API/CLI/Admin FR/EN cohérents ; réglages persistés et provenance visible |
| T6 | TR-1/TR-6 : protocole et mesures | Arguments, dates, sorties et codes réels ; manques, reprises, attente centrale, coût connu/inconnu |
| T7 | TR-6 : recette indépendante avec fournisseur réel | Démonstration de l’utilisation par les superviseurs, autonomie et intervention humaine séparées |

Les [consignes communes](../../../../../tools/agent-workflows/CONTRACT.md) et le
[modèle de remise](../../../../../tools/agent-workflows/templates/HANDOFF.md)
exigent désormais des traces d’exécution attribuées et des originaux. Cela
améliore la demande faite aux agents ; cela ne constitue pas une garantie moteur.
Les prochains candidats doivent intégrer ces documents et leurs vérifications.

## Observation T2, distincte d’un verdict final

Mission `w-ca937edf4e0abd85f8d3d67b`, tâche `plan-ca937edf4e-T2`, tentative
`a-5fff6319655d0b5b9b34413a`, producteur `auto-0f7f61a183068fd63528`.
La revue `review-8a5b4d26457e4456353da385` porte sur le candidat
`a27e175eb4f3e38f9cede1c888956a5ca5a0310f`.

Le premier fragment examine code et tests du juge, sans signaler de défaut de
code dans ce fragment. Il ne recoupe pas les résultats locaux annoncés avec les
reçus moteur : le contrôle Go autorisé est une suite générique, sans les nouveaux
`TestJudge*`/`TestConfigureJudge*`. La revue finale était encore en cours au
début de ce relevé. Un fragment `unknown` ne constitue pas le verdict final.

Une reproduction opérateur est préparée dans une copie temporaire isolée du
candidat. Un premier lancement avec l’option invalide `-timeout15m` échoue
avant les tests ; il reste enregistré comme tel. La relance utilise
`-timeout 15m`, avec arguments, stdout/stderr, dates, code de sortie et empreintes.
Un contrôle ciblé du juge utilise également `-run ^Test(Judge|ConfigureJudge)`.
Le contrôle ciblé retourne 0 : six tests de premier niveau réussissent, soit
13 résultats PASS avec leurs sous-cas. Les [sorties originales](evidence/20261010-T2-traceability/judge-tests.jsonl)
et le [relevé d’exécution](evidence/20261010-T2-traceability/judge-execution.json)
sont conservés. La suite complète s’interrompt ensuite : le lanceur retourne
143, sans relevé final du sous-processus Go. Les traces partielles contiennent
979 PASS, huit SKIP et un FAIL de premier niveau, celui du manifeste historique
`TestGraphDeliveryD03QualificationManifest`. Ce n’est pas un bilan de suite
complète. Le [relevé de l’interruption](evidence/20261010-T2-traceability/interrupted-suite-observation.json)
conserve les empreintes des originaux temporaires et les limites ; les
[événements de l’échec observé](evidence/20261010-T2-traceability/suite-failure-events.jsonl)
restent consultables. Le dernier test démarré est une corrélation, pas une cause
établie de l’interruption. Ces exécutions locales ne sont pas des reçus du
contrôleur et ne prouvent pas une acceptation.
Les résultats observés figurent dans le relevé adjacent.
Le dernier test démarré réussit ensuite seul, avec un processus Go placé dans
une session distincte : code 0, [sortie originale](evidence/20261010-T2-traceability/interruption-diagnostic.jsonl)
et [conditions exactes](evidence/20261010-T2-traceability/interruption-diagnostic-execution.json)
conservées. Cette vérification ne reproduit pas l’interruption de la suite
complète et n’en établit pas la cause.

La lecture publique des événements du producteur confirme des annonces de
commandes et un compteur « résultat d’outil reçu », mais ne restitue pas ici
les stdout/stderr originaux ni le code du processus Go. Les événements 104945
et 104968 annoncent des pipelines `go test … 2>&1 | tail …`. Une telle commande
peut borner les sorties et masquer le code Go si celui-ci n’est pas conservé
séparément ; le relevé public ne permet pas d’établir cette conservation.
L’[extrait original des événements](evidence/20261010-T2-traceability/producer-activity-excerpt.jsonl)
est attribué par agent, séquence et date. Il démontre le besoin de TR-1 ; il ne
prouve pas que les tests annoncés ont réussi ou échoué.

Une réparation du dossier doit fournir les originaux utiles plutôt que répéter
une déclaration. Si un nouveau contrôle moteur est requis, l’autoriser et le
rejouer par un parcours public pris en charge. Aucun candidat ni preuve en revue
n’est modifié ; aucune réussite anticipée, remise à zéro ou hausse de plafond.

## Vérification de la documentation et étude à mener

Douze schémas FR/EN sont analysés et rendus en SVG dans Chromium avec Mermaid ;
les diagrammes décrivent les responsabilités et scénarios, sans mesurer la
concurrence de cette mission. Les liens locaux et le diff sont contrôlés.

Pour une étude comparative, figer avant collecte objectifs, politiques, scénarios,
fournisseurs et répétitions. Comparer dossier déclaratif, traces d’exécution
attribuées et reçus contrôleur ; mesurer inconnus justifiés, acceptations sans
preuve suffisante, attribution des causes et coût de reprise. Le présent cas
après incident fournit une hypothèse et un défaut de dossier à traiter. Il
n’ajoute pas de résultat aux campagnes historiques du dossier de recherche.
## Mise à jour de la solution — contrôle avant transmission et reprise

Le retour utilisateur « cette erreur on l'a déjà eue » identifie une récidive de
méthode. Le dossier correctif T2 avait été annoncé prêt avant de mesurer l'union
des sources : **137 112 octets**, au-delà des **131 072** autorisés. La correction
est désormais effectuée dans la copie isolée puis dans la reprise publique :
**12 sources, 117 913 octets**. Les originaux du rejeu juge et l'observation de la
suite interrompue sont conservés ; la sélection exclut les textes complets des
miroirs EN déjà couverts par T1, dont les ajouts T2 restent examinables dans le diff.
Aucun critère ni code du juge n'est supprimé.

Le contrôle doit intervenir à deux niveaux : l'opérateur vérifie le dossier avant
d'annoncer sa disponibilité ; le moteur réutilise `managedReviewSources` dans
`recoverResult`, avant de remplacer le résultat/refus et les références de reprise.
Ce contrôle commun évite de recopier ses capacités dans un autre validateur.
Une union trop grande est refusée sans mutation publique, nouvelle tentative ou
appel de revue. Une correction effective peut ensuite passer les contrôles et
la revue ordinaires. Le précontrôle concerne les sources de la tâche réparée ;
il ne remplace pas la vérification cumulative du transport après intégration.

La vérification a découvert un problème lié : les ajouts aux consignes dépassaient
la capacité du cadrage embarqué de planificateur, sous-planificateur et worker.
Les fichiers individuels étaient valides, mais le contexte composé ne l'était
pas. La première réduction n'a corrigé que le worker ; cet échec est conservé.
Le contrat commun compacté fait **7 202 octets**, avec les obligations replan,
autorécursion, droits, historique, originaux et vérification conservées.
`tools/agent-workflows/check.py` appelle maintenant le test réel
`TestAgentWorkflowRolesAreBounded`, sans fournisseur. Les quatre rôles passent.
La garde existante de 32 000 octets porte sur le corps du cadrage avant ajout de
ses métadonnées : les prompts complets observés font 32 172, 32 178, 31 493 et
17 387 octets respectivement. Cette correction ne prétend pas avoir rendu ces
capacités configurables dans l'Admin ; l'intégration des politiques reste dans T5.

**Vérification locale : 22 tests de premier niveau, 46 PASS avec sous-tests,
aucun échec, sortie 0.** La nouvelle régression couvre soumission et révision :
deux sources individuellement admissibles mais trop grandes ensemble conservent
exactement état, résultat, refus et compteurs ; après correction des fichiers,
les contrôles ordinaires et le fournisseur simulé permettent l'acceptation.
Les sorties originales avant/après et les métadonnées sont dans
[le dossier de preuve](evidence/20261010-recovery-capacity/regressions-after-execution.json).
Les sept Mermaid des documents de preuve FR/EN ont été analysés et rendus sans
erreur. Ces tests de protocole ne constituent pas une acceptation indépendante
de T2 par un fournisseur réel.

**Effet réel dans Swarm :** la revue précédente s'est interrompue sur `429`
à 19:49:30 UTC, sans verdict sémantique final. L'opération publique
`planning revise-recovered-result`, événement `T2-traceability-repair-20261010`,
a enregistré la correction externe à la révision 81. Le nouveau candidat est
`56ca8cab5bfde7dc9ddf7a7a5d2169231d87c9a9` ; les trois blobs de code du juge sont
identiques à ceux du candidat reproduit. Le reçu moteur atteste **14 contrôles
PASS**, sept pour T1 et sept pour T2. Les originaux du juge sont des preuves
opérateur distinctes de ces contrôles génériques. **T2 reste bloqué/non accepté**,
la revue indépendante attend le délai Claude jusqu'à **21:20 UTC / 23:20 Paris**.
La commande retourne `provider_cooldown` après l'enregistrement et les contrôles,
et non un échec d'enregistrement. Les neuf appels de revue et l'unique tentative
T2 restent consommés ; l'ancien avis est conservé dans l'événement de reprise.
[Observation publique attribuée](evidence/20261010-recovery-capacity/public-recovery-observation.json).

Le nouveau contrôle de reprise est **vérifié localement, pas installé dans le serveur
actif**. Le correctif `recovery-source-preflight-20261010.patch` a été remis à T5
par `task update` à la révision 82 : instruction en tête, 533 caractères, antérieures
conservées. T5 doit le fusionner avec les correctifs précédents et reprendre le
contrat compact actuel, puis vérifier l'installation et les parcours publics.
La réception du texte public ne prouve pas la compréhension future du modèle.

Condition concrète de reprise : délai fournisseur réellement expiré, candidat et
preuves toujours intacts, puis revue fraîche selon la voie publique existante.
Ne pas effacer le quota, répéter la revue pendant le délai, rembourser les appels
ou fabriquer une acceptation. La remise corrigée et ses contrôles ont été exécutés
pendant cette dépendance externe ; le blocage n'a pas été laissé au seul journal.


## Mise à jour du 11 octobre — protocole strict et reprise effective

La récidive suivante a été diagnostiquée et corrigée : citation exacte refusée
à tort, événement terminal mal attribué et relance du producteur proposée pour
une erreur du vérificateur. Le protocole moteur distingue désormais cette erreur
d’un refus sémantique et impose la reprise de revue dans les droits du propriétaire.
Le changement de méthode exige une nouvelle revue complète, avec originaux et
appels conservés. La suite Go complète passe ; le binaire actif est vérifié et
T2 a repris publiquement sur le même candidat, sans nouvelle production.

[Diagnostic, correction, vérification et limites de la solution](20261011-agent-engine-protocol.md).
La reprise indépendante reste en cours à cette observation ; T2 n’est pas accepté
et la confirmation UX a été confiée explicitement à T5. Les observations
historiques ci-dessus décrivent l’état du 10 octobre et restent inchangées.
