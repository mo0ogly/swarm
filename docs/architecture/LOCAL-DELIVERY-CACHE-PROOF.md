# Preuves de livraison locale

Le lanceur contrôle les preuves avant de réutiliser une image inchangée. `_read_prepared` partage les critères de la sélection `--prepared-build` : schéma, empreinte, profil normal sans fixture, métadonnées et archive exacte des entrées. Un fichier manquant impose une nouvelle construction ; une incohérence refuse avant arrêt du serveur. Aucun succès du compilateur ni fichier binaire isolé ne remplace cette qualification.

Pour un `start` ou `restart` géré avec serveur actif, le contrôle de version et la revalidation de son identité s'appliquent au candidat fraîchement construit comme au candidat présélectionné. Une lecture impossible, une identité changée ou une version plus ancienne/non comparable refuse avant le contrôle de paire puis l'arrêt. Les mêmes octets restent admis par le comparateur existant ; l'étiquette de version n'authentifie pas les sources. `build` seul conserve sa préparation sans remplacer le serveur actif. Le nettoyage d'un refus ne supprime pas le chemin du binaire installé ; si le nettoyage du candidat échoue aussi, les deux causes sont conservées. Les identifiants historiques `prepared-build-*` du comparateur restent utilisés pour les deux producteurs. La recette `local-managed-version-gate.json` vérifie cet ordre par huit méthodes inertes, avec témoins et reprises ; elle ne qualifie ni une installation réelle, ni le choix du moteur courant pour le garde Authority, ni la numérotation entre checkouts.

La sélection explicite ne reconstruit pas les sources plus récentes et refuse une régression de version. Les erreurs et causes de nettoyage restent distinctes. Aucun droit de publication ou de migration de données n’en découle. Les preuves doivent être conservées hors cache avant nettoyage.

Recettes : `local-build-cache-proof.json` et `tests/test_swarm_prepared_build.py`. Ce dernier requiert `SWARM_TEST_BUILD_STATE` pour son cycle réel isolé ; une absence de ce prérequis est un skip, jamais PASS. La recette permanente vérifie trois cas de cache, sept gardes de stabilité et cinq contrôles de sélection/cycle. Aucune acceptation indépendante ou globale déduite.

L’aire de qualification demande le mode Unix 0700. Une erreur de mkdir ou chmod
remonte avec son errno et son objet d’origine avant tout enfant natif ; aucun
nettoyage, retry ou emplacement alternatif n’est tenté après ce refus. L’aire
peut déjà exister ou avoir été créée avant l’échec : aucun rollback n’est déduit.
Les deux gardes inertes de `tests/test_swarm_version_fixture_area.py` couvrent
des dispositions simulées de checkout et de lanceur installé, avec témoin,
refus transmis à `verify_candidate` et reprise saine. Recette maintenue :
`tools/verification/recipes/local-delivery-fixture-area.json`. Le choix courant
source/home reste inchangé ; ce contrôle ne qualifie pas la prise en compte du
réglage `runtime_temporary_directory`, un build ou une installation.

Pour le garde de paire Authority avant `start`/`restart`, un serveur précédemment identifié comme possédé reste associé à son record : une perte d'identité refuse, sans repli sur le chemin d'installation. Le lecteur ouvre son image via `/proc/<pid>/exe` après un premier contrôle de possession, maintient ce descripteur pendant la commande publique `engine-settings show --json`, puis revérifie identité et couple périphérique/inœud. L'enfant reçoit le descripteur par `pass_fds` et utilise `/proc/self/fd/<fd>` ; seul le mode lu ressort après fermeture. Une erreur d'acquisition, de lecture ou de fermeture refuse avant arrêt ; une erreur de fermeture secondaire conserve aussi la cause primaire. Sans serveur possédé, les appels directs gardent la lecture par le binaire courant, mais son absence sur un Root initialisé refuse désormais ; le cas jamais initialisé conserve le comportement antérieur. Cette observation Linux/procfs n'est ni un confinement, ni un bail continu, ni une garantie contre tout échange entre observations. Les politiques existantes de mode, de démon et de protocole restent inchangées. La recette `local-live-engine-reader.json` couvre les fixtures synthétiques du lecteur, le refus historique corrigé et les gardes de version du caller ; elle n'exécute ni le moteur réel, ni un service, et ne qualifie pas une livraison entre checkouts ou un binaire installé.
