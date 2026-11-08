# POC-06 — développement du mandat de tâche

1. Autorisation : Fabrice, 7 novembre 2026, développement avec GLM supervisé jusqu'aux preuves, Biba et documentation compris.
2. Périmètre : PoC Rust exécutable isolé, effets en mémoire ; aucune mission, configuration, identité ou donnée active modifiée.
3. Propriétaire : Codex supervise/intègre/vérifie ; GLM propose le code sans outils ; Fabrice reste propriétaire des choix produit.
4. Méthodes : replan et APEX ; skill skynet-agents, tâche unique bornée, vrai suivi processus/flux/issue.
5. Options : modifier les mandats Trust livrés et raccorder leur contexte natif, ou éprouver d'abord le contrat dans un exécutable de référence isolé.
6. Choix : PoC isolé, proportionné à la demande ; les commandes Trust inspectées n'exposent pas de tâche/tentative et de nombreux fichiers moteur sont modifiés par ailleurs.
7. Réutilisation : vrai `swarm_engine::check_biba` et types exportés ; aucune copie du moniteur Biba, aucun niveau client auto-authentifiant.
8. Limite essentielle : identité et contexte fournis par l'hôte de fixture, non authentification native ; mandats du PoC distincts des mandats Trust de production, aucun raccordement global prétendu.
9. Fichier réservé GLM : `rust/crates/swarm-engine/examples/poc06_task_mandate.rs` ; suivi/recette/docs par Codex, aucun fichier PAEA/BLP existant modifié.
10. Contrôles : six scénarios POC-06 avec témoins, refus exacts, état cible et compteurs d'effet, reprise ; injection déterministe entre préparation et application.
11. Droits : identité ∩ mandat ∩ politique courante, Biba supplémentaire ; délégation sans amplification, annulation et rejeu sans double effet.
12. Livraison : recette permanente et code versionnable, exécutable compilé avec sources/profil identifiés ; ni installation ni protection des agents natifs déduite d'un exemple.
13. Preuves : `/media/fpizzi/Lexar/swarm-evidence/poc06-20261107/` ; index/restauration local `.cache/poc06-20261107/CURRENT-GLM.json`. Montage Lexar vfat vérifié ; compilation sur disque Unix local.
14. Appel : `glm-implementation-01`, route demandée glm-5.2, backend annoncé GLM5.3 non déduit du nom ; pas de plafond client ni retry automatique.
15. Clôture : six critères exécutés et preuves vérifiées, revue politiques/docstrings/erreurs, doc FR/EN et limites ; raccordement aux parcours natifs à décider sur audit de leurs producteurs réels.

État initial : délégation lancée, aucun code intégré ni test du PoC exécuté.
Le document [PoC](../AGENT-CONTROL-POCS.md) reste le contrat d'acceptation.

## Premières preuves et replan de supervision

Le garde Biba réel a été compilé directement avec `rustc --test` : huit tests
nommés PASS, dont `exhaustive_4x4x3_matrix` (48 combinaisons). Sources, arguments,
version rustc, empreinte du binaire et sorties conservés dans `biba-guard-*`
sur Lexar. Ce contrôle de source ne qualifie pas le graphe du binaire produit.

Le lanceur GLM utilise initialement un argument de session positionnel, alors
que `status.py` n'accepte pour l'identité que `--session-id`/`--resume` : diagnostic
`process_identity_unverified` conservé dans les vues de supervision, pas de faux
statut sain. Contrôle distinct du PID, du boot, du start tick et de l'argument
positionnel exact dans `positional-runner-identity.json` ; aucun appel dupliqué.
Le champ de suivi `updated_utc` initialement manquant a été ajouté. La prochaine
invocation utilisera le format attendu ; ne pas interrompre l'appel API actif
pour corriger uniquement ses arguments de suivi. Les fragments continuent.

Banc maintenu préparé dans `tests/poc06_task_mandate.py` : compilation/exécution
sans watchdog, réservation du verrou de build et vérification des sources et
de l'image. Il reste à aligner son contrat de rapport sur le producteur réellement
remis par GLM avant toute qualification.

Correction de supervision appliquée sans nouvel appel : un superviseur détenu
avec `--session-id` suit le lanceur HTTP initial et son véritable `exit.json`.
Identité initiale conservée dans `process.initial-runner.json`, identité du
superviseur distincte dans `process.json`. Vue `supervision-011.json` : outil
exit0, identité du superviseur courante et aucune raison de diagnostic ; cela
ne prouve toujours ni remise GLM ni scénario produit réussi.
