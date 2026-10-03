# T1 — Résumé avant lancement : candidat corrigé et preuves

Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T1 ; base 1d9570bd4ef617130c6be96b7ec88844fdbcd00e, candidat non commité. Correctif écrit par le superviseur Codex, distinct du worker de la troisième tentative. Les premières revues et diagnostics restent archivés. Aucun succès autonome ni acceptation ne sont revendiqués par ce document.

## Critères et preuves

1. Le résumé avant lancement affiche objectif, rôle, fournisseur, niveau demandé, modèle résolu, observation inconnue, profil, skills, périmètre, budget, reprises et validations dans le web et le CLI. L’objectif vient de Work.Objective ; aucune valeur réelle n’est inférée du modèle configuré. Le périmètre complet est visible dans launch-v2-en-scope.png et dans la sortie CLI ci-dessous. Les autres champs sont visibles dans les quatre captures FR/EN × clair/sombre.
2. Programme legacy : modèle résolu « Unknown » ; modèle rapporté « Unknown before execution ». Sans profil ou skills sélectionnés, absence explicite. Les tests utilisent des fournisseurs simulés pour les contrats ; le navigateur réel utilise un serveur isolé sans lancer un fournisseur.
3. Recette du superviseur dans le navigateur réel : FR/EN, etat/sombre, auto ≠ sonnet ; Échap ferme la modale et rend le focus visible à mission-primary. launch-v2-focus.png montre le bouton Start mission avec son contour de focus, sans modale. launch-v2-focus.json est le relevé DOM lu après la touche Échap : dialogOpen=false, activeElement=mission-primary, focusVisible=true. Ce relevé n’est pas une ré-exécution du navigateur par le vérificateur ni par le contrôle Python ; celui-ci vérifie uniquement le relevé enregistré. Le worker n’a pas revendiqué cette recette, réalisée ensuite par le superviseur.

## Contrôles et attribution

Les contrôles Go et npm sont exécutés et reçus par controller://swarm-validation sous politique human ; leurs preuves sont transmises au vérificateur avec empreintes des sources et captures. La décision qualitative reste humaine. Le troisième worker a utilisé 8 outils, vérifié l’ancien candidat à six empreintes et exécuté Go/npm ; il ne revendique pas les correctifs ultérieurs du superviseur. Contrôles ciblés après ajout de l’objectif : PASS ; npm test : PASS ; go vet : PASS. Suite Go sur le correctif de reprise : PASS 420,734 s ; suite complète sur le candidat final en cours, aucune réussite non terminée revendiquée. Console navigateur capturée sans erreurs/avertissements ; pas de collecteur réseau exhaustif.

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
    "mission_status.go": "ac4c0bb0e446e62a3c6d0d3c85839becc9d12cfd515f3edd4160267b77bfc7ce",
    "mission_cli.go": "1ba1092aecd5d01d926784fd0f166d0105e703364e401e3bea446136b67bccd4",
    "web/mission.js": "f256b53de558ebefcdeba318bf7a7fb980fbdb5d368df62d05727ea2c86b7e57",
    "locales/en.json": "405d74e7e5b462b79586ed0ee892926fad4c37f1daee7a40adfdb7d29c41dbaa",
    "web/i18n-en.js": "d2e95248b7e93146ebb99e9ecaf1a2698050f02295a1aee733547b820ed7a3cb",
    "mission_launch_identity_test.go": "341d3a7c56d7d978a25bbec47d00f372bf9a14e530a6a9391173d2cd1ed737b8"
  },
  "dirty": true
}

```
