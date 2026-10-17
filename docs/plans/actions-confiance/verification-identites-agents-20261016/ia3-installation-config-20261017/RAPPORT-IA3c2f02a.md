# IA3c.2f-02a — noyau de configuration d’installation

17 octobre 2026 — **format et stockage local vérifiés en fixtures ; raccord absent.**

Le choix local du pilote dispose désormais d’un noyau de configuration souhaitée
persistante, dans un répertoire d’installation explicite plutôt qu’une base de
projet importable. Aucun réglage actif, endpoint, écran, service ou compte créé.
Le [plan canonique](../../identites-agents-replan.md), la
[cible](../../identites-agents-enrolement-IA3c2.md) et les contrats
[FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md) /
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) décrivent l’état réel.

## Livrable et frontières

Quatre nouveaux fichiers sous internal/enrollmentconfig : format strict,
stockage Linux et deux fichiers de tests. Le bootstrap doit fournir pins et
Authorizer indépendamment des demandes, agents et imports de projet. La racine
publique et le principal humain sont immuables dans un changement ordinaire.
Le mode local requiert un chemin socket canonique ; distant reconnu mais refusé.

EncodeChange/DecodeChange sont la frontière commune future setup/CLI/API/Admin ;
Encode/Decode valident le snapshot complet et sa chaîne. Révisions en chaînes
décimales uint64 pour préserver l’exactitude côté navigateur ; aucune policy de
budget/délai/retry/rétention. La limite 107 octets est le format sockaddr_un Linux,
exposé par une constante Go ; aucun champ frontend max caché n’est ajouté.

Open vérifie le chemin et peut initialiser un verrou privé, sans config par défaut.
Preview retourne une Projection distincte sans sauvegarde ; Create est un
bootstrap explicite, jamais une remise à zéro publique ; Apply refuse l’absence
et les révisions périmées ; Read attend un checkpoint indépendant. Autorisation
réévaluée même pour le retry exact courant. Données retournées possédées, pas
une capacité moteur. Fichiers privés, symlinks/hardlinks/FIFO et permissions trop
ouvertes sont contrôlés ; cela ne confine pas les agents sous un même UID.

L’écriture synchronise temporaire/fichier, rename atomique puis répertoire.
L’échec après rename est explicitement incertain ; observer puis reprendre
l’événement courant exact avec autorisation et nouvelle barrière, sans second
événement. Fault injection de fsync, pas de panne matérielle réelle. Un Head
restauré avec son fichier retire la détection de rollback : témoin externe durable
et maintien de son actualité restent ouverts.

## Preuves et résultats

| Critère | Observation | Preuve / limite |
| --- | --- | --- |
| Codec snapshot et demande | Versions, types, champs supplémentaires/dupliqués/casse, nulls, omissions, trailing refusés | Tests nommés réellement exécutés dans [normal-05](normal-05-run.json) |
| Stockage et reprise | Create, preview sans écriture, Apply, Read et réouverture ; historique/checkpoints exacts | Même lot, répertoire temporaire isolé |
| Refus et mutations | Auth refusée, mauvais principal, annulation, CAS, événements réutilisés/historiques, mode distant ; aucune configuration acceptée | Erreurs et résultats nuls contrôlés |
| Concurrence | Deux handles : ErrBusy sous réservation, puis conflit après application concurrente | Pas de timer, attente interne ou retry caché |
| Panne/reprise | Fsync répertoire en faute après rename, observation et retry exact ; initialisation et mise à jour | Injection locale, historique non dupliqué |
| Chemins et permissions | Répertoire, parent, owner, fichier/lock, symlink, hardlink, FIFO, changement après Open, umask | Fixtures même UID ; aucune preuve de confinement fournisseur |
| Ensemble normal final | 70 noms PASS, 0 FAIL/SKIP | [normal-05](normal-05-run.json), [sortie originale](normal-05.jsonl) |
| Même ensemble race | Mêmes 70 noms PASS, aucune course signalée | [race-05](race-05-run.json) |
| Vet et compilation du paquet | PASS | [vet-05](vet-05-run.json), [build-05](build-05-run.json) ; zéro test n’est pas compté comme PASS de tests |
| Entrées de vérification | 8 stables avant/après chaque lot final | [source-final](source-final.json), [archive](final-sources.tar.gz) |
| Sources antérieures | 19 préservées, 4 fichiers de paquet nouveaux | [préservation](preservation.json) ; pas de source moteur ou mémoire modifiée |
| Configuration des workflows | Résultat conservé séparément | [contrôle](workflow-check-run.json), ne prouve pas l’autorité d’identité |
| Documentation FR/EN et schémas | Contrats et installation à jour dans le même incrément | [rendus](diagrams.json), [inspection](visual-inspection.json), [contrôles finaux](checks-final.json) |

Les nombres incluent parents et sous-cas, pas autant d’attaques indépendantes.
Tous les go test ont -timeout=0. Pas de suite globale, binaire serveur reconstruit,
fournisseur réel, recette navigateur, revue indépendante ou acceptation globale.
Le paquet n’est raccordé à aucun consommateur application ; son build ciblé est
la compilation pertinente. Les entrées partagées antérieures sont inchangées.

## Historique de diagnostic conservé

1. normal-01 : 49 noms collectés, 38 PASS et 11 FAIL. Les fixtures refusées à
   l’ouverture n’atteignaient pas leurs contrôles de stockage ; ce n’était pas un
   résultat acceptable. Source de testing.T.TempDir : enfant créé 0777 ; la
   [sonde réelle](tempdir-probe.jsonl) mesure ici **0775**. Préparer explicitement
   les fixtures en 0700, conserver cas négatifs. Pas de protection produit relâchée.
   Une première note disait 0755 avant consommation du résultat : correction et
   cause conservées dans [diagnosis-correction](diagnosis-correction.json).
2. normal/race-02 : 52 PASS. La relecture ajoute trois gardes réellement
   discriminantes : [regressions-red-01](regressions-red-01-run.json) échoue les
   trois avant correction, sources dans [archive originale](before-regression-fixes.tar.gz).
   Les permissions passent après Open à 0770, mais Apply réussit ; Preview accepte
   année 10000 non sérialisable ; un umask 0700 produit une config 0000, pourtant
   annoncée comme sauvegardée. Ce sont des défauts nouveaux du candidat, pas une
   compromission publique déjà démontrée.
3. Correction : recontrôle du répertoire à réservation/persistance ; erreurs de
   sérialisation des têtes propagées et validation complète de preview ; vérification
   du temporaire réel avant écriture. normal/race-03 : 55 PASS. Les trois gardes
   passent ; aucun seuil ou exception d’ID ajouté.
4. Reprise bootstrap incertaine, réautorisation du replay, annulation depuis le
   callback, tamper cohérent et nettoyage : normal/race-04, 58 PASS. Décodeur de
   demande commun ajouté avant les futurs adaptateurs ; normal/race-05, 70 PASS.
   Les sources ne changent pas pendant ces contrôles.

5. Le diff documentaire final révèle trois anciennes flèches modifiées par langue
   par un remplacement trop large. Préfixes FR/EN restaurés à l’identique depuis
   l’archive initiale, nouveau chapitre conservé. [Original et reprise](documentation-graph-recovery.json).
   Aucun source Go touché ; les nouveaux rendus correspondent toujours aux blocs.

## Améliorations, transmission et reprise

| ID / origine | Valeur et périmètre | Propriétaire / état / observable |
| --- | --- | --- |
| I2f02a-01, permissions après ouverture | Ne pas déclarer sûr un handle dont le répertoire est devenu ouvert | Codex ; corrigé et garde FAIL avant/PASS après ; confinement réel non prouvé |
| I2f02a-02, date hors représentation JSON | Empêcher une projection que l’application ne peut persister | Codex ; erreurs propagées, preview et écriture refusent sans résultat partiel |
| I2f02a-03, umask modifié | Ne pas remplacer la configuration par un fichier illisible en annonçant succès | Codex ; tempfile réel contrôlé, ancien fichier intact et nettoyage vérifié |
| I2f02a-04, projections horodatées | Ne pas confondre proposed Head et checkpoint réellement sauvegardé | Codex ; contrat documenté ; futur consommateur doit utiliser le résultat d’application, effet public ouvert |
| I2f02a-05, futur mode distant | Préserver octets et codecs des versions historiques lors d’une migration | Codex ; exigence conservée, migration non réalisée ; reprise avec adaptateur/version et preuve de continuité |
| I2f02a-06, TempDir et note supposée | Préparer une fixture selon contrat, rapporter la mesure exacte plutôt qu’un umask supposé | Codex ; 0775 observé, fixture 0700, erreur de note corrigée et conservée |
| I2f02a-07, décodeur de demande central | Éviter des parseurs divergents setup/API/CLI/Admin ou une racine injectée dans la demande | Codex ; format testé, intégrations encore absentes |
| I2f02a-08, diff documentaire final | Éviter qu’une correction locale de schéma altère les anciens graphes | Codex ; original conservé, préfixes restaurés et égalité vérifiée |

Les fichiers persistants et guides transmettent ces résultats ; aucune consigne
commune ni contexte agent embarqué n’est modifié, aucun processus déjà démarré
ne reçoit automatiquement ce paquet. Les apprentissages propres au contrat sont
dans les documents d’architecture ; pas d’ajout redondant aux consignes générales.

IA3c.2f-02 reste partiel : témoin indépendant durable et opérations publiques à
raccorder. -03 service local, -04 installation/Admin, -05 admission et -06 distant
restent ouverts. Bootstrap réel attend un principal humain exact et un service
protégé ; ces préconditions ne bloquent pas les travaux indépendants sur le
contrat du témoin/service. F01–F07, IDA, IA3 public, IA4/IA5 et RETEX global restent
ouverts. Aucune politique de surveillance ou exécution supplémentaire organisée.
