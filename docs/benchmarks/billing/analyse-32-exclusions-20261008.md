# Analyse des 32 exclusions Swarm — 8 octobre 2026

## Résultat et portée

Les 32 exclusions ne sont pas 32 paiements erronés. Elles sont 27 erreurs explicites de stockage et cinq délais de progression. Leurs dénominateurs restent ceux de la campagne historique : 32/3 000 = 1,07 % ; sous F4e, 31/300 = 10,33 %. Ce sont des fréquences dans cette grille, pas des probabilités de panne en production.

| Frontière | Nombre | Preuve | Attribution |
|---|---:|---|---|
| Application de décision de planification | 14 | SQLITE_BUSY explicite dans résultat et événement public | Stockage / récupération du moteur, D5 |
| Revue indépendante | 13 | SQLITE_BUSY explicite dans résultat et événement public | Stockage / récupération du moteur, D5 |
| Planification sans décision finale | 5 | DÉLAI 120,482–120,893 s, holder conservé, aucune décision après claim automatique | Progression non résolue ; cause initiale non démontrée |

## Méthode et conservation

Les 32 répertoires historiques existent. Le binaire `bin/swarm` correspond exactement à l’empreinte publiée : `ee014204158e5a1dff2a64093ad9e45ff0e02fe1ac45f1bffc7bd8a3a636640b`. Les consultations publiques `planning show` et `work show` ont été exécutées sur 32 copies isolées avec ce binaire, sans migration. Aucun accès SQLite direct, lancement de fournisseur ou nouvelle campagne. Les sorties publiques contiennent les événements historiques ; les champs dérivés utiles, références temporelles et empreintes figurent dans le JSON adjacent. Les tokens, prompts et secrets des répertoires ne sont pas publiés.

Une première consultation sur cinq copies avec un CLI plus récent a été refusée (schéma 20 contre 27). Ces premières copies ont ensuite été migrées à titre exploratoire ; leurs projections ne servent PAS de preuve historique. L’analyse publiée utilise exclusivement de nouvelles copies et le binaire historique vérifié. Un export essayé sur les premières copies a refusé le livrable modifié : ce refus est attendu sous F4e et ne justifie aucun contournement.

## Les 27 erreurs confirmées

Le message « Proposition non appliquée : database is locked (5) (SQLITE_BUSY) » apparaît 14 fois ; « vérificateur indépendant en échec : database is locked (5) (SQLITE_BUSY) » apparaît 13 fois. Les événements publics corroborent ces 27 messages. Parmi eux, 26 sont sous F4e et un sous F7 (clé tentative, graine 1010).

La frontière qui échoue est déterministe : persistance d’une décision ou d’un résultat de revue. Aucun LLM réel n’intervient dans cette campagne : le responsable et le reviewer sont des scripts. L’absence de réessai borné transforme la contention transitoire en échec durable. L’origine exacte du verrou concurrent n’est pas identifiée par ces seuls messages ; il serait abusif d’imputer les 27 erreurs à une même transaction concurrente particulière.

Au commit historique, `planning_runner.go` réessaie seulement les conflits de révision lors de `planningChange`, et le chemin `planningFailure` peut lui-même rencontrer une erreur. Le correctif ajoute une récupération SQLite bornée et idempotente. Il faut distinguer origine de la contention et amplification par la politique de récupération.

Pour ces 27 lignes, le JSONL d’erreur ne contient pas de mesures de paiement finales : on ne peut pas conclure à zéro paiement par absence de champs. Les cinq délais, eux, contiennent un relevé final explicite.

## Les cinq délais : progression et mesure

Ils concernent les graines : aucune/1087 ; tentative/1029 et 1037 ; métier/1056 et 1087, toutes sous F4e. Dans chaque cas :

- l’ordre acceptation → modification du lot est prouvé dans le JSONL ;
- `prepare` et `verrou` sont acceptées ; `settle` reste `todo` ;
- la validation de `prepare` est `stale` dans le relevé final ;
- deux exécutants sont terminés, aucun règlement lancé ;
- le relevé contient zéro paiement, 12 impayés, zéro doublon, zéro paiement inexact et aucun faux succès ;
- le périmètre reste `ready`, un holder de planification reste présent ;
- le dernier claim automatique réserve un bail de 120 secondes ; aucune décision ultérieure n’est enregistrée ;
- aucun événement public SQLITE_BUSY, planning.failure, reviewer.failure ou dependency_stale n’apparaît.

La trace établit donc un défaut de progression observable, pas sa cause initiale. Un défaut de diagnostic de dépendance (D2), une sortie prématurée non journalisée ou une autre erreur de planification sont des hypothèses à départager ; SQLite n’est pas démontré pour ces cinq cas. Le journal texte conducteur ne contient que l’annonce d’écoute, sans stderr exploitable.

Le banc amplifie l’attente : `planning_busy` considère un holder présent comme une activité, sans vérifier l’expiration du bail. `watch` ne peut donc pas conclure au calme tant que ce holder reste présent. Le bail de 120 secondes et le délai global de 120 secondes laissent peu de place à une reprise après expiration. Cela explique le chemin de classement en DÉLAI ; cela ne démontre pas pourquoi la décision initiale a disparu. Aucun rejeu dynamique de ce défaut n’a été effectué pendant cette analyse.

## Ce que le correctif vérifie vraiment

La campagne corrigée contient 300/300 cas F4e valides, zéro exclusion SQLite et zéro délai ; son critère D5 demeure tenu. La comparaison historique doit préciser 26 ERREUR SQLite + cinq DÉLAI sous F4e, et non « 31 erreurs SQLite ». D2 montre aussi 300/300 événements dependency_stale et arrêts attribués au moteur dans la campagne corrigée. Plusieurs corrections ayant changé ensemble, ces observations ne permettent pas d’attribuer la disparition des cinq délais à D5 seul.

Le cas F7 historique reste présent dans le bilan ; le compteur D5 ciblé F4e ne le couvre pas directement. Les critères enregistrés, mesures et archives ne sont pas modifiés.

## Actions issues du diagnostic

1. Test isolé de perte de décision après claim : résultat attendu, événement causal et libération ou expiration du bail, sans hausse implicite des budgets.
2. Test de journalisation défaillante : distinguer erreur initiale et erreur d’enregistrement, avec stderr corrélé au claim.
3. Banc : classifier explicitement attente de bail, dépendance périmée, absence de décision et dépassement global ; ne pas assimiler tout holder à un travail utile.
4. Recette de récupération : arrêter un conducteur après claim, conserver même décision/event_id/révision, vérifier reprise sans doublon et trace causale.
5. Mesurer séparément sûreté, disponibilité, progression, temps bloqué et quantité d’information diagnostique.
6. Rejouer les cinq combinaisons sur des candidats figés avec ablations D2/D5, après préparation du protocole, pour isoler leur contribution.

Ces actions sont proposées, pas implémentées ni validées par cette analyse. Aucune garantie générale de processus financier n’en découle.

## Inventaire des 32 cas

| Clé | Faute | Graine | Statut | Frontière / constat |
|---|---|---:|---|---|
| none | F4e | 1000 | ERREUR | Application de décision |
| none | F4e | 1012 | ERREUR | Revue indépendante |
| none | F4e | 1016 | ERREUR | Application de décision |
| none | F4e | 1036 | ERREUR | Revue indépendante |
| none | F4e | 1038 | ERREUR | Application de décision |
| none | F4e | 1040 | ERREUR | Application de décision |
| none | F4e | 1045 | ERREUR | Revue indépendante |
| none | F4e | 1047 | ERREUR | Application de décision |
| none | F4e | 1087 | DÉLAI | Claim sans décision, holder conservé |
| attempt | F4e | 1002 | ERREUR | Application de décision |
| attempt | F4e | 1004 | ERREUR | Revue indépendante |
| attempt | F4e | 1006 | ERREUR | Application de décision |
| attempt | F4e | 1029 | DÉLAI | Claim sans décision, holder conservé |
| attempt | F4e | 1030 | ERREUR | Revue indépendante |
| attempt | F4e | 1032 | ERREUR | Application de décision |
| attempt | F4e | 1037 | DÉLAI | Claim sans décision, holder conservé |
| attempt | F4e | 1078 | ERREUR | Revue indépendante |
| attempt | F4e | 1080 | ERREUR | Revue indépendante |
| attempt | F7 | 1010 | ERREUR | Application de décision |
| business | F4e | 1008 | ERREUR | Application de décision |
| business | F4e | 1009 | ERREUR | Revue indépendante |
| business | F4e | 1017 | ERREUR | Revue indépendante |
| business | F4e | 1018 | ERREUR | Application de décision |
| business | F4e | 1021 | ERREUR | Revue indépendante |
| business | F4e | 1027 | ERREUR | Revue indépendante |
| business | F4e | 1041 | ERREUR | Application de décision |
| business | F4e | 1056 | DÉLAI | Claim sans décision, holder conservé |
| business | F4e | 1064 | ERREUR | Application de décision |
| business | F4e | 1073 | ERREUR | Revue indépendante |
| business | F4e | 1075 | ERREUR | Application de décision |
| business | F4e | 1078 | ERREUR | Revue indépendante |
| business | F4e | 1087 | DÉLAI | Claim sans décision, holder conservé |
