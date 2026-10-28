# IA3c.2f-03a — raccord privé de contrôle local

17 octobre 2026. Codex seul, branche codex/clear-product-method-names,
HEAD 4e58f7cbf8200a58f69f1f4d03d657c3313fb642 et modifications locales préservées.
**Livré et vérifié en fixtures ; service système et admission publique non livrés.**

Le raccord attribue chaque message à l’émetteur Linux réel et exige un contrôle
d’autorité sur la demande exacte, y compris pour lire. Il consomme exclusivement
Coordinator : configuration souhaitée et témoin coordonné, sans déduire d’autorité
à partir de l’UID, de l’Admin ou du premier appelant. Le contrôleur réel devra
maintenir la sérialisation de son autorité pendant l’opération ; ici ce sont des
callbacks locaux. Aucun compte, service, listener, fournisseur, mission ou clé
réelle installé/modifié. Aucun appel de modèle, commit, push ou déploiement.

## Périmètre et contrats

Deux sources moteur ajoutées :
- internal/engine/agent_identity_config_service_linux.go ;
- internal/engine/agent_identity_config_service_linux_test.go.

Le canal IA2 est réutilisé sans modifier ses sources. Requête canonique schéma 1,
opérations read/preview/create/apply, Change obligatoire hors read:null ; ni acteur
ni date du client. Projection et View distinctes. Le principal doit correspondre
aux pins indépendants. Seul le Change exact peut atteindre le coordinateur sous
ses réservations ; apply reprend aussi une intention pendante exacte. Aucune
commande libre de réparation, réinitialisation ou adoption de témoin.

Le contrôleur d’autorité reçoit une copie de la demande et le message attribué,
pas une autorité déclarée. Son omission, son erreur absorbée ou son second appel
ne fabrique pas un succès ; une invocation retenue après retour est refusée.
Il doit exécuter synchroniquement : ces gardes ne prouvent pas la sûreté d’un
contrôleur asynchrone ou malveillant. L’objet message et sa ressource pidfd sont
fermés à la fin ; les métadonnées invalides/paquets tronqués sont refusés par le
canal avant stockage. Pas de copie des primitives de réception IA2.

Les erreurs métier renvoient seulement un code limité et null pour les deux
résultats. Le service garde les causes Go originales. Échec après effet ou perte
de réponse après écriture : issue incertaine, sans rollback ; même EventID/base
réautorisés, sans second événement ni réécriture. Le client public devra encore
authentifier le service et gérer une réponse qu’il ne peut pas recevoir : aucune
signature ou garantie de pair service n’est inventée par cet adaptateur.

## Résultats liés aux entrées

| Critère | Preuve et résultat | Limite |
| --- | --- | --- |
| Format strict, quatre opérations, refus sans résultat | 44 noms moteur normal/race-03 PASS, zéro FAIL/SKIP | Tests et sous-tests, pas 44 attaques indépendantes |
| Preview, save/read/reopen, CAS et retry | View/Projection et octets config/témoin comparés | Racines temporaires isolées |
| Autorité exacte, lecture comprise | Refus de mauvais humain, contrôle absent, erreur absorbée, omission/double/retard ; demande copiée | Contrôleur réel non implémenté |
| Émetteur réel après transmission du socket | Deux enfants effectifs, acceptation et refus même UID ; PID du message distinct du parent créateur | Même compte de fixture, pas séparation de comptes |
| Intention pendante | Lecture refusée ; autre demande et autorité refusées sans mutation ; exact apply retrouve entrée/date originales | Intention construite en fixture, pas panne réelle |
| Perte d’acquittement | Client fermé après envoi, effet effectivement enregistré, ErrIndeterminate, retry identique sans doublon | Pas de reboot ni panne matérielle |
| Dommage transport | Troncature/injection de descripteurs refusées avant effet | Réponse EMSGSIZE spécifique non provoquée ; code traite l’erreur d’envoi |
| Coordinateur antérieur | 101 noms normal/race-01 PASS ; 12 entrées propres inchangées vérifiées | Réutilisation documentée, pas nouveau cumul de tests |
| Qualité Go et binaire de fixture | vet-03 et build-03 PASS, mêmes entrées/overlay | Aucun binaire déployé |
| Contrôleur de méthode | 4 tests Python PASS, dont variantes ; vrai contrôleur PASS avec 5 tests/sous-tests | Sorties contrôlées et Go réel séparés |
| Documentation | FR/EN, schémas rendus et inspectés, installation, méthodes et note scientifique actualisés | Ni validation UX, ni acceptation indépendante |

Les 44 noms moteur et les 101 config sont deux ensembles distincts ; la vérification
de méthode constitue un autre contrat. Ne pas additionner les relances normal/race,
les lots 01/02/03 ou les helpers enfants en autant d’attaques. Les fichiers *-run.json
contiennent commandes, dates, compte, noms, sorties et stabilité des entrées.
Les stdout/stderr originaux sont dans les .jsonl/.stderr et .stdout correspondants.

## Replan de la vérification dans le dépôt partagé

Lot 01 : 40 noms moteur et 101 config PASS normal/race, vet/build PASS. Ajouter
quatre noms moteur : variantes enfant autorisé/refusé, pending exact et absence/
annulation, après archivage du candidat 01. Lot 02 : normal 44 PASS, race 44 PASS
mais **preuve rejetée par le lanceur** : work_row.go et work_row_test.go changent
pendant compilation. L’AssertionError et les entrées sont conservés ; ce lot
race n’est pas annoncé stable ou final. Aucun défaut d’assertion identité observé.

Décision : Go overlay explicite pour ces deux sources mémoire, copiées une fois
sous .overlay.txt dans cette aire de preuve. Aucun checkout ou état de mission
copié, aucune modification du chantier mémoire. overlay.json et overlay-sources.json
fixent leurs octets ; les 830 entrées Go/modules réellement consommées restent
identiques entre normal, race, vet et build du lot 03. Toute autre mutation est
refusée. Comparer aussi l’overlay au dépôt à la fin dans checks-final.json : cette
preuve décrit ce candidat compilé, pas automatiquement un futur état partagé.
L’archive final-sources.tar.gz conserve les 830 textes sous leurs noms logiques.

Les 101 noms config sont réutilisés : config-evidence-reuse.json compare les douze
sources/modules propres à ce paquet et à identitycontract. Leur pertinence ne dépend
pas de l’édition de work_row ou du test moteur. Go global -timeout=0 et enfant
-test.timeout=0 ; aucun nouveau délai ou retry. Les helpers IA2 antérieurs gardent
leur délai de socket de fixture préexistant (5 s) : il n’a pas été transformé en
politique produit ni augmenté ; aucun de leurs appels n’a expiré ici.

## Améliorations et erreurs prises en charge

| ID | Origine, valeur et périmètre | État, propriétaire, critère et suite |
| --- | --- | --- |
| I2f03a-01 | Raccord de service : une connexion attribuée ne prouve pas l’émetteur après passage du descripteur ; réutiliser pidfd par message | Réalisé/vérifié en enfants même UID ; Codex ; confinement réel restant EP-07 |
| I2f03a-02 | Contrôleur pouvant omettre/absorber/doubler/retarder l’exécution ; éviter faux succès et second effet | Gardes et fixtures vérifiées ; Codex ; source d’autorité réelle et sérialisation à vérifier avant activation |
| I2f03a-03 | Envoi/réception et historique croissant : capacité de socket/buffer différente d’une politique métier | Perte de réponse testée ; taille de réponse spécifique à mesurer avant client public ; Codex ; protocole de transfert observable sans plafond métier caché |
| I2f03a-04 | Watchdog global 30 s hérité dans check.py, aucune justification contractuelle visible | Retiré, -timeout 0, vrai contrôle PASS ; Codex ; sources avant correction conservées |
| I2f03a-05 | Deux sources mémoire modifiées pendant compilation du même paquet ; preuve non liée au candidat | Rejet conservé, overlay borné et manifestes effectifs vérifiés lot 03 ; Codex ; réexaminer l’écart avant intégration |
| I2f03a-06 | Code 0 sans test exécuté accepté comme OK par le contrôleur | Un contre-exemple FAIL avant/PASS après, quatre tests Python et cinq noms Go réels ; Codex ; prévention dans check.py et docs méthodes FR/EN |

Le déplacement de notre binaire de fixture vers /tmp par rename a échoué EXDEV :
volumes distincts. Reprise par move avec copie inter-volumes, hash vérifié ; aucun
service modifié. Un lecteur auxiliaire a mélangé schémas de résultats Go/Python
et rencontré KeyError counts ; sélection explicite par type de résultat, puis
lecture réussie, sans convertir le manque en zéro/PASS. Ces erreurs de méthode
ne deviennent pas des défauts publics de Swarm ou des échecs Go. Leur cause et
leur reprise sont aussi conservées dans le plan canonique.

## Vérifier et reprendre

Recette du raccord : run_checks.py 03 utilise l’overlay ; les noms d’essai existants
sont protégés contre écrasement. Pour une nouvelle vérification autorisée, utiliser
un nouveau suffixe après comparaison des entrées, pas remplacer les originaux.
Les 101 config réutilisés ont leur commande dans normal-config-01-run.json et
race-config-01-run.json. Le contrôleur réel : python3 tools/agent-workflows/check.py.
Ses gardes : python3 -m unittest discover -s tools/agent-workflows -p test_check.py -v.
Le fichier checks-final.json et les diff finaux complètent la lecture des preuves.

L’incrément de raccord est terminé sur ses fixtures ; le besoin identité global
reste ouvert. Prochaine étape Codex : point d’entrée/service local et client
mutuellement établis, source d’autorité indépendante, droits de l’opération et
arrêt observable, puis setup/API/CLI/Admin. Opérateur : désignation explicite de
l’humain initial (issuer/subject/mapping) et déploiement du compte/confinement
avec l’autorité de session applicable. Aucune de ces protections ne peut être
inférée d’un fichier 0600, de l’UID ou du cookie cockpit. Tests complets de lanceur,
révocation finale/descendants, plan courant et effet mémoire dépendent de ces
raccords. EP-01/04/05/07, F01–F07, IDA, IA3–IA5 et RETEX restent ouverts/partiels.
Restaurer config et témoin ensemble reste une limite connue. Distant prévu et
refusé par le codec actuel ; pas de secours permissif. Aucune nouveauté ou
supériorité scientifique établie par cet incrément.
