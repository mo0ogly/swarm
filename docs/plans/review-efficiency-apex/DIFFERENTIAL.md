# RD3 — contrat de réutilisation (non encore exécuté par le moteur)

Une égalité de SHA de fichier ne suffit pas. Le plan courant a des observations locales mais pas de fermeture vérifiée des dépendances ; leur conversion automatique en inspection actuelle est interdite.

## Réutilisation proposée

- Conserver l'observation originale, son auteur, modèle, méthode, candidat, réponse et empreinte. Ne jamais fabriquer une réponse du fournisseur sous le SHA du nouveau candidat.
- Une référence historique distincte peut contribuer à un nouvel examen. Elle liste les pièces effectivement vues, leurs empreintes, le contrat de tâche et toutes les réserves/questions conservées.
- Vérifier la fermeture du périmètre : fichiers nouveaux/supprimés, appelants, API, configuration, tests et critères. Un périmètre non attribué reste à réexaminer ; une déclaration libre du modèle n'est pas une garantie de fermeture.
- Toute modification d'une pièce ou dépendance, du contrat, du modèle ou de la méthode invalide l'observation concernée. Une modification d'éléments partagés invalide tous leurs consommateurs.
- Un ancien `unknown` reste une réserve à résoudre ; un ancien refus reste un refus à confronter au changement. Aucun état ne devient `pass` par transport.
- Le nouvel examen final reçoit le delta entier, les réserves, les références historiques explicitement marquées et les contrôles du candidat actuel. Son avis frais reste obligatoire avant publication.
- Le journal de reprise doit distinguer un appel payé d'une référence historique : aucune réservation fictive, aucune restitution de quota. La provenance doit survivre au redémarrage.

## Ordre d'intégration

1. Enregistrer des dépendances attribuées et vérifiables dans la nouvelle inspection.
2. Construire un calcul d'invalidation pur, testé sur graphes cycliques, dépendance manquante, suppression, API partagée et changement de contrat.
3. Exposer le calcul en prévol sans exécuter de cache, comparer aux cas volontairement invalidés.
4. Ancrer les références historiques de manière atomique dans le Store avec contrôle de révision et de modèle.
5. Adapter la reconstruction de la preuve finale sans réétiqueter les anciennes réponses.
6. Mesurer sur un test isolé : appels réellement réservés, réserves résolues, défaut volontaire détecté, nouvelle décision sur le bon candidat ; tester le redémarrage et la corruption.

## Conséquence pour E6

Les journaux v1 existants ne fournissent pas ce contrat de dépendances. Le moteur doit annoncer zéro réutilisation démontrée plutôt qu'un gain fictif. La nouvelle commande `review-cost` l'expose. Ni le protocole 2 des refus, ni ce document ne prétendent ramener les quinze appels prévus d'E6 sous sept. Cette étape reste ouverte et doit être implémentée avant de revendiquer une revue différentielle opérationnelle.
