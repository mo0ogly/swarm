# Coordination de la validation finale

Ce document décrit le candidat consolidé après T2/T3, la couverture attendue de
sa validation finale et les limites de l’organisation actuelle. Le manifeste
machine lisible associé est [`final-validation-coverage.json`](final-validation-coverage.json).
Il sert à vérifier que les mêmes exigences, composants, contrôles et preuves
restent associés au même candidat ; sa présence ne constitue ni un reçu moteur,
ni une revue indépendante, ni une acceptation.

## Ce qui existe aujourd’hui

| Élément | Comportement livré | Limite explicite |
| --- | --- | --- |
| Candidat commun | Le manifeste fixe le commit de base et les empreintes des entrées documentaires, moteur, web et tests qui couvrent `req-5` et `req-6`. Tous les lots citent le même `candidate_id`. | Le checkout est sale : l’identité fiable est le commit **plus** les empreintes, pas le commit seul. Tout changement d’une entrée impose un nouveau manifeste et invalide les contrôles ou avis dépendants. |
| Recontrôle T2 | Le CLI et le web passent par le même contrat aperçu/application. Une correction conserve la tentative et n’ajoute pas de producteur ; une précondition inchangée est refusée. | Les preuves T2/T3 restent celles de leurs rapports et recettes isolées ; elles ne remplacent pas le contrôle final du candidat consolidé. |
| Preuves T3 | CLI et web partagent la projection du verdict courant, de l’historique et des mesures. Seules les mesures réellement disponibles sont affichées ; coût et tokens restent `unknown`. | Une durée de contrôle n’est ni du CPU, ni le coût de la mission, ni une mesure de l’activité d’un fournisseur. |
| Contrôles déterministes | `tests/supervision_final_acceptance.py` enchaîne l’inventaire Go complet, vet, configuration, diff, build canonique et recettes navigateur isolées. `tests/supervision_go_suite.py` répartit l’inventaire Go sans doublon entre quatre processus, refuse une couverture vide/incomplète et exige le code 0 de chaque groupe. | Les quatre processus sont des processus de tests, pas quatre agents. Un succès local ne remplace pas un reçu moteur sur le même candidat. |
| Coordination de revue | `review-plan` enregistre des lots bornés couvrant tous les fichiers et critères, liés au candidat et à l’empreinte de preuve. Le moteur vérifie le DAG et le budget, puis le vérificateur indépendant configuré inspecte les lots séquentiellement avant une synthèse globale. | Un lot ou une inspection partielle n’accepte rien. Le plan structurel n’assure pas à lui seul la pertinence des lots et ne crée ni sous-agent autonome ni discussion entre vérificateurs. |
| Clôture | Le moteur exige une gate fraîche et une revue indépendante courante pour chaque tâche avant de clôturer une racine non déléguée. | Le superviseur, le producteur et le présent document ne peuvent pas s’auto-accepter. |

## Coordination du candidat commun

Le manifeste associe chaque exigence à ses composants, contrôles et preuves. Le
superviseur de la validation doit vérifier avant exécution : identité du candidat,
couverture sans omission, caractère déterministe des commandes et disponibilité
du budget total. Après toute correction, il invalide seulement les contrôles et
analyses qui dépendent des entrées modifiées, puis fait rejouer ces éléments sur
le candidat renouvelé. Il transmet au vérificateur indépendant le manifeste, les
sorties originales et les limites ; seul le moteur publie la décision finale.

Le dépôt fournit déjà deux mécanismes distincts :

1. la recette déterministe à quatre **processus de tests**, qui exécute les
   contrôles mécaniques sans appel de modèle ;
2. les lots de revue bornés par domaine, exécutés séquentiellement par le
   **vérificateur indépendant** configuré, avec une revue finale des interactions.

L’architecture plus large où un **superviseur de validation** délègue les analyses
qualitatives ou les anomalies à plusieurs **sous-agents** spécialisés reste une
proposition. Elle n’est pas créée par le découpage des tests ni par `review-plan`.
Si elle est implémentée ultérieurement, les sous-agents ne devront pas lancer des
commandes déterministes pour le seul motif de paralléliser : ils produiront des
analyses bornées, tandis que le superviseur conservera le manifeste et la
couverture transversale, et que le vérificateur indépendant conservera le verdict.

## RETEX historique intégré

Les faits ci-dessous proviennent des rapports historiques indiqués dans le
manifeste. **T4 ne les a pas réexécutés** ; ils sont du contexte corroboré dans le
dépôt, pas de nouveaux contrôles.

| Fait historique | Attribution et nature | Mesure et conclusion permise |
| --- | --- | --- |
| Deux contrôles finaux ont expiré après 300 s avec la même sortie et sans code de fin exploitable. | Défaillance du contrôle moteur observable ; aucune panne fournisseur démontrée. | Deux durées murales de 300 s. CPU agrégé, coût et tokens : inconnus. Un timeout ne démontre pas un défaut produit. |
| Le conducteur a relancé le même contrôle sans correction de sa précondition et a démarré une seconde tentative producteur. | Défaut de reprise/coordination du moteur. Le second producteur n’avait modifié que le rapport ; la répétition ne peut pas lui servir de preuve produit. | Relance inutile, historique et deux tentatives conservés ; aucune hausse de plafond. |
| Le garde de reprise a été corrigé afin de refuser une correction automatique après contrôle non exécuté ou sans code de fin, et de reconnaître une cause identique malgré un chemin de reçu différent. | Défaut moteur livré puis correction moteur supervisée et tests ciblés. Ce n’est pas une correction du fournisseur. | Les tests ciblés historiques ont passé ; T4 ne les rejoue pas ici. |
| La recette finale corrigée a couvert 974 tests en quatre processus, puis vet, configuration, diff, build et dix contrôles navigateur, en environ 234 s. | Contrôle déterministe supervisé, puis avis favorable du vérificateur indépendant et gate moteur dans la mission historique. | Environ 234 s de durée murale pour la recette complète. Aucun CPU agrégé, coût ou total de tokens n’est fourni. Le chiffre final de 974 remplace l’inventaire local intermédiaire de 973 mentionné avant le reçu frais. |

Cette chronologie sépare donc un **contrôle défaillant** (les timeouts), un
**défaut moteur livré puis corrigé** (le garde et la reprise identique), un
**comportement agent sans correction de cause** (le second producteur) et la
**consolidation de supervision** (diagnostic, découpage déterministe et remise au
vérificateur). Aucun de ces faits ne permet d’attribuer la cause au fournisseur.

## Séquence de validation sans boucle

1. Figer une seule fois le manifeste du candidat et vérifier mécaniquement ses
   chemins, empreintes et couvertures.
2. Exécuter une seule recette finale déterministe sous le plafond existant. Un
   échec conserve sa sortie et sa cause ; une relance exige une précondition
   concrètement modifiée.
3. Faire analyser les domaines bornés prévus par le plan de revue sur exactement
   le même candidat. Aucune analyse partielle ne vaut verdict.
4. Faire produire la synthèse par le vérificateur indépendant, puis laisser le
   moteur vérifier fraîcheur, gate et acceptation.

Une correction change le candidat : renouveler les empreintes et seulement les
preuves dépendantes avant de reprendre à l’étape appropriée. Ne pas lancer un
nouveau producteur pour compléter un dossier documentaire et ne pas confondre
silence fournisseur, processus encore vivant, contrôle fini, preuve disponible
et résultat accepté.
