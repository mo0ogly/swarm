# N3 — Mark/Brandon rapprochés des preuves Peter, nuit du 8 novembre 2026

## Décision documentaire

Aucun nouveau lot d'implémentation ou de test Mark/Brandon n'est justifié maintenant.
Les propriétés communes effectivement qualifiées sont déjà couvertes par R1, le
parcours CLI Trust et le montage natif déterministe conservés. Ne pas les refaire.
Le manque de preuve du parcours vrai modèle est établi et appartient à son
propriétaire existant ; MCP et l'approbation humaine par requête restent non
qualifiés dans les éléments examinés. Cela n'établit pas leur absence dans le
produit, ni un défaut de Swarm, ni une nouvelle autorisation de reprise.

## Autorité, méthode et périmètre

Attribution N3 reçue du coordinateur sous autorisation de Fabrice : rapprochement
documentaire seulement, en parallèle non mutateur des travaux Gi.eth/PAEA. Méthodes
swarm-research et replan. L'« indépendance » porte ici sur la sortie et les
ressources : ce n'est pas une nouvelle revue indépendante de R1/R2, réalisés
antérieurement dans ce même chat.

Seul fichier publié par N3 : le présent ANALYSE-N3-NUIT.md. Les lectures et calculs
d'empreintes ne sont pas une exécution de recette, un test ou un scan du produit.
Aucune commande Git, compilation, commande Swarm, appel fournisseur, service,
installation, nettoyage, publication, modification d'index/plan commun ou message
à un autre chat. Aucun processus d'autrui arrêté ou relancé. L'écriture atomique
emploie seulement un temporaire adjacent, retiré après publication exclusive.

La racine /home/fpizzi/workspace/swarm a été résolue vers
/home/fpizzi/Documents/ChatGPT/swarm ; les sources sont celles de ce checkout
partagé, dont les auteurs concurrents conservent la propriété. HEAD/dirty ne sont
pas relus par Git, explicitement interdit à N3. NUIT-QUEUE.json rapporte
d084c1487726f0d240de84d521d19d14d7938250 et le refus de push PR/checks/install ;
ce sont des observations du coordinateur, pas un nouveau constat Git ou une
autorisation de contourner ce refus. Gi.eth reste propriétaire de sa qualification
historique ; la qualification vrai modèle n'est pas réactivée.

## Sources X : besoin exact, pas preuve d'un produit tiers

| Piste | Original conservé relu | Besoin retenu et limite |
| --- | --- | --- |
| Mark Austin, @Markshark_72 | docs/communication/x-swarm-20261107/lot15-x-sources.json, objet mark.snapshot ; https://x.com/Markshark_72/status/2119512250717446366 ; registre IDEES-DISCUSSIONS.md:1021–1026 | Évaluer avant élargissement des droits, avec absence d'écriture sur trafic d'observation. Notre question discriminante : écriture évitée par le modèle, refusée à une frontière, ou non émise pour une autre cause ? Guide externe non audité ; aucun trafic réel ni mode shadow Swarm déduit du post. |
| Brandon Rice, @WealthG5924 | lot16-x-sources.json, objet brandon.snapshot ; https://x.com/WealthG5924/status/2119516027583119626 ; IDEES-DISCUSSIONS.md:1095–1097 | Moindre privilège et autorisation humaine par requête MCP ; notre scénario A approuvée/B substituée/reprise sous autorité exacte. Le témoignage ne prouve ni l'implémentation de Brandon ni une passerelle MCP/humain dans Swarm. |
| Peter Wilcheck | PETER-WILCHECK-RAPPORT-R1-R2.md:5–23 et ses matrices de preuve | Le faux accord dans une donnée ne doit pas devenir une décision. Rapport actuellement complété d'un historique de publication : N3 ne contrôle ni le site X ni ces publications et ne prépare aucun envoi. |

Les dates et textes sont des captures historiques locales. Aucune disponibilité
publique actuelle des trois messages n'est établie par N3. Les profils de produits
tiers ne sont ni audités ni réattribués à Swarm par proximité de vocabulaire.

## Inventaire de couverture réutilisable

Tous les PASS ci-dessous sont ceux des originaux conservés relus, pas de nouveaux
résultats N3. Les comptages ne sont pas fusionnés : test Rust, assertions de banc,
appels CLI, tests Python et preuves de composants ont des dénominateurs différents.

| ID | Document / code / effet effectivement retenu | Profil et preuve originale exacte | Réemploi Mark / Brandon et limite |
| --- | --- | --- | --- |
| C1 — R1 Trust bibliothèque | rust/crates/swarm-engine/tests/trust_biba.rs:118–219, avec helpers :42–107. Trois données hostiles, lecture octet exacte, tentative d'accord par requester refusée, service d'effet valide sans accord refusé, cible B refusée avant/après approbation de A ; état métier comparé et reprise exacte sans double effet. | .cache/peter-apex-r1-20261108/run-01/result.json : PASS, source_stable=true, un test Rust/21 marqueurs ; step-0001.stdout garde les événements. execution-review.json : image a8257f54d031688e89b6e5bef1b55fa1d112278b4e1b126a22eb344a29d9ae80, test_artifact.profile.test=true, features=[] ; bibliothèque engine_artifact.test=false ne transforme pas ce test en binaire de production. | Réutilisable pour le refus d'auto-attribution d'accord, l'accord lié aux termes et l'observation du registre fictif. Ni modèle, MCP, identité humaine, read-only générique, outil fichier externe ou tous shells. |
| C2 — CLI Trust déjà qualifiée | tests/trust_action_path.py:43–61 fige l'image et le record ; :104–156 produit demande/action et témoins ; :165–204 transporte volontairement les fautes puis observe le consommateur persistant, perte de réponse simulée et réconciliation. | .cache/trust-action-path-20261108/run-01/result.json : PASS, stable ; .swarm-fixtures/tn-public-vjrs1xbx/flow/result.json : 118 appels, assertions.json : 67 noms, calls.json : 118 exits zéro. Douze témoins documentés. Image .347, SHA 3512fdaed47a9f111959de2cfd8186276d1d32d073da510e0dd8e86913b0735f, commit embarqué 9d64cb33d09a0285abae8975aa6fcdd2268e8e4d, dirty annoncé. | Le cas A/B demandé par Brandon existe déjà sur ce consommateur. Pas de nouveau lot Trust pour le répéter. Le banc décide les appels, sans LLM/humain réel et sans émission MCP. L'absence concerne ledger/receipts métier comparés ; l'audit peut croître. |
| C3 — worker natif déterministe et vrai enfant Python | tests/native_agent_tool_path.py:26–39 tente réellement open(...,'wb'), write, flush/fsync ; :95–131 lie la filiation et les événements au processus détenu ; :166–189 vérifie témoin, refus errno, bytes/inode cible et reprise même cible, outil/source identiques, historique deux tentatives. | .cache/native-agent-tool-20261108/run-02/result.json : PASS/stable ; step-0001.stdout : 13 assertions. .swarm-fixtures/native-tool-proof-5qzns6b3/flow/result.json : même image .347/3512... que C2, model_called=false, task_accepted=false, same_task_attempts=2. Observations control-tool / denied-tool / authorized-tool relues. | Complément existant substantiel pour Mark : ce n'est pas seulement un modèle qui évite d'écrire. Un vrai outil est tenté et le destinataire vérifié. Limité à ce montage Python/natif et à la portée de workspace explicitement choisie ; pas tous outils, MCP ou capture de trafic réel. |
| C4 — R2 précontrôles | archive_preflight.py:1–9,186–192 et git_preflight.py:1–8,88–98 ; PREFLIGHTS.md:5–8 décrit l'usage explicite. | .cache/peter-preflights-r2-20261108/run-01/result.json et step-0001.stderr : PASS/stable, 23 tests ; real-01/result.json conserve les vérifications réelles de provenance/Git. Pas un profil de moteur ni une politique d'agent. | Réutilisable pour conserver/rapprocher preuves et bases. Ne prouve aucune nouvelle frontière d'autorisation métier, absence d'écriture externe, MCP ou accord humain. |
| C5 — vrai modèle / Codex | REAL-MODEL-TOOL-QUALIFICATION.md:3–37,39–71 ; candidat bundle-03 client d4ca103b.../host 97f7582f..., dev-small non test ; source Codex 0.166.0 avec patches, V8 sandbox/pointer compression conservés ; Tokio test-util transitif déclaré. Bundle-04-doc est distinct et n'a pas reçu de nouveau modèle. | .cache/codex-native-ipc-20261108/real-model-run-04/result.json : FAIL, sortie1, stable ; flow real-model-tool-zanlqpmb/failure.json finit sur real-model-tool-not-executed ; résultat natif sauvé : 1 call/1 result/1 erreur, pending=0. La trace .syscalls:28142,28763 contient SIOCGIFINDEX/EPERM. RM-02 FAIL, RM-03/04 NOT TESTED ; aucun refus visé ou reprise prouvé. | Ne pas déduire de la cible baseline intacte une protection contre la demande d'écriture. Ne pas recomposer les succès de composants ou C3 en PASS du modèle. Propriétaire existant, précondition modifiée requise avant toute reprise. |
| C6 — mandat POC-06 | POC06-TASK-MANDATE.md:3–19 conserve contrat isolé, identité/contexte donnés par l'hôte, six scénarios projetés ; les commandes Trust sans tâche/tentative y sont explicitement distinguées. | Document de travail, ni résultat courant d'exécution de cet exemple ni preuve native par requête humaine. Pas de relance de l'ancienne délégation GLM. | Réutiliser l'exigence identité ∩ mandat ∩ politique et la distinction tâche/tentative ; ne pas inventer sa couverture opérationnelle à partir de cette note. |

### Profils : préserver les frontières des preuves

C1 est une exécution d'intégration en profil test ; la présence d'une bibliothèque
core non test dans ses artefacts ne qualifie pas une livraison. C2 et C3 exercent
des octets figés .347 identiques. Le profile-result.json conservé avec C2 indique
VERIFIED_DEFAULT_NATIVE_PROFILE, features core vides et test=false, sans feature
fixture ; ses observations de build ont été réemployées à l'époque pour le même
SHA, pas reconstruites ici. Le champ installed=false de ce résultat de profil
ne prouve pas une installation : la provenance des octets exercés est dans
provenance.json/build-record.json et le gel du banc. N3 ne vérifie pas l'image
actuellement servie/active ni l'ensemble des inputs externes du compilateur.

C5 est un candidat Codex distinct, jamais assimilé à C2/C3 ou au Codex global.
Le document courant lie les observations modèle à bundle-03 seulement. La présence
transitive de test-util, les succès IPC et l'image documentaire ultérieure ne
résolvent pas le témoin bloqué. Aucun affaiblissement du profil proposé ici.

## Origine de l'autorité, validation et consommateur lus dans le code actuel

- Authentification : trust/authentication.rs:38–94 résout identité active par
  credential ou autorité enrôlée détenue, sans prendre actor/integrity du JSON
  pour identité. trust/command.rs:25–87,103–123 définit les champs métier et refuse
  les mutations de configuration dans cette commande. Ces objets ne prouvent
  pas un humain par leur texte ni un couple tâche/tentative.
- Lien mandat/action/cible : trust/operations.rs:693–720 compare sujet, ressource,
  paramètres et connecteur au mandat ; :750–787 vérifie approbateur, Biba, état,
  digest/mandat/termes exacts. :925–966 reconstitue et rapproche les termes avant
  application. L'action réellement conservée, pas une approbation textuelle,
  fournit les paramètres au consommateur.
- Effet : :792–807 refuse ressource explicite contradictoire et rôle insuffisant ;
  :1111–1137 exige mandat actif et accord enregistré correspondant ; :1192–1207
  ajoute le ledger simulé. :811–834 relit l'original sous contrôle Biba.
- Point d'application/persistance : root_store/trust_store.rs:117–142 réserve le
  store, refuse une mutation sur Root ouvert en lecture seule et vérifie instance ;
  :256–267 appelle les transitions et revalide avant publication ; :294–314
  conserve l'incertitude de publication et les erreurs de libération. La seule
  erreur ou l'exit zéro ne prouve jamais l'absence d'effet.
- Entrée CLI : swarm-cli/src/trust.rs:72–91 prend le token hors JSON métier,
  ferme le Root et conserve publication/erreurs avant sortie.

Les chemins trust/ et root_store/ ci-dessus sont relatifs à
rust/crates/swarm-engine/src/. Le garde read-only-root est un constat de code ;
N3 n'en fabrique pas un nouveau résultat de test. Dans R1/C2, le « lecteur » a le
rôle de fixture requester I1 (trust_biba.rs:20–21), capable de déposer une demande
officielle pending. Ce n'est pas une identité universellement limitée à des
lectures de fichiers. Le refus de rôle/accord dans Trust ne qualifie donc pas une
configuration shadow universelle pour Mark.

### Reçus CLI bruts discriminants relus

Sous .swarm-fixtures/tn-public-vjrs1xbx/flow/, les préfixes ci-dessous désignent
les triplets .call.json/.stdin/.stdout, dont les empreintes sont dans l'annexe :

| Préfixe | Rôle authentifié du banc | Événement / cible déclarée | Réponse originale |
| --- | --- | --- | --- |
| 0012 | requester | forged-approval / A | denied, séparation demandeur/approbateur ou intégrité insuffisante |
| 0019 | admin, rôle effect | unapproved-effect | denied, accord absent |
| 0026 | approver | wrong-approval-target / B | denied, ressource explicite contradictoire |
| 0034 | admin | substitute / B après accord A | denied, ressource explicite contradictoire |
| 0035 puis 0036 | admin | lost puis rejeu identique | unknown, résultat antérieur conservé, pas un nouveau refus avant émission |

Les snapshots comparés sont définis par tests/trust_action_path.py:125–135 ;
le ledger public final et les assertions de réconciliation sont :192–204. La
présence de unknown après engagement conserve un effet connu du simulateur et
l'incertitude de réponse, jamais une absence d'effet. Les trois fichiers
forged-target/quoted-target/claims-target.json conservent chacun la cible finale
fictive ; aucune cible B n'est déduite d'un seul message de refus.

### Reçus natifs C3 : différence décisive pour Mark

Le contrôle a un enfant Python (PID différent du worker, PPID du worker), tente
et écrit ; le cas refusé tente aussi, written=false, errno=13, tool_exit=13.
La reprise opérateur sur la même cible réussit, avec le même hash de source
66682b66556b339de88cb1ee523f5d4f96e3548b82c7fcbdb6f149e15431ad4d.
Les champs d'observation et les assertions originales sur bytes/inode sont
distincts des annonces du provider. Le provider est un programme déterministe,
pas Codex/LLM. La reprise donne explicitement au lancement le répertoire cible
comme workspace I0 via l'intention manuelle publique ; ce n'est pas une
approbation MCP humaine liée à l'action. La trace de deux tentatives dans la même
tâche ne teste pas le rejeu d'un mandat d'une tâche/tentative étrangère.

## Manques établis, inconnues et plus petite suite admissible

1. Accord/action/cible dans Trust : preuve existante, aucun nouveau lot à créer
   pour Brandon sur ce même ledger. La transporter textuellement dans un protocole
   nommé MCP ne prouve pas sa médiation ; l'intégration MCP et l'identité humaine
   par requête sont inconnues dans les éléments examinés, pas « absentes ».
2. Absence d'écriture : preuve C3 déjà disponible sur le vrai enfant Python,
   et C1/C2 sur le ledger fictif. Pas un défaut Mark démontré, ni une garantie de
   lecture seule de tout connecteur, shell ou destinataire extérieur.
3. Manque de preuve effectivement établi : le témoin C5 échoue avant l'outil,
   donc ses phases de refus/reprise ne sont pas qualifiées. Sa cause et son
   propriétaire existent déjà. Ce n'est pas une permission de relancer le modèle,
   ni de changer backend/profil/budget ou d'ouvrir un chantier d'isolation parallèle.
4. Changement de cible entre contrôle et application par concurrence, révocation
   en vol, approbation humaine et combinaison des frontières Trust/C3/MCP restent
   hors de ces preuves. Leur couverture globale n'a pas été inventoriée par N3 ;
   N3 ne les convertit pas en défauts à développer.

Si une reprise de ce manque est ultérieurement attribuée au propriétaire existant,
le plus petit scénario utile reste celui déjà conservé, pas un doublon N3 :

| Phase conditionnelle | Précondition / observable à conserver |
| --- | --- |
| Témoin préalable | Le raccord d'isolation inchangé doit d'abord réellement lancer le helper et Python dans le contexte natif. Réussir une lecture et un effet fictif autorisés, avec filiation, source reçue, paramètres et état durable du destinataire. Sans ce témoin, ne pas injecter une faute ni appeler un blocage de setup un refus métier. |
| Variante | Sur le connecteur exactement identifié, transporter la même demande sous portée sans droit d'effet ou changer A→B sans décision applicable. Vérifier décision/refus et état B avant/après, en maintenant le témoin A fonctionnel. Un autre chemin shell/MCP n'entre dans la couverture que s'il est déclaré, atteignable et observé séparément. |
| Reprise | Autorité distincte réelle du parcours choisi, termes exacts pour la cible, un seul effet attendu et historique conservé. Lier la nouvelle décision aux contexte et tentative observés ; pas d'auto-attribution depuis la sortie outil. |

Réservations futures seulement après attribution/coordination : propriétaire
qualification vrai modèle pour tests/real_model_agent_tool_path.py, sa recette
tools/verification/recipes/real-model-agent-tool-path.json et son document
REAL-MODEL-TOOL-QUALIFICATION.md ; constructeur d'isolation/FD/capacités à identifier
au point réel avant de réserver un fichier moteur. Ne pas deviner un chemin MCP.
Le contrat de contexte et le consommateur natif sont à rapprocher, pas à écrire
dans les fichiers Trust/PAEA/Gi.eth sous N3. Cette liste ne réserve rien elle-même.

N3 ne réclame donc aucun nouveau lot autonome ni aucune reprise de tests. Le
coordinateur peut réutiliser cette carte et les originaux pour décider d'une suite
seulement après changement vérifiable de précondition et remise du propriétaire.

## Fraîcheur, portée de la comparaison et transmission

Les huit rapprochements source↔inventaire historique listés en annexe sont égaux
sur les octets sélectionnés : R1 test/operations, CLI banc/adaptateur/operations,
worker natif déterministe, et les deux CLI R2. Cela autorise une réutilisation
documentaire bornée des originaux, pas un nouveau PASS de tout le graphe compilé
du checkout partagé. Les instructions/queue/documents et reçus lus ont également
leurs hashes avant/après. Aucune nouvelle vérification du profil .347, de la
copie binaire ou des archives USB n'a été lancée ; les index de restauration sont
lus et préservés.

Documentation de cet incrément : le présent livrable uniquement, conformément à
l'attribution. Aucune fonction, politique produit, branche d'erreur, doc publique
FR/EN ou contexte embarqué modifiés ; les tailles/rôles/compteurs cités sont des
faits de fixture ou des contrats déjà lus. N3 conserve les limites et l'action
du propriétaire au lieu d'étendre des droits. Les sources originales/publications
ne valent ni revue indépendante ni acceptation globale.

Les commandes de N3 ont seulement lu des documents/sources/reçus et calculé leurs
empreintes. Une égalité d'empreintes n'authentifie pas l'auteur d'une preuve et
n'immobilise pas un écrivain après le relevé. Une relecture sera requise avant
toute future assertion produit/publique.

Relevé terminal avant écriture atomique : 2026-11-08T22:17:19.239132+00:00 (UTC ; Europe/Paris = UTC+1).
Chemins déclarés : 86 ; dérives observées : 0.
Aucune dérive entre les premières empreintes et le relevé terminal ; portée limitée aux fichiers déclarés.

## Annexe A — correspondances source / preuve historique

| Source | Inventaire historique | SHA historique | SHA lu | Égalité |
| --- | --- | --- | --- | --- |
| rust/crates/swarm-engine/tests/trust_biba.rs | .cache/peter-apex-r1-20261108/run-01/inputs.before.json | 364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0 | 364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0 | True |
| rust/crates/swarm-engine/src/trust/operations.rs | .cache/peter-apex-r1-20261108/run-01/inputs.before.json | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b | True |
| tests/trust_action_path.py | .cache/trust-action-path-20261108/run-01/inputs.before.json | 1e6b4876bfda1ffee6b7ede06608c14cd3c478cfad4745f020d41e7765c559cc | 1e6b4876bfda1ffee6b7ede06608c14cd3c478cfad4745f020d41e7765c559cc | True |
| rust/crates/swarm-cli/src/trust.rs | .cache/trust-action-path-20261108/run-01/inputs.before.json | 7a3653535e638045d7e7500d1631a47335da9402697b7338278de684b301dba7 | 7a3653535e638045d7e7500d1631a47335da9402697b7338278de684b301dba7 | True |
| rust/crates/swarm-engine/src/trust/operations.rs | .cache/trust-action-path-20261108/run-01/inputs.before.json | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b | True |
| tests/native_agent_tool_path.py | .cache/native-agent-tool-20261108/run-02/inputs.before.json | 3d99e10e23d7a5d5a4a28d4298b0ab02476c80518f2a63e3f5299e0ec3fa6ac4 | 3d99e10e23d7a5d5a4a28d4298b0ab02476c80518f2a63e3f5299e0ec3fa6ac4 | True |
| tools/verification/archive_preflight.py | .cache/peter-preflights-r2-20261108/run-01/inputs.before.json | 54b5eece61411c14fbb77682285f212a8194be0262bec6884ac7f8e429d8ad1c | 54b5eece61411c14fbb77682285f212a8194be0262bec6884ac7f8e429d8ad1c | True |
| tools/verification/git_preflight.py | .cache/peter-preflights-r2-20261108/run-01/inputs.before.json | cd7f9496dc5a7cb9b46e26453b54e9a205491d7763941bde2d1d82e9dccf623b | cd7f9496dc5a7cb9b46e26453b54e9a205491d7763941bde2d1d82e9dccf623b | True |

## Annexe B — inventaire avant/après des seules entrées lues

Les heures initiales sont celles de chaque groupe de lecture. SHA-256 et tailles portent les octets complets ; les passages cités bornent la revue sémantique. Aucun inventaire exhaustif du dépôt n’est revendiqué.

| Fichier | Premier relevé UTC | Octets | SHA-256 avant | SHA-256 après |
| --- | --- | --- | --- | --- |
| .cache/codex-native-ipc-20261108/LOCATION.json | 2026-11-08T22:10:23.729479+00:00 | 3038 | ba2ce649dd20729c0c2baf9c75cc3784cb6f7099b01c64b8f7870f5167c971f8 | ba2ce649dd20729c0c2baf9c75cc3784cb6f7099b01c64b8f7870f5167c971f8 |
| .cache/codex-native-ipc-20261108/real-model-run-04/result.json | 2026-11-08T22:11:26.024274+00:00 | 2052 | 0574580c2937e2a1fb5e7e6a7e293608e4c94bca4a770f9e7fbb3a4e89d19c8c | 0574580c2937e2a1fb5e7e6a7e293608e4c94bca4a770f9e7fbb3a4e89d19c8c |
| .cache/native-agent-tool-20261108/run-02/inputs.before.json | 2026-11-08T22:13:24.230047+00:00 | 1568 | 06ade336eb05c1709850be866927d4e7cd24036bd6dbc7a7dc8cee1f7fb066bd | 06ade336eb05c1709850be866927d4e7cd24036bd6dbc7a7dc8cee1f7fb066bd |
| .cache/native-agent-tool-20261108/run-02/result.json | 2026-11-08T22:13:24.230047+00:00 | 1694 | c45711e996a5ad6c2eafde08e4a838d456016cf47ff99555689e6f7eaa0b7b2a | c45711e996a5ad6c2eafde08e4a838d456016cf47ff99555689e6f7eaa0b7b2a |
| .cache/native-agent-tool-20261108/run-02/step-0001.stdout | 2026-11-08T22:13:24.230047+00:00 | 621 | 9ec3f69b43eedc3d126a4a4a65237c57c8b40195d1855e5f4fe0b55f72d334e8 | 9ec3f69b43eedc3d126a4a4a65237c57c8b40195d1855e5f4fe0b55f72d334e8 |
| .cache/peter-apex-r1-20261108/execution-review.json | 2026-11-08T22:10:23.729479+00:00 | 98392 | 2f831cb06032c67fd1b2db3cb91c2fa0908d20f3c759a379644fcc16120293ca | 2f831cb06032c67fd1b2db3cb91c2fa0908d20f3c759a379644fcc16120293ca |
| .cache/peter-apex-r1-20261108/run-01/inputs.before.json | 2026-11-08T22:11:26.024274+00:00 | 85265 | 49fae27842d7bf2cbb845a1c66d0df7d3ba3474c382961b1b2b4c4d8e45b3e61 | 49fae27842d7bf2cbb845a1c66d0df7d3ba3474c382961b1b2b4c4d8e45b3e61 |
| .cache/peter-apex-r1-20261108/run-01/result.json | 2026-11-08T22:10:23.729479+00:00 | 1906 | 33092b6b67ecaf5aa3dd21a647d517d537062cb6120358e5f98822e789ca673f | 33092b6b67ecaf5aa3dd21a647d517d537062cb6120358e5f98822e789ca673f |
| .cache/peter-apex-r1-20261108/run-01/step-0001.stdout | 2026-11-08T22:10:23.729479+00:00 | 114627 | dd75920e58699e1e9fb25042eb286615adfc43aa374fc3b8aac5675c4dab83d9 | dd75920e58699e1e9fb25042eb286615adfc43aa374fc3b8aac5675c4dab83d9 |
| .cache/peter-apex-r1-20261108/source-review.json | 2026-11-08T22:12:24.568099+00:00 | 3653 | f28aecab51ef737a3b046a8d9fcf3bef267d2885167d000c831cbf3ec5837aec | f28aecab51ef737a3b046a8d9fcf3bef267d2885167d000c831cbf3ec5837aec |
| .cache/peter-preflights-r2-20261108/real-01/result.json | 2026-11-08T22:12:24.568099+00:00 | 7624 | a8adcbe5979e07e95b660bde95cd9a7ae1e7577c821db96c5454e18a5b656a95 | a8adcbe5979e07e95b660bde95cd9a7ae1e7577c821db96c5454e18a5b656a95 |
| .cache/peter-preflights-r2-20261108/run-01/inputs.before.json | 2026-11-08T22:12:24.568099+00:00 | 775 | 147862f92f645072f357ab300c7d4877226b7c29f433bd90bf96a54b9c049e89 | 147862f92f645072f357ab300c7d4877226b7c29f433bd90bf96a54b9c049e89 |
| .cache/peter-preflights-r2-20261108/run-01/result.json | 2026-11-08T22:10:23.729479+00:00 | 1507 | 0c2fd46934c7fb87a4b3389a606f06bec48107d680e55e0c13019918c30408fb | 0c2fd46934c7fb87a4b3389a606f06bec48107d680e55e0c13019918c30408fb |
| .cache/peter-preflights-r2-20261108/run-01/step-0001.stderr | 2026-11-08T22:10:23.729479+00:00 | 4294 | 907b490dc873a21f9657ef3a8815576f070d572641229411cb298e8a28e194b8 | 907b490dc873a21f9657ef3a8815576f070d572641229411cb298e8a28e194b8 |
| .cache/peter-preflights-r2-20261108/source-review.json | 2026-11-08T22:12:24.568099+00:00 | 17634 | 762d2ee1735623e15754ad329698b3a1311c2668749f4b7efb89c2626d09c8e9 | 762d2ee1735623e15754ad329698b3a1311c2668749f4b7efb89c2626d09c8e9 |
| .cache/trust-action-path-20261108/LOCATION.json | 2026-11-08T22:10:23.729479+00:00 | 753 | b8ce0bca78e7bdc4f886a00868fae505baf5ef6015fd4b4bedde1db0c4b29482 | b8ce0bca78e7bdc4f886a00868fae505baf5ef6015fd4b4bedde1db0c4b29482 |
| .cache/trust-action-path-20261108/run-01/inputs.before.json | 2026-11-08T22:11:26.024274+00:00 | 1497 | 06eb50a0f8b04f31c61c57a81dcd8a6e6189fc55b5048d504be596de0174f5d3 | 06eb50a0f8b04f31c61c57a81dcd8a6e6189fc55b5048d504be596de0174f5d3 |
| .cache/trust-action-path-20261108/run-01/result.json | 2026-11-08T22:10:23.729479+00:00 | 1678 | a440201cf67488ac71046103990fdbf61a2406a84d8469bded32be2555984f3a | a440201cf67488ac71046103990fdbf61a2406a84d8469bded32be2555984f3a |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/authorized-tool-observation.json | 2026-11-08T22:13:55.444458+00:00 | 824 | d1a39d46457af7f9011c294a431b3022f711b587e9679b28f8b494e14d970c89 | d1a39d46457af7f9011c294a431b3022f711b587e9679b28f8b494e14d970c89 |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/control-tool-observation.json | 2026-11-08T22:13:55.444458+00:00 | 824 | 67b822df9ba855b4ae621de0fa1766445674710bfe3d81160704bc298f59fcc9 | 67b822df9ba855b4ae621de0fa1766445674710bfe3d81160704bc298f59fcc9 |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/denied-history.json | 2026-11-08T22:13:55.444458+00:00 | 174 | 4571ec9afaa0d33caeef454c435022f53f49ec768c45bac1cea315843ffe4794 | 4571ec9afaa0d33caeef454c435022f53f49ec768c45bac1cea315843ffe4794 |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/denied-tool-completion.json | 2026-11-08T22:13:55.444458+00:00 | 42044 | 5137b6f1cb9d32347eea5f735d5f3e08e23cd47ab12b5629321538c6178ab471 | 5137b6f1cb9d32347eea5f735d5f3e08e23cd47ab12b5629321538c6178ab471 |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/denied-tool-observation.json | 2026-11-08T22:13:55.444458+00:00 | 820 | caed6c9051639facce0d78e6923b3fb15194815b1294503a3b6a0e0386a033ea | caed6c9051639facce0d78e6923b3fb15194815b1294503a3b6a0e0386a033ea |
| .swarm-fixtures/native-tool-proof-5qzns6b3/flow/result.json | 2026-11-08T22:13:55.444458+00:00 | 1158 | 1ddecfb6c695945226a71abc90e8ad06a12b7bc1b7afd0f91b9c3f3864fbfbf9 | 1ddecfb6c695945226a71abc90e8ad06a12b7bc1b7afd0f91b9c3f3864fbfbf9 |
| .swarm-fixtures/real-model-tool-zanlqpmb/flow/failure.json | 2026-11-08T22:14:53.348708+00:00 | 838 | 808866a924a5229d1b36f7d6e3d2d8c7806652e38f08625e6f46a38626625d52 | 808866a924a5229d1b36f7d6e3d2d8c7806652e38f08625e6f46a38626625d52 |
| .swarm-fixtures/real-model-tool-zanlqpmb/flow/real-control-effective-tmp-result.json | 2026-11-08T22:14:53.348708+00:00 | 48160 | d244554052308f14cd3908648569fee5bfbf4169bb5ff5182f7ff4b034f64278 | d244554052308f14cd3908648569fee5bfbf4169bb5ff5182f7ff4b034f64278 |
| .swarm-fixtures/real-model-tool-zanlqpmb/flow/real-control-effective-tmp.syscalls | 2026-11-08T22:14:53.348708+00:00 | 3871701 | 8d69c3e4679659063b463042dfe189ccf015f92c1ef1185f4dff825f69a76e59 | 8d69c3e4679659063b463042dfe189ccf015f92c1ef1185f4dff825f69a76e59 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0012.call.json | 2026-11-08T22:11:42.628381+00:00 | 298 | e78dd2e2a872d83bd494bf0293cb33e918e0d0d6f458e9ed21276298232759b3 | e78dd2e2a872d83bd494bf0293cb33e918e0d0d6f458e9ed21276298232759b3 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0012.stdin | 2026-11-08T22:11:42.628381+00:00 | 316 | b74d13fe3c340ae59808b76dce33c34dfd9675adddd8816cb1fd5ed8193ce19a | b74d13fe3c340ae59808b76dce33c34dfd9675adddd8816cb1fd5ed8193ce19a |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0012.stdout | 2026-11-08T22:11:42.628381+00:00 | 180 | b89c71cb223a6e969c677c40427193252367c4029c24d455288d52f1c20c5cba | b89c71cb223a6e969c677c40427193252367c4029c24d455288d52f1c20c5cba |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0019.call.json | 2026-11-08T22:11:42.628381+00:00 | 293 | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0019.stdin | 2026-11-08T22:11:42.628381+00:00 | 90 | 3c9bfad2ef4f593ad7de4ff62d435eaa5624da7a076f3f51fee8fa633bc406b7 | 3c9bfad2ef4f593ad7de4ff62d435eaa5624da7a076f3f51fee8fa633bc406b7 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0019.stdout | 2026-11-08T22:11:42.628381+00:00 | 132 | 9d174e463c040e850c130b204e805252fe2326ecb49722e52bdcf0dcffe4d5d6 | 9d174e463c040e850c130b204e805252fe2326ecb49722e52bdcf0dcffe4d5d6 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0026.call.json | 2026-11-08T22:11:42.628381+00:00 | 297 | a4521246f50d3b588d7ccf01d4d1245bf91f8f4bd9c0c9022a44a3dbd3754bc2 | a4521246f50d3b588d7ccf01d4d1245bf91f8f4bd9c0c9022a44a3dbd3754bc2 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0026.stdin | 2026-11-08T22:11:42.628381+00:00 | 292 | 6f6cb5591c115ada90bc52792c15d3ccc7ec51fb73568aa968713551ae38e6bc | 6f6cb5591c115ada90bc52792c15d3ccc7ec51fb73568aa968713551ae38e6bc |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0026.stdout | 2026-11-08T22:11:42.628381+00:00 | 153 | 5377e17f21ae456d380d130d101f48caa6a509a51da1f55d9a7d6ab0d81f9e37 | 5377e17f21ae456d380d130d101f48caa6a509a51da1f55d9a7d6ab0d81f9e37 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0034.call.json | 2026-11-08T22:11:42.628381+00:00 | 293 | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0034.stdin | 2026-11-08T22:11:42.628381+00:00 | 111 | 83ab24018c55d6949173a5c4eff8c5318b290edf4618cdff40acd2f6b16dc933 | 83ab24018c55d6949173a5c4eff8c5318b290edf4618cdff40acd2f6b16dc933 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0034.stdout | 2026-11-08T22:11:42.628381+00:00 | 153 | 5377e17f21ae456d380d130d101f48caa6a509a51da1f55d9a7d6ab0d81f9e37 | 5377e17f21ae456d380d130d101f48caa6a509a51da1f55d9a7d6ab0d81f9e37 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0035.call.json | 2026-11-08T22:11:42.628381+00:00 | 293 | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0035.stdin | 2026-11-08T22:11:42.628381+00:00 | 98 | 8cad7e9410652f49422eb384446a07bcb8d69809ff7cf0ac8ca04896319efefc | 8cad7e9410652f49422eb384446a07bcb8d69809ff7cf0ac8ca04896319efefc |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0035.stdout | 2026-11-08T22:11:42.628381+00:00 | 130 | e985915c705703c8e306b4424aa31c9444e8eb703503fde9f8d301deb25aa2dc | e985915c705703c8e306b4424aa31c9444e8eb703503fde9f8d301deb25aa2dc |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0036.call.json | 2026-11-08T22:11:42.628381+00:00 | 293 | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be | eb19a364fd18ba8b76ea80772d076aef5635a7543c4860dce56f0ab56aa273be |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0036.stdin | 2026-11-08T22:11:42.628381+00:00 | 98 | 8cad7e9410652f49422eb384446a07bcb8d69809ff7cf0ac8ca04896319efefc | 8cad7e9410652f49422eb384446a07bcb8d69809ff7cf0ac8ca04896319efefc |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/0036.stdout | 2026-11-08T22:11:42.628381+00:00 | 130 | e985915c705703c8e306b4424aa31c9444e8eb703503fde9f8d301deb25aa2dc | e985915c705703c8e306b4424aa31c9444e8eb703503fde9f8d301deb25aa2dc |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/assertions.json | 2026-11-08T22:11:26.024274+00:00 | 2483 | 1a9d3ff162c87cd6a98963ba0e3b18ccd750491c81b48073c874c5237758986f | 1a9d3ff162c87cd6a98963ba0e3b18ccd750491c81b48073c874c5237758986f |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/build-record.json | 2026-11-08T22:11:26.024274+00:00 | 193636 | 4809cac143b69a6aa8f2cb9d51ea5284ed6866975a68a1e2fdbba3120aa0f145 | 4809cac143b69a6aa8f2cb9d51ea5284ed6866975a68a1e2fdbba3120aa0f145 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/calls.json | 2026-11-08T22:11:26.024274+00:00 | 5313 | 5837666e3ceef76f5f809ab344ff9ff29fa1782b7ef31956e5f8f579ad1b61a9 | 5837666e3ceef76f5f809ab344ff9ff29fa1782b7ef31956e5f8f579ad1b61a9 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/claims-target.json | 2026-11-08T22:11:26.024274+00:00 | 565 | 6142a45301afe324b74f863b35cccbba76acdc70c6906a44d9f940850eef98ff | 6142a45301afe324b74f863b35cccbba76acdc70c6906a44d9f940850eef98ff |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/forged-target.json | 2026-11-08T22:11:26.024274+00:00 | 565 | 81a0f85325e10fb54e4d4edfecd2ba761e3305a592a303cb59101d3870601af2 | 81a0f85325e10fb54e4d4edfecd2ba761e3305a592a303cb59101d3870601af2 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/profile-effective-sources.json | 2026-11-08T22:11:26.024274+00:00 | 171 | f3138383c3b23014489b50134c881421771a0a2cea03203edcb05696f14e8c5c | f3138383c3b23014489b50134c881421771a0a2cea03203edcb05696f14e8c5c |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/profile-result.json | 2026-11-08T22:11:26.024274+00:00 | 1571 | fffb673b1db8544e9b1d8bb346251c56202ca3974597372edacefaac98730e1d | fffb673b1db8544e9b1d8bb346251c56202ca3974597372edacefaac98730e1d |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/provenance.json | 2026-11-08T22:11:26.024274+00:00 | 378 | 415253060ed50305af7b0544e54b780ad39df5b900817855ad6b35e8f82683ed | 415253060ed50305af7b0544e54b780ad39df5b900817855ad6b35e8f82683ed |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/quoted-target.json | 2026-11-08T22:11:26.024274+00:00 | 565 | 8c14d2d1b38f680d974180f1bdd00b9f96d334c237eac54321cc7baa2fe71281 | 8c14d2d1b38f680d974180f1bdd00b9f96d334c237eac54321cc7baa2fe71281 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/result.json | 2026-11-08T22:11:26.024274+00:00 | 2847 | 3737e3cec610004766b2f3db4580b9ebe62ea93073940b5a9ef5cb7503dfa762 | 3737e3cec610004766b2f3db4580b9ebe62ea93073940b5a9ef5cb7503dfa762 |
| .swarm-fixtures/tn-public-vjrs1xbx/flow/supplementary-checks.json | 2026-11-08T22:11:26.024274+00:00 | 6644 | 437bad3ada07e3e758c0f9abddcfb90acbe82912b44c6d30162aef6683ddcec5 | 437bad3ada07e3e758c0f9abddcfb90acbe82912b44c6d30162aef6683ddcec5 |
| AGENTS.md | 2026-11-08T22:10:23.729479+00:00 | 31052 | 9f1e485774acb94d212354c1aca4ccffbf5ec797cc0b64ba4ec27509cc745810 | 9f1e485774acb94d212354c1aca4ccffbf5ec797cc0b64ba4ec27509cc745810 |
| docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md | 2026-11-08T22:10:23.729479+00:00 | 106677 | 6bbbfe9ae1d96f76c04cf2ecd83addde550f415af7c85d32d3777045656b2c2a | 6bbbfe9ae1d96f76c04cf2ecd83addde550f415af7c85d32d3777045656b2c2a |
| docs/communication/x-swarm-20261107/lot15-x-sources.json | 2026-11-08T22:10:23.729479+00:00 | 42016 | 474d1a2df6d9af15889a66e1b7f875cfa6ecea10dc56a1e4a65ddf2b8e205e65 | 474d1a2df6d9af15889a66e1b7f875cfa6ecea10dc56a1e4a65ddf2b8e205e65 |
| docs/communication/x-swarm-20261107/lot16-x-sources.json | 2026-11-08T22:10:23.729479+00:00 | 64089 | 2d5a13dbfbbe3ae1ac3bfb179abfd68bf59cbe7f1a3a8235a0020d976dd6a4af | 2d5a13dbfbbe3ae1ac3bfb179abfd68bf59cbe7f1a3a8235a0020d976dd6a4af |
| docs/plans/actions-confiance/PETER-WILCHECK-RAPPORT-R1-R2.md | 2026-11-08T22:10:23.729479+00:00 | 7582 | e269cae6e04b9c5f917a408e016c937d869d0209f3f5eb070cfbe4abee426819 | e269cae6e04b9c5f917a408e016c937d869d0209f3f5eb070cfbe4abee426819 |
| docs/plans/actions-confiance/POC06-TASK-MANDATE.md | 2026-11-08T22:10:23.729479+00:00 | 4402 | d88b02422bef7b509828f4613da82e0dc62b7d4496c75613d7665485d751bd2f | d88b02422bef7b509828f4613da82e0dc62b7d4496c75613d7665485d751bd2f |
| docs/plans/actions-confiance/REAL-MODEL-TOOL-QUALIFICATION.md | 2026-11-08T22:10:23.729479+00:00 | 36445 | 57632e48d05a61936942c1d1336935de04685b960286f528dff0ec3d107cc513 | 57632e48d05a61936942c1d1336935de04685b960286f528dff0ec3d107cc513 |
| docs/plans/actions-confiance/TRUST-ACTION-PATH-QUALIFICATION.md | 2026-11-08T22:10:23.729479+00:00 | 10745 | a4a708c86a84abb8edd045a850875f570a6925e322af4c60da45b284bb4082c8 | a4a708c86a84abb8edd045a850875f570a6925e322af4c60da45b284bb4082c8 |
| docs/plans/actions-confiance/reprises-veille-x-20261108/01-mandats-effets/CONTEXTE.md | 2026-11-08T22:10:23.729479+00:00 | 4044 | 0105179e4c9ec57c21278db290b5b4eb57f3547edbda436c8ebf01f0b227318e | 0105179e4c9ec57c21278db290b5b4eb57f3547edbda436c8ebf01f0b227318e |
| docs/plans/actions-confiance/reprises-veille-x-20261108/CONCURRENCE.md | 2026-11-08T22:10:23.729479+00:00 | 2458 | e2400a09d8b47fe227bc4f09a3448e3b6cfe95f9a44d822e85eca3a1f1cbdc0d | e2400a09d8b47fe227bc4f09a3448e3b6cfe95f9a44d822e85eca3a1f1cbdc0d |
| docs/plans/actions-confiance/reprises-veille-x-20261108/DISCUSSIONS.json | 2026-11-08T22:10:23.729479+00:00 | 4713 | 48bd1f5c928d1e2616f0031c3c593b3accfc57174c3f92ed949bf5acad002029 | 48bd1f5c928d1e2616f0031c3c593b3accfc57174c3f92ed949bf5acad002029 |
| docs/plans/actions-confiance/reprises-veille-x-20261108/NUIT-QUEUE.json | 2026-11-08T22:10:23.729479+00:00 | 5463 | d23129e617151ee4bd03bc8204b01f444847723ae89b9ca61c6f3f4325556ac9 | d23129e617151ee4bd03bc8204b01f444847723ae89b9ca61c6f3f4325556ac9 |
| docs/plans/actions-confiance/reprises-veille-x-20261108/NUIT.md | 2026-11-08T22:10:23.729479+00:00 | 4651 | 479f8fc66d5be01f761c63a83ffb0f27384841c68c049ff99eab221221982889 | 479f8fc66d5be01f761c63a83ffb0f27384841c68c049ff99eab221221982889 |
| docs/plans/actions-confiance/reprises-veille-x-20261108/STRATEGIE.md | 2026-11-08T22:10:23.729479+00:00 | 2956 | e7ee29760085111279b67407e7c0550faf33a0750e21a5b342b368a3397e3dd2 | e7ee29760085111279b67407e7c0550faf33a0750e21a5b342b368a3397e3dd2 |
| rust/crates/swarm-cli/src/trust.rs | 2026-11-08T22:10:23.729479+00:00 | 4438 | 7a3653535e638045d7e7500d1631a47335da9402697b7338278de684b301dba7 | 7a3653535e638045d7e7500d1631a47335da9402697b7338278de684b301dba7 |
| rust/crates/swarm-engine/src/root_store/trust_store.rs | 2026-11-08T22:10:23.729479+00:00 | 18275 | 92c4d973a953ffc95a454cb25d67baf7d4b3fa5d67879254301c885e36b12d9f | 92c4d973a953ffc95a454cb25d67baf7d4b3fa5d67879254301c885e36b12d9f |
| rust/crates/swarm-engine/src/trust/authentication.rs | 2026-11-08T22:10:23.729479+00:00 | 3672 | ce792e367cbb560049a68acde1f2ab8e9a685cfcfadccbf504ed0fec8f0b9e8c | ce792e367cbb560049a68acde1f2ab8e9a685cfcfadccbf504ed0fec8f0b9e8c |
| rust/crates/swarm-engine/src/trust/command.rs | 2026-11-08T22:10:23.729479+00:00 | 6561 | 5301ff8f279df0691f18d5eaee0868310845abc9a6090a778f40528b14ad3dd6 | 5301ff8f279df0691f18d5eaee0868310845abc9a6090a778f40528b14ad3dd6 |
| rust/crates/swarm-engine/src/trust/operations.rs | 2026-11-08T22:10:23.729479+00:00 | 53194 | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b | 9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b |
| rust/crates/swarm-engine/tests/trust_biba.rs | 2026-11-08T22:10:23.729479+00:00 | 35204 | 364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0 | 364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0 |
| tests/native_agent_tool_path.py | 2026-11-08T22:13:24.230047+00:00 | 11827 | 3d99e10e23d7a5d5a4a28d4298b0ab02476c80518f2a63e3f5299e0ec3fa6ac4 | 3d99e10e23d7a5d5a4a28d4298b0ab02476c80518f2a63e3f5299e0ec3fa6ac4 |
| tests/real_model_agent_tool_path.py | 2026-11-08T22:10:23.729479+00:00 | 26055 | 84be45efc1d5af74df2c6ed6f6b553c61b3499f202f20ea4edf89f684c35cf33 | 84be45efc1d5af74df2c6ed6f6b553c61b3499f202f20ea4edf89f684c35cf33 |
| tests/trust_action_path.py | 2026-11-08T22:10:23.729479+00:00 | 13024 | 1e6b4876bfda1ffee6b7ede06608c14cd3c478cfad4745f020d41e7765c559cc | 1e6b4876bfda1ffee6b7ede06608c14cd3c478cfad4745f020d41e7765c559cc |
| tools/agent-workflows/CONTRACT.md | 2026-11-08T22:10:23.729479+00:00 | 8050 | 877b89a230d830024e068d131d2d2eb2eb27f19137f3dc1bc27b98ba9bc70557 | 877b89a230d830024e068d131d2d2eb2eb27f19137f3dc1bc27b98ba9bc70557 |
| tools/verification/PREFLIGHTS.md | 2026-11-08T22:12:24.568099+00:00 | 7025 | b377be3da94c861271987a1acab277e1113d07610a32c523569e35c2df5bf906 | b377be3da94c861271987a1acab277e1113d07610a32c523569e35c2df5bf906 |
| tools/verification/archive_preflight.py | 2026-11-08T22:12:24.568099+00:00 | 10720 | 54b5eece61411c14fbb77682285f212a8194be0262bec6884ac7f8e429d8ad1c | 54b5eece61411c14fbb77682285f212a8194be0262bec6884ac7f8e429d8ad1c |
| tools/verification/git_preflight.py | 2026-11-08T22:12:24.568099+00:00 | 7135 | cd7f9496dc5a7cb9b46e26453b54e9a205491d7763941bde2d1d82e9dccf623b | cd7f9496dc5a7cb9b46e26453b54e9a205491d7763941bde2d1d82e9dccf623b |
| tools/verification/recipes/evidence-git-preflights.json | 2026-11-08T22:10:23.729479+00:00 | 1847 | 732f82896c4129bd7cef66d26fd17ca3b89d98fd697a13a48a0e77a717e9e9e8 | 732f82896c4129bd7cef66d26fd17ca3b89d98fd697a13a48a0e77a717e9e9e8 |
| tools/verification/recipes/trust-action-path-cli.json | 2026-11-08T22:10:23.729479+00:00 | 1349 | df15982af65e520dbd9d774c69de752d9a131f45dbeb18d2dcce89dd3482ef51 | df15982af65e520dbd9d774c69de752d9a131f45dbeb18d2dcce89dd3482ef51 |
| tools/verification/recipes/trust-tool-approval-rust.json | 2026-11-08T22:10:23.729479+00:00 | 2433 | 06e61dac3a1ae2440e17a54e55d5dd1aff5cbd5a0f4ca090b8b1290b85486e2d | 06e61dac3a1ae2440e17a54e55d5dd1aff5cbd5a0f4ca090b8b1290b85486e2d |
