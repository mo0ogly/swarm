# WIP — producteur Gi, custodie du candidat

Sauvegarde source inachevée, non compilée et non fusionnable.

Le seul producteur cfg(test) emprunte le candidat via sources::request et le transmet à préparation/claim/fraîcheur. Revue09 REQUEST_CHANGES : le holder préclaim porte review=None, le guard postclaim attend Some(review) et refuse review-managed-held-record-changed. Ce défaut P2 reste OUVERT. Aucune protection I0/égalité ne doit être supprimée. Suite en cours : transition explicite vers le holder de la revendication enregistrée, hors SQL/settings, puis nouvelle revue et vérifications ciblées.

Rapport source SHA57dde4f568fb90876e8fc90cd5c8b43cd0e76f95931387431159a7b58e6c237b, preuves privées /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-candidate-custody-cecms474. Base061be0a7ed7d4017e86677f4589e9bd4aeabd12e ; sourcef9cc3d9d83a860412cb7a6fee58e558dfad7c8bd94850f8bcaa77c6d4829b016. Aucun test, build, W/SDK/F, installation ou acceptation établi. Work refusé w-83e055eae3e47c27a0f5043c rev14/calls0 inchangé ; aucune mutation DB, budget, TTL, horloge ou garde moteur.
