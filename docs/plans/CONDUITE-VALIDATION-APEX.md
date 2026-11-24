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

## Preuves du premier candidat

- Témoin rouge `dev.398` après avis favorable et avant publication :
  [refus CLI](/run/media/fpizzi/SWARM/swarm-evidence/apex-conduite-validation.gRwPN4il/red-baseline.stderr).
- Témoin `dev.400` : [dix contrôles](/run/media/fpizzi/SWARM/swarm-evidence/apex-conduite-validation.gRwPN4il/green-isolated/result.json),
  dont statut CLI/HTTP identique avant et après publication. La fixture a
  retiré le consentement au départ automatique avant d’ouvrir le serveur Web.
  Les essais précédents conservent l’erreur de fermeture du conducteur qui
  s’était engagé dans la fixture. Le fonctionnement en concurrence avec un
  conducteur actif reste un scénario séparé.
- [Bascule navigateur](/run/media/fpizzi/SWARM/swarm-evidence/apex-conduite-validation.gRwPN4il/final-navigation-ui.log)
  vérifiée au clavier, en clair et sombre, en français et anglais.
  [Identité locale active](/run/media/fpizzi/SWARM/swarm-evidence/apex-conduite-validation.gRwPN4il/active.json).

## Revue séparée et reprise

La [revue de source](/run/media/fpizzi/SWARM/swarm-evidence/apex-conduite-validation.gRwPN4il/glm-review/answer.txt)
ne constate pas de contournement de l’acceptation dans le diff. Le lecteur
de résultats ordinaires exclut déjà les travaux planifiés. Le diagnostic
d’activation a été attribué au conducteur embarqué dans le serveur Web de
la fixture, sur la base de son rapport de fermeture. La recette compare
également les tentatives, événements, financements et marqueur du reviewer.
L’ajout de la feuille réellement servie a reproduit une perte de défilement
après rafraîchissement en vue Équipe. La reprise garde les coordonnées visibles
avant masquage et les réapplique au retour au graphe.

## Reprise V3 · bascule en plein écran

Le passage en plein écran déplaçait le panneau Graphe dans un dialogue modal.
Les boutons de bascule restaient dans le document rendu inactif par le dialogue.
La solution retenue rattache les mêmes boutons au panneau Graphe visible et
les remet devant le panneau Équipe au changement de vue. Une copie des boutons
aurait ajouté deux groupes à synchroniser avec le même état et le même focus.

- [Témoin HTTP du défaut sur dev.402](/run/media/fpizzi/SWARM/cv.6jIp/prior-served.log).
- [Recette HTTP dev.403](/run/media/fpizzi/SWARM/cv.6jIp/candidate-served.log) :
  URL directe, bascule visible et activable, validateur visible, focus au retour,
  deux langues et deux thèmes, ordinateur et mobile. La révision du travail
  de test reste identique. Le test de composant couvre aussi cette régression.
- [Octets servis et notes bilingues](/run/media/fpizzi/SWARM/cv.6jIp/protected-assets.log)
  et [identité active](/run/media/fpizzi/SWARM/cv.6jIp/status.log). La session
  personnelle derrière le port d’autorité n’a pas été ouverte durant la recette.
- Revue locale du diff : le changement utilise les états DOM du dialogue et
  de la bascule. Il n’ajoute ni paramètre opérateur ni requête métier.
  La fermeture issue de la bascule restitue le graphe avant le retour de focus.

À 320 px, mesurer aussi la première carte après fermeture de l’aide.
Les [mesures avant](/run/media/fpizzi/SWARM/cv.6jIp/mobile-old/browser/fr-etat-mobile-bounds.json)
et [après la première correction](/run/media/fpizzi/SWARM/cv.6jIp/mobile-new/browser/fr-etat-mobile-bounds.json)
montrent que le bas de la première carte passe de 816 à 880 px sur un écran
de 844 px. En plein écran étroit, l’aide reste disponible depuis son bouton et
le texte d’aide redondant cède la place aux cartes. Le
[test navigateur](/run/media/fpizzi/SWARM/cv.6jIp/mobile-component.log)
couvre la bascule et la commande d’aide à 320 px.
