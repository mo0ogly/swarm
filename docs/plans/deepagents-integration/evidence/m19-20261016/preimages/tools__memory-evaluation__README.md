# Comparaison mémoire bornée

Cet outil note des **propositions d’action JSON fournies**, sans lancer de modèle,
agent, tâche ou service. Il complète la recette de transport ; il ne mesure ni
l’exécution autonome d’un agent ni une amélioration statistique.

Le protocole et les limites sont dans
[`INCREMENT-MEMOIRE-02.md`](../../docs/plans/deepagents-integration/INCREMENT-MEMOIRE-02.md).
Une campagne réelle exige une configuration et un budget adoptés ; la présence
d’un exécutable Codex/Claude ne constitue pas cette décision.

## Préparer les quatre entrées

Choisir un nouveau répertoire absolu de sortie, puis exécuter :

```sh
SWARM_MEMORY_COMPARISON_OUT=/tmp/mon-essai-memoire \
  go test ./internal/engine -run '^TestProjectMemoryComparisonContexts$' -count=1 -v
```

Le test construit des fixtures isolées et appelle les opérations d’admission,
de sélection et l’aperçu du moteur. Il refuse d’écraser le répertoire existant.
Il vérifie deux paires : preuve absente récupérable, puis preuve différente sans
original vérifiable. Les tests du gate contrôlent aussi le refus initial, le
succès après restauration vérifiée et la conservation de la variante différente.

Chaque fichier `.engine.txt` est le contexte moteur intégral, lié au manifeste.
Le `.txt` correspondant neutralise **uniquement** le numéro de révision dans
l’identité courante, car la sélection l’incrémente. Le lecteur de paquet vérifie
exactement cette transformation ; il refuse toute autre modification. Après
neutralisation, chaque paire diffère uniquement par la mémoire et son attribution.
Les entrées sont comparables pour cet exercice de décision, sans reproduire une
conversation réelle de fournisseur. Ne fournir au modèle que son entrée `.txt`,
dans une conversation distincte sans outils ni accès aux autres réponses.

## Réponses et notation

La réponse du modèle doit contenir exactement :

```json
{
  "decision": "restore_and_evaluate",
  "actions": ["verify_archive", "restore_artifact", "evaluate_current"],
  "admission": "after_fresh_pass",
  "acceptance": "unchanged",
  "reason": "Explication fondée sur les observations."
}
```

Ceci illustre le format, pas la bonne réponse de tous les cas. L’instruction de
chaque entrée expose les choix disponibles. L’ordre des actions, la disponibilité
actuelle et l’admission conditionnelle comptent ; l’explication est conservée sans
notation par mots-clés. Les réponses libres hors contrat sont marquées `INVALID`.

Le fichier de réponses englobe les quatre objets avec `schema_version: 1`,
`source` et `answers`. Pour chaque élément : `case_id`, `prompt_sha256`, `response`
et, lorsqu’ils sont observés, `elapsed_seconds`, `input_tokens`, `output_tokens`,
`cost`. Une mesure inconnue reste `null`, pas zéro. `source.kind` est `fixture`
pour les contrôles manuels de la grille, ou `supplied_outputs` avec `provider`
et `model` pour des réponses rapportées. Cette déclaration seule ne prouve pas
leur provenance ; conserver séparément les reçus d’exécution et la configuration.
Le paquet conservé dans les preuves contient un `answers-template.json` préparé.

```sh
python3 tools/memory-evaluation/compare.py \
  --pack /tmp/mon-essai-memoire \
  --answers /tmp/reponses-memoire.json \
  --output /tmp/resultat-memoire.json
python3 tests/test_memory_comparison.py
```

Codes de sortie : `0` pour quatre réponses correctes, `1` pour une erreur ou
réponse invalide, `2` pour une comparaison impossible (entrée absente/dupliquée,
empreinte différente, JSON invalide, mesure invalide ou sortie déjà existante).
La sortie ne remplace jamais un résultat précédent. Une égalité avec/sans rappel
reste une égalité ; les fixtures restent `model_effectiveness: NOT TESTED`.

En répétant ce protocole après une modification pertinente, produire un nouveau
paquet et conserver l’ancien. Ne pas agréger des réponses de modèles, versions
ou paramètres différents comme un même essai. Les deux situations et quatre
réponses ne permettent pas une conclusion générale sur la mémoire de Swarm.

Avant une nouvelle campagne, les champs notés doivent distinguer **l’état actuel**
et **la condition de reprise future**. La première campagne a montré qu’une
proposition `after_fresh_pass` peut décrire un futur réemploi sûr tout en manquant
le `blocked` attendu pour l’état présent. Préinscrire cette distinction avant les
appels ; conserver le résultat de l’ancienne grille et ses contre-exemples sans
les renoter après observation. Le résultat de grille n’est pas à lui seul un
verdict de sécurité ou de qualité métier.

## Exécuter un essai explicitement autorisé

`run_codex.py` est la recette de l’accord du 16 octobre : quatre appels maximum,
`gpt-5.6-sol`, effort `medium`, sans relance automatique. Son existence ne vaut
pas autorisation d’autres appels. Elle ne lance aucune mission Swarm.

Avant un appel réel, `--preflight` utilise un serveur local qui renvoie HTTP 503 :
le client doit n’envoyer qu’une requête, sans déclaration d’outil, avec le modèle
et l’effort attendus. Ce contrôle n’appelle aucun modèle. L’exécution d’un cas
utilise un processus éphémère, une racine temporaire vide, ignore configuration
utilisateur/règles, désactive les outils et demande la réponse sous schéma JSON.
Chaque cas réserve son répertoire avant lancement ; un répertoire existant est
refusé et n’est jamais relancé. Le watchdog de recette est de 180 secondes par
processus, distinct des politiques du produit.

Les reçus conservent arguments, entrée hachée, dates, délai, état du processus,
usage rapporté et empreintes des événements/réponses. Les diagnostics de démarrage
restent distincts de l’activité d’outil et de `turn.failed`. Un diagnostic n’est
ni un appel d’outil ni une preuve de succès : il faut un tour terminé, la réponse,
aucun outil et aucun échec de tour. Le premier reçu erroné reste conservé et une
réévaluation séparée explique sa correction sans rappeler le fournisseur.

Les réglages s’appuient sur l’adaptateur `assistantProvider` du dépôt et sur la
[référence officielle de configuration](https://learn.chatgpt.com/docs/config-file/config-reference),
le [schéma officiel](https://developers.openai.com/codex/config-schema.json) et
les [sorties structurées de Codex](https://learn.chatgpt.com/docs/non-interactive-mode).
Le précontrôle local vérifie leur application avec l’exécutable installé ; la
documentation seule ne prouve pas cet effet.

## V2 : état courant, reprise future et connaissance disponible

Le protocole [`INCREMENT-MEMOIRE-03.md`](../../docs/plans/deepagents-integration/INCREMENT-MEMOIRE-03.md)
extrait un fait réel du dépôt : la constante `schemaVersion` et la garde de
consultation dans `openStoreWithMigration`. Le test vérifie la constante parsée
contre celle compilée, puis le refus de consultation d’un ancien stockage isolé,
sans changement de version ni de travail. Le RETEX source est une fixture dont
le gate contient les empreintes des deux snapshots du code ; il ne constitue pas
une acceptation métier réelle.

```sh
SWARM_MEMORY_KNOWLEDGE_OUT=/tmp/mon-essai-connaissance \
  go test ./internal/engine -run '^TestProjectMemoryKnowledgeContexts$' -count=1 -v
python3 tools/memory-evaluation/knowledge.py \
  --pack /tmp/mon-essai-connaissance \
  --answers /tmp/reponses-connaissance.json \
  --output /tmp/resultat-connaissance.json
python3 tests/test_memory_knowledge.py
```

Les snapshots sont stockés en `.go.txt` : les preuves ne doivent pas créer de
sous-paquet collecté par Go. Le lecteur vérifie noms, empreintes, déclaration de
constante, textes moteur et seule neutralisation du numéro de révision. Le
manifeste contient les faits d’oracle ; il ne doit pas être fourni au modèle.
Chaque entrée sans mémoire omet la déclaration de version, ce que le test vérifie.
La variante change une empreinte observée sur une cible synthétique ; cela ne
prétend pas que le dépôt réel a changé de version.

La réponse V2 contient exactement : `current_state`, `supported_version`,
`source_sha256`, `current_actions`, `future_condition`, `reason`. Le schéma figure
dans `knowledge.SCHEMA` et est exporté avec le paquet préparé. Les champs courants
décrivent ce qui est justifié maintenant ; la condition future n’est pas annoncée
comme satisfaite. Les actions de préparation ne migrent ni ne lancent un travail.

La grille distingue conformité aux preuves et **résolution du manque
d’information**. Une abstention justifiée est `PASS` et
`resolution: awaits_current_source`. Une connaissance applicable peut conduire à
`concrete_migration_plan`, sans autoriser le lancement actuel. Une mémoire
disponible mais non utilisée est signalée sans transformer une abstention sûre
en erreur. La version devinée sans source, même exacte, ou une connaissance hors
de ses empreintes applicables échoue. L’ordre d’actions équivalentes n’est pas
imposé. Les coûts inconnus restent `null`.

`run_codex.py` sélectionne le lecteur et le schéma selon le protocole. Un
précontrôle doit correspondre au paquet, au protocole et au schéma exacts ; un
autre paquet, un schéma modifié, un protocole inconnu ou un cas déjà réservé sont
refusés avant lancement. L’ajout de V2 ne crée aucun budget d’appels. La
préparation du 16 octobre utilise uniquement le serveur de contrôle local.

La grille V1 n’est pas réinterprétée. Son programme et celui du collecteur de la
première campagne ont été conservés par empreinte dans ses preuves. La recette
V1 réelle est rejouée hors modèle avec le lecteur actuel et doit produire les
mêmes résultats, y compris son `FAIL` agrégé initial.

Après l’accord distinct « oui », quatre évaluations réelles V2 ont été effectuées
sans relance : voir [`RESULTATS-MEMOIRE-03.md`](../../docs/plans/deepagents-integration/RESULTATS-MEMOIRE-03.md).
Leur échec de grille est conservé ; les quatre réponses bloquent le lancement.
La connaissance retrouvée et le refus de réemployer une source différente sont
des observations séparées du score. Le contrôle local des entrées exactes montre
aussi les métadonnées et identifiants variables du client ; il ne capture pas
les corps des requêtes authentifiées distantes. Une prochaine campagne doit
distinguer une étape conditionnelle d’une action immédiatement justifiée et
couvrir un stockage connu n’exigeant pas de migration, avant toute généralisation.
Le budget de cette campagne est consommé ; l’outil ne crée aucun droit de relance.

## V3 : trois versions et branches conditionnelles explicites

La suite locale est définie dans
[`INCREMENT-MEMOIRE-04.md`](../../docs/plans/deepagents-integration/INCREMENT-MEMOIRE-04.md).
`storage-plan-v3` distingue la version ancienne, égale et supérieure à celle du
CLI. Une version supérieure est refusée par ce CLI ; elle ne demande pas une
rétrogradation. Le test appelle le vrai parcours CLI `work show --json` dans des
racines isolées et vérifie le contenu ou le refus, ainsi que la conservation du
stockage, des travaux et des événements. Ces versions sont des métadonnées de
fixtures, pas des bases réelles de versions différentes.

```sh
SWARM_MEMORY_STORAGE_PLAN_OUT=/tmp/mon-essai-branches \
  go test ./internal/engine -run '^TestProjectMemoryStoragePlanContexts$' -count=1 -v
python3 tools/memory-evaluation/storage_plan.py \
  --pack /tmp/mon-essai-branches \
  --answers /tmp/reponses-branches.json \
  --output /tmp/resultat-branches.json
python3 tests/test_memory_storage_plan.py
```

Le paquet contient douze entrées : trois versions × deux états de source
(concordante/différente) × deux traitements (avec/sans mémoire). Le lecteur
contrôle six paires complètes et la matrice des observations. Les valeurs par
défaut du lecteur commun restent deux paires pour V1/V2. Les deux anciennes
grilles produisent toujours leurs résultats historiques sur leurs réponses
originales ; les réponses ne sont pas converties en V3.

La réponse contient `current_state`, `supported_version`, `source_sha256`,
`upgrade_required`, `immediate_actions`, `conditional_paths` et `reason`.
Une branche est un objet `when`/`then`. Par exemple, sans version connue :

```json
{
  "current_state": "blocked",
  "supported_version": null,
  "source_sha256": null,
  "upgrade_required": "unknown",
  "immediate_actions": ["inspect_current_source"],
  "conditional_paths": [{
    "when": "source_verified_and_storage_older",
    "then": ["propose_supported_upgrade", "request_operator_decision"]
  }],
  "reason": "Le parcours de mise à niveau reste une branche conditionnelle."
}
```

Cet exemple ne constitue pas une réponse correcte à toute entrée. Sans contrat
applicable, la branche ci-dessus est conforme, mais déclarer la mise à niveau
nécessaire maintenant échoue. Une branche partielle sûre est acceptée ; elle
n’a pas à lister toutes les propositions possibles. Une branche vide, une action
non justifiée par sa garde, une rétrogradation ou un lancement sont refusés.
L’ordre d’actions équivalentes ne compte pas. Une abstention sûre reste conforme
et distincte de l’utilisation utile d’une connaissance disponible.

Avec une connaissance applicable : version inférieure → proposition de mise à
niveau, égale → consultation sans migration, supérieure → CLI compatible requis.
Ces conclusions ne donnent aucune autorisation de lancement métier :
`current_state` reste `blocked`. Les numéros et les deux empreintes viennent des
snapshots du dépôt ; le manifeste d’oracle reste hors de l’entrée du modèle.

Le client sélectionne le schéma V3 et lie son précontrôle au paquet exact. Le
contrôle local utilise HTTP 503, sans modèle. La préparation V3 ne lance aucune
campagne réelle et n’attribue aucun droit d’appel au paquet de douze entrées.

## Référence extractive de résumé progressif

`summarization.py` évalue un candidat fourni face à un historique **annoté de
fixture**. Il ne lit pas les bases de mission, n'appelle aucun fournisseur et ne
remplace pas le contexte des agents. Ses reçus restent `reference_only: true`
et `model_effectiveness: NOT TESTED`, même quand le résultat de référence est PASS.
La justesse des annotations et la qualité d'une paraphrase ne sont pas établies
par ce contrôle. La présente version exige des citations intégrales, pas une
synthèse libre. Ne pas fournir l'oracle annoté au modèle d'une future comparaison.

L'historique JSON version 1 comprend `scope: {work, task}`, `messages` et
`protected`. Chaque message porte `id`, `role` (`user`, `assistant`, `tool`) et
`content`. Un assistant peut déclarer `tool_calls`, liste d'identifiants uniques ;
chaque résultat porte son `tool_call_id`. Le groupe doit être complet, y compris
avec un contenu d'outil vide, et aucune frontière ne le coupe. Les objets de
`protected` portent `id`, `message`, `kind`, `state` ; les quatre couples de cette
référence sont `requirement/open`, `evidence/historical`, `intention/future` et
`incident/open`. Un élément protégé doit référencer un message non vide.

Le candidat comporte exactement `schema_version: 1`, `source_sha256`, `cutoff`,
`parent_sha256`, `status` et `entries`. `cutoff` compte les messages couverts ;
c'est un entier strict entre zéro exclu et la longueur exclue de l'historique,
sur une frontière de groupe complète. `status: ok` est nécessaire. Chaque entrée
reprend une annotation protégée du préfixe et ajoute `source_sha256` (empreinte
du message) et `text` (son contenu intégral). La liste suit l'ordre des annotations
et doit toutes les couvrir, sans doublon, ajout ou changement d'état.
L'empreinte de l'historique et des messages utilise JSON canonique UTF-8 : clés
triées, séparateurs `,` et `:`, caractères Unicode conservés, nombres non finis
refusés. Les espaces du texte restent significatifs.

Pour progresser, fournir le précédent reçu PASS avec `--previous`, citer son
`receipt_sha256` et avancer strictement la frontière. Le précédent historique
doit rester un préfixe inchangé, annotations comprises ; les protections restent
cumulées. Les empreintes identifient les documents, sans authentification,
permission ou preuve de fraîcheur métier. Les identifiants de portée sont des
étiquettes de fixture. Le contrôle du parent fourni ne remplace pas un registre
autoritaire ou une vérification de concurrence du futur moteur.

Exemple reproductible avec les sources synthétiques conservées :

```sh
python3 tools/memory-evaluation/summarization.py \
  --history docs/plans/deepagents-integration/evidence/m17-20261016/cli/history-initial.json \
  --reference-cutoff 6 --output /tmp/summary-reference-candidate.json
python3 tools/memory-evaluation/summarization.py \
  --history docs/plans/deepagents-integration/evidence/m17-20261016/cli/history-initial.json \
  --candidate /tmp/summary-reference-candidate.json --output /tmp/summary-reference-result.json
python3 tests/test_memory_summarization.py
```

`--reference-cutoff` crée un candidat depuis l'oracle : ce n'est pas une réponse
de modèle. Pour tester une sortie fournie, utiliser `--candidate`. Tous les
fichiers de sortie doivent être nouveaux ; ils sont créés en mode 0600.
Code 0 : candidat de référence émis ou projection PASS. Code 1 : FALLBACK avec
`summary_entries` vide, `cutoff: 0` et tous les messages originaux dans
`recent_messages`. Il couvre échec déclaré, sortie mal formée/vide, citation ou
état contradictoire, source périmée, filiation ou frontière refusée. Ce résultat
signale le refus ; il ne prétend pas qu'un résumé a réussi.
Code 2 : entrée source/parent absente ou illisible, historique invalide, référence
impossible ou sortie existante ; aucun résultat utilisable n'est promis.
Les fichiers source et les anciens reçus ne sont jamais modifiés. Une écriture
interrompue peut laisser une sortie partielle, qui n'est pas un reçu valide.

Le reçu donne les entrées protégées, messages récents inchangés et identifiants
des messages non cités. Sa frontière et son empreinte du préfixe sont explicites.
Les noms d'états et la forme de l'oracle définissent une recette ; ils ne créent
pas de politique d'exécution, de rétention ou de résumé automatique du produit.
Cette version ne modélise pas la résolution ultérieure d'un incident ou l'abandon
autorisé d'une exigence ; ces transitions demanderont un contrat de conservation
des états historiques et de la décision, sans renoter les fixtures précédentes.
La consultation CLI/API de sources Swarm pour l'opérateur est décrite ci-dessous.
Leur admission dans le contexte d'un agent, l'interface de résumé et la qualité
LLM restent des travaux distincts au plan mémoire.

## Préparer la référence depuis des sources Swarm

`swarm --json memory summary-source --input requete.json` consulte le stockage
compatible du projet courant. La même lecture existe via POST authentifié
`/api/v1/project-memory?action=summary-source`, avec origine et CSRF usuels.
La requête porte `schema_version: 1`, `work`, `task`, `expected_revision` et
`dialogues`, liste explicite de `DialogueRef {task, attempt, sha256}` du même travail.
La cible est une tâche de travail ; chaque référence sélectionne une réponse de
cadrage conservée. `sha256` porte sur `question + "\n" + texte` en UTF-8.
Le champ `attempt` est la clé de réponse conservée : le producteur de cadrage
actuel y écrit l'ID de l'agent, pas `Agent.attempt_id`. Utiliser la référence lue,
sans reconstruire une identité de session ou de réservation.

Une révision périmée donne `revision_conflict` (CLI 3, HTTP 409). Référence modifiée,
doublon, tâche incorrecte, réponse vide et champ inconnu sont refusés. GET sur
cette action donne HTTP 405. Session/origine/CSRF restent requis. La commande
appartient aux consultations sans migration implicite : un ancien stockage
est refusé avec `storage_upgrade_required` avant toute mise à niveau.

Le paquet `swarm_summary_source_v1` comprend `payload` (chaîne JSON exacte) et
son `sha256`, avec `schema_version: 1`. Vérifier les octets UTF-8 de cette chaîne,
sans la resérialiser pour le calcul. Le contenu déclare projet, travail, tâche,
révision et destinataire `local_operator`. Il inclut le cadrage du travail,
l'instantané public complet de la seule tâche cible, puis les questions/réponses
choisies dans l'ordre demandé. Chaque texte a sa propre empreinte et chaque
paire conserve sa référence de dialogue. Le projet est aussi lié à l'instantané
de tâche ; une relocalisation ne donne pas une continuité implicite de résumé.

`provider_history_complete: false` est obligatoire : il s'agit des textes
conservés dans Swarm, qui peuvent déjà être bornés par leur producteur, sans
reconstruction de conversation fournisseur. Ni rapports des tâches voisines,
ni résultats d'outils, ni gardes privées ne sont joints. Cet export d'opérateur
n'est pas un contexte autorisé pour un agent délégué et ne prouve aucune fraîcheur
de verdict historique présent dans l'instantané de tâche.

Le convertisseur exige des annotations explicites liées au `sha256` du paquet :
`{schema_version: 1, source_sha256: EMPREINTE, protected: [...]}`. Chaque protection
utilise le format M17 et un identifiant de source lu dans le paquet ;
`current-task-snapshot` doit être protégé. Aucun état sémantique n'est deviné.

```sh
python3 tools/memory-evaluation/swarm_source.py \
  --source export.json --annotations annotations.json \
  --output-dir /tmp/mon-essai-sources
python3 tools/memory-evaluation/summarization.py \
  --history /tmp/mon-essai-sources/history.json \
  --reference-cutoff 2 --output /tmp/mon-candidat-sources.json
```

Choisir une frontière appropriée aux sources : le nombre ci-dessus illustre
l'argument, pas une politique de résumé. Conserver l'export CLI dans un fichier
nouveau privé ; la CLI l'écrit sur sa sortie standard. Le convertisseur crée un
dossier nouveau en 0700 et quatre fichiers en 0600 : `history.json`, export
original `source-packet.json`, annotations originales `annotations.json`, puis
`provenance.json`. Ce dernier lie fichiers, projet, révision, paquet et empreinte
du contexte M17 ; le reçu de référence doit citer cette empreinte d'historique.
Code 0 : préparation réussie ; code 2 : paquet/annotations invalides ou périmés,
sortie existante ou erreur d'écriture. Une interruption d'écriture peut laisser
un dossier partiel ; ce n'est pas une préparation réussie. Aucun fichier source
ou résultat existant n'est remplacé.

Recette complète isolée : `python3 tests/memory_summary_source.py BINAIRE SORTIE_NOUVELLE`.
Elle produit ses échanges par la vraie CLI avec un fournisseur scripté, compare
CLI/API, prépare M17, change publiquement les critères et vérifie les refus.
Les états des annotations restent à examiner ; les empreintes ne les valident
pas sémantiquement. La révision doit être recontrôlée avant tout futur emploi
moteur. Aucun résumé n'est admis, aucun modèle réel appelé et aucune politique
Admin ajoutée par ces commandes de lecture/préparation.

Pour étendre une commande d'inspection, vérifier aussi `cliStorageInspection` :
le lecteur fonctionnel ne suffit pas à interdire une migration à l'ouverture du
stockage. Conserver un test sur stockage ancien, avec refus explicite et version
inchangée ; une recette sur un stockage déjà courant ne détecte pas cet effet.
