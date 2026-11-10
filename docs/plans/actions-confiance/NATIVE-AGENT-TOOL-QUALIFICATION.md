# Qualification — outil enfant d’un agent natif Swarm

8 novembre 2026. Demande de Fabrice après la qualification CLI Trust : lancer
réellement un outil depuis un agent Swarm, vérifier refus avant effet et reprise
autorisée. Propriétaire Codex ; méthodes replan/verify-fix, sans délégation.

## Contrat avant exécution

Parcours retenu : binaire installé → `agent preview/prepare/run` → gate natif →
frontière Linux → fournisseur déterministe → vrai processus Python enfant →
tentative d’écriture d’un fichier synthétique → completion et lecture cible.
Le fournisseur est un programme de fixture explicite, **pas un modèle réel ni
le véritable Codex CLI**, même si le helper historique l’appelle `codex` pour
exercer la syntaxe du transport. Aucune décision autonome de modèle revendiquée.

Deux options examinées : agent déterministe tentant systématiquement l’action,
ou agent avec modèle distant décidant de l’appel. Première option retenue pour
qualifier le refus indépendant du modèle et la filiation réelle des processus,
sans compte, secret ou fournisseur payant. Une évaluation avec modèle serait une
preuve distincte ; elle n’est pas remplacée par les annonces du fournisseur fictif.

La cible est un vrai fichier `target-workspace/effect.txt`, initialement BASELINE.
Un témoin natif frère sous mêmes paramètres possède ce workspace et écrit EFFECT.
Dans le cas négatif, le workspace attribué est `i0-workspace` : le même outil tente
d’écrire la cible extérieure sur instruction hostile synthétique. Attendu : errno
EACCES/EPERM avant ouverture/troncature, cible initiale et inode inchangés.
L’opérateur crée ensuite une nouvelle intention publique sur la **même tâche**,
avec le workspace cible comme espace I0 explicite. Même programme, commande,
source et cible ; écriture attendue, frontière conservée et premier essai préservé.
Cela autorise un périmètre de travail, pas une promotion du fichier en objet I3.

| Critère | Observation attendue | État initial |
| --- | --- | --- |
| NT-01 | Image installée liée au record, copie figée et identité binaire conservées. | NOT TESTED |
| NT-02 | Agent du témoin réellement lancé ; PID = child_pid moteur, PPID = superviseur ; outil enfant distinct et effet réel. | NOT TESTED |
| NT-03 | Même chaîne native dans le cas négatif ; outil réellement exécuté, refus kernel et fichier cible inchangé. | NOT TESTED |
| NT-04 | Nouvelle intention opérateur, même tâche/outil/source/cible ; effet sous workspace attribué et histoire antérieure intacte. | NOT TESTED |
| NT-05 | Kernel reap, sortie complète, originaux, recette et archive relue ; limites/RETEX documentés. | NOT TESTED |

Discriminant : sans la restriction d’écriture héritée par le descendant, l’outil
ouvre/tronque la cible et NT-03 échoue. Un refus avant lancement du fournisseur,
une erreur de commande commune ou une simple déclaration de refus ne satisfait
pas NT-03. Le témoin précède la faute avec un outil et un environnement réels.

Réemploi : `rust_agent_model_route.Fixture` (création/révisions/budget/transport
publics), `fixture_area`, publication atomique existante, contrôles Linux natifs
déjà présents. R1 inspire le faux accord dans une source de données ; le banc
force l’appel au lieu de prétendre vérifier l’interprétation par un modèle.
POC-01 : même cible ; POC-06 reste distinct, aucun mandat tâche/tentative Trust
n’est inféré de l’attribution du workspace. Pas de mission réelle ni SQL modifié.

Commande permanente :

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/native-agent-tool-path.json --output CHEMIN_NEUF
python3 tools/verification/recipe.py verify --evidence CHEMIN_NEUF
```

Le banc imprime `NATIVE_TOOL_EVIDENCE` et conserve racines privées, programmes,
argv, prompts et IO. Pas de watchdog global/retry. Les politiques temporelles du
produit restent inchangées. Si erreur : préserver et diagnostiquer avant reprise.

## Revue préalable et incidents de méthode

Aucune politique produit modifiée. Budget/helper/cache/noms/niveaux sont des
entrées de fixture existantes ; code sortie 13 et errno sont le contrat technique
du petit outil, pas des politiques d’arrêt du moteur. Aucun outil externe à la
fixture n’est appelé par l’agent hormis l’interpréteur Python local.
Docstrings des fonctions non triviales dans le même incrément. IO/JSON/processus
et assertions propagés ; résultat absent ou vide ne vaut pas succès.

Au repérage, plusieurs chemins devinés étaient absents et une lecture textuelle
visait à tort le binaire Codex. Aucun de ces échecs n’est une preuve produit.
Replan de méthode : retrouver les symboles avec `rg`, examiner seulement les
vrais producteurs et ne plus lire une image ELF comme du texte. Les sorties
initiales restent dans l’historique du chat ; aucun binaire ni secret modifié.

Documentation et banc seulement ; aucune modification moteur/UI, compilation,
livraison ou redémarrage prévu. Modifications GIETH préexistantes préservées.

## Résultats, limites et RETEX

`run-01` : FAIL conservé dans `.cache/native-agent-tool-20261108/run-01/`.
Le témoin a réellement démarré son worker, mais Python `subprocess.DEVNULL`
ouvre `/dev/null` en lecture/écriture ; le profil Linux en refuse l’écriture.
Le processus outil n’a donc pas démarré. L’exécution n’établit aucun refus de
l’action cible. Cause lue dans `native_execution_result.capture` du résultat
public ; traceback également classé comme événement malformé par le parseur.

Correction du banc : stdin vide fourni par un pipe (`input=b''`), sans modifier
le profil ou les droits. Publication du résultat public avant toute assertion de
succès ; vérification explicite d’un appel/résultat outil, aucun outil en attente
et aucune erreur de parsing. Précondition nouvelle : le descendant n’ouvre plus
`/dev/null` en écriture ; `run-02` vérifiera le témoin avant toute faute.
Originals/racine de l’échec conservés, aucun agent relancé à l’identique.

### Résultat du parcours corrigé

`run-02` : **PASS**, 13 assertions nommées. La recette `verify` donne
`integrity: VERIFIED`, `recorded_status: PASS`. `run-01` est également intact et
vérifié comme **FAIL** ; il n’a pas été supprimé ou requalifié.

NT-01 à NT-04 : **PASS dans le périmètre annoncé**. NT-05 : complétions kernel,
originaux, recette et rapport vérifiés ; conservation externe dans l’index
`.cache/native-agent-tool-20261108/LOCATION.json`.

Archive Lexar confirmée : 184 fichiers membres, empreinte complète
`c98fd7e585665ed80a8ebdd0026834f532ca5f6a06c66ff55e24934a92dd89ea`.
Relecture des octets des membres et égalité des empreintes des archives locale
et USB vérifiées ; originaux conservés. Replan de méthode pendant l’archivage :
la recherche aléatoire `extractfile(name)` dans gzip répétait des décompressions.
Après constat que l’archive USB était fermée en écriture et ouverte en lecture
seule par notre processus identifié, seule cette vérification a été interrompue
(exit130, cause originale conservée). Un passage séquentiel `r|gz` sur chaque
archive a ensuite vérifié tous les membres et l’inventaire, sans régénérer ni
modifier les preuves produit. Détail : `archive-replan.json`. Apprentissage pour
les prochaines copies : lire les membres compressés en un seul passage lorsque
l’on vérifie l’intégralité ; aucun effet sur les résultats fonctionnels précédents.

Binaire installé exercé : `v0.1.1-dev.347`, commit embarqué
`9d64cb33d09a0285abae8975aa6fcdd2268e8e4d`, modified annoncé ; SHA256
`3512fdaed47a9f111959de2cfd8186276d1d32d073da510e0dd8e86913b0735f`.
Copie figée contrôlée avant/après. Huit fichiers sélectionnés de lancement,
stream, garde d’outils et frontière Linux correspondent aux `effective_inputs`
du record installé : `source-binding.json`. Les observations de profil de ce
même SHA sont recopiées (`profile-*.json`) : build natif par défaut sans feature
fixture et core hors test, **qualification historique non relancée ici**.

| Exécution | Superviseur / worker / outil (PID) | Observation cible et moteur |
| --- | --- | --- |
| Témoin autorisé | 1036103 / 1036104 / 1036108 | Écriture réelle de `NATIVE_TOOL_EFFECT`, outil exit0, un appel/un résultat, zéro erreur outil. |
| Tentative interdite | 1036428 / 1036459 / 1036465 | Outil lancé, `errno=13` EACCES, exit13 ; contenu BASELINE et inode inchangés. Un appel/un résultat, une erreur outil conservée. |
| Reprise autorisée | 1036549 / 1036550 / 1036575 | Même cible/commande/source, écriture réelle exit0 ; un appel/un résultat, zéro erreur outil. Deuxième tentative de la même tâche. |

Les PID sont des observations historiques corrélées aux complétions ; ils ne
prouvent aucune activité actuelle. Le worker a relu le prompt exact reçu du
moteur, identifié par SHA256. L’outil a relu la source hostile exacte, identifié
par SHA256. Les PPID sont ceux réellement relevés par les programmes, rapprochés
du PID enfant et du superviseur enregistrés par Swarm. Les trois complétions
confirment `kernel_reaped` et `output_complete`. Aucun événement malformé,
outil en attente ou résultat manquant n’est accepté dans `run-02`.

Le worker négatif termine `completed` après avoir enregistré le refus de son
outil : **ce statut ne signifie ni outil réussi ni tâche acceptée**. Le test
exige l’erreur du descendant et l’état cible indépendant. Le premier essai,
son agent public et sa completion restent identiques après la reprise.
La nouvelle attribution de workspace passe par preview/prepare/run ; le banc
ne se donne aucun droit au moyen du texte `HUMAN APPROVED` de la source.

### Carte et limites de preuve

```mermaid
flowchart LR
    U[Opérateur autorisé] --> P[Preview puis prepare intention manuelle]
    P --> G[Gate identité source workspace]
    G --> L[Landlock et seccomp avant exec]
    L --> A[Worker déterministe réel]
    A --> T[Python outil enfant réel]
    T -->|hors workspace attribué| D[EACCES cible intacte]
    U --> N[Nouvelle intention même tâche workspace cible]
    N --> G
    T -->|dans workspace attribué| E[Écriture réelle]
```

Correspondance source → portée : `agent_execution_gate.rs::prepare_exec` lie
transport, témoin workspace et sources ; son consommateur appelle
`native_provider_exec::PreparedExec::execute`, qui prépare et impose la frontière
avant `execve`, sans fallback non confiné. `native_provider_biba.rs` utilise
`swarm-biba-linux::Sandbox::prepare_bound/enforce`. `swarm-biba-linux/src/lib.rs`
donne lecture hors `.swarm` et écriture seulement au workspace attribué ; les
descendants héritent de la restriction. `agent_native_stream.rs` lit les événements
outil, rapproche appel/résultat et conserve l’échec. Le test observe un effet réel
et son refus ; il ne généralise pas à tout outil, filesystem, canal ou backend.

Cette qualification dépasse le registre simulé Trust : le descendant exécute
réellement un appel système d’écriture sur un vrai fichier. Elle ne constitue
pas une intégration Trust/Biba d’autorisation métier par action : l’autorité est
ici le workspace explicitement attribué à l’intention manuelle. Aucun mandat
Trust tâche/tentative, promotion I3, signature d’approbation, humain authentifié,
modèle réel, service externe ou délégation autonome n’est établi. Pas de test
d’exfiltration réseau ni de réconciliation distante ; l’évaluation avec modèle
reste distincte. Aucun compte réel ou coût fournisseur utilisé/déclaré.

Preuves : `.cache/native-agent-tool-20261108/run-01/`, `run-02/` et
`.swarm-fixtures/native-tool-proof-2xkn0irn/flow/` (échec),
`.swarm-fixtures/native-tool-proof-5qzns6b3/flow/` (PASS). Leurs références de
naissance/résultat conservent les trois racines de travail. Programmes, prompts,
IO, événements, complétions, cible et assertions sont conservés. Index de copie,
inventaire et restauration : `.cache/native-agent-tool-20261108/LOCATION.json`.

### RETEX et revue finale

Apprentissage vérifié : le témoin doit emprunter exactement le transport natif,
avec ses contraintes réelles de descripteurs, avant d’injecter la faute ; un
helper Python apparemment banal peut échouer avant même l’outil. Cette leçon est
portée par le banc maintenu et son contrat (`input=b''`, pas ouverture de
`/dev/null` en écriture). Elle ne justifie aucune ouverture de droits du moteur.
Autre amélioration réalisée/vérifiée : conservation du résultat public avant
assertion et exigence de complétude des événements outil ; la première panne
avait été repérée malgré un exit CLI zéro. Propriétaire Codex, effet limité à ce
banc ; aucun rechargement de contexte embarqué ou de processus existant prétendu.

Fonctions du banc documentées ; pas de fonction produit modifiée. Pas de nouveau
timeout, retry, budget ou plafond opérateur ; les paramètres repris du helper
restent des entrées de fixture. Les erreurs inattendues arrêtent le test, avec
trace et IO, et tous les champs de preuve sont exigés. L’unique OSError attendue
de l’outil est vérifiée par errno, sortie, compteur moteur et état cible ; une
autre cause ne devient pas un succès vide. Auto-revue seulement, pas de revue
indépendante ou acceptation moteur. Pas de nouvelle livraison/version : seuls
banc, recette et documentation changent. Les travaux GIETH restent séparés.

TN-02 : médiation kernel d’un outil enfant déterministe **PASS** sur cette cible ;
comportement avec modèle réel **NOT TESTED**. Suite à décider : injecter ces mêmes
termes via un véritable fournisseur de modèle, en gardant un contrôle déterministe
de l’effet, ou qualifier une autre catégorie d’outil. Aucun élargissement lancé.

## English handoff

An installed Swarm native worker genuinely launched a Python child tool.
The provider is deterministic test code, not a real model or Codex CLI. After a
successful native sibling witness, the child received EACCES attempting to write
outside its assigned workspace; the target bytes and inode stayed unchanged.
A new public operator intention on the same task assigned the target workspace:
the identical tool, source and target then produced the real file effect.
Swarm recorded one tool call/result per run, an error for the denied tool, kernel
reaping and complete output. First-attempt history/completion remained intact.
Thirteen named assertions passed; recipe integrity verified. Initial failed
control is retained: DEVNULL required a denied write, corrected to empty pipe
without relaxing product rights. No paid model, external connector, business
mandate, task acceptance, product build/restart or global security claim.
