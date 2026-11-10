# Revue Root1218 — périmètre exact et limites
Revue sémantique du diff entre inventaires normal1156 et normal1211, pas une
acceptation indépendante ni une garantie globale. Les sources hors diff ne sont
pas déclarées intégralement auditées. Profil normal338entrées/110artefacts,
4cœurs sans features/fixture. Binaire9e565a…de8db, aucune installation.

## Politiques
Conductor : cadence source persistée settings().values.mission_supervision_policy
→ validate/cadence → découverte ; chaque session garde la politique de son bail.
Admin/API/CLI existants restent les writers validés, sans nouvelle constante.
Un thread local par Work et un bail CAS sont des invariants de propriété, pas
un nouveau plafond opérateur. one-shot est la notification unique de durée de
vie ; les canaux stop ne donnent aucune identité ou permission. Autonomie,
consentement, paused/archive sont lus au producteur existant, jamais fournis
par le navigateur comme identité. source web est une attribution, pas un grant.
revision/Refused(revision_conflict) sont des codes typés de protocole ; seule une
preuve complète sans tentative ni effet permet relecture au prochain tick, même
event_id. Pas de retry immédiat, plafond, timeout ou budget implicite ajouté.
Tests : slots1,priority7,nonce,programmes et budgets sont des entrées explicites
de fixture persistées, pas des defaults produit. Les notes de version ne sont
pas une politique. Aucun nouveau réglage opérateur n'est créé.

## Fonctions et documentation
lib : typed_causes traverse la matrice moteur et garde causes secondaires/IO,
cycle detection existante. mission_watch : owned_session/session/local/cadence/
owned_panic et stale_dispatch_observation ont contrats, préconditions, issues et
effets explicités. CLI OS garde son listener ; source locale distincte. role_job
finish reconnaît ControllerShutdown et conserve l'ordre drain/publication.
conductor : cause/join/web, Session stop/finish, run/serve/merge documentent
propriété, erreurs et limites ; Display/Debug Context ne montrent que le code
statique. listener.bound_mission_root documente binding, close et obligations.
Les impls de formatage triviales n'ajoutent aucun contrat caché. Rustdoc1213
PASS ; exactitude examinée ici, non déduite de la compilation. FR/EN et Mermaid
à jour ; l'ancien test sans conducteur reste explicitement historique1156 et
les helpers sont réutilisés. Recette actuelle vérifie le consommateur réel.

## Erreurs et reprise
Ouverture Root → binding → close des deux causes avant lecture Work/claim.
Les identités de tous nouveaux peers sont comparées au serveur ; remplacement
de DB provoque arrêt puis original restauré et serveur neuf vérifié.
Erreur session/coordinator : notifier HTTP avant drain, interrompre seulement
les sessions possédées, envoyer le stop même si wake échoue, joindre chaque
thread/rôle, fermer store/binding. Les workers indépendants ne sont pas tués.
Send sur récepteur déjà fermé : seul le join réel tranche ; aucune panne IO
externe masquée. Panics opaques gardés par le helper existant sans payload public.
Bail occupé est le Refused typé exact, sans cleanup error : pas de prise de bail
ni libération du concurrent. Les autres erreurs restent fatales. Cas d'erreur
après HTTP : StoppedOutput garde rapport complet et cause IO personnalisée ;
traversant CLI réutilise native_error_links pour les branches secondaires.
RED1203 démontre la cause secondaire perdue, garde actuelle PASS.
Replan1210 traite la vraie course pause/révision observée1209 : seulement phase
revision/queue et préparation NotAttempted/ressources vides/Refused exact.
Préparation, commit, issue inconnue et autres causes restent des erreurs.
Le même event_id reste en attente jusqu'à une observation sous révision relue.
Erreur de publication garde le refus original et son erreur de sortie.

## Preuves et limites
44tests CLI, Clippy-Dwarnings et Rustdoc1213 ; recette web finale13assertions
+4gardes Python ; CLI3cas pause/SIGINT/SIGTERM ; rôles6cas réels avec témoin
valide et progression pendant attente. Le planner web retenu prouve son arrêt
local ControllerShutdown et reçu TERM réel sans décision fabriquée.
Tout résultat PASS est borné à ses sources/profil/fixture ; fournisseur inerte,
aucun modèle payant. La revue GLM1192 porte des contrats antérieurs au diff.
Revue indépendante du candidat, managed recovery, multi-tâches et acceptation
globale de migration restent ouverts. Pas d'UI/browser ni installation supposée.
