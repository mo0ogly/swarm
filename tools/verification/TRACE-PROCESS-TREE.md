# Observation offline d’un arbre déclaré

`trace_process_tree.parse_declared_tree(textes, nœuds, arêtes)` est une fonction
pure, sans CLI, lecture de fichier, enfant, hachage ou admission. Le contrôleur
fournit les textes strace complets par PID, leur provenance/hashes et un arbre
déclaré indépendant des textes. Les dictionnaires Python ne permettent pas de
retrouver des clés déjà écrasées par un décodeur amont : ce contrôle lui appartient.

Chaque nœud déclare executable, argv, environment et exit_code ; clés exactes,
types stricts, PID entier positif (bool refusé), code normal 0..255 (représentation
du protocole, pas quota). Les noms env sont non vides, sans '=' ni NUL ; valeurs
sans NUL, afin que NAME=VALUE garde la même signification. Chaînes déclarées
comparées en UTF-8 aux octets -xx ; ce n’est pas une prise en charge générale des
chemins Unix non UTF-8. L’arbre a une racine, sans cycle, duplication d’arête,
parent multiple ou nœud déconnecté. Un arbre d’un seul nœud est possible.

Grammaire retenue : execve réussi -xx/-v (helpers EXEC/hex_array existants),
clone3 réussi avec les flags du témoin contrôlé, wait4 avec WIFEXITED/WEXITSTATUS,
exit_group et terminal normal. Adresses/stack_size/taille struct restent des
observations, sans constante noyau inventée. Comparer argv[0] et fichier exécutable
séparément : un spoof déclaré est valide. Chaque clone a son wait correspondant
dans le flux parent ; le code attendu rejoint le terminal observé de l’enfant.
Cette grammaire ne couvre pas les créations concurrentes quelconques : un clone
attend son wait avant le suivant. Forme non couverte, tronquée/reprise/incomplète,
terminal signalé, legacy fork/clone/vfork/execveat/waitid ou ligne mal formée de
famille sélectionnée : ValueError au motif court, jamais succès vide.

Résultat : nodes/edges avec lignes brutes et numéros, observation_scope et
unexamined_lines par PID. Les autres lignes, notamment SIGCHLD/démarrage Python,
ne sont pas interprétées ; leurs numéros/comptages restent visibles et les
originaux demeurent chez le contrôleur. Les deux flags complete_build_qualification
et behavioral_admission restent false. Aucun graphe universel, identité inode de
chaque syscall ou preuve d’exécution à partir d’un texte non authentifié.

Le résultat conserve argv/env et les lignes exec en clair : le contrôleur doit
autoriser ces données et conserver ses fichiers privés. Cette fonction n’est pas
un filtre de secrets ; le témoin actuel n’utilise que LC_ALL=C. Un environnement
descendant arbitraire ou Cargo n’est pas qualifié par cela.

Recette `recipes/trace-process-tree-inert.json` : huit gardes purement synthétiques,
aucun enfant. À la première exécution des sept gardes initiaux : cinq PASS, deux FAIL (nom de famille
mal formé oublié ; '=' dans nom env changeant la signification du dictionnaire).
Ces deux gardes identiques passent après correction, avec témoin/reprise ; les
cinq PASS initiaux sont réutilisés dans leur portée, pas rejoués. La recette
entière n’a pas été lancée après correction. Les refus et phases sont conservés
dans root00-cd01-process-tree-0bitlzr_ sur SWARM.

L’analyse offline des trois traces historiques du worker564552 observe trois
nœuds, deux arêtes, 13 lignes de familles retenues ; 2617 lignes explicitement
non examinées restent dans les originaux hachés. Aucun enfant n’a été rejoué et
aucune capture n’est réattribuée au worker actuel. OFFLINE-ALLOCATION.json et
OFFLINE-OBSERVATION.json lient déclarations, sources/outils et originaux.
Compilation complète, admission C-D01 et 12+3 corps restent ouverts ; aucun
changement du produit servi, installation ou version applicative.

PT-01, revue indépendante : la reprise strace <... wait4 resumed> ne contient
pas le motif erroné « ... resumed> ». Un huitième garde, avec témoin/reprise,
passe de FAIL à PASS après refus de tout préfixe de continuation <... . Les
preuves antérieures et l’analyse offline restent liées à leurs anciens octets ;
aucun rejeu ni requalification de cette analyse sur le correctif PT-01.
