# RD3 — observations historiques et nouvelle décision

## Décision d’architecture

Une observation locale ne constitue ni une validation de tâche, ni une preuve que
ses dépendances sont inchangées. Le protocole 3 conserve cette distinction.

Deux approches ont été comparées :

1. **Certification automatique de fermeture des dépendances.** Nécessite un graphe
   complet et vérifiable couvrant code, configuration, tests et fichiers générés.
   Un graphe partiel ou une déclaration libre du modèle ne suffisent pas. Cette
   certification n’est pas implémentée et n’est pas revendiquée.
2. **Références historiques explicites et nouvel examen des impacts.** Le moteur
   conserve des observations dont toutes les entrées sont identiques. Il oblige
   ensuite le vérificateur actuel à examiner le delta intégral et à résoudre une
   question d’impact pour chaque groupe historique. Approche retenue.

Cette décision révise la proposition initiale RD3 : aucune ancienne inspection
n’est convertie en inspection actuelle. Les dépendances présentes dans les pièces
vues sont contrôlées mécaniquement ; les conséquences hors de ce périmètre restent
une obligation du nouvel examen indépendant. Cela ne fournit pas une preuve
formelle de correction du logiciel ni une certification Cursor.

## Admission

La réparation est liée au dernier refus via `revise-recovered-result`. Le moteur
retrouve l’avis d’origine dans son journal d’événements durable, puis vérifie :

- même producteur, tentative, contrat, fournisseur, configuration de modèle et
  méthode ; contexte et journaux d’origine toujours conformes à leurs empreintes ;
- chaque pièce du paquet original conserve exactement son type, nom, empreinte et
  contenu ; une seule modification invalide tout ce groupe ;
- seules les réponses `inspected` et `unknown` sont conservées ; les refus sont
  réexaminés et les réserves restent explicitement ouvertes ;
- tous les artefacts canoniques du nouveau candidat apparaissent exactement une
  fois, parmi les nouvelles inspections ou les groupes historiques ;
- le delta Git intégral entre l’ancien et le nouveau candidat est calculé par le
  moteur. Les candidats peuvent être des commits frères : aucune ascendance n’est
  inventée.

La réponse originale reste octet pour octet sous son ancien candidat. Le nouveau
plan contient une référence historique séparée, ancrée par son empreinte durable.
Une référence ne peut pas recevoir une réservation d’appel fraîche. Les appels
historiques restent consommés. Les journaux d’origine ne sont pas réécrits.

## Décision et preuves

Les deux appels finaux restent nécessaires : sélection de pièces originales, puis
avis indépendant. Ils reçoivent les contrats, rapports, contrôles actuels, extraits
et opinions distincts, provenance historique, réserves et delta complet.

Pour chaque groupe historique, une résolution d’impact doit citer un changement
actuel exact (ligne ajoutée/retirée, renommage ou changement de mode). Une ancienne
citation, un rapport seul ou un nom de fichier ne suffisent pas. Une réserve
omise, dupliquée, inventée ou encore `unknown` interdit l’acceptation. La pertinence
sémantique de la résolution reste la responsabilité du vérificateur indépendant.
Le moteur exige toujours contrôles et avis sur le même candidat avant publication.

Les empreintes des preuves sont recontrôlées après redémarrage et avant chaque
réservation. Les contrôles exécutés dans une transaction ne rouvrent pas une
requête sur la connexion SQLite unique : cela évite un interblocage. La provenance
est relue une fois par avis source dans chaque validation, sans cache global périmé.

## Prévol partagé CLI / HTTP

`planning review-cost WORK --input request.json`, avec
`{"task_id":"TASK"}`, et
`GET /api/v1/planning?work=WORK&task=TASK&action=review-cost` exposent :

- inspections nouvelles, observations historiques, deux appels finaux ;
- budget restant et compatibilité du budget ;
- taille totale et taille restant réellement à inspecter ;
- `transport_ready` et, sinon, `transport_blocker` : la capacité des messages
  finaux est vérifiée avant de dépenser un appel.

Le prévol fonctionne même avec un budget nul et ne modifie aucun compteur.
`fits_budget=true` ne suffit pas si `transport_ready=false`.

## Limites explicites

- Pas de réutilisation en cascade depuis un plan historique de protocole 3 : retour
  conservateur à une revue complète. Les protocoles 1 et 2 restent lisibles.
- Changement de contrat, fournisseur, méthode ou modèle : revue complète ; preuve
  historique corrompue : refus, sans réparation silencieuse.
- Diff binaire : pas de réutilisation différentielle.
- Pièce indivisible, delta, questions ou décision finale trop volumineux : refus
  avant appel, aucune troncature. La réduction des appels n’est pas garantie.
- Les tests fournisseur utilisent un sous-processus déterministe : ils prouvent le
  protocole et l’application des refus, pas la qualité d’un modèle réel.

## Mesures

Cas pur : 8 appels prévus deviennent 4. Parcours public isolé avec véritable Git,
Store, contrôles et transport sous-processus : 6 appels initiaux, puis 4 pour la
correction ; nouveau candidat accepté, ancien journal conservé, rejeu sans appel,
preuve encore valide après réouverture et corruption historique refusée. Un défaut
signalé pendant la nouvelle inspection bloque la publication après un seul appel.

Prévol E6 réel, en lecture seule : 275 pièces, 1 762 047 octets ; 5 groupes
historiques réutilisables ; 1 101 354 octets restent à inspecter. Le coût passe de
15 à 10 appels, pour 7 disponibles. La décision finale dépasse encore la capacité
courante. E6 n’est donc pas déclarée débloquée ni acceptée. Aucun appel réel ni
augmentation de budget n’a été effectué pour cette implémentation.
