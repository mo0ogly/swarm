# APEX · Fichier d’actions par tâche

Base qualifiée en production locale `v0.1.1-dev.405`. Le lot porte sur la lecture
des tâches de Conduite et sur les observations natives des tentatives.

| ID | Critère de réalisation | Propriétaire |
| --- | --- | --- |
| A1 | Chaque tâche existante, y compris historique ou sans tentative, expose un fichier JSON de plan et de suivi. La lecture et l’export partagent la même projection protégée. | Moteur et API |
| A2 | Les tentatives natives enregistrent les appels d’outils reconnus et leur résultat observé. Le journal garde leur attribution, leur ordre et le signal de couverture partielle. Il conserve la prochaine action et les reprises enregistrées. | Moteur |
| A3 | La carte ouvre un journal de tâche avec son plan, ses tentatives, ses ajustements et son export. Une action ouvre ses propres détails, avec navigation clavier et retour du focus. | Conduite |
| A4 | Une tâche en mode humain attend l’acceptation de l’opérateur ; cette attente et le résultat des autres politiques sont affichés selon l’état enregistré. Une lecture ou un export ne démarre ni tentative ni validation. | Moteur et Conduite |
| A5 | Le parcours est vérifié sur binaire de livraison, en FR/EN et clair/sombre. Les refus de lecture et les missions historiques sont couverts. La version locale, la révision et les limites sont relues après activation. | Coordinateur |

## Analyse et choix

Le `ApprovedPlan` conserve le plan approuvé et son rapprochement avec les tâches.
La définition de tâche, sa prochaine action, les tentatives et leurs reprises
sont des entrées moteur persistées. Le flux natif rapproche déjà les débuts
et résultats d’outils pour la garde. Le reçu de fin persiste avec la tentative
et possède un parcours de reprise. La capture brute peut être désactivée,
tronquée ou contenir des entrées privées, et n’est pas un journal d’actions.

Écarter les fichiers écrits par les agents dans leurs espaces projet. Ils
dupliqueraient le stockage et n’établiraient ni l’attribution des observations
ni la complétude en cas d’interruption. Le moteur produit un JSON téléchargeable
par tâche à partir de sa définition et des reçus natifs. Les nouveaux reçus
retiennent des catégories d’outils et leurs résultats observés ; les sorties,
arguments, identifiants du fournisseur et raisonnements privés ne font pas
partie du fichier. Un ancien reçu peut manquer de détails. Une lecture en cours
de tentative explicite la disponibilité des observations conservées.

## Ordre et vérification

1. Étendre les reçus sans changer l’admission ni les limites configurées.
   Vérifier les résultats appariés, les réémissions et les interruptions.
2. Construire la projection et la route protégée. Lier travail, tâche et
   tentative, dériver les résultats seulement des reçus attribués et propager
   les refus. Vérifier projet invalide et aucun effet sur les révisions.
3. Raccorder carte, modales et export. Essayer tâche neuve, historique,
   exécution avec actions, arrêt, reprise, états vides et erreurs.
4. Exercer le mode humain par une soumission de fixture puis son acceptation
   explicite. Réutiliser les gardes moteur existantes. Vérifier les ressources
   embarquées, conserver les preuves et livrer le binaire local.

Le lot part de 0/5 critères vérifiés. Les missions en cours, les coûts et les
acceptations existantes restent sous leurs droits et politiques effectifs.
