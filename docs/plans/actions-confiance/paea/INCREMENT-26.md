# PAEA — incrément 26 : contrôle natif sous autorité courante

## Reprise et décision (Codex direct, sans GLM)

PA-02 exige un contrôle actuel, indépendant du cockpit et des agents. Les
anciens rapports de transport Go ne sont pas une implémentation Rust. Les
tranches 23–25 restent des preuves de laboratoire ; 0/17 invariants protégés.

Deux chemins examinés : ajouter immédiatement un transport Unix, ou composer
le journal signé et le stockage réel avant le transport. Le premier exigerait
l'authentification indépendante des deux extrémités et une identité de processus
par message ; SO_PEERCRED/cookie/UID seuls ne remplissent pas ce contrat.
Choix : noyau de contrôle de bibliothèque, avec attentes injectées uniquement
par un futur démarrage protégé. Aucune activation ou identité humaine déduite.

| Étape | Contrat et résultat observable | Propriétaire / vérification |
| --- | --- | --- |
| 26a | Journal existant retenu par connexion, sans recréation ni réouverture par opération ; fermeture explicite | Codex ; refus, annulation, contention, remplacement de chemin |
| 26b | Lire/prévoir/créer/appliquer sous transaction d'autorité courante et permission exacte ; stockage et témoin réels | Codex ; refus avant effet, deux ordres de révocation, rejeu |
| 26c | Erreur d'autorité après appel du stockage conserve un résultat d'effet inconnu et ses causes | Codex ; annulation après publication, lecture de reprise |
| 26d | Recette permanente, revue politiques/fonctions/erreurs liée aux octets et documentation FR/EN | Codex ; résultats et limites à compléter |

Les grants résident dans le journal signé ; aucun délai, plafond, nouveau droit
implicite ou réseau n'est ajouté. Le verrou ne promet pas une expiration atomique
avec un effet externe. Le transport public, l'authentification humaine, le compte
service, le confinement, PA-03, E2/E3 et le RETEX global restent ouverts.

Installation : vérifier le graphe réel. Une bibliothèque sans consommateur livré
n'est pas un service installé ; ne pas annoncer son activation dans le cockpit.
Aucun commit/push autorisé par cette reprise. Les travaux préparés d'autrui restent
intacts. Preuves : `evidence/20261026-control26-lxj7a8br/`.


## Résultats vérifiés et RETEX de la tranche

La recette permanente finale `recipe-final-03` passe **41 tests nommés en sept
étapes**, sans watchdog global : 3connexion retenue, 9journal, 1réglages SQLite,
9contrôle courant, 10payloads d'autorité, 8publication et 1preview coordonné.
Son intégrité est vérifiée séparément. Les essais utilisent de vraies connexions
SQLite et les deux répertoires du Coordinator, avec threads concurrents dans un
processus de test. Ils ne démontrent ni comptes séparés ni transport interprocessus.
Témoin valide avant faute, refus sans configuration, reprise, réouverture, rejeu
sans doublon et effacement de la permission privée sont observés. Le double échec
conserve l'annulation d'autorité et l'erreur de témoin ; même événement repris sans
doublon. Aucune preuve humaine ou E2/E3 n'est ajoutée.

Les originaux `bound-01` (annotation de type manquante) et `recipe-01` (huit refus
path avant les scénarios) restent intacts. Cause de ce dernier : parent de fixture
non explicitement privé sous umask0002 ; passage explicite à0700, comme les
fixtures existantes du Coordinator, puis huit et neuf cas PASS. Le formatter avait
aussi refusé un module de test non encore créé ; module créé avant nouvelle passe.
Aucune borne, protection ou critère retiré. La recette finale02 suit la mise à
jour des contrats des helpers de fixture ; les résultats01 sont conservés.

`normal-profile-02` : build de bibliothèque normal **sans test-fixtures** dans
les quatre crates d'enrôlement, profils Cargo et hashes des artefacts retenus.
Le vrai depfile référence **17sources locales effectives**, rapprochées des
empreintes avant build et archivées ; leurs octets restent identiques. Les
sources de test hors graphe normal ne constituent pas des entrées compilées de ce
profil. Le binaire de test, avec config/test-fixtures, est observé séparément.
Rustdoc des deux crates modifiées compile avec les liens internes cassés refusés.
Ce contrôle n'est pas la qualification globale des dépendances/compilateurs de la
migration. La recette archive ses entrées déclarées, distinctes de ce depfile.

Revue **locale, non indépendante** des politiques, fonctions, erreurs et documents
liée aux hashes dans `source-review.json` ; aucune conformité globale déduite.
Les grants exacts utilisent le protocole effect.execute existant ; aucune nouvelle
politique d'exécution à persister dans Admin. Les durées/identités/clés littérales
des fixtures restent des données de test. Les wrappers conservent cause initiale,
refus métier et échec secondaire ; une erreur mutante ne devient jamais un succès
vide ou une absence d'effet. Documentation fonctionnelle et architecture FR/EN
mises à jour dans le même incrément.

Le graphe normal de `swarm-cli` ne consomme ni authority ni state : ces octets ne
sont pas embarqués dans le serveur livré. Aucun redémarrage ne pourrait activer
ce noyau. Le raccord de livraison attendu dépend encore du transport authentifié,
du bootstrap indépendant et de l'enrôlement humain personnel adoptés le17octobre.
Le choix local/distinct et l'authentification locale par défaut restent acquis ;
ne pas les redemander ni les remplacer par un premier appelant/cookie/UID.
Il n'y a donc ni installation protégée ni modification de version_history.json,
d'asset, de traduction UX ou de dialogue Version à attribuer à cette bibliothèque.
Les108noms de chemins déjà préparés dans Git restent identiques ; aucun add,
commit, push ou arrêt d'un service d'autrui effectué.

Apprentissage transmis aux contrats FR/EN et au plan : tenir un verrou jusqu'au
retour d'un stockage indépendant ne rend pas leurs deux commits atomiques.
Conserver résultat et causes secondaires, puis exiger lecture/rejeu exact sous
une autorité encore valable. Les processus agents déjà démarrés ne reçoivent
pas ces documents automatiquement.

PA-02 reste **partiel**. Prochaine dépendance conservée : raccorder le démarrage
protégé et l'authentification locale personnelle au transport Rust, puis qualifier
le service sous compte distinct/confinement. Codex possède ce raccord ; opérateur
possède la preuve initiale d'installation et l'enrôlement réel. L'activation exige
leurs preuves effectives ; le noyau vérifié ne les fabrique pas. PA-03, E2/E3,
0/17invariants protégés et le RETEX final du programme restent ouverts.

Le contrôle final de format a détecté l’ordre non canonique du nouveau module.
Rustfmt l’a corrigé ; normal-profile-02 et recipe-final-03 relient compilation
et tests aux octets finaux. Le refus de format et les résultats antérieurs sont
conservés. Le disque dispose encore d’environ700Mo : pas de nettoyage d’autrui ;
capacité à vérifier avant le prochain build, propriétaire Codex/opérateur.


## Enregistrement demandé par l’utilisateur

Après vérification de la tranche, l’utilisateur demande le commit. Enregistrement
isolé des15fichiers revus, avec préservation des108chemins déjà préparés par
d’autres travaux. Sources exécutables inchangées ; les41tests et le build normal
restent ceux des preuves ci-dessus. Aucun push ni nouvelle activation de service
par cette opération. Reçu Git : `evidence/20261026-control26-lxj7a8br/publication-01/result.json`.
