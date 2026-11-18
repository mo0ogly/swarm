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

## Décision pure de numérotation — appelant non raccordé

`select_version(published, digest, previous, *, installed=None)` conserve le calcul historique à trois arguments. L'observation facultative `installed` est un dictionnaire dédié portant exactement `version` (forme canonique `vM.m.p-dev.N`, sans zéro de tête) et `binary_sha256` (64 hexadécimaux minuscules). Elle décrit une entrée du calcul, pas une preuve : l'appelant doit encore établir la possession et la fraîcheur de l'image, qualifier son propre historique sous les verrous et revérifier avant remplacement. L'appelant doit fournir l'historique propre à cette source ; le calcul n'en fabrique ni n'en copie.

| Entrées du calcul avec observation | Décision |
| --- | --- |
| Pas d'historique, même famille et compteur observé 381 | Nouveau numéro 382 |
| Historique de même famille sous le compteur observé | Nouvelle allocation au-dessus du compteur observé |
| Historique au-dessus, empreinte de source inchangée | Numéro antérieur conservé |
| Même numéro observé et même source | Conservation seulement si l'image enregistrée est la même ; une empreinte consommée invalide refuse |
| Source changée | Nouvelle allocation qui avance l'historique de la famille cible et respecte le plancher |
| Famille observée inférieure | Pas de plancher de compteur entre familles ; une allocation inchangée dans la famille cible peut être conservée, même au compteur zéro |
| Famille observée ou antérieure supérieure à la cible | Refus, sans inventer une lignée de release |
| Historique présent sans version, observation mal formée ou version non canonique | Refus explicite |

Le calcul ne fait aucune E/S, aucun lancement de processus et aucune mutation de ses entrées. Les limites natives de conversion d'entiers Python restent possibles ; aucun plafond ni réglage d'interpréteur n'est ajouté. Un numéro n'authentifie ni la source ni l'acteur. **Statut : non raccordé** à `prepare`, `main`, l'admission du cache, `SWARM_VERSION` et la capture réelle. Ces contrats restent ouverts ; la recette `local-managed-numbering-pure.json` qualifie seulement la fonction et sa compatibilité historique. Aucun build, parcours de livraison ou nouveau binaire installé n'en découle.

## Refus de build sur une sélection externe

Refus de construction d'un binaire externe : `swarm_local.py build` refuse tout binaire résolu qui n'est pas exactement `bin/swarm` ou `bin/swarm-local` du projet, qu'il soit sélectionné par --binary, SWARM_BINARY ou la configuration persistée. Le refus intervient avant toute création de Root/runtime, tout verrou, toute observation du serveur possédé ou écriture du build, conserve le contrat d'exit 2 et affiche un message clair en français invitant à sélectionner explicitement un chemin source géré, sans aucune valeur de configuration ni secret. Le démarrage, le redémarrage et la configuration d'un binaire externe restent inchangés : il reste externe, sans reconstruction, repli implicite, extension de liste ni historique source copié.

Les réglages et métadonnées de chemin peuvent déjà avoir été lus ; ce contrôle de sélection ne confine pas le processus. Les preuves sont des appels inertes à main avec limites processus/réseau/producteur simulées : elles ne démontrent ni compilation native, ni remplacement réel, ni installation. La numérotation pure demeure non raccordée.
