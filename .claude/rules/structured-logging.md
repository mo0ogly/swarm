# Journalisation structurée obligatoire dans Swarm

Toute création ou modification d'un parcours de sécurité, de vérification ou
d'exécution doit journaliser les étapes utiles à son diagnostic. Cette règle
s'applique notamment à l'identité, aux habilitations, à Biba, à Bell–LaPadula,
aux validations, aux revues et aux reprises des agents.

## Événements et niveaux

- Journaliser aux frontières où l'état devient observable. Couvrir le démarrage,
  la décision, l'issue et, en cas d'échec, le motif et la condition de reprise
  si le moteur la connaît. Une étape seulement planifiée ne devient pas un
  contrôle effectué. Un succès de transport ne devient pas un succès métier.
- Utiliser l'émetteur structuré partagé. Donner un horodatage, le niveau, le
  composant, la famille, l'opération, l'étape et le résultat. Pour un refus,
  conserver le code stable du producteur et l'identifiant de corrélation
  autorisé. Garder la durée et les compteurs quand le producteur les observe.
  Regrouper les événements par opération, famille et, dans le périmètre
  autorisé, par projet, tâche et tentative.
- Utiliser **Info** pour les étapes significatives et les résultats normaux,
  **Warn** pour les refus attendus et anomalies récupérables, **Error** pour
  les échecs d'exécution et **Critical** pour une panne empêchant le parcours
  ou un échec d'intégrité grave. Utiliser **Debug** pour les décisions et
  observations détaillées. Éviter les doublons et les traces répétées à chaque
  rafraîchissement.

## Stockage et accès

- Résoudre les chemins depuis la configuration déclarée. Centraliser les
  destinations, niveaux, limites de rotation, rétention et taille de page.
  Les valeurs opérateur sont persistées, validées au point d'autorité et
  affichées dans Administration. Éviter les valeurs locales codées en dur.
  Documenter les invariants techniques et leur origine.
- Réutiliser les journaux et consultations protégés. Le journal applicatif
  couvre les opérations globales et les refus du relais. Les journaux de
  mission conservent leurs frontières et leurs droits. L'écran Admin indique
  la portée réelle, offre recherche et filtres, et lit les archives selon
  la politique en vigueur.
- Publier uniquement des champs autorisés. Exclure les secrets, jetons,
  assertions, corps de requêtes, prompts et captures brutes. Les identifiants
  et les liens restent soumis à la visibilité de leur source. Éviter tout
  texte libre issu d'une erreur à la frontière de projection.

## Vérification de livraison

- Démontrer un succès et un refus réel par le parcours concerné, retrouver
  l'événement dans la surface Admin prévue et vérifier sa portée. Contrôler
  le niveau enregistré après redémarrage et, si le stockage change, la
  rotation, la recherche d'une ligne archivée et les limites de rétention.
- Vérifier que le journal respecte ses droits et que les données exclues sont
  absentes. Une lecture échouée doit produire un diagnostic sûr. Préserver
  la cause d'origine et distinguer une panne de journalisation d'une opération
  en échec. Appliquer la reprise au problème métier.
- Documenter le producteur, le consommateur, la configuration et les limites
  dans [Console Debug FR](../../docs/architecture/ADMIN-DEBUG.md) et
  [Debug console EN](../../docs/en/ADMIN-DEBUG.md).
