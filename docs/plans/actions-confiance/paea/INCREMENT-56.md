# PAEA — incrément56 : ancrage privé avant création initiale

Reprise55 dev200 installée/vérifiée ; initialisation publique encore ouverte.
Codex seul, aucun GLM/agent/commit/push. PA-02 partiel,0/17 globaux inchangés.

Le pin du paquet doit être conservé hors de ses octets avant publication. Un
point d’ancrage privé distinct du paquet sera sélectionné/protégé par l’opérateur,
pas découvert dans un paquet reçu. Son contenu ne prouve pas autorité courante,
rollback, humain, autre UID, hardware ou confinement. Aucune génération de clé
ni création de service dans la tranche d’ancrage.

Première dépendance constatée : Documents sait lire sans fsync ou lire/acquitter
sous SHA256 indépendant. Calculer un SHA depuis le fichier reçu puis appeler la
seconde API comme si ce SHA était une ancre indépendante serait une fausse preuve.
Comparer cette réutilisation ambiguë avec une entrée explicitement dédiée à la
lecture durable d’un fichier dont le LOCALISATEUR est déjà protégé. Retenir
`read_trusted_private_durable`, conservant BytesRead et jamais PinMatched ; le
contrat pin obligatoire existant reste intact. Ce choix est mécanique de protocole,
sans politique opérateur supplémentaire ni fallback.

Séquence Codex : (1) tests publics avant implémentation, (2) mutualisation FS sous
option interne explicite, contrat/docstrings FR/EN et refus/fermetures/reprise,
(3) record canonique immuable request_pin/package_pin et ancrage NOREPLACE,
(4) recette exacte/profil normal/build/install/parcours affecté. Création publique
puis reprise sous l’ancre requises dans la suite ; une ancre orpheline ne permet
jamais de régénérer implicitement un paquet après résultat incertain.

Critères lecture : témoin privé réel, file+directory sync, réouverture exacte,
aucune preuve de pin inférée, mauvais pin historique refusé avant sync, pannes
FileSync/DirectorySync/ReadClose et causes indépendantes, modes/liens/nom/
annulation refusés puis restauration/reprise ; tests existants du lecteur épinglé.
Propriétaire Codex ; état proposé/qualification non exécutée.

Lecture read02 :5nouveaux+4historiques+8descripteurs et Rustdoc strict PASS stable VERIFIED. Ancre anchor04 :8tests PASS ; un neuvième témoin ajouté couvre refus pur +close indépendant, puis schéma/Rustdoc requis. Tests du record écrits avant première compilation mais après source initiale : écart à la règle tests-d’abord conservé, pas de preuve historique inventée. CLI checkpoint-check :6témoins publics échouent réellement avant route (cli-red05), suite en cours. Schéma :regex SHA avec dollar acceptait un dernier newline ; témoin conservé et longueur64 explicite ajoutée, protocole Rust déjà strict. Autres schémas à examiner séparément, sans conformité globale déduite.

Résultats conservés : read02 stable VERIFIED (17Rust), anchor06 stable VERIFIED (9Rust+3schéma+Rustdoc), cli07 stable VERIFIED (6Rust publics+3schéma+Rustdoc). Depuis cli07, seuls quatre schémas hérités déclarés mais non consommés par ses pas ont changé : min/max64, témoin digests-red08 puis digests09 (5champs+9tests de témoins complets). Octets Rust, nouveau schéma CLI et tests de cli07 inchangés ; aucune égalité de tout son inventaire déclaré courant revendiquée. Le test générique initial affichait PASS après subTest en échec malgré exit1 ; compteur de succès vérifié ajouté, original intact. Suite :5parcours CLI natifs sur binaire normal après livraison, contrôles effectifs/UI/source-review ; initialisation publique toujours ouverte.

## Résultat local vérifié — incrément56

**v0.1.1-dev.201**, PID930797, ELF
`12cc17c22139e29a0eac2a2310c4eeba1aeae55d3f72c12dd2819232afd6af7c` :627 entrées effectives,62 entrées PAEA de production
identiques aux sources ; profil normal sans test-fixtures, canal sans feature.
Build/restart pris en charge et sélection du candidat par SHA, sans commit/push.

| Contrat | Preuves réelles | Limite |
| --- | --- | --- |
| Lecture de localisateur protégé | read02 :5nouveaux+4épinglés+8FD Rust, Rustdoc stricte | BytesRead, jamais PinMatched ; confiance non prouvée |
| Ancre immuable | anchor06 :9Rust+3schéma et Rustdoc stricte | Métadonnées, aucun paquet/clé/proof/service créé |
| CLI publique | cli07 :6Rust+3schéma et Rustdoc stricte | Tests-d’abord réels de route, distincts du défaut de méthode du record |
| Schémas antérieurs | digests09 :5champs SHA exacts+9tests complets ; diff sémantique vérifié | Aucune nouvelle protection native déduite |
| Noyau épinglé affecté | affected-operator12 :5Rust | Refus/cleanup/nonce sous lecteur modifié |
| Nouvelle CLI sur201 | active-anchor14 :5parcours/67commandes | Échec ENOSPC réel garde lecture acquittée |
| Export sur201 | active-operator15 :6parcours/154commandes | Pas séparation réelle/service admis |
| Précontrôle/installation/appel | active-provisioning16 :5/84commandes ; active-install17 :9/66 ; active-call18 :5/56 | Fixtures privées, aucun daemon |
| Personnel/pair/interface sur201 | active-personal19 :7tests+3gardes ; active-peer21 :3paires/6commandes ; active-ui20 :56+garde script | Version/assets protégés/notes/lien Git/état modifié FR/EN, pas E2/E3 |

Tous ces résultats sont stables et intégrité VERIFIED. Les commandes source de
cli07 ont été exécutées avant le seul renforcement de quatre schémas déclarés
mais non consommés par ses pas ; aucune égalité de tout son inventaire courant
n’est revendiquée. Code CLI/noyau et nouveau schéma inchangés, réexercés sur201.
Schémas et dialogues FR/EN rendus/examinés. Revues exactes et source archive dans
`evidence/20261027-anchor56-0sgzrl4e/`.

Échecs originaux conservés : API absente red01 ; constructeur de test privé
anchor03 et édition syntaxique corrigée avant compilation ;6routes absentes
cli-red05 ;5regex de schémas acceptant newline dans digests-red08 ; route absente
sur ancien binaire200 native-red11 ; profil201 refusé E0432 LIVE_PAGE_ROWS.
Le fournisseur a ensuite été rétabli par l’autre travail ; build frais13 accepté,
sans modification de ses sources ici. Collecteur14 initial arrêté avant recette
par pointeur d’incrément dérivé trop largement, original conservé puis corrigé.
Trois chemins de cache de tests/deux ELF physiques archivés et vérifiés avant
retrait, y compris les deux hardlinks ; aucun cache partagé, preuve ou service supprimé.

La confiance du localisateur reste une précondition, l’égalité de données ne
prouve aucun humain/mandat courant/autreUID/rollback/confinement. Ni paquet
opérateur ni clé/proof/service générés par checkpoint-check. Suite57 : intention
durable avant génération, ancre avant publication du paquet, refus de régénération
implicite après résultat incertain ; utiliser les réservations nonbloquantes
existantes et garder tous les acquittements/refus. PA-02 partiel,0/17 globaux,
humain/UIDhelpers/admission/canal/daemon/E2/E3/PA13/RETEX final ouverts.
