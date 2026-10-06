# Annexe de traçabilité des sources

Cette annexe est réservée à la provenance technique et aux licences. Les noms des produits étudiés ne deviennent ni commentaires de code, ni textes d’interface, ni noms de fonctionnalités Swarm. Aucun code de ces sources n’a été incorporé par la rédaction du plan.

Recherche n8n : arbre et fichiers lus à la révision `e18afc84ccee5078feadfb776867f42f72f67731` de master. Parcours représentatif : événement de connexion du canvas → validation des extrémités et connexions dans useCanvasOperations → historique des opérations. Lecture complémentaire : workflow-runner et workflow-execute pour les identités, reprises et résultats d'exécution. Aucune installation ni recette n8n réalisée.

La liste de candidats n8n est une source d'idées, pas un inventaire de fonctions déjà réalisées dans Swarm. L'absence d'un motif dans une recherche de fichiers ne démontre pas l'absence d'une capacité : A01 doit compléter la cartographie avant de coder.

## Sources primaires

- [Dépôt n8n](https://github.com/n8n-io/n8n/tree/e18afc84ccee5078feadfb776867f42f72f67731).
- [Canvas inspecté](https://github.com/n8n-io/n8n/blob/e18afc84ccee5078feadfb776867f42f72f67731/packages/frontend/editor-ui/src/features/workflows/canvas/components/Canvas.vue).
- [Opérations inspectées](https://github.com/n8n-io/n8n/blob/e18afc84ccee5078feadfb776867f42f72f67731/packages/frontend/editor-ui/src/app/composables/useCanvasOperations.ts).
- [Moteur inspecté](https://github.com/n8n-io/n8n/blob/e18afc84ccee5078feadfb776867f42f72f67731/packages/core/src/execution-engine/workflow-execute.ts).
- [Licence n8n](https://github.com/n8n-io/n8n/blob/master/LICENSE.md).
- [Vue Flow](https://vueflow.dev/guide/) ; [licence](https://github.com/bcakmakoglu/vue-flow/blob/master/LICENSE).
- [Licence Dagre](https://github.com/dagrejs/dagre/blob/master/LICENSE).
- [Arrêt SAS Institute C 406 10](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=celex%3A62010CJ0406) : distinction fonctionnalité et expression du programme ; ne constitue pas une autorisation de traduire le code protégé.
