# Swarm — référence technique

Pour la présentation du produit et le démarrage rapide, voir [le README](README.md) et le [guide utilisateur](GUIDE-UTILISATEUR.md).

Les références aux ressources du dépôt d’origine sont historiques ; elles ne sont pas toutes embarquées dans ce dépôt autonome.

Compagnon Go pour conserver des travaux entre sessions Claude/Codex.
Interface française, terminal 80 colonnes et Markdown ; sortie JSON pour outils.
Le cockpit peut lancer explicitement des agents Claude/Codex et suivre leurs processus.
`work list` et `resume` ne lancent aucun agent et ne choisissent pas le travail.
Voir [la procédure du cockpit](COCKPIT.md) pour le lancement et le pilotage.

## Dépôt autonome

Swarm se développe dans ce dépôt et pilote des projets distincts via `--root`.
Linux et Go 1.24+ sont le périmètre actuellement testé. Les ressources web sont
embarquées dans le binaire. Node 22 et npm sont nécessaires pour les reconstruire.

```sh
make build
./bin/swarm --root /chemin/du/projet init
./bin/swarm --root /chemin/du/projet providers init
./bin/swarm --root /chemin/du/projet web 127.0.0.1:18787
```

Ouvrir le lien de session imprimé par le serveur, puis utiliser le sélecteur des
missions dans le même onglet. Les agents installés s’authentifient séparément ;
les connexions API se configurent dans « IA et connexions ».

```sh
make test          # Go et tests Node déterministes
make frontend     # npm ci + reconstruction Monaco/xterm
make smoke        # reprise/export/import dans un projet temporaire
PUPPETEER_SKIP_DOWNLOAD=true npm ci
CHROME_BIN=/chemin/vers/chrome npm run test:connections
```

Les méthodes APEX, KS et PDCA restent des ressources du **projet piloté** :
voir [les limites de migration](docs/migration/README.md). Les clés, bases et
missions `.swarm/` ne sont pas livrées avec le code.

## Construire et commencer

Go 1.24+ ; SQLite embarqué via `modernc.org/sqlite` (sans cgo), versions figées
par `go.mod` / `go.sum`. La compilation initiale télécharge les dépendances ;
le binaire construit fonctionne hors ligne, sans Python ni serveur SQLite.
Depuis ce dossier :

```sh
go test -race ./...
mkdir -p bin
CGO_ENABLED=0 go build -trimpath -o bin/swarm .
```

Depuis le dépôt Swarm, ou avec le binaire installé dans le PATH :

```sh
./swarm init
./swarm work list
./swarm work create --input work.json
./swarm resume IDENTIFIANT
```

`--root CHEMIN` cible une autre racine ; il ne fait pas de recherche implicite
vers le dépôt Git parent. Copier seulement `bin/swarm` suffit sur une machine
Linux de même architecture. Les autres plateformes doivent être compilées et
testées séparément. Aucun hook ne compile ni n’accède au réseau.

## Requêtes versionnées

`--input -` lit stdin ; `--json` produit du JSON sans texte décoratif.
Les champs communs aux mutations métier sont `schema_version: 1`, un `event_id`
unique et `expected_revision` (0 à la création, révision courante ensuite).
La sortie contient le travail, l’historique, les gates recalculées et la fiche.
Les dates sont UTC. Les identifiants utilisent lettres ASCII, chiffres, `_`, `-`.

Création (`work.json`) :

```json
{
  "schema_version": 1,
  "event_id": "creation-api-1",
  "expected_revision": 0,
  "title": "Faire évoluer le contrat API",
  "objective": "Client et serveur utilisent le même schéma",
  "scope": "Contrat, client, serveur et tests associés",
  "criteria": ["Tests de contrat PASS sur les fichiers courants"],
  "next": "Définir la tâche de contrat"
}
```

`task add TRAVAIL` : ajouter `id`, `title`, `deliverable`, `criteria`,
`owner`, `depends` (liste optionnelle de tâches existantes), `next`.
`task update TRAVAIL` : `id`, puis les champs à modifier. `status` est facultatif.
`owner`, `blocker`, `next` restent des métadonnées. Le contrat est également modifiable :
`title`, `deliverable`, `criteria` (1 à 12 textes), `depends`, `max_attempts` (1–3),
`max_tool_calls` (1–100). Une liste omise/null est inchangée ; `depends: []` retire
les dépendances ; `criteria: []` est refusé. Les chaînes vides sont inchangées,
les textes uniquement composés d'espaces sont refusés.

Le parcours conseillé est **Configurer les validations** dans la vue Tâches :
il présente chaque critère, les contrôles structurés qui le couvrent, la portée,
les limites et l'effet exact. Le premier clic demande un aperçu au moteur ; le
second confirme cet aperçu. Modifier un champ rend l'aperçu caduc. Le retrait est
une action distincte. Aucune suggestion de l'assistant IA n'alimente ce formulaire.

La CLI utilise le même contrat et le même jeton d'aperçu :

```sh
swarm validation preview TRAVAIL --task t4 --input politique.json --json
# recopier preview_token dans le même document, ajouter un event_id stable
swarm validation apply TRAVAIL --task t4 --input politique-confirmee.json --json
swarm aide validations
```

Le document porte `schema_version`, `expected_revision`, `task_id`, `intent`
(`replace` ou `remove`) et, pour `replace`, `policy`. `apply` exige en plus
`event_id` et le `preview_token` rendu. Une révision ou une politique modifiée
impose un nouvel aperçu. Pour retirer la politique, omettre `policy` :
`{"schema_version":1,"expected_revision":12,"task_id":"t4","intent":"remove"}`.

`validation_policy` fixe ce que le conducteur peut vérifier après le handoff.
`mode: "human"` conserve toujours la revue humaine et n'accepte aucun
contrôle. `mode: "automatic"` exige de 1 à 8 contrôles structurés. Chaque contrôle
nomme la commande exacte, les indices de critères couverts, une justification
objective de cette couverture, un répertoire relatif optionnel et un délai de
1 à 300 secondes ; le budget cumulé reste inférieur ou
égal à 300 secondes. Seuls `go`, `git`, `node`, `npm`, `python`, `python3` et
`pytest` sont exécutables, directement et sans shell. Le texte du plan, du
handoff, des journaux ou d'une réponse IA n'est jamais interprété comme commande.
Tous les critères doivent être couverts ; pour une tâche issue d'un plan, chaque
check obligatoire du plan doit porter le même identifiant qu'un contrôle.

Exemple de document d'aperçu opérateur :

```json
{
  "schema_version": 1,
  "expected_revision": 9,
  "task_id": "t4",
  "intent": "replace",
  "policy": {
    "mode": "automatic",
    "controls": [
      {
        "id": "contrat-api",
        "command": ["go", "test", "./api", "-run", "TestContract", "-count=1"],
        "criteria": [1],
        "justification": "Le test échoue si le contrat API attendu par le critère 1 régresse.",
        "dir": ".",
        "timeout_seconds": 120
      }
    ]
  }
}
```

Le document de confirmation est identique avec `event_id` et `preview_token`.
Une politique humaine est le choix sûr dès qu'un critère demande d'apprécier la
lisibilité, la pertinence, la qualité ou tout autre jugement non démontré par un
contrôle objectif. Une politique automatique doit couvrir tous les critères.

Le contrat se modifie sur une tâche `todo` ou `blocked`, sans transition simultanée,
et sans agent actif dans le travail ni tâche `running`. Les dépendances inconnues,
répétées ou cycliques sont refusées atomiquement. Toute modification effective du
contrat invalide gate, override et revalidation ; les checks du plan sont recalculés.
Les tentatives et événements restent conservés. Les plans approuvés antérieurs restent
historiques : le contrat courant est celui de la tâche, les pièces mémoire doivent
être synchronisées par le conducteur.

La mission continue distingue trois faits. `mission start` enregistre l’autorisation ;
le conducteur n’est actif que tant que `swarm web` ou `swarm mission watch TRAVAIL`
renouvelle sa vérification ; les agents déjà lancés publient leur activité séparément.
`swarm mission status TRAVAIL` et le cockpit affichent le même verdict, avec dernière
vérification, erreur éventuelle, prochaine vérification relative et dernière action
persistée du **Conducteur Swarm**. Une autorisation seule n’est jamais présentée comme
une supervision active, et ce conducteur local est distinct d’une supervision externe
Codex. Si le service tombe, l’autorisation reste conservée mais aucun nouveau départ
automatique n’est promis. Au redémarrage,
le conducteur relit les tentatives actives avant de décider un départ ; les gardes
transactionnelles empêchent deux conducteurs de réserver deux fois la même place.
Après le relais d'un handoff, le conducteur n'accepte automatiquement que si la
mission est encore autorisée, non pausée et autonome, si la politique automatique
est explicite et complète, et si tous ses contrôles objectifs réussissent sur les
fichiers courants. Il crée sous `.swarm/validation/` un reçu lié à la tentative,
à l'empreinte de politique, au livrable et aux résultats. Une preuve modifiée
invalide ensuite la gate. Un échec bloque la tâche et ses dépendants, pas les
branches indépendantes. Si `max_attempts` autorise une correction, le conducteur
renvoie au producteur l'identifiant, le code de sortie, l'empreinte de sortie et
le reçu de chaque contrôle échoué. La nouvelle tentative invalide la décision
courante, rejoue tous les contrôles, puis publie un nouveau reçu ; la borne de
tentatives du plan interdit toute boucle ouverte. Une pause retient les contrôles ; la boucle les reprend
depuis la tentative terminée et le handoff encore attribuable après reprise.

### Échanges agent à agent

`swarm exchange send TRAVAIL --input échange.json` enregistre une `handoff`,
`help_request` ou `help_answer`. Chaque message porte `agent_id`, `task_id`,
`attempt_id`, `recipient_task_id` et `recipient_role`. Le rôle doit être celui
du plan courant et la tâche destinataire doit déjà exister : un message ne peut
donc ni accorder un rôle ni étendre le plan. Une remise exige un `result_state`
et au moins un artefact `{path, sha256}` ; les empreintes sont vérifiées à
l'envoi et à la consommation. Une tâche consommatrice doit dépendre de la tâche
productrice.

`swarm exchange consume TRAVAIL --input accusé.json` lie l'accusé à la tentative
destinataire. La transition est atomique et un rejeu exact reste sans effet.
Une tentative source remplacée ou un artefact modifié rend la remise obsolète.
Une `help_request` impose `timeout_seconds` entre 1 et 3600 ; le conducteur la
fait passer à `escalated` si aucune réponse n'arrive avant l'échéance, y compris
si elle avait seulement été accusée. `swarm exchange list TRAVAIL --json` et
`GET /api/v1/exchanges?work=TRAVAIL` exposent les mêmes états persistés.

`mission preview` annonce la première vague réellement calculée, pas le nombre
de créneaux demandé comme une promesse. Le profil commun cible un même dossier :
ce mode reste séquentiel, avec un seul écrivain dans tout arbre de chemins qui se
recouvre. Des profils de tâche pointant vers des dossiers déjà préparés, distincts
et non imbriqués peuvent partir en parallèle. Une file FIFO persistée départage
les missions qui attendent le même arbre ; une fin confirmée libère le tour.
Cette capacité experte ne crée ni ne supprime les copies. L'intégration est une
commande séparée, sérialisée et liée à une remise SHA-256 :
`swarm workspace integrate TRAVAIL --input manifeste.json`. Chaque cible indique
`source`, `target`, `base_sha256` (vide si elle doit être absente) et
`result_sha256`. Toute base différente produit un reçu `conflict`, un code 3 et
zéro écriture. `swarm workspace status TRAVAIL` expose les tours et reçus.
Swarm ne crée toujours aucun worktree, ne fusionne pas Git et ne rejoue pas les
contrôles de validation après intégration ; ce n'est donc pas un gestionnaire de
branches de bout en bout. Voir
`docs/plans/swarm-autonomie-coordination/a7-espaces.md`.

Exemple : `swarm task update TRAVAIL --input correction.json` :

```json
{"schema_version":1,"event_id":"correction-t4-1","expected_revision":9,"id":"t4","title":"Résolution exacte et provenance","criteria":["Aucune preuve d'une autre identité dans le score"],"depends":["t1","t3"],"max_attempts":2,"max_tool_calls":80}
```

`work update TRAVAIL` modifie `title`, `objective`, `scope`, `criteria` du travail
avec la même enveloppe de révision. Toutes ses tâches doivent être `todo`/`blocked`,
sans agent actif. Les validations sont invalidées, l'historique reste conservé.
Quitter `running` exige `outcome`: `completed`, `failed` ou `interrupted`.
`submitted` exige `completed`. `blocked` exige un motif. Une réouverture de
`accepted` passe par `todo`, puis une nouvelle tentative et une nouvelle gate.

`checkpoint TRAVAIL` : `summary` obligatoire, `next`, `memory` (chemins relatifs
de fichiers existants). `ooda TRAVAIL` : `observation`, `orientation`, `decision`,
`owner`, `next` obligatoires ; `result` facultatif. Ne pas enregistrer de pensée
privée ni de transcription intégrale.

`gate TRAVAIL --input gate.json` attend un objet :

```json
{
  "schema_version": 1,
  "event_id": "gate-contract-1",
  "expected_revision": 4,
  "task_id": "contract",
  "phase": "delivery",
  "document": {"method_version": "2", "scope_id": "contract"}
}
```

L’objet `document` doit être complété avec `artifacts`, `domains`, `checks` et
`results` selon le contrat méthode 2 (`tools/agent-workflows/references/scoring.md` dans le projet d’origine).
L’exemple ci-dessus montre l’enveloppe, **pas une preuve acceptable**.
`evaluate --input evidence.json --phase delivery` calcule sans persister.
Les phases sont `entry`, `validation`, `delivery`, `audit`.
Codes de sortie : 0 succès ; 1 évaluation calculée mais bloquée ; 2 erreur,
entrée invalide ou conflit. Une gate bloquée est persistée si son entrée est valide.

## Reprise et fiabilité

Un échec classé « environnement » (droits, sandbox, montage ou ressource
indisponible) retient la tâche concernée : le conducteur ne la relance pas à
vide et continue d'examiner les branches indépendantes. La reprise est
explicite et cible la dernière tentative avec `previous`. Elle exige alors
`precondition_evidence`, une courte observation nouvelle de la vérification
effectuée. La même observation ne peut pas être rejouée après un nouvel échec
d'environnement. Cette preuve de précondition ne valide pas le livrable et ne
permet pas d'assouplir les limites : la reprise conserve les plafonds les plus
stricts de la tentative précédente. Exemple de champs dans l'entrée de
`swarm agent start` :

```json
{
  "previous": "agent-identifiant",
  "precondition_evidence": "Lecture et écriture de la ressource vérifiées sur l'hôte",
  "instruction": "Reprendre au contrôle interrompu"
}
```

Le cockpit web et la console interactive demandent cette vérification quand le
diagnostic structuré de la tentative contient la catégorie `environment`. Ils
ne proposent jamais de désactiver le sandbox ni de modifier les montages.

`work list` et `work show ID` sont des lectures. `resume` sans identifiant
présente la liste ; avec identifiant il régénère `.swarm/views/ID.md`.
Les mutations régénèrent également la fiche ; une erreur de rendu n’annule pas
une transaction déjà enregistrée. Relancer `resume` répare la vue.

La fiche présente objectif, contexte Git, tâches actives, livrables, preuves,
blocages, gates, prochaine action, dernières décisions OODA et mémoire utile.
Qualité, progression des critères et nombre de tâches acceptées sont distincts.
Une donnée absente reste « non renseigné » ; la prochaine action n’est pas inventée.
Une tâche enregistrée en cours n’est jamais affichée comme processus vivant.

SQLite est la source canonique : état et événement sont écrits dans une même
transaction, avec contrôle de révision et déduplication des événements.
Les fichiers Markdown sont des vues dérivées datées/révisionnées : les relire
via le moteur pour obtenir l’état courant. Les preuves sont revérifiées à la
reprise et à l’acceptation ; une empreinte valide établit l’identité d’un fichier,
pas la vérité de son contenu ou l’exhaustivité du périmètre.

`.swarm/` est ignoré par Git, y compris dans un projet indépendant via son propre
`.gitignore`. Les fichiers de code, hooks et procédures restent versionnables.
Ne pas y stocker des secrets ou des conversations brutes. L’espace est local,
pas un service partagé ; chaque worktree a son propre espace dans cette version.

## Transport et sauvegarde

```sh
./swarm export ID --output /tmp/travail.zip
./swarm --root /autre/projet init
./swarm --root /autre/projet import --input /tmp/travail.zip
```

L’export logique lit état et événements dans une transaction SQLite cohérente.
Il embarque les fichiers explicitement référencés comme preuves ou mémoire, avec empreintes,
et signale ceux qui manquent. Il ne copie pas tout le code ni les fichiers ignorés.
Une preuve modifiée fait échouer l’export au lieu de certifier la nouvelle version.
L’archive est limitée à 128 Mio décompressés ; les entrées JSON à 16 Mio.

L’import garde les identifiants et refuse collisions, versions inconnues,
chemins d’archive arbitraires et pièces altérées. Les pièces sont archivées dans
`.swarm/imports/ID/` ; elles n’écrasent jamais les fichiers du projet cible.
Les gates sont revérifiées contre ce projet : absence ou différence = non valide.
Aucune fusion d’historiques divergents et aucune synchronisation Git automatique.

## Cycle de vie des missions

Les quatre actions ont des contrats distincts et passent toutes par un aperçu
lié à la révision et à la génération de cycle de vie :

```sh
./swarm lifecycle list
./swarm lifecycle preview ID archive --input /tmp/requete.json --json
./swarm lifecycle apply ID archive --input /tmp/confirmation.json --json
```

Le document d’aperçu contient `schema_version`, `expected_revision`, `action` et,
pour `purge`, `retention_days` (1 à 36 500). La confirmation reprend ces champs,
ajoute un `event_id` unique et le `preview_token` reçu. `archive` crée un ZIP dans
`.swarm/lifecycle/archives/` et marque la mission sans retirer ses données ;
`restore` enlève ce marquage. `purge` retire uniquement les journaux d’agents,
sorties terminal et échanges assistant plus anciens que la rétention. Les
événements métier, reçus, verdicts et preuves en sont exclus.

`delete` déplace atomiquement les lignes SQLite de la mission et de ses objets
liés dans `mission_trash`; `restore` les réinsère atomiquement. Les sources,
rapports, preuves et autres fichiers du projet ne sont jamais supprimés, même
s’ils sont référencés par une mission. Les opérations refusent les agents,
intentions, réservations, dialogues de préparation, conducteur ou contrôles
actifs et refont ces contrôles sous verrou lors de l’exécution. Les reçus rendent
un double envoi idempotent ; une collision pendant restauration laisse toute la
mission en corbeille. Le web utilise exactement le même moteur via les actions
`lifecycle-preview` et `lifecycle-apply`.

Dans le cockpit, **Gérer les missions** regroupe recherche, filtres Active,
Archivée et Corbeille récupérable, puis les actions **Archiver**, **Restaurer**,
**Purger l’historique** et **Supprimer**. Chaque action ouvre d’abord l’aperçu du
moteur. Supprimer exige en plus la saisie du nom exact de la mission ; fermer la
modale ou consulter l’aperçu ne modifie rien. La sélection reste sur la même
mission lorsqu’elle passe en corbeille ou revient dans la liste active.

## Workflows et tests

Lire [WORKFLOWS.md](WORKFLOWS.md) pour les points d’enregistrement APEX/PDCA/KS.
Les hooks propres à un projet ne sont pas installés par Swarm. Leur présence
ne prouve pas leur exécution par un fournisseur : voir `VALIDATION.md`.

```sh
go test -race ./...
go vet ./...
go build -o /tmp/swarm-companion .
SWARM_BINARY=/tmp/swarm-companion python3 tests/parity.py
```

Python intervient seulement dans les tests différentiels, comme oracle de la
méthode existante. Le compagnon distribué n’en dépend pas.


## Assistant IA de chaque page

Le cockpit web propose un assistant sur Dialogue IA/APEX, Tâches, Agents,
Décisions, Journaux, Reprise/OODA et Budget. Sélectionner un élément, puis
« Poser une question sur cette page ». La modale montre le contexte et le prompt
exacts avant confirmation. Le moteur reconstruit les faits côté serveur ; le
navigateur transmet seulement les coordonnées de la vue.

Les faits cités sont consultables et les actions proposées ouvrent les formulaires
habituels. Une réponse périmée conserve ses références mais ne permet plus d’ouvrir
une action. Le contrôle de structure et de références ne garantit pas la vérité
d’une interprétation. L’assistant ne valide ni tâche ni gate. Une réponse IA seule
ne constitue jamais une preuve et ne peut ni créer une politique de validation,
ni alimenter un résultat PASS. Une revue IA peut préparer une décision qualitative,
qui reste humaine (`mode: "human"`).

Claude et Codex disposent d’adaptateurs sans outils de réalisation, dans un
répertoire isolé, avec délai maximal de 300 secondes et annulation explicite.
`assistant_timeout_seconds` dans la configuration du fournisseur règle ce délai
entre 1 et 300 secondes. Les fournisseurs inconnus, dont l’adaptateur GLM actuel,
ne peuvent pas répondre à ces questions ; leur lancement comme agent de travail
reste disponible. Un refus reste explicite, sans relance silencieuse.

Questions, réponses/refus, contexte, prompt et usage déclaré sont conservés dans
SQLite (schéma 4), par travail, avec export JSON complet et inclusion dans l’archive.
Seuls les 40 derniers échanges apparaissent dans l’historique courant. Les budgets
comptent aussi les réservations des questions ; ils ne constituent pas une facture.

Conception et preuves : `docs/plans/swarm-page-assistant/` à la racine du projet.
Procédure illustrée : `docs/procedures/procedure_complete_swarm_claude_codex.docx`
(v7, 39 pages ; schémas Mermaid dans `docs/procedures/diagrams/`).

## Fournisseurs et niveaux de modèles

Le menu **Administration fournisseurs** règle trois niveaux par fournisseur,
avec modèle et effort explicites. Le modèle résolu apparaît avant lancement et
reste enregistré avec la tentative. Les questions de page utilisent par défaut
le niveau simple, les travaux et le dialogue APEX le niveau standard. Le niveau
exigeant est un choix explicite ; aucun repli automatique après un refus.
La politique proposée pour Codex utilise Luna, Sol puis Astra. Skynet conserve
son modèle configuré aux trois niveaux, avec sa facturation déclarée sur site.

La même résolution sert les lancements JSON (`level`: `auto`, `simple`,
`standard`, `exigeant`), le web et le terminal. `model_policy_hash` lie un lancement
à la politique montrée dans l’aperçu. Le catalogue est local et l’accès se teste
explicitement depuis une carte fournisseur ; ouvrir l’administration n’appelle
aucune IA. Export/import ne transporte ni secrets ni commandes.

Procédure, limites et relation avec l’administration ML :
`docs/plans/swarm-provider-routing/ADMINISTRATION.md` (document du projet d’origine).

### Lire la session d’un agent automatisé

Dans les détails de la tâche, **Voir la session de l’agent** ouvre le journal
actualisé en direct. Les nouvelles tentatives conservent les messages publics
explicites de Codex et Claude, indépendamment de la capture brute. Le dernier
message apparaît aussi au-dessus du journal. Les raisonnements internes ne sont
pas collectés dans ce canal. Les messages sont bornés à 2 048 octets chacun et
1 Mio par tentative ; une limite atteinte est signalée.

La **Capture détaillée des sorties** reste nécessaire pour conserver le contenu
des réponses d’outils. Le journal extrait les réponses lisibles des événements
capturés ; la sortie originale reste accessible. Les anciennes tentatives sans
capture ne peuvent pas restituer rétroactivement ces détails. La fin de la
session ne vaut pas validation du résultat de la tâche.

### Planification hiérarchique et livraison Git (v19)

Sur une mission neuve, `swarm planning enable WORK --input activation.json`
active des responsables de périmètre, une boîte de réception durable et des
décisions atomiques. `planning step WORK` exécute au plus une décision IA ;
`planning show WORK` affiche son état. Une mission autorisée et surveillée peut
ensuite réactiver son responsable automatiquement après un retour d'agent.

Contrats, exemples, preuves et limites :
`docs/plans/swarm-architecture-implementation/IMPLEMENTATION.md` (document du projet d’origine).
Le web propose l’activation sur une mission vide, la lecture des décisions, les
copies Git gérées, la validation héritée et le téléchargement du résultat.
`planning history`, `bundle`, `cleanup-preview` et `cleanup` exposent les mêmes
fonctions au CLI. Les limites et les preuves de recette sont dans la note liée.

## Préparation, révision et modèles IA

Voir [le parcours web et les contrats CLI](PREPARATION-UX.md) : formulaire de missions, responsable durable, validation explicite, révision avec historique et menu **IA et connexions**.
