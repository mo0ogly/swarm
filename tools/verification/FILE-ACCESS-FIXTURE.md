# Échantillons contrôlés d’accès à un fichier

`file_access_fixture_worker.py` est un petit producteur stdlib Python ≥ 3.11,
exécuté avec `-I -S -B`. Le contrôleur fournit et réserve un fichier régulier,
non vide, dont il possède la durée de vie ; le worker ne vérifie pas cette
autorité et ne fournit aucun confinement. Le démarrage Python charge aussi ses
propres ressources : « un fichier » décrit les opérations explicites du worker,
pas tous les accès du processus.

`--input` exige un chemin absolu ; `--mode` exige `open`, `read` ou `map`.
`open` ouvre en lecture seule, observe puis ferme, sans calculer de digest.
`read` consomme exactement la taille observée, refuse EOF prématuré et octet
supplémentaire, puis calcule SHA-256. `map` crée un mapping en lecture seule et
calcule SHA-256 du buffer. Le worker exige l’égalité avant/après de dev, ino,
taille, mtime_ns et ctime_ns. Cela ne prouve ni immutabilité entre observations,
ni provenance des octets. Un événement mmap seul ne prouve pas leur consommation.

Le JSON stdout sain contient pid, mode, input, fd historique (déjà fermé),
before, after et sha256 (null pour `open`, donc non calculé). Les erreurs de
corps et de nettoyage sont conservées ensemble ; un close refusé ne prouve pas
la fermeture. Pas de retry, watchdog global ou quota ajouté : la taille vient
du fichier fictif ; la sonde EOF d’un octet est une vérification de frontière.

La recette `recipes/file-access-fixture-inert.json` ne lance que le garde
synthétique de dérive des métadonnées. Fournir `SWARM_TRACE_TEST_PARENT` avec
une cible de fixtures admise. Le parent sain, la variante injectée (mtime changé,
taille identique) et la reprise saine ouvrent seulement leurs nouveaux fichiers.
Cette recette ne lance ni strace ni compilation. Le runner crée ses propres
temporaires selon son contrat ; son environnement hérité n’est pas une preuve
d’isolation du worker réel.

Les cinq captures réelles du 17 novembre sont dans
`root00-cd01-file-access-4s_1x_t_` sur le volume SWARM : lecture/ouverture/lecture/
mapping/lecture. Le contrôleur ponctuel conservé dans `actual-wrapper.py` utilise
`strace -P` sur le seul fichier fictif, `-y -xx -v`, et `raw=` pour les lectures
(jamais `-e read=`, qui exporterait le contenu). Les traces originales, argv,
résultats, outils/sources épinglés et mesures de réserve restent conservés.
Ce contrôleur n’est pas une recette générale ni une grammaire qualifiée.

Les traces montrent read 64 octets puis EOF dans les cas `read`, aucune lecture
de contenu dans la trace filtrée du cas `open`, et mmap PROT_READ/MAP_SHARED
avec duplication de fd dans le cas `map`. Le filtre ne conserve pas munmap dans
cet échantillon : aucune preuve de nettoyage système complet n’en est déduite.
Le marqueur du worker et le terminal du PID concordent pour chaque échantillon.
Le contrôle d’absence du contenu littéral ne vaut que pour cette donnée fictive.

Aucune qualification complète des entrées du compilateur, admission C-D01,
exécution des 12+3 corps, installation ou acceptation globale n’est établie.
