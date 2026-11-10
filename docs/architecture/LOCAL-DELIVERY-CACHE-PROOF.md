# Preuves de livraison locale

Le lanceur contrôle les preuves avant de réutiliser une image inchangée. `_read_prepared` partage les critères de la sélection `--prepared-build` : schéma, empreinte, profil normal sans fixture, métadonnées et archive exacte des entrées. Un fichier manquant impose une nouvelle construction ; une incohérence refuse avant arrêt du serveur. Aucun succès du compilateur ni fichier binaire isolé ne remplace cette qualification.

La sélection explicite ne reconstruit pas les sources plus récentes et refuse une régression de version. Les erreurs et causes de nettoyage restent distinctes. Aucun droit de publication ou de migration de données n’en découle. Les preuves doivent être conservées hors cache avant nettoyage.

Recettes : `local-build-cache-proof.json` et `tests/test_swarm_prepared_build.py`. Ce dernier requiert `SWARM_TEST_BUILD_STATE` pour son cycle réel isolé ; une absence de ce prérequis est un skip, jamais PASS. La recette permanente vérifie trois cas de cache, sept gardes de stabilité et cinq contrôles de sélection/cycle. Aucune acceptation indépendante ou globale déduite.
