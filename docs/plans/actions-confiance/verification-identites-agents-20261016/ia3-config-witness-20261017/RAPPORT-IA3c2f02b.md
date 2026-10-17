# IA3c.2f-02b — témoin local et reprise coordonnée

17 octobre 2026 — **101 noms PASS en fixtures ; protection réelle et raccord ouverts.**

Le noyau de configuration dispose d’un coordinateur qui prépare une intention
exacte dans un témoin séparé, écrit la configuration puis confirme le checkpoint.
Aucun endpoint, service système, compte, clé privée ou réglage d’application créé.
Le [plan](../../identites-agents-replan.md), la
[cible](../../identites-agents-enrolement-IA3c2.md), les contrats
[FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md) et
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) décrivent types et limites.

## Contrat livré

OpenCoordinated ouvre deux répertoires explicites privés et distincts, avec pins
et Authorizer indépendants. Réservations non bloquantes, aucun retry/timer ajouté.
Create n’initialise que des états absents ; jamais de témoin reconstruit depuis
une configuration existante. L’intention contient entrée et Head cible vérifiés,
liés au Head confirmé. Même schéma de config v1, aucune migration des octets
historiques ni du journal d’enrôlement/SQLite.

Read/Preview refusent une intention ouverte. Recover ou demande pendante exacte
compare base/cible, réautorise, synchronise l’intention puis applique ou confirme
sans doublon. Une autre histoire ou autorité refusée conserve le blocage, sans
réparation implicite. Déjà confirmé : observation, comparaison et barrières,
pas nouvelle autorisation métier. Le transport futur doit autoriser ces lectures.
Une erreur ne retourne aucun View partiel ; cause, incertitude et étape sont
conservées. Une confirmation visible non acquittée se réconcilie sans seconde
écriture de config. Aucun reset ni abandon automatique d’intention livré.

witness.json contient schema_version, pins, committed et pending (null ou
entry/target). Codec canonique strict, acteur, dates, tête/cible et contexte
contrôlés. Contrôles Linux partagés de fichier/répertoire, sans deuxième Store.
Deux répertoires ne constituent pas une séparation de comptes, d’administration,
de sauvegarde ou d’hôte. Restaurer les deux ensemble reste accepté dans le test
limite : ce noyau n’est ni un compteur matériel monotone ni une racine externe.

## Fichiers et préservation

Quatre nouveaux fichiers : coordinator_linux.go, witness_linux.go et leurs tests.
Deux sources antérieures modifiées : config.go (commentaire de paquet seulement)
et store_linux.go (extraction privée lecture/persistance de fichier, wrappers
publics conservés). Les six autres entrées de référence restent identiques.
[Baseline](baseline.json), [avant](before-sources.tar.gz),
[préservation](preservation.json), [sources finales](source-final.json),
[archive finale](final-sources.tar.gz), [diff des sources existantes](source.diff).
Aucune source moteur, mémoire, frontend, configuration de workflows ou processus
modifiée par cet incrément. Branch/HEAD et environnement dans environment.json.
Les fichiers sont non commités dans le checkout partagé ; aucun push/déploiement.

## Vérification et limites

| Exigence | Résultat observé | Preuve / limite |
| --- | --- | --- |
| Contrats antérieurs | 70 gardes rejouées dans le lot final | Nouvelle extraction privée vérifiée sans changement de format |
| Codec témoin | Nulls/omissions/types/version, doublons/casse/unknown/trailing et intention incohérente refusés | Tests réellement collectés ; mauvais contexte valide refusé |
| Création/preview/read/reopen | Deux états, head confirmé, aucune écriture de preview | Comparaison de contenu et réouverture de nouveaux handles |
| Rollback et absence | Restaurer config seule OU témoin seul refusé ; témoin absent non appris | Restaurer les deux accepté : limite conservée, pas protection hôte prouvée |
| Faute avant config | Intention visible, barrière non acquittée ; config absente jusqu’à reprise | Injection fsync et refus de métadonnées, pas panne matérielle |
| Config visible sans confirmation | Read/Preview/demande différente refusés ; refus d’autorité sans mutation | Reprise exacte conserve l’instant et le nombre d’événements |
| Confirmation incertaine | Aucun View sur défaut fsync ; replay et barrières sans nouvel événement | Cible confirmée visible puis relue ; cause conservée |
| Histoire étrangère/manquante | Reprise refusée ; pas de réinitialisation silencieuse | Intention gardée, opération autorisée de restauration encore à concevoir |
| Fichier témoin | Symlink, permissions ouvertes, hardlink, format et pins invalides refusés | UID de fixture ; bind mount de même inode non monté, contrôle présent dans le code |
| Annulation et concurrence | Après intention : config absente, décision reprise avec contexte neuf ; autre handle ErrBusy puis stale ErrConflict | Pas de watchdog ni boucle interne |
| Sortie de processus | Enfant sort par os.Exit sans Close, nouveau coordinateur reconstruit et reprend | Processus de test local, pas SIGKILL, reboot, coupure électrique ou séparation UID |
| Normal final | 101 noms PASS, 0 FAIL/SKIP | [normal-04](normal-04-run.json), [original](normal-04.jsonl) |
| Même ensemble race | Mêmes 101 noms PASS, aucune course signalée | [race-04](race-04-run.json) |
| Vet/build ciblés | PASS | [vet-04](vet-04-run.json), [build-04](build-04-run.json) ; pas des tests de comportement supplémentaires |
| Inputs | 12 stables avant/après chaque contrôle final | Source-final et manifests de chaque lot |
| Documentation et schémas | FR/EN mis à jour dans le même incrément | [rendus](diagrams.json), [inspection](visual-inspection.json), [contrôles finaux](checks-final.json) |

Les comptes incluent parents/sous-cas et ne mesurent pas autant d’attaques.
Tous les tests et le processus enfant ont timeout=0. Pas de fournisseur réel,
service natif Swarm, recette navigateur, revue indépendante ou acceptation globale.
Le paquet n’est pas importé par l’application ; compilation ciblée pertinente,
aucun binaire serveur remplacé. Les injections fsync vérifient la gestion d’erreur,
pas la persistance après panne matérielle de tous les systèmes de fichiers.

## Replan, améliorations et transmission

91 puis 99 noms PASS avant la revue du diagnostic. [stage-red-01](stage-red-01-run.json)
reproduit un défaut : une faute E/S générique conserve cause/ErrIndeterminate,
mais ne nomme pas la frontière échouée. [Sources avant correction](before-error-context-fix.tar.gz).
Correction générale des contextes prepare witness / write configuration /
confirm witness et barrières ; errors.Is conserve la cause. 100 noms PASS,
puis un test de sortie de processus ajouté pour une preuve réellement distincte :
101 PASS. Aucun échec de compilation présenté comme preuve comportementale.

| Découverte | Valeur / périmètre | Propriétaire, état et critère |
| --- | --- | --- |
| I2f02b-01, erreur générique injectée | Diagnostic de la frontière échouée et reprise exploitable | Codex ; FAIL avant/PASS après, cause et étape conservées |
| I2f02b-02, plusieurs reprises sur un même objet | Vérifier réception depuis fichiers dans un nouveau contexte processus | Codex ; enfant sans Close puis reprise fraîche PASS ; pas preuve de reboot/power loss |
| I2f02b-03, panne ou autorité perdue avec pending | Éviter un abandon ou reset implicite ; arbitrage d’une annulation/compensation | Codex/opérateur ; conservé, pas d’API abort ; reprise avec rôle/contrat autorisé et preuve de base/cible |
| I2f02b-04, deux répertoires sous même hôte | Ne pas confondre témoin séparé et indépendance réelle des sauvegardes | Opérateur/Codex ; limitation testée, protection physique/OS encore à établir |
| I2f02b-05, Store bas niveau encore disponible | Obliger le futur consommateur à vérifier le témoin coordonné | Codex ; appel hors protocole refusé par Coordinator, raccord public encore absent |

Documentation persistante, sources et schémas sont disponibles aux prochains
contextes ; aucun contexte agent embarqué ou processus déjà lancé n’est modifié.
Aucune modification des consignes communes n’est nécessaire : droits, préservation
et diagnostic sont déjà prescrits ; les apprentissages du contrat sont ici.

IA3c.2f-02 reste partiel : protection réelle du témoin et opérations publiques
non qualifiées. Service isolé -03, installation/Admin -04, admission -05, distant
-06 restent ouverts. Principal humain exact requis avant activation, pas inféré
du callback de fixture. F01–F07, IDA, EP-06, IA3 public, IA4/IA5 et RETEX global
restent ouverts/partiels selon leurs preuves. Aucun suivi récurrent supplémentaire.


La [note de recherche identité](../../../../publications/20261005-cursor-moteur/article-scientifique/identites-autorite-cycle-vie.md)
est actualisée pour 02a/02b ; état préalable dans research-note-before.md.txt.
Cette note ne transforme pas les fixtures en preuve d’originalité ou de service
réel. Elle est en français, sans version anglaise existante à synchroniser ; les
contrats opérationnels et schémas sont bien fournis dans les deux langues.
