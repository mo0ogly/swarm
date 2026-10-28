# PAEA — incrément 35 : chargement privé de la clé de signature

Reprise du lot34 installé dev136, 59 tests de bibliothèque et 56 contrôles UI
vérifiés. PA-02 partiel ; aucun humain, service personnel ni confinement établi.
Codex direct, sans délégation, GLM, commit ni push. Le lot35 ne génère pas de clé
réelle ni de secret initial. La rotation/récupération de mot de passe conservée
au lot33 reste nécessaire avant clôture globale.

Deux options : importer une clé libre fournie par la requête, ou charger un
fichier privé de démarrage lié à la clé publique indépendante. Choix : second
chemin, avant le raccord du lifecycle. Le chemin sera fourni par le démarrage
protégé ; aucune découverte depuis une requête ou un journal. Le prochain lot
raccordera initialisation/enrôlement/reprise au wrapper détenu.

| Étape | Critère observable | État initial |
| --- | --- | --- |
| 35a | Clé Ed25519 depuis exactement 32 octets privés, public attendu indépendant | à réaliser |
| 35b | Répertoire0700/euid, fichier0600 régulier un seul lien, NOFOLLOW, FD et nom revalidés | à vérifier |
| 35c | Seed détenu effacé, objet non clonable/non sérialisable, diagnostics expurgés | à vérifier |
| 35d | Chaque FD fermé explicitement, causes indépendantes conservées, témoin/refus/reprise | à vérifier |
| 35e | Recette, revue octets politiques/fonctions/erreurs, FR/EN, profil normal et installation | à réaliser |

32 octets est la représentation du seed Ed25519, pas une politique opérateur.
Aucun délai/retry/volume configurable ajouté. Droits0700/0600 et comparaison
inode sont invariants de stockage. La clé chargée est un instantané jusqu’à
fermeture/destruction, pas une preuve d’autorité courante ; ses consommateurs
doivent encore vérifier compte, contexte et journal. Aucun export du seed ni
API publique de signature arbitraire. Pas de garantie d’effacement de toutes
copies du compilateur/OS, du cache disque, du swap ou de confinement même UID.
Le helper partagé réellement relu marche tous les ancêtres sans suivre de liens,
avec owner root/service et refus des écritures groupe/autres hors root-sticky ;
ce contrôle ne constitue pas un confinement OS.

Documentation : architecture FR/EN + Rustdoc. Tests : valide, mauvaises pins,
longueurs, permissions/types/liens, substitution de nom, lecture/stat/close et
reprise. Défauts injectés uniquement en compilation unitaire ; closes réels
réalisés même lors d’un défaut. Aucun watchdog global.

Recherche initiale : un chemin de credentials supposé sous personal/ était
absent ; rg --files a retrouvé src/credentials.rs. Aucun fichier modifié par
cet échec ; contrats relus sur leurs chemins réels. Propriétaire Codex.


Revue avant qualification : référence temporaire à validate_private_directory
non existante détectée par lecture des exports, corrigée avant compilation par
contrôle fstat du FD déjà détenu. Aucun appel silencieux ni API inventée conservé.
Le helper open_private_directory a été relu intégralement : il protège déjà les
ancêtres, contrairement à notre réserve initiale trop faible, corrigée ici et dans
Rustdoc. Test additionnel de vraie lecture partielle après fstat (EOF conservée,
buffer possédant Zeroizing), sans simuler l’erreur d’IO par son texte.


12 tests nommés/2 étapes PASS, intégrité VERIFIED ; Rustdoc courant PASS. Premier
build137 refusé : six entrées effectives d’autrui ont changé pendant compilation.
Original exact conservé dans own-normal-build-refused. La première copie choisie
par répertoire le plus récent était une autre compilation en cours : provenance
corrigée, preuve conservée séparément, jamais utilisée comme preuve de notre échec.
Dev136 reste actif ; installation35 non établie. Reprise après profil supporté
stable correspondant aux sources propres, sans répétition ni suppression de garde.


## Résultat local et suite

12 tests nommés/2 étapes PASS, intégrité VERIFIED ; Rustdoc PASS. Profil normal
supporté dev137 qualifié après stabilité des six sources parallèles, 516 entrées
effectives archivées et vérifiées. Les propres sources de production, manifestes,
lock et notes correspondent au candidat. Événement Cargo réel de dalek sans profil
test avec feature zeroize conservé. Harnais CLI figé : 6 tests PASS, dont5 parcours
processus sur le candidat exact et1 témoin bibliothèque ancien, distinct des8 nouveaux
tests du chargeur. Redémarrage supporté, empreinte du processus vérifiée ;56 contrôles
UI protégés FR/EN/thèmes et garde du script manquant PASS, captures Version relues.
Voir les preuves du présent incrément. Ce bilan est une revue locale du même
contexte, sans acceptation indépendante.

Le refus initial reste conservé, sans réécriture. Un appel d’outil de replay a aussi
été refusé avant exécution pour JSON d’arguments mal formé ; corrigé en générant
la liste d’arguments en Python, aucune recette n’avait alors démarré. Ni ce refus
ni une compilation d’autrui ne sont comptés comme résultat d’un scénario.

Le chargeur est installé dans le binaire local, mais aucun service personnel,
humain réel, confinement ou nouvelle signature de production n’est établi. Suite36 :
wrapper propriétaire avec initialisation depuis le head initial épinglé, enrôlement
via clé privée détenue et reprise depuis l’intention exacte, puis transport/UX.
PA-02 partiel ;0/17 invariants globaux, E2/E3 et PA-13 ouverts. Aucun commit/push.
