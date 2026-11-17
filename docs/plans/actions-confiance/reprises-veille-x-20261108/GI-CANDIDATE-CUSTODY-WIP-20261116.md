# WIP — transition du candidat du producteur Gi.eth

Source revue et compilation SDK réussie ; vérification fonctionnelle encore ouverte : ne pas fusionner comme validée.

Le contre-exemple R09-CUSTODY-1 de la première sauvegarde7088121 est résolu à la source par deux détenteurs distincts. sources::request acquiert/rafraîchit le candidat préclaim hors SQL ; son callback garde les transactions de préparation/revendication et renseigne Work/claim connus avant sa fermeture. sources::job acquiert ensuite le détenteur de la review réellement enregistrée/running hors SQL ; son callback garde owner/fraîcheur/horloge/expiration/historique. Aucune égalité, protection I0, politique de budget/TTL ou limite supprimée. Les erreurs après claim ne signifient pas rollback.

```mermaid
flowchart LR
 A[Préclaim sans review] --> B[Revendication enregistrée]
 B --> C[Fermeture préclaim]
 C --> D[Holder de la review exacte]
 D --> E[Contrôles et expiration]
 E --> F[Fermeture et rapports]
```

Revue09 APPROVE_SOURCE_ONLY : SHA6b3d83798f27900c0c035197cf96ef41e870693c99adeef52a15b554e0fc9263. Source5c0e1ab29b4eb87673b126f2314c34d5c8ef343d5ca957894d8a8b567f1ff14a ; preuves privées /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-custody-transition-u41ymcya. Une ligne blanche séparatrice supprimée, aucun autre corps de fonction changé.

Restent requis : vrai témoin managed valide avant mauvais/nonrunning review et cleanup conservant knownWork, puis reprise contrôlée. Aucun test fonctionnel/W/SDK/F/installation/acceptation réalisé pour ce candidat. Le Work refusé w-83e055eae3e47c27a0f5043c, dernière lecture rev14/calls0, n’est pas modifié. Les anciens scénarios ne sont pas rejoués.

## Observation de compilation seule —16novembre2026

Un build engine offline/locked `--lib --no-run` terminé avec code0 ; aucun corps de test exécuté. Artefact engine réel test=true/features=[],558entrées workspace effectives conformes aux947candidats archivés avant Cargo et relus. ELF temporaire figé9b1d5baae0f4f20a305527dcbce611681a243632f367db41cf603a4df2e43f1c, commit source86c29e91f28dc3df016b8bdcf4d7a4ef01be6b8a. Reçu racine82046168701a3ca30654254617955bc5ad2b9cdcd77f9e8335dfede4aba73dae dans `/run/media/fpizzi/SWARM/swarm-evidence/root00-sdk5c-build-3l38zm8u`. Portée compilation/emprunts et profil test workspace, pas authentification complète par unité/compilateur/registry/générés, témoin managed, W/F, production ou version installée. La documentation ne reconstruit ni ne rebaptise le SDK source.


## Sauvegarde source après reprise stockage

Préparateur managed SHA-256 6a357d1f4f31fb5dd70629c54eea31cfb53425e731cc59dcbdc9b0ddb29fd7ad : deux propositions GLM et corrections bornées du principal, revue09 APPROVE_SOURCE_ONLY (reçu 9be1ec21b08420aeab808b9bf8bc7750c6d0f8a7e4b7a68afd59a6e2304de29f). Point WIP source seulement, sans résultat runtime/W/SDK/F ni installation. Réserve restaurée par archivage vérifié de64preuves historiques ; CLI5f restauré localement, SHA complet relu et787entrées effectives inchangées. Essai géré séparément alloué après capacité et provenance, sans campagne historique. Les anciens échecs restent conservés. Erratum : LF[10] correct dans les deux propositions, aucun défaut de séparateur.


## Observation du parcours préparatoire —16novembre2026

Après restauration locale vérifiée du CLI5f et comparaison des787entrées effectives +7dépendances revues, le préparateur source6a357d1f sauvegardé en91d08318adbeba3be8ddc888343082e4dbf80a95 a exécuté un seul nouveau parcours géré. Reçu racine `MANAGED-PREPARATION-VERIFIED-01.json` SHA-256 f8da5f816643a0a516c1bc022f503df3891708c139f2ce124a384351b828d644 dans `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy` :10assertions,123fichiers conservés, état READY_MANAGED_CONTROLS_NOT_REVIEWED. Un worker inerte/report soumis, contrôle natif réussi, inspection readonly sans invocation reviewer, appels0. Arrêt réel sigint avec enfants0/lease libérée/Root fermé et owner réapé. Aucun W/SDK/F, faute, acceptation, fusion ou installation établi. Les anciennes campagnes ne sont pas rejouées. Le témoin W suivant réutilise ce Work ; adaptateur GLM en préparation, non exécuté. Le build SDK source86c29 reste compilation seule, non renommé par ce commit documentaire.


## W échoué et sauvegarde de correction non exécutée

Le W sourceaa009304 a réellement appelé le reviewer : code2, erreur review-reply-noncanonical-field, Workw-091168d1977007fbe4807a52 révision19,1appel consommé sur2. Cause du principal : PROGRAM/retainer ordinaires contre protocole géré, contexte de revue incomplet. Originaux SR1/certificat/marker/captures conservés dans managed-W-adapter-glm/FAILED-W-ORIGINALS ; aucun reset ni Root de remplacement. Le retainer sourcee0158c6c ajoute managed_protocol optionnel pour snapshot/contexte adressé/cohorte/markerI0, voie ordinaire explicite conservée. Candidat de reprise145lignes e073a0fd proposé mais09 REQUEST_CHANGES (reçuf80dc8e4e95498d7797a215a31386c192667f885a651ca22e813d1cecee87d0c) : acquire ReviewCandidate/observe_submission, review_recovery et prepare_review_snapshot_using refusent la reprise gérée. Une décision planning review-retry et acquittement canonique manquent également. Le nouveau script persistant est SOURCE ONLY bloqué, non importé/exécuté ; AST et revue du protocole seulement, aucun second appel ni PASS de récupération. Son en-tête explicite change son SHA par rapport au candidat externe conservé. Suppléance principale bornée après fluxGLM figé332s, consommation serveur inconnue ; nouveau paquet GLM managed-recovery-contract-glm pour le plus petit port correct, sans affaiblir droits/source/quotas ni rejouer les campagnes. Rien fusionné/installé.


[Plan de reprise gérée](MANAGED-REVIEW-RECOVERY-20261116.md) : audit09 recevable pour curation/conception source, pas pour patch. P2 admission gérée toujours ouvert ; Work courant inchangé, appels1sur2. Source GLM du plan lue intégralement, code non adopté ; prochaines lectures/design bornés avant tout correctif. Réserve obtenue par158copies historiques relues sur SWARM, sources et données actives locales.


### Incrément préalable MRR-D02 — source et helper vérifiés, reprise non portée

Le helper `append_recovery_context` dans `review_preview/inputs.rs` mutualise le contexte correctif ordinaire et géré. La directive reste hors des matériaux citables/catalogue et précède l’encodage et la limite totale du prompt. Sources `9a8cebf74b4c6cfdb380fd45a3148152d78e096bf9267dbe56b4baafc658d93f`, revue indépendante09 APPROVE_SOURCE_ONLY (tour01a1e550-c91b-7451-8583-e2c2adf6cc83). Trois nouveaux tests inertes PASS (absence inchangée ; directive complète et matériaux préservés ; refus non-objet après témoin valide) via Cargo `--lib` et filtre `root_store::review_preview::inputs::tests::recovery_`. Preuves et inventaire effectif dans `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/prompt-prerequisite-check-01/`. Profil bibliothèque de test, sources stables ; aucune compilation production, installation, admission de reprise ou validation du parcours géré complet.

Suppléance principale bornée car GLM produisait encore du raisonnement sans patch final ; aucun appel doublé ou plafond ajouté. Sa remise finale après531.299s  propose la même mutualisation, lue entièrement et comparée ; sa docstring « exactement une fois » ne serait pas une garantie d’idempotence et n’a pas été reprise. Usage déclaré exact67451tokens, dont59693sortie ; aucun tarif/quota Codex déduit. Version principale relue/testée conservée, pas de remplacement post-PASS. MRR-D01/D03, MRR-D02 parcours complet et P2 restent ouverts. Work19/calls1/2 inchangé. Aucun rejeu historique, second W, service, provider ou acceptation.


### 2026-11-16T22:17:46.225443+00:00 — préalable MRR-D03 possédé : source revue, compilation seulement

Deux fichiers intégrés dans le checkout CI : `task_validation/review_candidate.rs` SHA256`2c4e9e72aaa63ee96f628431f6c218252609a161584b98f79618bbcb3a783918` et `managed_execution.rs` SHA256`e98ca58abb3c9f94f052930e9ac7c89edfe9baf82d980039617b579220fc5b56`. GLM a produit une proposition complète après495.539s, usage déclaré122240tokens ; ni tarif ni quota Codex déduit. Root a reconstruit les hunks aux anciens blocs exacts (un contexte de doc inexact corrigé), refusé toute construction implicite de ReviewerRetry par le mapping historique, dérivé le purpose avant acquisition et refusé explicitement le preview historique sur holderRetry. Revue indépendante09 APPROVE_SOURCE_ONLY, tour01a1e66b-2350-73f2-9b48-fa159ded1571, primaire `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/managed-review-purpose-glm/REVIEW09-PURPOSE-PRIMARY.json`.

Purpose/record sont possédés et projetés par emprunt vers ReviewSource ; les helpers explicites préservent ce purpose lors d’acquisition, préparation et refresh. Les hooks privés acquire_for_retry/preview_for_retry restent non câblés aux opérations publiques/claim. Recovery reste une observation, pas un permis ; vérification Root originale/décision canonique et contre-vérification consommateur restent nécessaires. Aucun changement de politique, compteurs, autorité, digest source ou binding contrôle/candidat. Fonctions nouvelles/modifiées documentées ; causes typées et paires rollback/fermeture revues.

Cargo `--lib --no-run --offline --locked` terminé0, profil de test bibliothèque par défaut sans features supplémentaires, 558entrées effectives archivées/relues ; sources avant/après stables. ELF de test`738db0f581eb36080c77d8eccfa208c8cb0084720ebb428baedb6cd36791221e` conservé comme preuve0400nonexécutable dans `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/purpose-prerequisite-compile-01`. **Zéro corps de test exécuté**, aucun PASS fonctionnel déduit. Les3tests helper précédents sont historiques et réutilisés dans leur portée inchangée, pas rejoués. Aucune installation/acceptation : paquet préparatoire incomplet, aucun binaire de production actuel qualifié pour ce contrat. MRR-D01, MRR-D03 comportemental, W/SDK/F et P2 restent ouverts.

Limite de recette : le runner dérivé a gardé dans sa docstring une mention « missing tests » alors que son argv/compteur/summary indiquent explicitement --no-run/0. Cette prose ne constitue pas un contrôle exécuté. Script réellement employé et copie archivés inchangés ; corriger la documentation d’une prochaine recette avant usage, pas de rejeu pour remplacer cette preuve.


## Root recovery core: source-only checkpoint

Core de reprise Root ordinaire/géré et comparaison exacte des artefacts assemblés : source revue09, compilation bibliothèque --no-run seulement,0tests. Acquisition publique Fresh non raccordée ; aucun nouveau W/SDK/F, production, installation ou acceptation. WIP, ne pas fusionner comme correctif vérifié.


## Initial managed selector: WIP

Sélecteur initial géré : observation Root originale/décision avant custody et revalidation consommateur, revue09 source seule. Aucune compilation ou preuve fonctionnelle de ce sélecteur. MRR-T01 recette ACK, profil normal, témoin/reprise/fermetures/débit1→2 restent ouverts. WIP non fusionnable comme correctif vérifié.


## Actual managed witness and dispatcher failure

Recette W witness/retry source revue09. Témoin réel PASS0→1 (15preuves/8CLIreaped0), refus sanscorrection PASS sansmutation. Reprise originale FAIL dispatcher planning-decision-managed-operation-unported avant secondreview; Work21/reviewer1of2/planner3of3, nativeappelterminalRefused. MRR-D04 et autorité de reprise de l’ancien Work épuisé ouverts, aucune réinterprétation/restitution. WIP, pas de merge/install/acceptation.


## MRR-D04 dispatcher: source-only checkpoint

MRR-D04 : entrée managed review-retry portée en source et revue09, un littéral de capacité ajouté, gardes suivantes inchangées. Non compilé/vérifié runtime à ce checkpoint. AncienWork21/planner3of3/reviewer1of2 etRefusedterminal conservés. Previewquota3→4/child2→3 valide mais nonautorisée/nonappliquée. WIP non fusionnable comme correctif vérifié.


### MRR-D05 — précontrôle de preuves fermées, 17 novembre 2026

Le pilote complet GLM (121281jetons déclarés,542.911s) est refusé/non appliqué pour schémas incorrects et terminal sans conservation des owners. Suppléance bornée root00 : tests/rust_managed_resume_evidence.py, tests/test_rust_managed_resume_evidence.py. Ce module lit seulement des fichiers fermés : aucune CLI, DB Root, quota, provider, activation ou permission. Lecture positive64fichiers/15preuves/43helpers/787sources, seul dispatcher différent ; ancien témoin30910bc9 conserve sa portée historique, normalfec4 n’a pas exécuté la reprise.

Sept gardes healthy_witness PASS initialement ; revue09 REQUEST_CHANGES P2 compteur Workfinal non lié au reçu. Cas ciblé FAIL avant correction puis1PASS après correction, sans rejeu des7gardes non concernées. Source6143b0ada8f21412788dd35dee3875997c7459a9adb644120397fdb78f11b912, testbf3d5649292c47ff21828d2d7f2c998d0f5228911fab17e1a2f5af6573107cac. Revue09 delta01a1e6e0-487f-74d2-9f71-c65a47f7bf19 APPROVE_SOURCE_ONLY. Les tests ne couvrent pas tous les chemins closed_failure/profile_bridge ; lecturepositive n’est pas preuve de refus exhaustive.

Trois axes : oracles de reçus/protocole, pas politique configurable ; fonctions non triviales documentées ; causes IO/JSON/refus propagées sans succès vide. Pas de confinement d’écrivain concurrent ou qualification runtime. Première erreur principale work_final pris pour Work corrigée d’après son producteur (pin), original refus conservé dans registre canonique. Preuves SWARM : swarm-evidence/root00-managed-preparer-cq1joyiy/managed-resume-evidence-preflight-01.

Correction du plan de reprise antérieur, AVANT toute mutation : ne PAS émettre planning.resume. La génération enfant7 est déjà postérieure au refus natif6 ; resume changerait l’époque source et invaliderait les contrôles. Quota public préparé3→4/child2→3 toujours NON AUTORISÉ/NON APPLIQUÉ, consommations conservées. Question humaine déjà en attente ; heartbeat/délai ne vaut pas réponse. Work initial dernierobservé21/reviewer1sur2/planner3sur3, Refused terminal conservé. Pilote supervisé de continuation reste à préparer/revoir après ces préconditions ; aucun runtime, merge main, installation ou acceptation globale déclaré.


### D05 — types exacts des compteurs, 17 novembre 2026

Compteur bool True accepté comme1 reproduit FAILf51251, puis type(x)is int appliqué aux deux compteurs du reçu et au compteur Workfinal. Deux tests ciblés PASS040f2f,7gardes antérieures réutilisées, aucun runtime/CLI/provider. Revue09 delta01a1e6ea-afb4-7253-a055-1332ce9f19ee APPROVE_SOURCE_ONLY ; le cas bool atteint le reçu, branche Workbool non couverte séparément. Source0dc1e8f4…,test71177b19…. Validation de schéma, pas politique opérateur ; docstrings/erreurs inchangées et cohérentes. Le pilote/runtime/quota restent ouverts et non autorisés. WIP borné, pas merge main ou installation.
