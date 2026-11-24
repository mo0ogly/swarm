# APEX · Statut de validation et vues de Conduite

Base locale `v0.1.1-dev.398`, source `07fcaf977`, même arbre que
`main@2ed17713`. L’audit sur Toshiba SWARM a observé une revue gérée puis une
première publication réussies avec un fournisseur inerte. Le statut CLI a
refusé après revue et le statut HTTP protégé a renvoyé 501 après publication.
[Rapport d’audit](/run/media/fpizzi/SWARM/swarm-evidence/audit-revue.OpJsJxNU/audit.txt).

| ID | Responsable | Résultat à vérifier |
| --- | --- | --- |
| V1 | Moteur | Le statut partagé CLI/HTTP projette la soumission et la première publication gérées avec la bonne tentative. La projection distingue preuves périmées, observations manquantes et erreurs. |
| V2 | Moteur | La revue de la tentative et la décision de publication restent distinctes. Les lectures ne lancent ni contrôle, ni fournisseur, ni acceptation. Le témoin précède toute altération de preuve. |
| V3 | Cockpit | Une commande visible bascule entre Graphe des tâches et Équipe et validation. Les états de tâches et l’aide aux gestes restent dans le graphe. La deuxième vue montre rôles, revue, contrôles et acceptation avec leur portée. |
| V4 | Vérification | Recette isolée CLI/HTTP sur binaire de profil livré, test navigateur FR/EN et deux thèmes, revue des fichiers, livraison et relecture d’identité du service. |

## Analyse et ordre

- Étendre le statut partagé. Son garde initial exclut les états gérés après
  soumission. Le lecteur de revue gérée et la disponibilité de la remise sont
  des contrats distincts. Réutiliser les preuves de publication et les lecteurs
  de contrôles, avec les mêmes contrôles d’identité et de fraîcheur.
- Une route de consultation séparée pourrait exposer une revue enregistrée sans
  modifier le statut agrégé. Elle laisserait cependant le 501 de Conduite et la
  divergence CLI/HTTP ouverts. L’élargissement du statut partagé est le premier
  choix. Tant que les preuves d’une branche manquent, garder son refus.
- V1 et V2 précèdent l’affichage du statut produit dans V3. La navigation de V3
  se vérifie indépendamment. V4 réunit les preuves du même candidat. Toute
  décision d’acceptation reste du ressort de l’opération existante.
- Hors périmètre, relance de Biba et appel à son fournisseur de revue. Le
  diagnostic de la fixture ordinaire reste distinct. Sa reprise exige un
  fournisseur de test compatible avec le répertoire d’exécution I0.

## Reprise

Critères entièrement vérifiés à l’ouverture : 0/4. Les preuves de l’audit
constituent les témoins de départ, avec un scénario en échec conservé.
Le coordinateur Codex intègre et vérifie un paquet de proposition SKYNET
borné au statut Rust. Le paquet exclut l’UX, les données de mission et la
publication. Les essais et les révisions de plan seront rattachés ici.
