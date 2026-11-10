# Auto-revue locale après corrections

Candidat : huit sources `agent_identity*.go` archivées dans `source-final/`,
empreintes liées à `inputs-final.json`. Relecture sans modification de ce candidat.
Rôle producteur et vérificateur dans le même contexte : revue non indépendante.

Verdict local : **approve pour le composant interne IA2.3a**, avec les limites du
rapport ; aucune acceptation du chantier identité ou du moteur en production.

- IA2-G01 : le socket copié fonctionne avec le mode partagé non bloquant ; A
  passe avec son octroi, échoue avec B, et B fonctionne encore après copie.
- IA2-K03 : échec initial conservé ; le temps est désormais lu après acquisition
  de Trust/runtime et résolution des namespaces, juste avant l'évaluation pure.
  Contrôle positif réel puis expiration durant attente observée : réussite finale.
- UAPI : sockopt généré par architecture, bits `_IO` conservés ; fonctionnement
  vérifié sur Linux amd64 seulement. Pas de repli via PID numérique.
- Pair B confirmé explicitement ; erreurs de fermeture et de révision Trust
  distinguées ; erreurs de lecture de l'inventaire FDs bloquantes.
- Propriété des descripteurs et ordre des verrous relus ; rejet des droits
  injectés et troncatures, réinscription/révocation, table vide, projet étranger
  et émetteur terminé couverts. Les 19 nouveaux noms passent sous `-race`.
- Les 114 contrôles préexistants sélectionnés passent ; statique et compilation
  réussies. Pas de test global, fournisseur réel ou déploiement revendiqué.

Écarts maintenus ouverts : autorité durable et bootstrap protégés, engagement
Trust/SQLite, tous les exécuteurs publics, accord humain, droits mémoire, reprise
et effets, qualification IDA et RETEX. Un acquittement de transport dans la fixture
n'est pas une approbation, consommation ou preuve d'effet métier.
