# Cockpit : lancer, observer et piloter les agents

Le compagnon peut désormais lancer de vrais processus Claude Code/Codex CLI sous
Linux. La console est un affichage interactif ; un superviseur détaché suit chaque
processus. Fermer la console ne coupe pas les agents. `resume` reste une commande de
lecture et de reprise documentaire : elle ne lance aucun modèle.

## Pilotage web : retrouver le sens du travail

Dans **Conduite → Pilotage des agents**, choisir **Agents** pour les tentatives
et tâches à préparer, ou **Dépendances** pour voir les liens entre tâches.
L’orientation **Horizontale / Verticale** et les informations
**Simplifiées / Détaillées** sont deux réglages indépendants.

- Les flèches vont du prérequis vers la tâche qui en dépend. Une dépendance
  satisfaite reprend le verdict du moteur, y compris la fraîcheur des preuves.
- **− Replier la branche** masque les descendants qui ne restent pas accessibles
  par une autre branche. **+ Déplier** les retrouve. Les alertes masquées restent
  comptées ; les liens parent/reprise des agents ne changent pas ce repli.
- Un clic sur une carte ouvre sa mission, ses critères et son état dans un
  panneau. La lecture seule ne lance aucun agent et ne valide aucun rapport.
- Le panneau distingue **exécution observée**, **activité reçue** et
  **validation actuelle de la tâche**. Une intention sans signal ne signifie
  pas qu’un agent est terminé. Un code de sortie 0 ne valide pas le livrable.
- **Origine de cette tentative** distingue la filiation (double trait) de la
  reprise (pointillés). Les boutons ouvrent l’identité exacte, même ancienne.
- **Prochaine intervention** parcourt les demandes les plus anciennes d’abord,
  avec un ordre figé pendant la lecture. Les nouvelles arrivées sont annoncées.
  **Revenir à ma vue** retrouve les filtres, replis et la sélection de départ.
- **Ouvrir les conclusions du rapport** sert à lire le résultat. Les commandes
  de validation restent explicites et soumises aux contrôles courants du moteur.

Les préférences sont mémorisées par travail. Deux onglets gardent leur propre
sélection pendant l’utilisation. Les éléments supprimés sont retirés des
préférences ; si le stockage du navigateur est interdit, le cockpit reste
utilisable. Une panne conserve la dernière situation avec sa date et indique
qu’elle n’est plus actualisée. Après une réponse de commande perdue, relire
l’état confirmé avant de décider d’un nouvel envoi.

Sur mobile ou à fort zoom navigateur, le panneau devient un dialogue ; **Échap**
ou **Fermer le détail** rendent le focus au déclencheur. Les commandes de repli
sont séparées du zoom et utilisables au clavier.

## Démarrage dans Wattson

Depuis `/home/fpizzi/wattson_devcontainer/flaskProject` :

```sh
./tools/swarm-companion/swarm providers init
./tools/swarm-companion/swarm work list
./tools/swarm-companion/swarm console IDENTIFIANT_DU_TRAVAIL
```

`providers init` crée `.swarm/providers.json` à partir des exécutables trouvés dans
PATH. Il refuse d'écraser un fichier existant. Aucun modèle n'est appelé par cette
commande. Les chemins et arguments sont configurables dans ce fichier local.
Les fournisseurs absents de PATH n'y sont pas ajoutés.

La migration du précontrôle est progressive. Un profil existant qui ne contient
pas `preflight_required` conserve son droit de départ après les contrôles
historiques de l'exécutable, du workspace et des ressources. Son précontrôle est
enregistré `compatible` / `unverified`, jamais `ready` / `verified` : `--version`
ne prouve ni l'exécution dans le sandbox effectif du fournisseur, ni ses droits
sur le workspace. Une sonde configurée reste bloquante si elle échoue, même dans
ce mode compatible.

Pour rendre la preuve obligatoire avant toute réservation, ajouter
`preflight_required: true`, puis configurer un adaptateur déterministe avec
`preflight_kind: "provider-context"`, ses arguments dédiés dans `preflight_args`
et les capacités `process`, `workspace_read` et `workspace_write` dans
`preflight_capabilities`. Il doit retourner sur stdout un reçu JSON de schéma 1
marquant chacune de ces capacités `verified`. Une capacité inconnue ou refusée,
un reçu invalide et une commande historique ou `operator-command` bloquent alors
le départ. Activer cette politique fournisseur par fournisseur seulement après
avoir qualifié l'adaptateur dans le contexte réel de la commande métier.

Exemple de politique stricte, à compléter avec un adaptateur réellement fourni :

```json
{
  "preflight_required": true,
  "preflight_kind": "provider-context",
  "preflight_args": ["--swarm-preflight"],
  "preflight_capabilities": ["process", "workspace_read", "workspace_write"]
}
```

Swarm ne fournit actuellement aucun adaptateur Codex ou Claude qualifié. Ne pas
activer la politique stricte avec une fixture ou un JSON déclaratif qui ne sonde
pas le même contexte fournisseur que la commande métier.

Adaptateurs initiaux vérifiés sur l'aide des CLI installées le 11 septembre 2026 :

- Claude : `claude -p --output-format stream-json --verbose` ; prompt sur stdin.
- Codex : `codex exec --json --sandbox workspace-write -` ; prompt sur stdin.

Les permissions et l'authentification restent celles du fournisseur. Aucun flag
qui supprime les contrôles de permissions n'est ajouté. Une permission qui ne peut
pas être traitée en mode non interactif peut faire échouer l'agent : examiner les
journaux et la configuration locale du fournisseur. Le cockpit ne contourne pas
ce refus. Une configuration fournisseur est du code de confiance : ne pas adopter
un fichier provenant d'un dépôt tiers sans le revoir.

Les noms des variables d'environnement supplémentaires à transmettre s'inscrivent
dans `env_allow`, par exemple `ANTHROPIC_API_KEY` ou `OPENAI_API_KEY` **si votre
installation les utilise**. Leurs valeurs ne vont pas dans la configuration ni
les journaux du superviseur. HOME, PATH et les variables usuelles de configuration,
certificats et proxy sont hérités. Les sorties du fournisseur peuvent contenir des
données sensibles ; la capture complète est désactivée par défaut.

## Utiliser la console

La console s'actualise chaque seconde. La ligne du bas accepte ces commandes,
validées par Entrée. `help` rappelle les commandes. Ctrl-C ou `q` ferme l'affichage.

| Commande | Effet |
|---|---|
| `new audit | Auditer les fichiers | Rapport avec preuves | chemins confinés ; droits vérifiés` | Crée une tâche avec critères |
| `assign audit codex` | Attribue un responsable à la tâche |
| `priority audit 9` | Remonte sa priorité dans l'affichage ; 0 à 9 |
| `capture on` | Active la capture des sorties pour les prochains lancements |
| `start audit codex` | Lance Codex sur cette tâche dans la racine du projet |
| `start audit claude copies/audit` | Lance Claude dans ce sous-répertoire existant |
| `select agent-IDENTIFIANT` | Affiche activité et derniers journaux |
| `filter audit` | Filtre par texte ; `filter` réinitialise |
| `status running` | Filtre l'état observé ; `status` réinitialise |
| `freeze` | Fige/reprend le défilement de l'affichage |
| `pause` / `unpause` | Interdit/réautorise les nouveaux départs du travail |
| `stop agent-IDENTIFIANT` | Demande l'arrêt ; attendre sa confirmation |
| `note agent-IDENTIFIANT Vérifier le cas limite` | Conserve une consigne pour la reprise |
| `retry agent-IDENTIFIANT Consigne complémentaire` | Crée une nouvelle tentative avec le contexte précédent et ses notes récentes |
| `reconcile agent-IDENTIFIANT` | Réconcilie une session perdue quand l'absence de processus peut être démontrée |
| `submit audit docs/handoff-audit.md` | Soumet le livrable existant après examen humain ; n'accepte pas la tâche |
| `ready audit` | Rouvre une tâche sans agent actif avant un nouveau départ |

Un agent doit être terminé avant `retry`. Lancer plusieurs agents exige des
répertoires de travail distincts **et non imbriqués** dans la racine du projet. Le
cockpit refuse deux agents sur une même tâche ou sur des espaces qui se recouvrent.
Il ne fabrique ni ne nettoie les copies. Le profil commun reste donc un mode
séquentiel honnête ; les créneaux sont un plafond. Une file FIFO persistée ordonne
les missions qui attendent le même arbre. Des profils par tâche peuvent cibler
des copies préparées séparément. Leur remise peut ensuite être intégrée par la
commande explicite `workspace integrate`, sérialisée et en échec fermé si une
empreinte de base a changé. Ce mécanisme copie les artefacts déclarés ; il ne
crée pas de worktree, ne fusionne pas Git et ne valide pas le résultat intégré.
Les règles projet et le guide de terrain doivent être présents et pertinents dans
chaque copie. Le contrat exact est dans
`docs/plans/swarm-autonomie-coordination/a7-espaces.md`.

Les priorités ordonnent l'affichage. Il n'existe pas encore d'ordonnanceur qui
prélève automatiquement les tâches. Les rôles planner/subplanner/worker et la
parenté sont disponibles dans le contrat JSON ; les rôles sont aussi proposés
dans les formulaires terminal et web. Ils sont transmis au prompt,
ils ne constituent pas une barrière de permissions ni une récursion automatique.

## Ce que signifie l'état affiché

- `queued/unconfirmed` : intention enregistrée ; processus non confirmé.
- `starting`, `running` : superviseur actif, avec signal daté et PID observés.
- `stopping` : arrêt demandé, pas encore confirmé.
- `completed`, `failed`, `interrupted` : le superviseur a constaté la fin.
- `unknown/no-heartbeat` : plus de signal depuis 10 secondes. Ce n'est pas une
  preuve d'arrêt ; le verrou du workspace est conservé.
- `unknown/other-host` : historique actif provenant d'un autre hôte/redémarrage/espace de PID ;
  vérification nécessaire avant de libérer le travail.

L'activité indique la tâche et le type du dernier événement fournisseur reconnu,
pas une reconstruction de son raisonnement. Les jetons déclarés sont conservés
quand le format est reconnu ; le coût facturé reste indisponible.

Après un arrêt demandé, le superviseur envoie SIGTERM au groupe de processus,
puis SIGKILL après trois secondes si nécessaire. Le budget de temps vaut 30 minutes
par défaut ; le JSON permet de le régler de 1 seconde à 24 heures. Ce budget
n'est pas un plafond financier. Les modifications déjà réalisées restent sur disque.

Après la fin d'un processus, la tâche est **soumise à examen**, jamais acceptée.
Un code de sortie nul ne suffit pas : le conducteur relaie le handoff seulement
si la tentative s'est terminée normalement et qu'elle a produit **un seul**
rapport lisible et non vide, `docs/<tâche>.md` ou `docs/<tâche>-*handoff*.md`,
modifié après son départ. La tâche passe alors en `submitted` et le journal de
la tentative nomme le fichier relayé. Sans rapport, avec un rapport vide,
plusieurs rapports candidats, un rapport antérieur à la tentative, ou après un
échec ou une interruption, la tâche reste **bloquée avec son motif** et le
refus est journalisé. `submit` reste disponible pour soumettre un handoff à la
main. L'acceptation passe toujours par les gates existantes et leurs preuves
courantes. Aucun bouton ne force SHIP.
Si une écriture métier rencontre une révision concurrente, le journal demande une
réconciliation ; `reconcile` peut terminer cette mise à jour après relecture.

## Fil d'activité et repère de visite

Le mode Conduite affiche, en bandeau au-dessus du graphe, la ligne de vie du
travail : chaque
départ automatique, chaque relais, chaque refus motivé, chaque décision
d'opérateur, dans l'ordre du temps. Deux sources y sont fusionnées — les
décisions locales et les mutations du travail — et rien n'y est reconstruit :
un motif absent s'affiche « motif non renseigné » plutôt que de recevoir une
explication plausible.

Chaque ligne porte son origine, en toutes lettres autant qu'en couleur :

| Marqueur | Signification |
|---|---|
| `▸ moteur` | le système a agi seul : départ automatique, relais de handoff, refus |
| `● vous` | un opérateur a décidé : suspension, réglage, gate, acceptation, dérogation |
| `▸ moteur` sur une décision | le moteur a fermé une demande : revalidation constatée, sujet remplacé, dérive devenue un état |

L'origine se lit d'abord dans le type d'événement. Deux types sont écrits par
les deux voies et ne peuvent pas être classés ainsi :

- `task.update` — l'opérateur depuis le cockpit, la CLI ou le terminal, et le
  moteur au départ, au relais et à la fin d'une tentative. L'origine est écrite
  dans la mutation elle-même ; les trois portes extérieures effacent ce champ,
  donc personne ne peut se déclarer moteur depuis l'extérieur.
- la fermeture d'une demande — un acquittement humain et une fermeture décidée
  par le moteur portent des types distincts depuis qu'une fermeture automatique
  s'est affichée « vous » sur un message disant « moteur ».

Quand rien ne tranche, l'absence d'origine se lit « humain » : le fil
**sous-déclare** l'autonomie plutôt que de l'exagérer. Attribuer au moteur un
geste humain ferait croire à une autonomie qui n'a pas eu lieu, ce que ce fil
existe précisément pour démentir.

Le fil se replie sur son titre, qui continue d'annoncer ce qu'il masque, et
l'état plié ou déplié est conservé d'une visite à l'autre. Un liseré reprend
l'origine de chaque entrée et les entrées arrivées depuis votre visite sont
surlignées : la couleur redouble le mot, elle ne le remplace jamais.

Un trait marque votre dernière visite. Il est figé à l'ouverture du travail :
relu à chaque rafraîchissement, il glisserait sous vos yeux dès qu'une autre
session — cockpit terminal, second onglet — enregistre une visite. La visite
s'enregistre en **quittant** le travail, jamais en l'ouvrant, sans quoi elle
effacerait le repère que vous venez d'ouvrir pour le consulter.

Le filtre « décisions seulement » ne garde que ce qui a demandé ou reçu un
arbitrage humain. Le fil se lit par pages ; « Voir plus » remonte le temps.

Un bandeau résume l'état en une phrase, toujours au même endroit : ce qui a
avancé depuis votre visite, ce qui attend une décision, le coût. Quand rien n'a
changé, il le dit en toutes lettres plutôt que d'aligner des zéros.

## Coût : ce qui est rapporté, ce qui ne l'est pas

Le budget réserve un forfait à chaque départ ; les fournisseurs, eux, rapportent
ce qu'une tentative a réellement coûté — quand ils le rapportent. Les deux ne se
confondent jamais.

**Règle qui commande tout affichage : un total partiel se déclare partiel.**
Les tentatives qui ne rapportent rien sont comptées à part, jamais estimées :
les compter pour zéro reviendrait à affirmer qu'elles n'ont rien coûté.

| Situation | Affichage |
|---|---|
| Aucune tentative | « aucune tentative » |
| Tentatives sans coût déclaré | « coût réel non rapporté » |
| Coût connu, partiellement | « 4,20 USD rapportés sur 3 tentative(s) · 2 sans coût rapporté » |

Un coût inconnu ne s'écrit jamais « 0,00 ». Le nœud du graphe porte le cumul de
sa tâche sous la même règle.

Une tâche dont le coût rapporté dépasse **deux fois la réserve par départ**
cesse de partir seule, et une demande est adressée à l'opérateur avec le
montant. Cette borne empêche une dépense à venir : elle n'annule rien de ce qui
est déjà engagé, et n'accepte ni ne refuse aucune tâche. Sans réserve
configurée, aucun plafond n'est déduit ; des tentatives muettes ne la
déclenchent pas, leur coût étant inconnu et non nul.

## Graphe des dépendances

La vue Dépendances dessine un nœud par tâche et une flèche par prérequis visible.
L’orientation est choisie par l’utilisateur. Le niveau détaillé affiche l’action
publique et les coûts disponibles ; une valeur absente reste « non rapportée ».

Une arête pleine correspond à un prérequis satisfait selon les preuves courantes
du moteur ; une arête pointillée indique un prérequis en attente. Le statut
`accepted` seul ne suffit pas : une preuve périmée retire ce verdict. Les états
sont aussi écrits dans les nœuds et expliqués par la légende.

Le clic ou le clavier ouvre le panneau d’inspection. Les actions restent
explicites dans ce panneau. La vue Agents constitue l’alternative en cartes ;
elle ne dépend pas du repli du graphe. Dagre et les ressources sont embarqués
localement : aucun CDN n’est nécessaire pour dessiner le graphe.

## Deux niveaux d'interface

Le cockpit ouvre en **mode Conduite** : un seul écran — barre de conduite,
« À traiter », plan avec l'action courante de chaque agent. Les journaux bruts,
l'administration des fournisseurs, l'OODA, la hiérarchie et l'assistant sont
rangés, pas retirés.

« Passer en mode expert » rend toutes les vues ; la bascule est conservée d'une
session à l'autre et peut être imposée par l'URL (`?mode=expert`, `?mode=conduite`).

En terminal, la commande `mode` fait la même bascule : écran de conduite compact
ou tableau de bord complet. `--plain`, `NO_COLOR` et `TERM=dumb` restent
respectés dans les deux cas.

## Niveaux d'autonomie et départs automatiques

Chaque travail porte un niveau, réglé localement comme la suspension des départs.
Le niveau décide de ce que le moteur fait sans demander, jamais de ce qu'il
accepte : aucun niveau n'accepte une tâche, n'accorde une dérogation ni ne fait
passer une gate.

| Niveau | Départs | Relais du handoff | Gate et acceptation |
|---|---|---|---|
| `manuel` | humain | humain | humain |
| `assiste` | humain | automatique si preuve lisible | humain |
| `autonome` (défaut) | automatiques dans les créneaux | automatique | humain |

```sh
./tools/swarm-companion/swarm autonomy TRAVAIL            # lire le réglage courant
./tools/swarm-companion/swarm autonomy TRAVAIL assiste    # changer de niveau
./tools/swarm-companion/swarm autonomy TRAVAIL autonome 3 # niveau et créneaux
./tools/swarm-companion/swarm dispatch TRAVAIL            # ordonnancer maintenant
```

Le premier lancement humain enregistre un **profil de lancement** — fournisseur,
rôle, espace de travail, consigne, limites — sur la tâche et sur le travail.
L'ordonnanceur rejoue ce profil : le formulaire n'est plus à remplir tâche après
tâche. Le profil d'une tâche prime sur celui du travail.

Une tâche part automatiquement si, dans cet ordre : le niveau est `autonome`, les
départs ne sont pas suspendus, un créneau est libre, ses dépendances sont
acceptées et fraîches, un profil existe, elle n'a pas épuisé ses deux tentatives
automatiques, et son espace de travail n'est pas déjà occupé. Deux départs
simultanés supposent deux espaces de travail distincts. Pour un arbre commun, le
premier tour FIFO encore éligible passe ; pause, dépendance redevenue invalide ou
tâche non candidate libèrent un tour en attente sans annoncer la tâche terminée.

Ne repartent jamais d'elles-mêmes : une tâche dont le handoff a été refusé, une
tâche arrêtée à la demande de l'opérateur, une tâche après deux tentatives
automatiques infructueuses. Chaque refus d'ordonnancement est journalisé dans le
travail avec son motif.

## Ce qui réclame un humain

Le cockpit distingue ce qui s'affiche de ce qui se décide. Une information
visible n'est pas une demande ; seules ces catégories réclament un opérateur,
et chacune n'apparaît qu'une fois par sujet :

| Catégorie | Déclencheur |
|---|---|
| Rapport à examiner | une tâche est soumise |
| Gate à revalider | gate échouée, refusée, ou preuves périmées |
| Handoff non relayé | tentative terminée dont le rapport manque, est vide, périmé ou ambigu |
| Tentative infructueuse | échec ou interruption, avec le nombre de tentatives |
| Garde-fou déclenché | tentative arrêtée par une limite d'exécution |
| Signal perdu | tentative déclarée vivante sans signal depuis le délai de surveillance |
| Budget estimé | seuil atteint ou enveloppe épuisée ; estimation, jamais une dépense facturée |

Si la tâche porte déjà la demande, la tentative qui l'a produite n'en ajoute pas
une seconde. Une tentative vivante n'est pas une décision. Une tâche acceptée et
fraîche n'interrompt personne. Un acquittement n'accepte pas la tâche et
n'arrête aucun processus.

## Automatiser les mêmes actions

```sh
./tools/swarm-companion/swarm work show IDENTIFIANT_DU_TRAVAIL
./tools/swarm-companion/swarm agent start IDENTIFIANT_DU_TRAVAIL --input lancement.json
./tools/swarm-companion/swarm agent list IDENTIFIANT_DU_TRAVAIL
./tools/swarm-companion/swarm agent show IDENTIFIANT_AGENT
./tools/swarm-companion/swarm agent stop IDENTIFIANT_AGENT
./tools/swarm-companion/swarm agent logs IDENTIFIANT_AGENT
./tools/swarm-companion/swarm control IDENTIFIANT_DU_TRAVAIL --input commande.json
```

`lancement.json`, avec la **révision courante lue** et un nouvel `event_id` :

```json
{
  "schema_version": 1,
  "event_id": "agent-audit-001",
  "expected_revision": 2,
  "task_id": "audit",
  "provider": "codex",
  "workspace": ".",
  "role": "worker",
  "instruction": "Commencer par les tests de chemins, produire un handoff factuel.",
  "timeout_seconds": 1800,
  "capture_output": false
}
```

Réessayer exactement ce JSON conserve la même session et ne redémarre pas le
processus. Une nouvelle tentative a un nouvel identifiant. `previous` peut désigner
une session terminée du même travail et de la même tâche ; `parent` un agent du
même travail. `commande.json` utilise par exemple :

```json
{"command": "priority audit 9"}
```

Le canal `control` s'adresse à un opérateur local. Les actions sont persistées ;
`agent start` possède une clé d'idempotence et une révision, alors que les commandes
courtes de la console opèrent sur l'état courant. Une note n'est **pas envoyée à la
conversation d'un modèle déjà lancé**. Pour une nouvelle consigne d'exécution,
arrêter puis relancer explicitement : la reprise inclut la tâche, le checkpoint,
la référence précédente et les notes récentes. La restauration native d'une session
Claude/Codex n'est pas encore intégrée.

## Persistance, journaux et portabilité

SQLite demeure la source unique : sessions, tentatives, signaux, intentions d'arrêt,
contrôles et journaux y sont enregistrés. Les heartbeats ne changent pas la révision
métier du travail. Le stockage courant utilise la version 3, avec sauvegarde
avant migration ; requêtes et archives restent en version 1. Un ancien binaire
ne lit pas nécessairement la base migrée. Utiliser un stockage local, pas une base partagée entre hôtes via CIFS/NFS.

Les sorties capturées sont plafonnées à 1 Mio ou 1 000 lignes par session ; chaque ligne est
limitée à environ 2 Kio et les 2 000 dernières entrées par session sont conservées.
Les sorties sont regroupées au maximum une seconde en mémoire avant écriture
transactionnelle ; un crash brutal peut perdre ce dernier intervalle. Les commandes
et changements de cycle de vie sont persistés immédiatement.
Les contrôles terminal et caractères directionnels sont retirés. Ce nettoyage
n'est **pas** une anonymisation ni une détection exhaustive des secrets.

La vue **Gérer les missions** affiche aussi les archives et la corbeille
récupérable. Elle sépare clairement rangement, restauration, purge des vieux
journaux et mise en corbeille. L’aperçu est obligatoire avant confirmation ; la
mise en corbeille demande le nom exact et ne touche jamais aux fichiers du projet.

`agent logs ID` renvoie les 200 dernières entrées. `agent logs ID -1` commence au
premier événement conservé ; fournir ensuite le dernier `seq` pour parcourir
les fenêtres suivantes, sans rescanner tout l'historique. `--output nouveau.jsonl`
exporte la fenêtre demandée sans écraser un fichier existant.

`export` inclut un historique borné du cockpit (200 dernières sessions, 200 lignes
par session, 500 contrôles). `import` le conserve dans le manifeste de reprise sans
l'injecter dans les sessions actives. Consultation :

```sh
./tools/swarm-companion/swarm agent history IDENTIFIANT_DU_TRAVAIL_IMPORTE
```

Le superviseur de processus et le terminal interactif sont implémentés pour Linux.
Le binaire peut être déplacé vers un autre projet Linux avec sa configuration de
fournisseurs ; réinstaller les CLI sur l'hôte cible. La web locale protégée
est disponible ; aucune écoute distante n'est autorisée.

## Vérification reproductible

```sh
cd tools/swarm-companion
go test -race ./...
go vet ./...
go build -o /tmp/swarm-cockpit .
python3 tests/cockpit_smoke.py --binary /tmp/swarm-cockpit --report /tmp/cockpit-smoke.json
```

La recette utilise de vrais processus détachés et un pseudo-terminal, avec un
fournisseur **fictif et explicitement identifié**. Elle ne consomme aucun modèle.
L'authentification, les permissions effectives et le cycle complet des fournisseurs
réels restent à vérifier dans votre environnement lors du premier lancement.

```mermaid
flowchart LR
  U[Opérateur] --> C[Console / commandes]
  C --> D[(SQLite : travail, commandes, sessions, logs)]
  C --> S[Superviseur détaché par agent]
  S --> P[Processus Claude / Codex]
  P --> S
  S --> D
  D --> C
  H[Handoff et preuves] --> G[Gates existantes]
  G --> D
```

### Console : sélectionner une tâche puis agir

`swarm console` ouvre directement l’accueil plein écran « VOS TRAVAUX ».
Choisir un travail avec ↑/↓ puis Entrée : noms, objectifs, prochaine action et
compteurs des tâches sont visibles, sans saisir de numéro ou d’identifiant.
Sélectionner ensuite la tâche avec ↑/↓,
puis **Entrée** (ou `?`) pour ouvrir son dialogue d’actions. Aucun identifiant
agent n’est à recopier : le dialogue retrouve la dernière tentative de cette tâche.
Depuis le panneau agents, il utilise l’agent sélectionné explicitement.

Actions : **Lancer**, **Relancer**, **Arrêter**, **Journaux**, **Détail complet**,
**Réconcilier**, **Fermer**. Les actions impossibles expliquent pourquoi : par
exemple un agent actif doit être arrêté avant une relance. Arrêter demande une
confirmation ; Échap annule sans effet. La fin effective est suivie séparément.

Pour lancer : choisir le fournisseur avec ←/→, dont `skynet-glm` s’il est configuré ;
Tab passe au champ suivant. Le répertoire du projet est renseigné automatiquement.
Une consigne est préremplie et modifiable (Ctrl-U pour effacer). Seul Entrée sur
**Confirmer le lancement** crée un processus. Les noms de fournisseur et limites
viennent de la configuration, pas d’une liste Claude/Codex inscrite dans l’interface.

Pour relancer : fournisseur et espace sont repris de la tentative ; modifier la
consigne si nécessaire, puis Tab vers **Confirmer le lancement** et Entrée.
Le lien vers la tentative précédente et les notes de reprise sont conservés.
Le détail complet est défilable avec ↑/↓. Journaux sélectionne automatiquement
l’agent de la tâche. Tab change de panneau ; q puis Entrée ferme la console.

Minimum : 64 colonnes × 20 lignes. `--plain` reste le dialogue texte de secours ;
`TERM=dumb` le sélectionne aussi. Le mode texte conserve les commandes avancées.
Sans terminal, console avec identifiant produit un instantané JSON.

Après remplacement du binaire, fermer et rouvrir la console. Une console déjà
en mémoire n’est pas mise à jour. Les versions à partir de ce correctif affichent
un avertissement et refusent un nouveau lancement quand leur exécutable a été
remplacé. Une ancienne version peut afficher `json: unknown field "limits"` :
quitter cette console puis relancer le wrapper, sans supprimer le champ de sécurité.

Recette : `tests/console_dialog_smoke.py` utilise un PTY 80×24 et des fournisseurs
factices, y compris un fournisseur nommé skynet-glm avec limits. Elle effectue
lancement, annulation d’arrêt, arrêt, relance et remplacement du binaire sans
saisir d’identifiant agent ni de chemin de workspace. Ce test ne contacte pas GLM.

### Directives et conditions de sortie des tentatives

Chaque nouveau prompt précise la racine, le workspace, les critères et les pièces
utiles. Il demande des recherches ciblées avec `rg`, interdit les balayages du
poste comme `find /`, impose des commandes bornées et un handoff conservé tôt.
Après deux corrections sans progrès démontré : OODA, blocage explicite et retour
au conducteur. Ces consignes ne sont pas un sandbox : elles ne peuvent empêcher
à elles seules un premier appel inadapté.

Le superviseur applique aussi des seuils, même avec `capture_output: false` :

| Condition observable | Défaut |
|---|---:|
| Durée totale de tentative, déjà existante | 1 800 s |
| Aucune sortie fournisseur reçue | 180 s |
| Outil identifié commencé sans résultat | 300 s |
| Appels d’outils reconnus par tentative | 100 |
| Appels identiques consécutifs (nom + entrée structurée) | 4 |
| Résultats d’outils en erreur consécutifs | 3 |

À la limite : SIGTERM, puis SIGKILL au groupe après trois secondes si nécessaire ;
statut `interrupted` seulement une fois la fin du processus constatée. La tâche
reste bloquée pour examen ; le motif est journalisé. Aucun retry automatique.
La limite de durée totale reste l’ultime borne même si le fournisseur émet du bruit.
Un processus en attente noyau non interruptible ou un stockage indisponible peut
retarder la confirmation d’arrêt : ne pas libérer son workspace sans vérification.

Les seuils sont configurables par fournisseur dans `.swarm/providers.json` :

```json
"limits": {
  "silence_seconds": 180,
  "tool_seconds": 300,
  "max_tool_calls": 100,
  "max_repeated_calls": 4,
  "max_consecutive_errors": 3
}
```

Ce bloc s’ajoute à l’objet du fournisseur, à côté de command/args/env_allow.
Zéro ou champ absent signifie défaut ; les valeurs négatives et excessives sont
refusées. Limites figées dans chaque tentative et consultables avec `agent show`.
Les superviseurs déjà lancés ne sont pas mis à jour en remplaçant le binaire.

Le décompte comprend les événements Claude `tool_use`/`tool_result` et les événements
Codex `command_execution`/`mcp_tool_call` started/completed avec leurs identifiants.
Les répétitions d’un même identifiant sont ignorées. Les autres événements ou formats
non reconnus ne permettent pas d’appliquer les compteurs d’outils ; silence et durée
restent disponibles. Les lignes JSON dépassant la borne de décodage ne sont pas
analysées. Une sortie ou un heartbeat n’est pas une preuve de progrès métier.
Les limites détectent des comportements observables ; elles ne jugent pas la qualité
du travail. Le polling légitime peut atteindre la limite de répétition : le borner
ou choisir explicitement un seuil adapté, puis conserver cette décision.

### Lisibilité et palettes

La touche `t` bascule entre clair et sombre depuis l’accueil ou le tableau de bord.
La palette initiale utilise SWARM_THEME=light/dark si défini, puis COLORFGBG quand
il annonce un fond clair ; sinon elle utilise la palette sombre. NO_COLOR désactive
les couleurs ; TERM=dumb et --plain conservent le mode texte de secours.
Les couleurs accompagnent toujours les libellés et le marqueur de sélection.
À moins de 110 colonnes, les panneaux tâches et agents s’affichent alternativement
sur toute la largeur (Tab pour changer). Au-delà, ils sont côte à côte.
Les informations détaillées restent dans le dialogue de tâche et son détail défilable.

### Activité observable et arrêt (correctif du 11 septembre)

Sélectionner une tâche suffit à afficher sa dernière tentative : fournisseur,
état, appels et résultats d’outils observés, dernier outil et date du dernier
résultat. Après arrêt, le panneau affiche le motif de fin et la prochaine action ;
Entrée ouvre les commandes de reprise. Les compteurs sont persistés dans l’agent.
Ils mesurent une activité technique, pas un pourcentage de réalisation : le
livrable et sa gate restent nécessaires pour valider la tâche.

Le décodage accepte désormais des événements JSON jusqu’à 1 Mio, indépendamment
de la capture des journaux. Un événement trop grand ou JSON illisible rend la
surveillance explicitement incomplète : le chronométrage par outil est suspendu
pour cette tentative, car un résultat peut avoir été perdu. La durée totale,
le silence et les budgets observables restent bornés. Le parseur reprend à la
ligne suivante ; les journaux ne deviennent pas illimités.

Une console déjà ouverte doit être quittée puis relancée après installation.
Les anciennes tentatives gardent leur état historique ; corriger le moteur ne
les relance pas automatiquement et ne valide pas leur travail.

### Journal opérationnel sans capture brute

Les nouvelles tentatives enregistrent des événements horodatés « Outil lancé »
et « Résultat d’outil reçu », même quand la capture des sorties est désactivée.
Ces messages contiennent le nom de l’outil et les compteurs, pas ses arguments,
le contenu lu ou les échanges privés du modèle. La rétention habituelle des
journaux s’applique. Les sorties brutes restent une option distincte.

Le panneau SUIVI EN DIRECT indique le temps écoulé depuis le heartbeat, la durée
et le dernier résultat, puis EN COURS lorsqu’un outil est en attente de résultat,
ou EN ATTENTE de la prochaine sortie fournisseur. Un heartbeat confirme le suivi
du processus, pas une avancée métier. L’affichage figé est signalé explicitement.

Rouvrir uniquement la console suffit pour voir ces indicateurs sur les agents
déjà actifs. Le nouveau journal par événement concerne les superviseurs lancés
avec cette version ; aucun historique n’est reconstruit artificiellement et
aucun agent actif n’est arrêté pour appliquer cette amélioration.

### Comprendre l’action et consulter les détails

La touche `d` ouvre directement les détails de la tâche sélectionnée. Flèches :
défilement ; Entrée : actions ; Échap : retour. La tentative et son historique
sont rechargés à chaque rafraîchissement, même quand la fenêtre reste ouverte.

Les nouvelles tentatives persistées par cette version décrivent les opérations :
lecture/écriture/modification de fichier, recherche ou commande. Une description
fournie par le modèle est préfixée « Annonce de l’agent » ; elle ne constitue pas
une preuve de résultat. La commande ou le chemin est borné, nettoyé des caractères
de contrôle et masqué lorsqu’un marqueur sensible courant est repéré. Cette
protection heuristique n’est pas une garantie de détection de tout secret :
ne jamais transmettre de secrets en clair dans les descriptions ou commandes.
Aucun corps de résultat d’outil ni raisonnement privé n’est copié dans ce suivi.
Le journal conserve les actions et cibles opérationnelles ainsi que la réception
des résultats. Leur contenu brut reste gouverné par l’option de capture séparée.

Les anciennes tentatives n’ont pas ces métadonnées. Afficher « explication non
fournie par cette tentative » plutôt que reconstituer ou inventer une explication.

### Blocage visible dès l’accueil

L’accueil indique lorsqu’aucune tâche n’est en cours et présente le motif de la
première tâche bloquée du travail sélectionné. Ouvrir un travail ne lance pas
un agent. La fenêtre d’actions d’une tentative terminée affiche son motif et le
budget d’outils consommé avant les commandes. Le détail commence aussi par le
motif de fin. La limite de 100 appels reste inchangée ; une reprise est explicite.

### Revue et acceptation depuis la fenêtre de tâche

Entrée ouvre les actions. Les raccourcis de cette fenêtre sont :

- `l` : lire le rapport dans une fenêtre défilable (←→ change de rapport détecté) ;
- `s` : soumettre un rapport pour revue, avec choix des fichiers détectés dans docs/ ;
- `v` : consulter les contrôles de gate, leurs preuves et les motifs de blocage ;
- `a` : demander l’acceptation, avec confirmation explicite ;
- `r` : ouvrir une reprise de tentative (ne vaut pas revue).

Pour une tâche soumise, l’ouverture se positionne sur la revue des gates, pas sur
un nouveau lancement. Soumettre conserve l’état « À vérifier ». Accepter exige
une gate delivery valide, des preuves courantes et des dépendances acceptées.
Ces contrôles sont refaits à la confirmation. Une gate absente, échouée ou périmée
n’est pas contournable par ce bouton. Le conducteur reste responsable d’enregistrer
l’évaluation des preuves ; le bouton ne fabrique pas une gate verte.

La recherche de rapports est bornée à docs/, sur le préfixe de tâche, sans suivre
les répertoires symboliques. Un chemin local peut être saisi, y compris avec
espaces. Lecture jusqu’à 256 Kio avec annonce si tronquée ; caractères de contrôle
neutralisés. Aucun rapport ni contenu de preuve n’est exécuté.

La largeur Unicode utilise la segmentation de graphèmes d’uniseg v0.4.7
(https://github.com/rivo/uniseg). Les séquences clavier CSI/SS3 sont consommées,
le collage encadré reste du texte et SIGWINCH redessine sans quitter la console.
Les caractères trop larges pour une colonne utilitaire d’une seule cellule sont
signalés par une ellipse ; les fenêtres normales ont une largeur supérieure.

### Acceptation et dérogation manuelle

Sélectionner une tâche avec les flèches, puis **Entrée** :
- **a** ouvre l'acceptation normale ; **Entrée** confirme le choix présélectionné.
  Un refus reste affiché dans la fenêtre avec sa raison. Annuler affiche un retour explicite.
- **f** ouvre **Forcer par dérogation**, également accessible depuis l'acceptation.
  Saisir un motif (au moins 10 caractères), **Entrée** pour sélectionner la confirmation,
  puis **Entrée** pour l'enregistrer. **Échap** annule sans mutation.
- **v** expose les gates et la décision manuelle persistée.

La dérogation produit le statut distinct `waived` (« Dérogation »), avec motif,
UID local, horodatage, état précédent et révision dans SQLite et l'événement
`task.override`. Ce n'est pas une authentification individuelle au-delà du compte
système exécutant la console. Les scores et preuves restent inchangés.
Les dépendances doivent être acceptées par revue ou dérogation. L'agent doit être
arrêté et l'état réconcilié avant la décision. Une dérogation ouvre la suite du
workflow, mais ne lance aucun agent automatiquement.
Rouvrir avec `ready IDENTIFIANT` retire la décision courante ; l'historique subsiste.
Après remplacement du binaire, fermer puis rouvrir la console pour disposer de ces actions.

### Confirmer un lancement et comprendre un refus

Le formulaire Lancer sélectionne directement **Confirmer le lancement**.
Entrée tente le lancement ; Tab permet de modifier fournisseur, espace et consigne.
Un refus ouvre une fenêtre **LANCEMENT REFUSÉ** avec sa cause et est conservé
dans `cockpit_events` sous `launch.refused`. Entrée revient au formulaire,
**v** ouvre les gates de la dépendance bloquante lorsqu'elle est identifiée,
**o** propose une réouverture explicite de la tâche sélectionnée.
L'action **Rouvrir cette tâche** existe aussi dans le menu des tâches. Elle retire
la validation courante après confirmation et ne lance aucun fournisseur.

### Terminer la revue entièrement dans la console

Depuis les actions de la tâche :
1. **l** : lire le rapport existant ; Échap revient au tableau.
2. **s** : sélectionner le rapport détecté, puis soumettre.
3. **g** : sélectionner un fichier `TASK.evidence.json` détecté sous `docs/`.
   Entrée passe au bouton Examiner ; Entrée affiche le verdict prévisionnel.
   Confirmer enregistre la gate et revérifie les preuves et la révision.
4. Depuis les actions, **v** expose les preuves, puis **a** ouvre l'acceptation.

La console ne fabrique ni rapport ni document de preuve. Ils sont produits et
revus avant leur chargement. Une gate refusée reste refusée ; la dérogation **f**
constitue une décision différente. Aucun identifiant d'agent ni commande API
n'est nécessaire pour ce parcours. **q** ferme directement le tableau ;
**Échap** ferme une fenêtre avant de quitter.

Les mutations structurées de tâches partagent `executeRequest` entre CLI et
terminal ; verrous d'agent actif et conflits sont vérifiés dans la transaction.
Les réponses JSON conservent `error` et ajoutent `failure.code/message/retryable`.
Les mutations versionnées utilisent event_id ; l'ancien champ texte de
`control` reste une interface opérateur sans clé d'idempotence fournie par le client.

Toute migration d'une base existante version 1 vers 2 crée d'abord une sauvegarde
SQLite cohérente `.swarm/state-pre-v2-*.db`, permissions 0600.

## Cockpit terminal et web du 12 septembre 2026

Depuis `flaskProject`, `../wattson.sh swarm` ouvre le sélecteur de travaux au
clavier. `../wattson.sh swarm web` affiche un lien de session à ouvrir dans le
navigateur local. Le serveur initialise les fournisseurs détectés si le fichier
n'existe pas ; une configuration existante n'est jamais remplacée.
Le serveur écoute uniquement sur loopback, port éphémère par défaut. Le lien de
session est nécessaire ; cookie HttpOnly/SameSite, contrôle Host/Origin et jeton
CSRF protègent les commandes. API locale versionnée sous `/api/v1`. Les événements
SSE rejouent les révisions métier après le curseur ; les signaux et journaux ont
leur propre actualisation. Fermer un client ou le serveur web ne coupe pas les agents.

**Parcours utilisateur** : choisir une tâche → Piloter → fournisseur, rôle,
espace existant et consigne → Confirmer. Annuler ne lance rien. Une erreur reste
visible dans la modale. Après la fin : lire le rapport → soumettre → examiner la
gate → confirmer son enregistrement → accepter après revue. Une dérogation exige
un motif d'au moins dix caractères ; elle reste `waived`, sans fabriquer de PASS.

Dans les actions du terminal : `h` hiérarchie, `i` décisions, `j` recherche de
journaux, `u` reprise, `e` OODA, `p` attribution, `b` budget. La web propose ces vues
dans sa navigation. L'acquittement d'une décision conserve auteur/date/résolution,
sans accepter la tâche ni arrêter un processus. Une nouvelle période de silence
après reprise du signal génère une nouvelle décision.

La reprise conserve une dernière visite par hôte/UID, les changements, blocages,
résultat et prochaine action. `export/import` restaure décisions et visites sans
réactiver de processus. Les paramètres fournisseurs et budgétaires restent locaux.
La recherche de journaux est littérale, paginée par curseur et bornée ; le début
non conservé est signalé. Pause/direct concerne l'affichage. L'export terminal
contient au maximum 200 lignes correspondantes ; la web exporte la page affichée.

**Budget** : plafond USD et estimation USD par départ, source et date explicites.
Réservation atomique avec l'intention de départ ; refus si le prochain départ
excède le plafond. Alerte à 80 %. Les agents actifs continuent. Zéro désactive le
contrôle. Une tentative ayant démarré impute l'estimation à sa fin ; les jetons
fournisseur sont conservés séparément avec la portée « dernier événement », jamais
présentés comme un cumul garanti. Coût facturé indisponible ; aucun calcul durée/coût.

**Retour arrière** : préserver le binaire et une sauvegarde SQLite compatibles.
Le schéma courant est 3. Avant migration 2→3, `state-pre-v3-*.db` est créé par
`VACUUM INTO`, en 0600. Suspendre les départs, confirmer la fin des agents et fermer
les clients avant restauration. Conserver aussi une sauvegarde cohérente de l'état
le plus récent pour réconcilier ses écritures. Vérifier l'ancien couple binaire/base
dans un dossier séparé ; ne pas écraser silencieusement les écritures postérieures.

## Ce qui n'interrompt pas

Le produit tient une promesse simple : n'interrompre que pour ce qui demande un
arbitrage maintenant. Trois cas ont été retirés de l'écran « À traiter » après
mesure sur un travail réel, où 86 demandes s'affichaient pour 16 tâches toutes
acceptées, avec un seul et même texte.

**Une demande déjà close ne revient pas.** La demande courante d'une tâche
bloquée est rouverte à l'affichage ; les versions précédentes du même sujet ne
le sont pas. Sans cette distinction, des décisions closes en base — dont
certaines acquittées par un opérateur — réapparaissaient à chaque lecture, et
l'écran défaisait en silence des décisions prises.

**Un sujet ne donne qu'une carte.** Une évaluation plus récente remplace la
précédente, close avec son motif plutôt qu'empilée.

**Une acceptation dont les preuves ont dérivé n'est pas une demande.** Les gates
signent des fichiers source partagés : travailler sur le produit périme d'un
coup toutes les acceptations passées. La dérive reste visible sur la tâche et
dans le graphe ; elle ne réclame un arbitrage que si une tâche non terminée en
dépend, parce que la faire avancer supposerait une preuve qui n'est plus
vérifiée. Une tâche rouverte fait exception : son garde-fou de revalidation
reste dans les demandes.

Toute fermeture porte son motif. Aucune carte ne disparaît sans raison lisible.

### Synthèse des conclusions

Depuis le pilotage, ouvrir un rapport affiche automatiquement une analyse IA
en deux lignes : le constat et la suite utile. Le fournisseur est visible et
modifiable. Une réponse est réutilisée si le rapport, le contexte et le
fournisseur sont identiques. Le contenu est relu côté moteur et limité à un
extrait de 12 000 octets, avec indication lorsque le rapport est tronqué.
Une analyse ne valide pas la tâche ; le texte du rapport reste consultable
en cas de panne de l’IA.

### Résoudre une tentative bloquée

Le panneau « Agir sur cette tâche » est placé sous la mission.

| État observé | Action visible | Résultat |
| --- | --- | --- |
| Démarrage en attente, non pris en charge | Annuler ce démarrage en attente | Intention annulée, réservation libérée, tâche à reprendre |
| Tentative active, arrêt non demandé | Demander l’arrêt de la tentative | Demande envoyée, attente de confirmation |
| Arrêt demandé sur cette session | Vérifier la fin et libérer la tâche | Libération seulement après contrôle des processus |
| Exécution prise en charge dans une autre session | Actualiser l’état et explication de la vérification à l’origine | Pas de libération sans preuve de fin |
| Tentative arrêtée, conditions remplies | Relancer cette tâche | Formulaire prérempli, lancement après décision explicite |

L’annulation d’une intention queued se dispute atomiquement la ligne avec le
superviseur qui veut passer à starting. Si le superviseur gagne, l’annulation
est refusée. Une autre session peut annuler une intention jamais prise en
charge ; elle ne peut pas déclarer terminé un processus étranger démarré.
La tâche revient bloquée pour une reprise explicite, jamais validée par l’arrêt.

Recette : `node tests/pilot_recovery_ui.cjs BINAIRE SORTIE` utilise une base
isolée, confirme réellement l’annulation, contrôle la réservation en base,
recharge l’écran et ouvre la relance. Aucun fournisseur n’est lancé.

Compatibilité des anciens démarrages : un verrou starting dont le corps est
encore queued, sans superviseur ni PID ni heartbeat, peut être annulé avec
une demande d’arrêt déjà persistée. Un démarrage nouvellement pris en charge
enregistre maintenant ses métadonnées et son statut en une seule opération.
Toute annulation humaine porte stop_kind=operateur : le mode autonome ne doit
pas la relancer. La recette de résolution couvre désormais ce mode autonome.

## Préparation documentaire — candidat du 15 septembre 2026

Le candidat `/tmp/swarm-prephase-web` ajoute « Préparer un projet » au cockpit.
La page `/prepare.html` partage les documents avec `prepare` côté CLI : besoin,
brief, plan JSON, versions, sauvegarde et résolution de conflits. Monaco est
embarqué localement et dispose d’un éditeur texte de secours. La méthode APEX,
KS ou audit-PDCA sélectionnée référence les fichiers du projet ; son dialogue IA
reste à raccorder. Aucun agent n’est lancé depuis cette page.

Recette de l’éditeur et non-régression du pilotage :
[suivi de réalisation](../../docs/plans/swarm-prephase-apex/IMPLEMENTATION.md).
Le serveur installé n’est pas remplacé par ce candidat (schéma SQLite v7).

### Dialogue de préparation — candidat suivant

`/tmp/swarm-prephase-dialogue` (SQLite v8) ajoute une conversation avec Claude ou
Codex à côté de Monaco : réponse persistante, proposition de brief à comparer,
utilisation explicite puis adoption séparée. Les appels sont sans outils, limités
à 120 secondes et 20 échanges par préparation. Arrêter affiche la confirmation
moteur ; les réponses anciennes ne remplacent pas les documents courants.
Le CLI expose `prepare providers|dialogue|send|stop|use-proposal` en JSON.
Le mode interactif CLI, la génération du plan et sa conversion restent à réaliser.
Voir [ADR-002](../../docs/plans/swarm-prephase-apex/ADR-002.md).

### Plans IA et dialogue terminal — candidat suivant

`/tmp/swarm-prephase-plan` propose le plan à partir d’un brief adopté. Le moteur
contrôle structure et dépendances ; comparer, enregistrer et vérifier restent
séparés. Les questions ouvertes bloquent la vérification jusqu’à leur résolution.
`prepare` en terminal ouvre le dialogue ; `prepare chat ID` / `prepare resume ID`
reprennent une préparation. `/aide` liste les commandes, `/plan` appelle l’IA,
`/voir plan` consulte sans appel, `/edit plan` utilise VISUAL/EDITOR avec protection
contre l’écrasement d’une version concurrente. Modes JSON et --plain disponibles.
Aucune mission n’est créée par ce lot. Voir
[ADR-003](../../docs/plans/swarm-prephase-apex/ADR-003.md).

## Session interactive dans le graphe

Au lancement manuel d’une tâche, choisir **Mode de session → Interactif — terminal
natif**. Le clic sur sa case ouvre le terminal en modale ; **Prendre la saisie**
donne le clavier à cette vue. Les sorties ANSI restent colorées. **Détails et
validation** ramène aux rapports, preuves et commandes métier.

**Fermer la vue** ou Échap ne coupe pas le fournisseur. **Arrêter l’agent…** puis
**Confirmer l’arrêt** demande une interruption au superviseur. Un arrêt demandé
reste distinct d’un arrêt confirmé. La fin du processus ne valide jamais la tâche.
Les anciennes tentatives automatisées proposent leurs journaux en lecture seule :
on ne transforme pas une entrée déjà fermée en session interactive.

La même session est accessible dans le terminal local :

```sh
swarm --root /chemin/projet agent attach ID_AGENT
```

`Ctrl+]` détache cette vue ; `Ctrl+C` est transmis au fournisseur. Le lancement
JSON `agent start` accepte `"mode": "terminal"`. Le mode automatisé reste le défaut.
Une seule vue web ou CLI peut écrire à la fois ; le bail expire après 20 secondes
sans renouvellement. Une réponse réseau perdue suspend la saisie : vérifier
l’écran, puis reprendre explicitement. Aucune saisie incertaine n’est rejouée.

Les commandes standard de `providers init` sont reconnues pour Codex et Claude.
Pour une commande personnalisée, ajouter `interactive_args` dans le fournisseur :
arguments natifs, sans le prompt, qui est ajouté comme dernier argument. Les
permissions et le sandbox restent ceux de la commande ; aucun drapeau de
contournement n’est ajouté. Un fournisseur personnalisé doit donc accepter un
prompt positionnel final. Le choix de modèle continue de passer par la politique.

### Limites du mode natif

- Durée totale par défaut : 1 800 secondes ; configurable via `timeout_seconds`.
- Historique de sortie persistant : 4 Mio par tentative, puis arrêt supervisé.
  Le terminal conserve ses couleurs et ses dimensions de rendu ; aucun rejeu des
  commandes n’est nécessaire pour le rouvrir. La capture est obligatoire dans ce mode.
- Appels d’outils, temps par outil, répétitions, progression métier et coût ne
  sont **pas mesurés**. Les plafonds de plan et les limites explicites de
  fournisseur/mission incompatibles font refuser le lancement natif. Aucun
  assouplissement implicite des contrats existants. La réservation de budget
  et la durée maximale restent contrôlées ; le coût n’est pas inventé.
- Réouverture après fermeture du navigateur ou redémarrage du serveur web : même
  superviseur, même tentative. Après fin du processus : consultation seule ; une
  relance explicite crée une nouvelle tentative. Un redémarrage de la machine ou
  une perte du superviseur ne restaure pas automatiquement le processus natif.
- Le profil d’ordonnancement automatique n’est pas remplacé par un lancement natif.

Stockage v10, sauvegarde SQLite avant migration d’une base existante. Le terminal
utilise un socket Unix local contrôlé par UID et identité de superviseur. Son cadre
web possède la seule exception de styles nécessaire à xterm ; le cockpit conserve
sa CSP précédente. Bibliothèques embarquées, aucune dépendance CDN à l’exécution.

## Extraits et budget de préparation (stockage v11)

Dans **Préparer avec l’IA**, **Joindre des fichiers** recherche les chemins du
projet, affiche une plage de lignes et demande un ajout explicite. Huit extraits
au maximum, 12 000 octets par extrait et 24 000 au total ; seuls les fichiers
texte réguliers de 256 Kio au maximum sont consultables. Liens symboliques,
chemins exclus, binaires et secrets reconnaissables sont refusés. Le détecteur
est heuristique : relire le contenu avant de le transmettre. L’IA reçoit
l’instantané enregistré avec son empreinte, sans accès autonome au dépôt.
Changer ces sources demande de relire et réadopter le brief.

**Budget et consommation** configure une enveloppe estimative et une réserve
par appel en USD, avec source et date de référence. Un appel réserve atomiquement
les budgets de la préparation et, si elle est liée à un travail plafonné, du
travail. Une annulation avant appel libère la réserve. Dès qu’un appel peut avoir
démarré, son estimation reste engagée, même si la mesure du fournisseur manque.
Les coûts et jetons rapportés affichent leur couverture ; l’absence de mesure
n’est pas un coût nul. Les appels historiques ne sont pas réévalués après coup.

Équivalents dans `prepare chat` : `/fichiers [recherche]`, `/fichiers-dans DOSSIER`, `/lire CHEMIN DEBUT FIN`,
`/joindre CHEMIN DEBUT FIN`, `/retirer CHEMIN`, `/sources`, `/budget` et
`/budget LIMITE RESERVE AAAA-MM-JJ SOURCE`. Les ajouts et changements demandent
`/confirmer`. Les commandes JSON `prepare files`, `source`, `source-add`,
`source-remove` et `budget` utilisent les mêmes contrats de révision et de reçu.

## Dialogue avec contrôles d’outils

Le choix **Dialogue — contrôles d’outils** (`mode: "dialogue"`) permet des échanges
successifs dans la modale du graphe ou avec `agent attach ID`. Il accepte les
plafonds de mission et conserve les compteurs entre échanges. Chaque lancement
est autorisé par le superviseur ; les messages déjà saisis ne contournent pas un
plafond atteint. Les identifiants de session reçus du fournisseur sont contrôlés
à chaque échange ; aucune reprise de « dernière session » implicite.

Le dialogue utilise les commandes standard Claude/Codex de `providers init` et
la politique de modèle habituelle. Les permissions restent celles du fournisseur.
Un outil soumis à une autorisation native peut être refusé dans ce protocole non
interactif ; le dialogue n’accorde aucune permission supplémentaire.

Les limites réagissent aux événements observables, comme le mode automatisé :
elles ne garantissent pas de bloquer un effet avant sa réalisation. Une activité
non prise en charge arrête le dialogue. Les identifiants d’outils réutilisés par
le fournisseur sont distingués par échange. L’attente de votre réponse n’est pas
un silence fournisseur ; la durée totale de tentative continue de courir.

`/fin` termine la session, 20 échanges au maximum. Une réponse ne valide pas la
tâche et ne libère pas la tentative. Une perte de supervision impose une revue
et une nouvelle tentative explicite. Le terminal natif reste disponible avec
ses limites de mesure précédemment décrites.

### Repli local

`SWARM_PREPARATION_DISABLED=1 swarm --root PROJET web 127.0.0.1:18787` désactive
l’écran et les nouvelles mutations/appels de préparation. Le graphe, la lecture,
l’export CLI et l’arrêt des appels actifs restent disponibles. Retirer la variable
réactive les documents conservés. Ce repli ne rétrograde pas la base.

Avant migration, arrêter tous les anciens écrivains de cette racine (serveur,
superviseurs, assistants). Les sauvegardes `state-pre-vN-*.db` sont créées avant
chaque migration. Un ancien binaire refuse v11 : pour un retour complet, conserver
la base courante séparément et restaurer la sauvegarde compatible dans une racine
de reprise, puis vérifier son contenu avec l’ancien binaire. Ne jamais remettre
un ancien binaire sur la base migrée.

La recherche de fichiers est bornée à 20 000 entrées parcourues. Si elle annonce
une limite, choisir un dossier plus précis avec les boutons « Dossier » ou le
champ Dossier, par exemple `tools/swarm-companion`. Le CLI accepte
`prepare files ID [recherche] [dossier]`. Chaque source jointe permet de lire
l’instantané effectivement transmis, indépendamment du fichier courant.

### Retrouver le même travail

Le lien « Lien permanent vers ce travail », en haut du pilotage, ouvre directement
le travail sélectionné. La clé locale reste identique après redémarrage sur la
même adresse et le même port. Elle est conservée dans un fichier privé
`.swarm/web-session-*` (permissions 0600). Pour révoquer les anciens liens,
arrêter le serveur, supprimer sa clé puis le relancer. Un port choisi
aléatoirement change l’adresse ; utiliser un port fixe pour conserver un favori.

Le résumé du pilotage présente les validations, l’activité et les décisions
attendues. Son bouton principal ouvre la décision la plus bloquante, le suivi,
la reprise ou les résultats selon l’état réel. Les dérogations ne sont pas
comptées comme des validations.


### Lire les rôles et les éléments manquants

Les cartes du graphe et de la liste affichent le rôle déclaré : ◇ Planificateur,
⑂ Responsable de branche, ⚙ Exécutant, ✓ Vérificateur lorsqu’un tel rôle est
explicitement fourni. Un rôle absent apparaît comme « Rôle à préciser » ; le
titre d’une tâche de revue ne suffit pas à lui attribuer un rôle.
Le badge de rôle possède sa propre paire de couleurs. Le contour de la carte
exprime l’état de la tâche ; une sélection utilise un contour discontinu.

La ligne opérationnelle distingue « À compléter » (livrable, critère ou consigne),
« En attente » (motif du moteur), « À résoudre » (blocage) et résultat validé.
Les textes longs restent accessibles dans le titre SVG et en ouvrant la carte.
Les flèches conservent le verdict du moteur, y compris la fraîcheur des preuves.
L’aide du pilotage explique cette lecture, sans dépendre de la couleur.

CLI : `swarm help pilotage`, `swarm mission status TRAVAIL` et
`swarm console TRAVAIL` exposent l’aide, les motifs et les actions existantes.
Cette amélioration ne fournit pas encore d’éditeur de dépendances avec simulation.


### Rectification : vérificateur indisponible

Le badge Vérificateur décrit une possibilité du rendu, mais le lancement ne
reconnaît pas `reviewer`. Il ne constitue donc pas une fonction livrée. Les
planificateurs du protocole hiérarchique sont des périmètres distincts du graphe
des tâches ; ce protocole doit être activé explicitement. Consulter
[le contrat détaillé](../../docs/architecture/swarm/ROLES-RESPONSABILITES-ET-LIMITES.md).


### Organisation obligatoire avant les départs autonomes

Une mission sans responsable et sans politique de validation explicite ne peut
plus être autorisée en autonome. Le web et `mission status` exposent les manques.
Les recettes isolées et l’installation ne valident pas une mission utilisateur.
Voir le [contrat et la recette visible](../../docs/architecture/swarm/GARDE-ORGANISATION.md).

## Préparation, révision et modèles IA

Voir [le parcours web et les contrats CLI](PREPARATION-UX.md) : formulaire de missions, responsable durable, validation explicite, révision avec historique et menu **IA et connexions**.
