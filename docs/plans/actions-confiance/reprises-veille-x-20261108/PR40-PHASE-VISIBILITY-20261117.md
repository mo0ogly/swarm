# PR40-OBS-01 — visibilité des phases d’installation

État au 2026-11-17T04:47:29.874451+00:00 : source intégrée dans le worktree isolé, vérifications à doublures réussies, revue indépendante en attente. Ce n’est ni une reprise de PR40 ni une qualification Docker/GitHub.

## Besoin et périmètre

Le job install historique regroupait contrat de sorties, installation native et Docker dans une seule étape Actions. Les journaux conservés situent l’arrêt dans la construction des paquets runtime, sans cause réseau établie. La PR40 observée le17novembre01:22UTC restait bloquée, head f5d1f42f33ac730d1f0b830a172c0ff000bb18df, install annulé après6h le16novembre06:27:30UTC. Le changement préparé rend les trois phases distinguables dans une future exécution ; cet effet GitHub n’est pas observé ici. Les anciens6+8PASS sont conservés, pas rejoués.

Propriétaire : Swarm00. Un paquetGLM unique a produit le patch Makefile/workflow identique à la proposition privée conservée du16novembre, plus le test. Modèle API demandé/rapporté glm-5.2,217.118s,usage déclaré48517tokens ; aucune version réelle du service ni coût déduit. Réservations sources : Makefile, .github/workflows/ci.yml, tests/test_install_phase_targets.py, tools/verification/recipes/install-phase-targets.json. Base23df31007846882fe83bfc6ffea722227449c373. Autres chats en attente sauf09 pour cette revue.

## Changement actuel

- make test-install reste le point d’entrée. Deux appels récursifs de recettes distinctes préservent l’ordre native puis Docker même avec-j ; les cibles test-install-native/test-install-docker sont PHONY. Les deux commandes d’installateurs sont inchangées.
- Le workflow remplace seulement le bloc groupé par trois étapes nommées. Comparaison brute et reconstruction YAML : reste du document identique, dont triggers, job install, toolchain, permissions et conservation always. Aucun nouveau déclencheur, délai, retry ou contournement.
- Le test ne copie que Makefile pour l’exécuter dans une fixture. Un python3 fictif accepte seulement les chemins des deux installateurs ; les vrais fichiers ne sont pas copiés/exécutés. L’environnement enfant est minimal. Ce contrôle de noms n’est pas un parseur de sécurité Make ni un confinement.
- --output est obligatoire, neuf, dans un parent existant admis par l’appelant ; pas de repli implicite/tmp. Les fixtures temporaires du lanceur permanent gardent leur résolution configurée, sans déplacement du dépôt, des données ou du service.

## Preuves et limites

Préimages Makefile7b7c104c… et workflowe6e34e4e… relues identiques à la proposition conservée. Postimages d271ecfa… et8c8db73c…. Test e52cff13… ; manifeste2d308a31…. Sources exactes et commandes dans /run/media/fpizzi/SWARM/swarm-evidence/root00-pr40-phase-visibility-20261117.

Avant curation, deux contre-exemples : absence de make → testsRun0/skipped1/wasSuccessfulTrue ; MAKEFILES hérité charge un include qui écrit un marqueur uniquement dans la fixture. Corrigés par refus de prérequis, environnement contrôlé et gardes maintenues du CLI. Chaque injection est précédée d’un témoin valide sur la même fixture. Hypothèse distincte «filtre sans test retournerait0» infirmée : Python installé retourne5 ; l’assertion du diagnostic principal attendait0 à tort, aucun code modifié pour fabriquer ce défaut.

Cinq méthodes nommées PASS sur les octets finaux via CLI direct (fake-qualification-01), puis cinq via le **nouveau consommateur** recette→TMPDIR→sortie explicite (recipe-adapter-01) ; ce second passage qualifie le raccordement ajouté, pas une campagne native historique. recipe.py verify confirme intégrité/statut sans rejouer. Les deux passages sont distincts, pas dix tests métier uniques. Les canaux, sources, argv et effets de doublures sont conservés. Les gardes du CLI incluent témoin puis make absent, configuration ambiante exclue, succès ordonné, refus avant Docker fictif et reprise contrôlée.

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/install-phase-targets.json --output /PARENT_ADMIS/PREUVES_NEUVES
python3 tools/verification/recipe.py verify --evidence /PARENT_ADMIS/PREUVES_NEUVES
```

Les appels ci-dessus exigent allocation/admission ; ils ne lancent pas les vrais6+8 mais exécutent des doublures. Déclarations de sources et stdlib/make ne constituent pas une qualification de compilateur ou du runtime Docker. Le disque SWARM ext4/UUID vérifié et le fdvolume ont été tenus pendant enfants/publications ; réserve hôte/sortie20Gio maintenue. Les fixtures configurées restent locales ; preuves surSWARM.

## Revue continue et reprise

Politiques : -j2, retours fictifs3/64, noms/compteurs attendus sont des données de fixture ou des assertions de protocole, pas politiques du produit. Aucune limite utilisateur/timeout n’a été modifiée. Documentation : fonctions/classes non triviales et cible Make documentées, versions FR/EN liées. Erreurs : prérequis manquant est un échec ; argv/canaux/code sont retenus avant assertion, refus natif fictif interdit la phase Docker fictive, et restauration vérifiée.

Aucun merge, PR créé, workflow_dispatch, rerun GitHub, Docker/Cargo/provider ou installation locale. Le code de l’application n’a pas changé ; aucun version_history/restart n’est pertinent pour ces seuls outilsCI. Préserver le WIP par commit/push après revue ; ne pas créer une PR qui lancerait implicitement les6+8 sans condition acquise. L’ancienne panneAPT reste ouverte : une précondition Docker nouvelle et vérifiable manque encore pour l’unique rerun exceptionnel autorisé. Le budget natif planner3→4/child2→3 reste en attente humaine et n’est pas touché par ce lot.

RETEX : borner un blocage à son contrat ; l’attente du budget d’une fixture ne doit pas masquer un artefact source indépendant attribuable. Cette reprise conserve les refus et droits, ne crée aucune autorisation nouvelle et ne prouve aucun effet futur du heartbeat. Index transmis par REPRISE-SUPERVISION-20261115.json ; aucun réveil général des chats.

[English](../../../en/plans/PR40-PHASE-VISIBILITY-20261117.md)


## Revue reçue — 2026-11-17T04:50:19.386855+00:00

09, tour01a1e7d2-e6f6-72e0-8ec7-c5e0d56afb73 : APPROVE_SOURCE_AND_RETAINED_FAKE_PROOF, limité à PR40-OBS-01. Quatre empreintes et base23df3100… vérifiées,11artefacts du résultat de recette relus/hashes contrôlés ;5noms/Ran5tests/OK/code0 et inventaires stables rapprochés. Aucun test relancé par le reviewer. Les limites GitHub/Docker/vrais6+8/installation/budget restent ouvertes. Sauvegarde bornée à préparer, aucune fusion.
