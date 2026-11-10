# Dialogue : état durable avant les autorisations de tour

16 octobre 2026, après « suite ». Codex seul, APEX et `replan`, dans le checkout
existant. L’[incrément 7](INCREMENT-MEMOIRE-07.md) conserve l’état nécessaire aux
gardes entre les frontières des tours ; il n’autorise pas de redémarrage du
fournisseur. Aucun modèle réel, délégation ou mission utilisateur n’est lancé.
Le serveur ouvert précédemment et sa base restent inchangés.
Les quatre tâches M11 sont réalisées et vérifiées dans ce périmètre local.

## Écart observé et réalisation

Les deux cas exécutés avant modification du moteur montrent l’écart : après
deux tours répondus, ou après un tour interrompu avec outil en attente, la CLI
ne dispose d’aucun état durable de frontière. L’usage connu du moniteur n’est
pas encore dans `agent.usage` avant son prochain battement. Les deux FAIL sont
dans `evidence/m11-20261016/checkpoint-red.json/.log`, avec la source originale.
Ils établissent ce manque de conservation ; ils ne remettent pas en cause la
qualification technique de fin apportée par M10.

Les gardes complètes ne pouvaient pas être reconstruites à partir de
`AgentProgress`, qui est une projection. Les journaux ne sont pas un remplacement :
`logBatch` ne conserve que 2 000 entrées ; la capture fournisseur est optionnelle
et tronquée. Le nouveau champ `agent_dialogue_turns.checkpoint` contient un état
versionné distinct de ces journaux. Il conserve les limites, signatures, séries
et totaux de répétition/erreur, déduplication, résultats, outils en attente,
diagnostics connus, usage et déclaration du modèle lorsqu’ils ont été reçus.

Le moniteur l’écrit avant d’autoriser le début, puis avant de confirmer la fin,
après la persistance des journaux. Une fermeture observée du canal conserve une
frontière interrompue si le stockage fonctionne. Une écriture refusée au début
empêche le fournisseur de partir ; en fin, elle empêche `answered` et le tour
suivant. Les erreurs ne sont pas converties en état vide réussi.

L’état est lié au projet canonique, à l’agent et à son lancement enregistré :
travail, tâche, tentative, rôle, espace, commande/arguments, noms d’environnement
autorisés, prompt, limites et durée. L’empreinte est un contrôle de cohérence,
pas une signature d’autorité ni une preuve de l’environnement actuel. Les
valeurs secrètes d’environnement ne sont pas enregistrées.

## Lecture et qualification de la frontière

La CLI `agent show` et l’API authentifiée `agent-detail` ajoutent `dialogue` pour
les agents de ce mode. Cette projection donne l’état, sa date/empreinte, sa
portée, les compteurs et l’usage de la frontière ; les structures internes des
gardes ne sont pas exposées. L’API refuse un autre travail avant de lire l’état.
Un ancien tour sans checkpoint est explicitement inconnu. Un état non vide
altéré, hors tour/projet/lancement, de version inconnue ou incohérent est refusé.

Une fin enregistrée et son statut sont deux écritures successives. La vue exige
**phase `confirmed` et statut `answered`** pour `boundary_confirmed: true`.
Le test de panne du statut conserve bien l’état de fin et l’usage reçu, mais
refuse cette qualification et ne lance pas le fournisseur suivant. L’état de
début reste explicitement incomplet quant aux effets ultérieurs. Dans tous les
cas, `restart_supported` reste `false`.

La garde reconstituée depuis le JSON réellement conservé est identique à celle
du moniteur : deux appels en échec, deux séries d’erreur/répétition et l’historique
des signatures restent présents. Un troisième appel identique déclenche la même
garde de répétition. Cette vérification technique n’exécute aucun outil réel ni
conversation restaurée. Les collections sont copiées : modifier la garde
reconstituée ne modifie ni l’état conservé ni le moniteur initial.

## Migration, histoire et parcours public

Le stockage passe de **33 à 34** ; le format de checkpoint commence à **1**.
La migration sauvegarde l’original en mode `0600`, puis ajoute la colonne sans
réécrire les agents, questions, statuts ou sessions. Le test ouvre la sauvegarde
et vérifie sa version 33 et l’absence de la nouvelle colonne. La consultation
d’un ancien stockage refuse toujours la migration implicite. Aucun checkpoint
historique n’est inventé.

La suppression récupérable et la restauration locale conservent l’empreinte et
l’usage du checkpoint, via le mécanisme qui capture toutes les colonnes des
tables. La purge des journaux ne supprime pas les tours. L’export historique
standard conserve son périmètre partiel : il ne devient pas un état exécutable
de conversation. Les sources et réponses des évaluations antérieures en version
33 restent historiques et inchangées. Les tests de connaissance et de grille V3
lisent désormais la source courante 34 et vérifient les branches 33/34/35 ; aucun
nouvel essai LLM ne leur est attribué.

Le vrai binaire est construit et exécuté par
`TestDialogueActualSupervisorConfirmsTurnReceipts`, avec superviseur, bail de
terminal, deux saisies et relecture par sa CLI. Les fournisseurs restent des
scripts locaux. Les originaux du candidat final sont dans
`evidence/m11-20261016/actual-supervisor-final/` :

| Parcours | Tours répondu/interrompu | État durable reçu par la CLI |
| --- | --- | --- |
| Deux tours puis `/fin` | 2 / 0 | Frontière confirmée, deux appels et résultats, usage 20/4. |
| Refus fournisseur avec sortie nulle | 0 / 1 | Frontière interrompue, usage non rapporté absent. |
| Outil sans résultat | 0 / 1 | Frontière interrompue, un appel, zéro résultat reçu, un outil en attente, usage 10/2. |

Les nombres de tokens sont des valeurs de fixture. Aucun coût réel n’est déduit
de ces nombres. Aucune tâche n’est acceptée par ces parcours.

## Vérification et replan du candidat

Le premier candidat passe 25 tests nommés et 44 sous-tests, puis 15 tests nommés
avec `-race`. La relecture trouve ensuite un défaut de la nouvelle vue :
`loopGuard.summary()` utilise l’heure actuelle, ce qui fait croître après arrêt
la durée d’un outil conservé comme en attente. Un cas daté de deux jours avec
une frontière située deux secondes après son début échoue avant correction :
`boundary-time-red.json/.log` et sources liées dans `boundary-time-source.json`.
Ces premiers lots verts ne sont pas présentés comme la preuve du candidat final.

Le calcul commun `summaryAt(at)` permet maintenant une projection à la date
enregistrée. La supervision courante garde `summary()` à l’heure actuelle ; la
lecture historique utilise la date du checkpoint. Cette correction a justifié
de vérifier aussi les événements, défauts de capture et arrêts du superviseur
qui utilisent les mêmes gardes. Les originaux des lots précédents sont conservés.

| Contrôle final | Résultat | Preuve sous `evidence/m11-20261016/` |
| --- | --- | --- |
| Tours, état durable, refus, CLI/API, migration, connaissance actuelle, cycle de vie et garde commune | PASS, 32 tests nommés et 47 sous-tests | `checkpoint-final.json/.log` |
| Concurrence, état incomplet, panne du statut et heure de projection | PASS, 16 tests nommés avec `-race` | `checkpoint-final-race.json/.log` |
| Analyse statique | PASS | `engine-final-vet.json/.log` |
| Compilation et parcours du vrai binaire | PASS dans le test du superviseur | Même lot final, `actual-supervisor-final/` |
| Reprise d’une conversation réelle et arrêt brutal simultané des deux processus | NOT TESTED | Aucun droit de restauration n’est activé. |

Les préimages conservent les modifications antérieures, et les empreintes des
entrées Go/ressources embarquées lient chaque contrôle à son candidat. La revue
est locale, dans le contexte d’implémentation ; elle ne vaut pas approbation
indépendante. Aucun écran n’a changé, aucune recette navigateur n’est revendiquée.
Les contrats sont décrits en [FR](../../PROJECT-MEMORY.md) et
[EN](../../en/PROJECT-MEMORY.md).

Les 16 tests `-race` sont compris dans les **32 noms distincts**, sans addition
artificielle. Les 1 007 entrées du manifeste du candidat final restent identiques
pendant et après ses contrôles. Les premières sources vertes, puis celles du cas
rouge de temps, sont conservées par empreinte ; les résultats antérieurs ne sont
pas attribués à la correction suivante. `final-audit.json` lie les sources,
journaux, reçus publics et contrôles documentaires finaux.

## RETEX et suite

La perte de l’usage entre deux battements se traite au contrat de la frontière,
pas en accélérant le battement ni en reconstruisant depuis un journal tronqué.
Les gardes métier et leur projection d’affichage sont distinctes. Une vue d’un
état historique doit utiliser sa propre date ; cette règle est appliquée dans
le calcul commun et protégée par le cas de régression. Le lecteur vérifie les
liaisons et distingue absence historique, corruption et fin non confirmée.

Incidents d’exploration : deux lectures ont encore visé des fichiers supposés
absents (`recovery_evidence.go`, `agents.go`). Les producteurs réels ont ensuite
été localisés dans `agents_store.go` et par recherche des symboles. Un patch
contenant deux opérations sur le même fichier a été refusé avant mutation ; il
a été regroupé avant application. Une erreur de ponctuation dans un premier
texte documentaire a été corrigée après relecture. Ces incidents ne constituent
ni échecs de produit ni tests exécutés ; la règle existante de découverte des
chemins reste suffisante, sans nouvelle consigne doublon.

L’incrément conserve un état de frontière ; il ne constitue pas un checkpointer
complet de conversation. Une mort brutale peut laisser l’état de début, sans les
effets suivants. La disponibilité de la session chez le fournisseur et les
permissions actuelles ne sont pas établies par l’empreinte. La restauration
autorisée doit encore définir sa décision, sa fraîcheur, les consommations et la
vérification des effets incertains. Le refus de redémarrage reste nécessaire.
Les critères MEM-C15 et MEM-C16 restent PARTIAL ; résumé progressif, suppression
et qualité LLM gardent leurs critères ouverts.
