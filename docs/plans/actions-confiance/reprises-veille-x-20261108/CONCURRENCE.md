# Concurrence entre les discussions

Ces discussions isolent les contextes conversationnels, pas le système de fichiers : elles partagent le même checkout. Aucun worktree supplémentaire, verrou ni ordonnanceur n’est créé. Aucun nombre maximal matériel de sessions n’a été mesuré.

| Activité future | Concurrence prévue | Condition |
| --- | --- | --- |
| Lire/analyser | Possible, selon capacité réelle | Vérifier fraîcheur ; signaler qu’une source change ; chaque axe produit dans son propre dossier |
| Écrire les analyses | Dossiers d’axes distincts | Un propriétaire par dossier ; ne pas modifier index, suivi ou plan commun depuis ces sessions |
| Modifier code/contrats communs | À séquencer dans ce checkout | Un responsable d’intégration, périmètre explicite et sources stables ; pas deux éditeurs sur le même fichier ou contrat consommé |
| Build/tests | Non autorisés par ces contextes | Après autorisation : ressources/fixtures distinctes, inventaire réel, pas d’édition des entrées pendant exécution ; respecter les vrais verrous, ne jamais les supprimer |
| Installation/redémarrage/intégration | À coordonner et séquencer | Un candidat identifié, preuves applicables, service appartenant au projet et autorité de livraison ; ne pas arrêter un travail concurrent |
| Communication publique | Session communication responsable | Sources exactes fraîches, portée, approbation et contrôle après envoi |

Règle pratique : les quatre cadrages peuvent être repris indépendamment, mais cela ne donne pas quatre autorisations de développement. Les axes 1 et 3 partagent effets/reçus ; 2 et 3 partagent inventaire des preuves/recettes ; 1 et 4 partagent autorité/annulation. Les chevauchements imposent un arbitrage avant changement.

Avant chaque reprise : relever rôle, état réel, droits, fichiers visés, propriétaire des travaux présents et activités concernées. Si conflit, poursuivre la lecture et produire une proposition dans son dossier, sans attendre silencieusement ni toucher les fichiers partagés. Aucune réservation purement écrite n’est une exclusion technique. Ne pas inférer qu’un processus inactif le restera. Ne pas déléguer, augmenter un budget ou lancer un fournisseur sur cette seule consigne.

Sources : tools/agent-workflows/CONTRACT.md et docs/architecture/AGENT-PARALLELISM.md ; exigences documentaires, pas garantie de coordination des chats Codex.
