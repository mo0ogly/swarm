# T1 — Résumé avant lancement : candidat corrigé et preuves

Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T1 ; base 1d9570bd4ef617130c6be96b7ec88844fdbcd00e, candidat non commité. Correctif écrit par le superviseur Codex, distinct du worker de la troisième tentative. Les premières revues et diagnostics restent archivés. Aucun succès autonome ni acceptation ne sont revendiqués par ce document.

## Critères et preuves

1. Le résumé avant lancement affiche objectif, rôle, fournisseur, niveau demandé, modèle résolu, observation inconnue, profil, skills, périmètre, budget, reprises et validations dans le web et le CLI. L’objectif vient de Work.Objective ; aucune valeur réelle n’est inférée du modèle configuré. Le périmètre complet est visible dans launch-v2-en-scope.png et dans la sortie CLI ci-dessous. Les autres champs sont visibles dans les quatre captures FR/EN × clair/sombre.
2. Programme legacy : modèle résolu « Unknown » ; modèle rapporté « Unknown before execution ». Sans profil ou skills sélectionnés, absence explicite. Les tests utilisent des fournisseurs simulés pour les contrats ; le navigateur réel utilise un serveur isolé sans lancer un fournisseur.
3. Recette du superviseur dans le navigateur réel : FR/EN, etat/sombre, auto ≠ sonnet ; Échap ferme la modale et rend le focus visible à mission-primary. launch-v2-focus.png montre le bouton Start mission avec son contour de focus, sans modale. launch-v2-focus.json est le relevé DOM lu après la touche Échap : dialogOpen=false, activeElement=mission-primary, focusVisible=true. Ce relevé n’est pas une ré-exécution du navigateur par le vérificateur ni par le contrôle Python ; celui-ci vérifie uniquement le relevé enregistré. Le worker n’a pas revendiqué cette recette, réalisée ensuite par le superviseur.

## Contrôles et attribution

Les contrôles Go et npm sont exécutés et reçus par controller://swarm-validation sous politique human ; leurs preuves sont transmises au vérificateur avec empreintes des sources et captures. La décision qualitative reste humaine. Le troisième worker a utilisé 8 outils, vérifié l’ancien candidat à six empreintes et exécuté Go/npm ; il ne revendique pas les correctifs ultérieurs du superviseur. Contrôles ciblés après ajout de l’objectif : PASS ; npm test : PASS ; go vet : PASS. Suite Go sur le correctif de reprise : PASS 420,734 s ; suite complète du bandeau : PASS 442,451 s ; suite complète de revalidation : PASS 436,151 s, code 0. Console navigateur capturée sans erreurs/avertissements ; pas de collecteur réseau exhaustif.

## Sortie CLI réellement obtenue sur la racine isolée
```text
0 possible start(s) now · actual concurrency 0/2
Announced concurrency is the first wave calculated now; slots, dependencies, budget and available workspaces may limit it.
- Objective : Comprendre les agents avant de lancer
- Role : Worker
- Provider : claude
- Requested level : Automatic
- Model resolved by configuration : sonnet
- Model reported by provider : Unknown before execution
- Project profile : No project profile
- Selected skills : No selected skills
Authorization reviewed once:
- Scope: Racine isolée de recette · 0 task(s) · directory /tmp/swarm-qw1-visual-joy403z4
- Budget: No financial limit configured; tool and attempt limits for each task remain enforced.
- Retries: At most 2 automatic attempts per retry chain, including the first start; an unchanged environment awaits a new check and task-specific limits take precedence.
- Validations: 0 task(s) under human review · 0 task(s) with 0 preauthorized automatic check(s).
- Limit: At most 2 simultaneous start(s), depending on actually available workspaces.
- Limit: A workspace is used by only one agent at a time.
- Limit: After 2 unsuccessful automatic attempts, a human decision is required.
- Limit: Dependencies, budgets, stop requests and validation remain blocking.

```

## Relevé DOM réel après Échap
```json
{
  "activeElement": "mission-primary",
  "buttonText": "Start mission",
  "dialogOpen": false,
  "focusOutline": "rgb(78, 165, 232) solid 2px",
  "focusVisible": true,
  "theme": "sombre",
  "url": "http://127.0.0.1:44631/?work=w-5005b909a2a3a9131f96fde6&lang=en"
}
```

## Extraits des champs ajoutés

### mission_status.go
```
type MissionLaunchIdentity struct {
	Objective      string   `json:"objective"`
	Role           string   `json:"role"`
	Provider       string   `json:"provider"`
	RequestedLevel string   `json:"requested_level"`
	ResolvedModel  string   `json:"resolved_model"`
	ActualModel    string   `json:"actual_model"`
	ProjectProfile string   `json:"project_profile"`
	Skills         []string `json:"skills"`
}

```

### mission_cli.go
```
for _, row := range [][2]string{
		{uiText("Objectif"), value(identity.Objective)},
		{uiText("Rôle"), role}, {uiText("Fournisseur"), value(identity.Provider)},
		{uiText("Niveau demandé"), value(level)}, {uiText("Modèle résolu par la configuration"), value(identity.ResolvedModel)},
		{uiText("Modèle rapporté par le fournisseur"), uiText("Inconnu avant l’exécution")},
		{uiText("Profil du projet"), profile}, {uiText("Skills sélectionnés"), skills},
	} {
		
```

### web/mission.js
```
 launchIdentityView(identity={}){
  const list=node('dl',undefined,'mission-launch-contract');
  list.dataset.launchIdentity='true';
  const role={worker:tr_web_mission_js('Exécutant'),planner:tr_web_mission_js('Responsable'),subplanner:tr_web_mission_js('Sous-planificateur')}[identity.role]||identity.role;
  const rows=[
   [tr_web_mission_js('Objectif'),identity.objective],
   [tr_web_mission_js('Rôle'),role],
   [tr_web_mission_js('Fournisseur'),identity.provider],
   [tr_web_mission_js('Niveau demandé'),identity.requested_level==='auto'?tr_web_mission_js('Automatique'):identity.requested_level],
   [tr_web_mission_js('Modèle résolu par la configuration'),identity.resolved_model],
   [tr_web_mission_js('Modèle rapporté par le fournisseur'),tr_web_mission_js('Inconnu avant l’exécution')],
   [tr_web_mission_js('Profil du projet'),identity.project_profile||tr_web_mission_js('Aucun profil de projet')],
   [tr_web_mission_js('Skills sélectionnés'),identity.skills?.join(', ')||tr_web_mission_js('Aucun skill sélectionné')]
  ];
  for(const [label,value]of rows){const row=node('div');row.append(node('dt',label),node('dd',value||tr_web_mission_js('Inconnu')));list.append(row)}
  return list;
 },

```

Transport moteur : preview.Identity.Objective = w.Objective ; identity.Objective = w.Objective pour chaque départ. Tests : TestMissionLaunchPreviewIdentityUsesTaskOverrides vérifie les deux objectifs et la révision immuable ; tests/mission_overview_test.cjs vérifie libellé Objectif et valeur rendue dans le DOM simulé ; TestMissionLaunchIdentitySeparatesResolutionFromObservation vérifie inconnus, modèle résolu, profils et skills. Ces tests ne remplacent pas la recette manuelle en navigateur réel.

## Empreintes du candidat T1
```json
{
  "base": "1d9570bd4ef617130c6be96b7ec88844fdbcd00e",
  "files_sha256": {
    "mission_status.go": "c9bcbb57a9bb385cffb692b3a51e66a2bce88b51bb82cf79d511dbafc5e92596",
    "mission_cli.go": "1ba1092aecd5d01d926784fd0f166d0105e703364e401e3bea446136b67bccd4",
    "web/mission.js": "9157125da2f2d5683c87788513f869b7d27835acfaa5dd7970a7dab9365e9301",
    "locales/en.json": "2d6c6fc139588b10e7d7350f3457bd9d95e441c69950778e5fe3ac65dc87ecf9",
    "web/i18n-en.js": "8e2cb1c57b67383d30e5714ce6cab9e896edbf5b9f52c436cc1520a37f30e938",
    "mission_launch_identity_test.go": "341d3a7c56d7d978a25bbec47d00f372bf9a14e530a6a9391173d2cd1ed737b8",
    "agents_store.go": "7c8553b2057dc4978ca0e074d3a13b5647e1b3c9bf4fce2c33123d4c33ab28ca",
    "agents_process.go": "bc3656fc705f3e5ff6523c0460bcb0c5f08a037c9fbd53c5bd1b0def836aa9d3",
    "agent_terminal_process.go": "ef6cc4957c0ea65ddaf51ddacdbae6153526fe5da692851e4226f3bcf4abc1ab",
    "agent_reported_model.go": "762a7a6807ffd0fe61c73dc2dd98f69669250ec8a04e030b3a3401fe5d0c35fb",
    "agent_reported_model_test.go": "f5a8991ad91e7317bbd7c7742ce0c056f2d70c20693897fcc84b54621409eb4f",
    "web/task-models.js": "7af3c5745d8b066549629d4993c0e24b948fbbb185d7153b440a272e2f7ebdca",
    "web/pilot-inspector.js": "5395c6f5855ba77eb92a27993e75fa36a3a1b0da08a53fc494aa9bf8abd6c152",
    "tests/reported_model_test.cjs": "f4c6789628e0b391b350976fb07fd178a495c92d0c3cd83529f8f9ba40392976"
  },
  "dirty": true
}
```

## Revalidation après correctif du superviseur

La logique de priorité du bandeau mission a changé dans mission_status.go ; les champs du résumé pré-lancement restent identiques. Les catalogues ajoutent une traduction de blocage ; web/mission.js dirige une acceptation périmée vers la revalidation du même résultat. Tests ciblés des résumés, suite JavaScript et vet sont rejoués ; la suite complète du correctif de bandeau a passé 442,451 s. La suite complète du dernier complément de revalidation est terminée : PASS 436,151 s, code 0, journal /tmp/swarm-qw-revalidate-tests.log. Les six captures de pré-lancement restent celles de la recette documentée précédente ; aucune capture ancienne n’est présentée comme nouvellement réalisée. Revue indépendante et nouveaux reçus restent nécessaires. Même tentative producteur conservée, trois tentatives déjà dépensées, aucun départ T1 supplémentaire.

### Recette anglaise claire rejouée après le refus de revue

Le superviseur a redémarré la racine isolée avec le binaire courant, sélectionné le thème clair, ouvert l’aperçu et choisi legacy sans lancer de mission. La capture launch-v2-en-light-unknown.png a été refaite, avec Unknown et Unknown before execution visibles. L’ancienne capture est conservée sous *-before-recapture.png. Relevé réel DOM :

```json
{
  "bodyBackground": "rgb(246, 246, 246)",
  "dialogBackground": "rgb(255, 255, 255)",
  "language": "en",
  "theme": "etat"
}
```

Échap a fermé la modale et rendu le focus à mission-primary ; focusVisible=true observé. Cette observation nouvelle est réalisée par le superviseur, jamais attribuée au worker. La prochaine revue indépendante examine la capture renouvelée ; le refus précédent reste dans l’historique.

## Complément superviseur — recette réellement rejouée le 3 octobre

Le rapport initial docs/plan-115c11e8f8-T1.md décrit les limites du worker à 08:58. Il ne décrit pas la recette suivante, réalisée après sa fin. Le contrôle Python `browser not replayed` décrit uniquement ce contrôle Python, et non la session navigateur réelle du superviseur. Les limites de ces deux opérations restent conservées ; elles ne doivent pas être attribuées à cette nouvelle recette.

Interaction réelle CUA : ouverture avec Entrée du bouton de lancement, sélection de Claude avec Home/ArrowDown/Enter/Tab, attente du résumé visible, lecture du DOM rendu, assertion de la langue/thème/champs, puis Échap et assertion que la modale est fermée et que mission-primary récupère le focus visible. Aucun fetch/API ni injection de données dans le navigateur. Racine isolée, mission jamais démarrée ; aucune consommation d’agent pour cette recette.

Résultats des quatre exécutions :

| Langue | Thème | Résumé présent | Échap ferme | Focus rendu et visible |
|---|---|---|---|---|
| EN | etat | PASS | PASS | PASS |
| EN | sombre | PASS | PASS | PASS |
| FR | sombre | PASS | PASS | PASS |
| FR | etat | PASS | PASS | PASS |

Dans chaque résumé, rôle worker/Worker, fournisseur claude, demande auto/Automatic, configuration sonnet, modèle rapporté inconnu avant exécution, profil et skills non sélectionnés, périmètre et limites sont rendus. Les quatre relevés DOM complets sont dans launch-v4-browser-replay.json. Trois nouvelles captures launch-v4-* et la capture anglaise claire legacy launch-v2-en-light-unknown.png ont été inspectées visuellement : fonds distincts, fermeture lisible et focus contrasté dans les deux thèmes.

Distinction exacte : auto ≠ sonnet vérifie la différence entre niveau demandé et résolution de configuration. Cela ne prouve pas qu’un fournisseur a exécuté un modèle différent de celui demandé. Avant lancement, la valeur réelle est intentionnellement inconnue : on ne transforme pas sonnet configuré en modèle observé. Une exécution réelle avec modèle fournisseur divergent n’a pas été provoquée ; cette limite est explicitement conservée. La recette est exécutée par le superviseur, la revue IA reste une autre session sans outils.

Relevés des actions exécutées et résultats :
```json
[
  {
    "language": "en",
    "theme": "etat",
    "visible": true,
    "afterEscape": {
      "dialogOpen": false,
      "focus": "mission-primary",
      "focusVisible": true
    }
  },
  {
    "language": "en",
    "theme": "sombre",
    "visible": true,
    "afterEscape": {
      "dialogOpen": false,
      "focus": "mission-primary",
      "focusVisible": true
    }
  },
  {
    "language": "fr",
    "theme": "sombre",
    "visible": true,
    "afterEscape": {
      "dialogOpen": false,
      "focus": "mission-primary",
      "focusVisible": true
    }
  },
  {
    "language": "fr",
    "theme": "etat",
    "visible": true,
    "afterEscape": {
      "dialogOpen": false,
      "focus": "mission-primary",
      "focusVisible": true
    }
  }
]
```

## Correctif de la lacune modèle demandé / modèle rapporté

Suite au refus 10, ajout moteur `Agent.reported_model` : modèle, source d’événement fournisseur, date d’observation. Seuls system/init.model et assistant/message.model structurés sont retenus ; aucun modèle n’est inféré des réglages ni du texte libre. Valeurs absentes/invalides restent inconnues. La route demandée demeure distincte et inchangée ; la déclaration n’est pas une preuve cryptographique d’exécution du modèle. Avant lancement, le résumé conserve donc « inconnu avant l’exécution ».

Test de protocole exécuté réellement : une nouvelle racine temporaire reçoit work create, task add et agent start via CLI public. Un processus fournisseur isolé (pas Claude réel, pas de LLM ni de tokens facturés) déclare system/init.model=fixture-reported-model, tandis que la politique résout la demande standard en sonnet. Le processus termine code 0 ; agent show retourne les deux champs distincts persistés. Cela reproduit une divergence contrôlée de modèle demandé / modèle rapporté, sans prétendre connaître l’implémentation physique d’un service Claude.

Preuve CLI exécutée :
```json
{
  "id": "da0b9711-8efa-48fc-b0b8-54e80bc82dc5",
  "work_id": "w-eae97600d116e687fb000f42",
  "task_id": "model-proof",
  "attempt_id": "a-19604d2dcaecfa574372061b",
  "status": "completed",
  "model_route": {
    "level": "standard",
    "model": "sonnet",
    "effort": "",
    "billing": "external",
    "policy_hash": "1cb0234143e9aa8ed95f3cd427d818ab0066d8c33139ce7f75a31f670f609b17",
    "reason": "Politique du fournisseur · work · sans montée en gamme automatique"
  },
  "reported_model": {
    "model": "fixture-reported-model",
    "source": "provider system/init",
    "at": "2026-10-03T14:18:30.087233546Z"
  },
  "provenance": "Public CLI agent show after a real child protocol fixture process; no real LLM call"
}
```

Recette navigateur réellement exécutée sur ce processus : ouverture du détail de sa carte, libellé français « Modèle demandé : claude · sonnet · Modèle rapporté : fixture-reported-model » ; anglais « Requested model: claude · sonnet · Reported model: fixture-reported-model ». Captures model-divergence-{fr,en}-{light,dark}.png, toutes inspectées visuellement, et relevés model-divergence-browser.json. Les quatre modes passent ; Échap ferme le détail et un focus visible est conservé. En EN/etat, le focus rendu est le rôle button SVG de la carte model-proof, avec son aria-label exact. Aucune donnée injectée dans le DOM ni appel API direct de navigateur.

Les tests Go vérifient le processus enfant, la persistance et la conservation de la demande ; ils rejettent une fausse déclaration dans un texte libre ou un événement thinking_tokens. Le test JavaScript vérifie FR/EN et l’inconnu en absence de déclaration. Go ciblé PASS 0,207 s ; npm PASS ; suite race ciblée PASS 4,395 s. La suite Go complète du nouveau correctif est terminée : PASS 451,168 s, code 0 ; go vet PASS, aucun diagnostic ; npm PASS, race ciblé PASS 4,395 s.

## Complément superviseur — focus réel après Échap, 3 octobre

Le nouvel avis a demandé une capture du focus dans le cas de divergence.
La recette a confirmé le retour au groupe SVG de la carte, mais révélé que sa
bordure d'alerte masquait le repère de focus : priorité corrigée dans
`web/graph.css`, uniquement pour le clavier, avec `--wattson-lien`.
La recette est rejouée sur le cas existant `w-eae97600d116e687fb000f42`,
racine `/tmp/swarm-reported-model-ui-89ndh7l7`, serveur construit depuis
le candidat courant, sans nouvel appel LLM : modèle demandé sonnet, modèle
déclaré fixture-reported-model, fournisseur de protocole explicitement isolé.
Les captures `model-focus-{fr,en}-{light,dark}.png` et relevé
`model-focus-browser.json`, sous docs/screenshots/clear-launch-recovery/,
montrent la fermeture puis le retour à la carte après Entrée/Échap.
L'ancien avis refusé et les dépenses restent conservés ; ce complément n'est
ni un quatrième producteur T1, ni une validation autonome. La revalidation
est nécessaire avant toute acceptation.
