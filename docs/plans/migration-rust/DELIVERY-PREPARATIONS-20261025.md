# Livraison locale — préparations Rust,25 octobre 2026

Reprise autorisée par « ok fais » après le lot vérifié isolément. Owner : Codex.
Périmètre : correctif d’intégrité des reçus, clarification de la borne du titre,
notes FR/EN, construction et activation par le lanceur du projet. Aucun commit,
message inter-conversation, changement de réglage live ou reprise de mission.

## Critères observables

- Build par défaut qualifié, entrées effectives stables, candidat exact identifié.
- Parcours CLI/API et Admin sur Roots isolées avec ce candidat.
- Reçu corrompu refusé par la CLI publique, sans mutation ; réparation de fixture
  puis rejeu du même événement. Régression reproduite contre dev77 avant correction.
- Binaire réellement exécuté et version concordants après redémarrage pris en charge.
- Dialogue Version FR/EN : notes, commit réel, modifications non commitées et liens.
- Lecture protégée du service et ressources servies cohérentes avec le build.

## Replan conservé

Le premier essai dev78 a compilé en 38,26 s, mais la qualification a refusé des
modifications concurrentes de version_history.json et de cinq fichiers SCALING
(task_validation et candidate_preparation/managed_execution/runtime). Le serveur
dev77 est resté actif ; aucun contournement de la garde d’entrées ni répétition
inchangée. Preuves : sous-dossier privé `delivery-5xptu6ez/refused-build78` du
[dossier du lot](evidence/20261025-preparation-writes/README.md).

Le test permanent `rust_preparation_writes.py` ajoute deux assertions publiques
au lot initial 47 : refus précis du reçu courant altéré et réparation/rejeu sans
nouvelle ligne. Le témoin valide précède l’injection, limitée à la Root nouvellement
créée. Les causes originales, sorties et fixture dev77 sont conservées. Cette
injection de données ne modifie pas le schéma, contrairement à l’ancien essai de
trigger refusé avant le défaut recherché. Aucun acteur live ni fournisseur.

## Résultat vérifié

Installé par `./swarm.sh restart --no-open` : **v0.1.1-dev.78**, PID2462270,
SHA-256 `bef3310cab879016fdc84973d0a11aa47ab27a9bd0a2d34b0948ec9473c43f1c`.
Le lanceur a réutilisé le candidat testé, sans autre construction à la bascule.
Profil par défaut qualifié : 439entrées effectives archivées, features de fixture
absentes du binaire produit. Empreintes du statut et de `/proc/PID/exe` identiques.

- 49 assertions publiques CLI/API, 8 identifiants et 2 tests SDK sur le candidat78 ;
- 6 assertions de reprise v41→v42, 45 contrôles Admin par parcours de fixture ;
- 14 assertions du dialogue installé FR/EN : version, notes, vrai commit,
  modifications non commitées, lien et retour du focus, aucune erreur ni POST ;
- 11 contrôles HTTP/identité : session protégée et ressources identiques aux octets
  du build (Admin, traductions, thème, rendu de version) ; catalogue 5/document 42,
  unité texte et exemption du libellé automatique également vérifiés ;
- Les deux missions restent suspendues, révisions 50 et 469 inchangées.

Les captures FR/EN ont été inspectées. Un titre anglais mal peint dans une capture
complète a fait l’objet d’un contrôle ciblé : texte complet, visibilité vraie,
opacité 1, couleur lisible, aucun masque, capture d’élément lisible. Aucun défaut
produit démontré ; les captures initiales et le témoin ciblé sont conservés.
Aucun réglage live adopté, aucune mission démarrée, aucun appel fournisseur,
commit/push ou message inter-conversation. La qualification de ce parcours ne
clôt pas Biba/SCALING ou la migration globale.

Preuves et limites : [résultat de livraison](evidence/20261025-preparation-writes/delivery-result.json),
archives et sorties originales dans le sous-dossier privé `delivery-5xptu6ez`.
Les deux recettes finales ont été contrôlées par `recipe.py verify` : VERIFIED/PASS.

## Erreurs liées et améliorations conservées

L’appel initial de recette historique utilisait mon mauvais choix dev77, créant
déjà v42 : refus avant le parcours visé (`historical-v41`). L’original est conservé ;
la reprise utilise le dev74 réellement conservé (SHA635f62a6…), et passe 6 assertions.
Ce défaut du lanceur de recette ne constituait pas une régression du produit.

Un contrôle auxiliaire a également sélectionné à tort `bin/swarm-local` au lieu
du binaire configuré `bin/swarm`. Ce vieux fichier contient encore des sections Go
et refuse le stockage 37. La vérification a été corrigée vers le vrai exécutable
processus ; aucune donnée n’a changé. Suivi owner outillage installation/Codex :
identifier son propriétaire historique et sa valeur de retour arrière, puis
réconcilier l’ancien chemin par l’installateur pris en charge. Critère : chemins
exécutables documentés et réellement utilisés concordant avec le Rust qualifié.
Pas de suppression aveugle ; ce fichier n’est pas le serveur actif. Preuve et
empreinte dans `legacy-artifact-followup.json` privé.

Amélioration de transmission (owner documentation/version) : les notes d’autres
chantiers contiennent des formulations « pas compilé/installé ici » liées à leur
conversation. Préserver cette provenance tout en distinguant à l’avenir état des
sources, compilation du binaire et acceptation fonctionnelle. Un build global
n’accepte pas leurs fonctionnalités ; aucune réécriture de leurs preuves.
