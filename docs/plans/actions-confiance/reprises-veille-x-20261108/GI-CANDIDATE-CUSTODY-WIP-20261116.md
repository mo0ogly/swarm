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
