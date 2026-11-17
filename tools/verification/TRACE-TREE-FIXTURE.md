# Worker multiprocessus fictif

`trace_tree_fixture_worker.py` produit un arbre parent/enfant/petit-enfant
contrôlé, sans tracer ni compiler. Python ≥3.11, `-I -S -B` ; `--root`,
`--python` et `--role` sont obligatoires. Le contrôleur réserve un dossier privé
neuf avec `seed` non vide en lecture seule, vérifie chemins/montage/identité et
maintient le support. Le worker ne fournit ni ces droits ni un confinement.
L’interpréteur explicite est transmis à tous les rôles : argv[0] de l’enfant
est volontairement différent du fichier exécutable. Chaque environnement enfant
est exactement construit avec LC_ALL=C, sans héritage ni shell. Cela ne qualifie
aucun autre programme ou descendant arbitraire.

L’enfant change son cwd, ouvre seed par dirfd, duplique un FD déjà possédé,
lit/hash les octets déclarés, écrit exclusivement derived, renomme vers renamed,
fait relire ce fichier au petit-enfant puis le supprime. Le dossier/seed ne sont
jamais supprimés. Le bail privé rend le scénario contrôlé ; le contrôle d’absence
avant rename n’est pas une primitive atomique anti-écrasement face à un adversaire.
Le worker conserve les fstat réellement observés, sans les attribuer implicitement
à chaque événement strace. Le ctime n’est pas un invariant à travers un rename :
séparer les observations avant production/renommage et pendant lecture stable.

Chaque spawner crée les sorties brutes de son enfant exclusivement dans root
(umask restrictive du contrôleur), conserve PID/code avant flush/close/hash/decode,
et publie un JSON sain seulement après validation. Les diagnostics bruts restent
dans ces fichiers, pas dans les messages d’erreur. Les causes de corps/nettoyage
et reçus connus sont conservés à travers les groupes ; rôle/reçu sûr sur stderr
en cas d’échec. Une erreur de wait n’efface pas un returncode déjà connu dans
Popen. Si wait échoue avant collecte, le worker tente kill/wait du seul enfant
détenu, sans retry/watchdog : ce nettoyage ne prouve ni sa réussite ni l’arrêt de
tous les descendants. Les sorties partielles restent disponibles.

Les tailles viennent des fixtures, les littéraux de rôles et argv[0] sont des
données de test ; sonde EOF et refus de progression nulle sont des frontières
techniques, pas des quotas opérateur. Le fichier entier est en mémoire. Aucun
changement de politique produit, limite de ressources, service ou installation.

`recipes/trace-tree-fixture-inert.json` déclare cinq gardes synthétiques, avec
`SWARM_TRACE_TEST_PARENT` requis : stdout tardif, unlink tardif, fermeture ajoutée
erreur de wait après code connu et flush stdout différé. Chacun a témoin sain et reprise fraîche ;
Popen est remplacé, aucun enfant réel ni strace. Preuves nouvelles : 2 FAIL→2 PASS,
puis 1 FAIL→1 PASS pour fermeture, puis 1 FAIL→1 PASS pour wait et 1 FAIL→1 PASS pour flush, en exécutions
sélectionnées distinctes. La recette entière n’a pas été lancée ; ne pas annoncer
cinq tests nouvellement rejoués sur une même révision.

Preuves externes : `root00-cd01-tree-worker-c1jqksj3`. GLM10 n’a pas fourni de
fragment avant interruption du seul client vérifié ; calcul/usage serveur
inconnus, aucun retry. Le principal a produit ce worker en suppléance autorisée.
Une unique capture sans filtre de chemin conserve trois exec/argv/env/terminaux
concordants, clone3/wait4, dup2, chdir, renameat et unlinkat. Elle utilise les
classes strace file/process/desc/memory, pas tous les syscalls. L’allocation de
chaînes vient de tous les argv/env connus de ces trois rôles.

Le contrôleur a échoué après capture sur son assertion ctime trop stricte.
`ACTUAL-TREE-FAILURE.json` reste original ; `ACTUAL-TREE-REANALYSIS.json` corrige
la lecture de ces mêmes preuves, sans enfant rejoué. Cette capture est liée au
worker SHA `564552fc9efdae57a2f941cfe332eeb90a2c3fa98b94b2ab6fb59a6055a78bf3` ;
le correctif wait ultérieur dispose seulement de sa garde ciblée. Ne pas réattribuer
les traces aux nouveaux octets. Les sources/outils, sorties/PID et hashes sont
conservés. Seule la mesure numérique post-admission native conservée est attribuée
à l’exécution ; les mesures ultérieures ne reconstituent pas la liste RAM perdue.

Ni grammaire générique, fermeture compilateur, admission C-D01, 12+3 corps,
installation ou acceptation globale. Les accès Python de démarrage sont conservés
dans les traces mais ne sont pas qualifiés comme graphe complet.

TW-01 (revue indépendante) : le JSON sain doit être explicitement flushé dans
le try qui conserve le reçu. Une écriture tamponnée peut réussir puis échouer
plus tard au flush de fermeture ; le nouveau garde reproduit ce cas sans enfant
réel, avant FAIL et après PASS. Aucun rejeu de la capture ancienne.
