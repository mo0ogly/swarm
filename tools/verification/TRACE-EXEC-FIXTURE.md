# Producteur contrôlé d'essai exec (`trace_exec_fixture_worker.py`)

## Portée

Ce module est un producteur exclusivement dédié aux fixtures : il réalise une
seule tentative contrôlée `execve` ou `execveat` au même PID contre une image
ELF fixe louée par l'appelant, afin qu'une observation par un tracer,
autorisée séparément et plus tard, puisse vérifier la fidélité des octets
d'argv et d'environnement. Python >= 3.11 sur Linux est requis pour
`execveat`. Ce n'est ni un tracer, ni un décodeur, ni un cadre de capture
générique, ni un collecteur Cargo, ni une voie d'admission de compilateur.
Le worker lui-même ne lance jamais `strace`, ne choisit aucune largeur de
chaîne, n'analyse aucune trace, ne collecte rien de Cargo et n'autorise
aucun usage. Il ne rejoue pas les essais antérieurs de l'arbre à trois
processus.

## Bail de l'appelant et observations

L'exécutable absolu, le document privé de protocole et la nouvelle feuille de
reçu sont loués par l'appelant. Le worker observe la régularité, le
propriétaire euid et les modes d'autorisation, et refuse les entrées
inadaptées ; ces observations ne sont ni une autorisation, ni une
authentification de chemin ou d'inœud, ni un confinement, et aucune
autorité n'est dérivée des noms ou des empreintes. L'exécutable peut être un
fichier système possédé par root comme `/usr/bin/true` ; la propriété euid
n'y est délibérément pas exigée. La magie ELF et l'empreinte du tampon sont
des observations de format, pas des preuves de sûreté ; une image ELF fixe
est exigée, ce qui évite les cas particuliers d'interpréteur shebang. La voie
`execve` repose sur la précondition d'un nom de chemin stable côté appelant :
l'empreinte dérivée du descripteur n'est pas une liaison atomique
chemin-contenu.

## Données fictives et non-héritage strict

L'appelant n'autorise explicitement que les octets publics fictifs d'argv et
d'environnement portés par le document. Le module ne lit ni n'inspecte explicitement l'environnement du parent et ne
le fusionne pas dans la table transmise au délégué : celle-ci contient exactement
les octets déclarés. Le démarrage de Python et du chargeur dynamique reste hors
de cette observation ; aucun confinement global de ces phases n'est établi.

## Protocole `trace-exec-fixture/1`

Le document JSON privé porte exactement `schema`, `argv_hex` et
`environment_hex`. `argv_hex` est un tableau non vide — règle du domaine du
protocole de fixture, pas une affirmation générale sur Linux — de chaînes hex
minuscules de longueur paire ; les arguments décodés peuvent être vides ou
non UTF-8 mais jamais contenir NUL. `environment_hex` associe des noms hex
minuscules à des valeurs hex minuscules ; les noms décodés sont non vides,
sans NUL ni `=` ; les valeurs peuvent être vides ou non UTF-8, sans NUL. Les
membres JSON dupliqués sont refusés à toute profondeur. Le tampon brut est
haché et comparé à l'empreinte déclarée par l'appelant avant analyse, sans
plafond de taille. Le descripteur de données est ouvert avec
`O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK` avant `fstat`, si bien qu'un FIFO ne
peut jamais faire attendre le worker.

## ABI Linux

Le mode `execveat` appelle libc `execveat(fd, vide, argv, envp,
AT_EMPTY_PATH)` via ctypes avec `restype` et `argtypes` déclarés ; chaque
objet bytes brut et les deux tableaux terminés par NULL restent référencés
pendant tout l'appel natif. `AT_EMPTY_PATH` (4096) est la constante de l'ABI
Linux, pas une politique d'exploitation. Si le symbole est indisponible ou
l'appel refusé, l'errno original sûr remonte ; il n'y a jamais de repli vers
`execve`. Les deux voies de succès remplacent le processus au même PID, si
bien que le worker ne renvoie jamais zéro, et un retour inattendu d'un mock
lève au lieu de signaler un succès.

## Publication, IO partielle et intention UNKNOWN

Avant l'exec, le worker publie un reçu : une feuille temporaire 0600 fraîche
à tentative unique dans un parent privé possédé par l'euid, une boucle
d'écriture complète tolérante aux écritures partielles, `fsync`, un lien
matériel sans écrasement vers la feuille absente, puis un `fsync` du
répertoire. Les publications partielles ou échouées sont préservées comme
preuve brute ; rien n'est supprimé, réessayé ni écrasé. Le reçu ne porte que
les champs d'intention et `outcome` reste `UNKNOWN` ; il ne contient jamais
de valeur d'environnement, et les faits de tentative ou du reçu n'infèrent
jamais un exec réussi. Chaque descripteur possédé est fermé exactement une
fois ; les causes primaires et de nettoyage sont préservées comme objets
originaux dans des erreurs groupées avec des métadonnées sûres typées, et
aucun `str()` de données non fiables n'est jamais produit dans les reçus ou
les erreurs. Un échec CLI géré est non nul — 130 pour une interruption connue
primaire ou de nettoyage — et si l'écriture du diagnostic stderr échoue, les
deux causes sont levées ensemble.

## Témoin futur sous contrôle root

Un témoin formateur ultérieur avec strace 6.19 installé, alloué et relu
séparément sous contrôle root, pourra consommer une trace de cette tentative ;
ce source n'est pas une attestation d'un binaire installé. Paraphrase des
sources primaires : `execve.c` de strace décode `pathname`, `argv` et `envp`,
en imprimant `envp` comme simple compteur d'arguments (`printargc`) quand
l'abréviation est active, tandis que `printargv` parcourt le vecteur de
pointeurs jusqu'à NULL en signalant toute troncature supplémentaire.
strace.1 documente `-s`/`--string-limit` (défaut 32 ; les noms de fichiers
sont toujours complets) et `-v`/`--no-abbrev`, qui désactive l'abréviation de
l'environnement. Une largeur finie explicite — `-s 0` n'est pas illimité —
plus l'abréviation maintiennent les valeurs d'environnement hors de la sortie
collectée. Aucune capture arbitraire de descendants n'est garantie ni
fournie.

## Références

- <https://raw.githubusercontent.com/strace/strace/v6.19/src/execve.c>
- <https://raw.githubusercontent.com/strace/strace/v6.19/doc/strace.1.in>

## Consommateur d'erreur et vérification inerte

Les diagnostics conservent types/errnos des groupes, causes et contextes imbriqués
sans messages d'exception ; les alias/cycles sont signalés. Les interruptions runtime
hors Exception, dont SystemExit(0) et les nettoyages, renvoient 130. Aide et arguments
invalides gardent leurs sorties distinctes 0/2 avant ce traitement. Un échec de
publication du diagnostic lève toujours les deux objets originaux. Sept gardes inertes
nommées couvrent ces contrats ; tous les délégués exec sont doublés. Ces résultats
n'établissent ni exec natif réel, fidélité du tracer, admission Cargo, installation
ni fusion main. Les erreurs initiales de fixture et les échecs avant correction
restent conservés dans les preuves du paquet26.

La frontière exécutable entry_main transforme l’échec de publication du diagnostic
en 130/1 sans nouvelle écriture stderr ni traceback original non intercepté.
La fonction bibliothèque cli_main lève toujours ensemble les objets originaux ;
ses appelants doivent protéger cette frontière au lieu de rendre leurs messages.
Une garde inerte couvre une première écriture refusée puis stderr rétabli, pour
erreur ordinaire et interruption. Elle utilise le formateur traceback Python sur
le SystemExit terminal ; aucun sous-processus/exec natif n’est exercé.
