# S2 — worker dépendant après publication, résultat et RETEX

## Résultat vérifié

Swarm local dev121 installé via le lanceur pris en charge. Le parcours public
reproduit puis corrige le candidat absent du clone après publication initiale,
puis le refus de dépendance survenant après création d’une copie bloquante.
Le fetch vise exactement l’OID publié dans le bare tenu. Le précontrôle réutilise
les prérequis et la fraîcheur communs avant tout effet ; l’admission conserve
ses contrôles complets, aucune autorité mémorisée ni politique supplémentaire.

## Preuves et portée

- `flow2` : échec public initial managed-git-command, originaux réellement créés.
- `git-cause.json` : base présente/candidat absent après clone, puis présent après
  fetch explicite du même OID ; aucun pointeur produit modifié.
- `recovery` : copie au bon candidat après correction ; assertion de recette
  erronée sur un marqueur hérité, conservée. Son agent queued a été annulé
  publiquement, sans supprimer copie, historique ou consommation.
- `recipe2` : refus de dépendance laissant une copie avant précontrôle commun.
- `recipe3` : précontrôle effectif sans copie ; attente de diagnostic trop étroite
  dans la recette, corrigée sans modifier le refus produit.
- `recipe4` : première publication, refus initial sans copie et conservation
  vérifiés avec originaux natifs valides réutilisés ; transmission du chemin de
  naissance de fixture à corriger dans notre adaptateur.
- `recipe5` : PASS et intégrité VERIFIED. Quatre obligations nommées.
  Publication/native déjà valides réutilisés explicitement ; la dernière étape
  prépare/exécute réellement le worker, vérifie HEAD, contenu publié, manifeste,
  rejeu, absence/restauration de copie, maintien du prédécesseur et une réservation.
- `activation2` : identité du processus, SHA/profil sans fixture, ressources
  protégées, Works inchangés et14 assertions Version FR/EN PASS. Entrées
  effectives identiques au profil testé, malgré métadonnées de build renouvelées.
- `installed-public-readback` : trois vérifications CLI sur le binaire installé,
  sans nouvel appel/débit : worker terminé, rejeu publication, ancien worker intégré.
- Rustdoc workspace/all-features compile ; le choix engine seul refusait le
  verrou de dépendances. Cette compilation documentaire ne prouve pas le profil
  de production. Les sources de compilation et le lock sont conservés par le
  profil normal. Diff ciblé vérifié ; aucune suite globale répétée.

## Revue et limites

Revue locale exacte des politiques/fonctions/erreurs/documentation dans
`local-source-review-final.json`. Revue GLM ciblée de fetch/custody/erreurs reçue :
aucun défaut démontré ; recontrôle writer et comportement Git ne sont pas déduits
du modèle, mais vérifiés par leurs sources et le parcours. Le complément de revue
sur la boucle de dépendances est reçu : aucun garde supprimé ni droit/fallback
introduit selon les extraits actuels. Les deux revues restent statiques et bornées
aux fonctions transmises, sans observation d’exécution ni acceptation du modèle.
Usage déclaré des deux réponses complètes :9 844 et4 389 tokens, sans tarif inféré. Un premier appel GLM devenu
périmé a été arrêté ; consommation finale inconnue, fragments privés conservés.
Aucune acceptation indépendante globale ni conformité Cursor globale revendiquée.

## RETEX de l’incrément

La preuve manquante de vraie copie après publication était nécessaire : lire le
seul pointeur SQL masquait un clone incomplet. Tester aussi la demande trop tôt
a montré un second effet de bord avant refus. Prévenir la récidive en conservant
le scénario dépendant et le marqueur exclusif propre au nouveau worker, sans
supprimer les marqueurs réellement hérités. Réutiliser les originaux valides
avec leur provenance explicite évite de rejouer les mêmes premiers agents.

Défauts de méthode conservés : chemins absolus interdits dans le manifeste,
attente incorrecte de deux tâches, contrôle de version lancé avant fin du restart,
revue GLM lancée avant gel du dernier complément. Les preuves échouées ne sont
pas écrasées. Gel final, inventaire exact et attente du prérequis corrigent ces
reprises. Les conflits de build dus à des fichiers voisins/inventaires Git ont
conservé le serveur précédent ; aucun contrôle n’a été désactivé.

Suite : baseline cumulative et projections courantes sans réécriture des anciens
originaux, attribution après plusieurs publications, boucle du superviseur,
S3/S4 et RETEX final. Ce RETEX local ne clôt pas ces exigences.
