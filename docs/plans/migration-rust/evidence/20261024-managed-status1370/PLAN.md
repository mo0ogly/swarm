# Parcours managed : statut initial

Reprise de1365 ; baseline normal1361/after.json. Propriétaire racine.
But : rendre lisibles les deux tâches réellement planifiées sous profil automatique, sans démarrer de worker.
Contrat Go dispatcher.profileFor : profil tâche avant Work, workspace dérivé copies/<task>-<attempt>/<subdir>.
Refus actuel : mission-status-managed-automatic-readiness-unported. Contre-exemple public avant correction, puis même recette permanente.
Sources réservées : mission_status_readiness.rs, Rustdoc mission_status_historical.rs, test rust_managed_planning.py, recette managed-native-planning, docs FR/EN et notes de version.
Aucun lancement/copie/settlement managed ouvert ; aucune installation/acceptation.
Tests ciblés readiness et recette publique ; profil normal, review exacte, Clippy/Rustdoc pertinents.
Aucun watchdog global ajouté.

Contre-exemple1371 : même parcours natif à deux tâches, statut refusé managed-automatic-readiness-unported sur47004… ; original stdout/stderr conservé.
Tests1372 : tests ciblés exécutés ; warning de variable devenue inutilisée après retrait de la garde. Correction de la condition sans changer sa sémantique ; tests à rejouer sur ces octets finaux.

Résultat1378 :15tests unitaires sur octets finaux et8assertions publiques passent ; recette verify, même Root original, Clippy et Rustdoc passent. Revue locale3entrées compilées sans déclaration ouverte ; aucun contrôle d’acceptation indépendant/global. Profil normal1373 non installé. Runtime propre archivé ; suite G3 et conditions de reprise dans le PLAN actif.
