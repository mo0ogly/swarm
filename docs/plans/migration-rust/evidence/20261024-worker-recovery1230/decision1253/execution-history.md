
###1230 — finalisation native enregistrée sous bail
Replan/APEX : finalisation de reçu non appliqué SQL distincte de l'intégration
repository managed (encore refusée dans complete_with_owner). Ne pas l'annoncer
comme migration du règlement Git. Reconcile générique rejeté : peut annuler queued.
Choix : journal/finalisation partagés, fence mission dans la même transaction.
Racine réserve agent_completion_journal,agent_execution_store, nouveau module
mission_worker_recovery, exports mission_supervision_store/root_store/lib,
mission_watch, recette et docs FR/EN/version ; pas de LIVE ni index Go modifié.
Parcours : worker réel→refus SQL fixture→ancien helper parti→serveur redémarré
finalise même Agent, sans provider supplémentaire ; reçus absents/invalides,
superviseur actif, mauvais Work/bail refusent ou attendent sans mutation.
Gardes ciblées, puis profil normal+recette publique ; aucune installation.

1234 revue avant recette : présence d'un journal ne prouve pas reap/release/gate.
Extraire verify_original_facts de l'observateur existant et le réutiliser AVANT
la mutation de récupération. Le chemin manuel conserve son contrat ; ajouter
mutants kernel_reaped=false, incomplete_output et original invalide à la recette.

1238 replan local : édition1236 partielle (2correspondances au second span),
reçu provenance écrit mais fence finale non ajoutée. Build1237 lancé trop tôt
après cette erreur : compilation seule, jamais preuve du contrat complet.
Correction par contexte de fonction unique ; profil frais requis. Enregistrer
provenance du finish dans la même transaction et revérifier bail après readbacks.

1245 replan recette : RED1240 est prérequis BLOB JSON, pas preuve produit.
RED1242 ancien watch synchronise sans reprise ; barrière recette incomplète,
SIGINT au seul processus authentifié puis garde première synchronisation ajoutée.
Recette1244 finalise/rejette quatre mutants mais échoue ensuite au status HTTP
mission-status-planning-unported (scope fermé). Défaut de couverture conservé
ouvert G3/status ; show HTTP et status-native CLI restent lectures distinctes.
Ne pas présenter la recette settlement comme parité du statut historique.

1248 Clippy refuse la nouvelle enum (Agent inline1560octets). Box du seul Agent
terminal ; pas de seuil/politique ni allow. Tests1243 :157mission PASS/1ignoré,
1journal PASS,44CLI PASS ; Clippy FAIL et Rustdoc non lancé. Requalification
requise après Box. Recette1247 PASS sur1239 ; RED1246 ancien binaire refuse la
reprise à première synchronisation (aucun timeout).
Correction diagnostic1245 : scopes réels waiting/ready, pas closed. Le refus
HTTP vient du couple task blocked/Agent completed sans revue courante dans
planned_pair ; preuve .cache/worker-recovery-hprotzij/run/fault et reçu original.
Cette limite préexistante de projection reste G3/status ouverte, pas un refus
de settlement. Ne pas reprendre l'hypothèse scope fermé comme cause établie.
