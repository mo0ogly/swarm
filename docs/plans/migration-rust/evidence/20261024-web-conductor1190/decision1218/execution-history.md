
###1190 — conducteur serveur, intégration après1181
APEX/replan : comparer job HTTP long (rejeté : lane Root monopolisée) et
sessions possédées par serveur (retenu). Racine réserve mission_watch.rs,
mission_watch/role_job.rs, native_web.rs, nouveau native_web/conductor.rs,
service/listener.rs, recette publique et docs FR/EN/version. LIVE/telemetry
non modifiés. Réutiliser drive/bail/guards ; arrêt local distinct OS,
aucune nouvelle politique de cadence/retry. Critères : serveur seul puis
start→progrès, pause/stop réactifs, concurrence bail, drain/reopen/refus.
Sources gelées pendant compilation/recette ; installation non autorisée.

###1192 — huit agents GLM supplémentaires explicitement demandés
Huit revues contractuelles en parallèle du code Root : bail, arrêt local,
admission Work, identité Root, fixture publique, cadence, erreurs, HTTP réactif.
Deux HTTP simultanés/2000low/0retry, aucun watchdog global ni écriture GLM.
Paquets corrigés avec producteurs réels ; preuves wave1192-conductor-contracts.
Ces avis ne qualifient pas le futur diff non transmis.

1194 compilation FAIL conservée : mauvais nom operation/phase, settings privé,
conversion MissionSupervisionError et capture Receiver non Send corrigés sur
les signatures réelles ; branche ControllerShutdown exhaustive ajoutée. Aucun
contournement, relance après correction uniquement (1195).
1195 FAIL conservée : chemin exécutable emprunté trop court ; ownership transféré au thread1196.

1200 test-build FAIL conservé : match de fixture rôle non exhaustif pour le
nouveau ControllerShutdown ; correction1201. Revue Root détecte aussi perte
du rapport HTTP en cas d'erreur tardive du conducteur : StoppedOutput conserve
rapport+cause, garde nommée ajoutée. Notification d'arrêt HTTP déplacée avant
le drain des rôles, sans imposer de délai. 1199 reste preuve bornée de parcours.
1202 test-build FAIL : unwrap_err exige Debug du DTO succès ; match explicite1203, aucun changement DTO.

1203 :17tests watch PASS, garde causes composée web FAIL : typed_causes
parcourt source() mais perd la branche cleanup Root. Replan général : réservation
additive lib.rs (traversée CLI commune), réutiliser native_error_links existant,
pas un correctif propre au conducteur. Rejouer la matrice CLI lib car contrat
partagé ; garde RED originale conservée. Pas d'extension de visibilité moteur.
1204 :43tests CLI PASS, Clippy refuse deux emprunts superflus après extraction
et l'initialisation de rapport dans la garde. Corrections1205 sans suppression
de check. La garde RED1203 a confirmé une perte secondaire réelle : corrigée
au contrat commun CLI, pas seulement dans le conducteur.

1205 :43tests CLI,Clippy etRustdoc PASS. Avant profil final1206, correction
d'observation : si un signal arrive avant claim, lease_released reste false
(pas de libération inventée). Session automatique déjà préclamée garde true
après close vérifié. Recette1199 PASS ; extension finale concurrence serveur,
refus et changement de datastore à exécuter1207. Sources gelées.

1210 replan : recette web1207 PASS ; recette CLI1207 échoue avant exécution
(variable SWARM_PUBLIC_WATCH_BINARY absente), corrigée en1209.1209 reproduit
un vrai conflit concurrent : pause modifie la révision entre lecture et dispatch,
phase revision/queue NotAttempted/preparation NotAttempted/aucune ressource.
Le watch s'arrête alors à tort avant son SIGINT. Correction générale retenue :
observer ce refus sans effet, garder le même event_id, attendre la cadence sauvée
et relire la révision au cycle suivant. Tout effet, cause secondaire ou phase
autre reste fatal. Aucune nouvelle identité/retry immédiat ni hausse de budget.
La garde typée couvre variantes de refus ; matrices publiques CLI/web à rejouer.
