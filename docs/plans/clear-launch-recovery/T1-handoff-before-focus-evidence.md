# T1 — correctif du superviseur, 3 octobre 2026

Le résumé avant lancement est maintenant implémenté dans le moteur, le CLI et le cockpit. Il sépare la configuration demandée/résolue de l’observation réelle, toujours inconnue avant le départ ; l’acceptation indépendante de T1 reste à réaliser.

## Identité

- Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T1.
- Auteur du correctif : superviseur Codex dans cette conversation ; aucune tentative Swarm fictive créée pour ces modifications.
- Base : 1d9570bd4ef617130c6be96b7ec88844fdbcd00e ; candidat non commité. Empreintes exactes : T1-candidate.json.
- Ancien rapport T1-handoff.md conservé comme diagnostic de la première tentative ; ses constats d’absence ne décrivent plus ce candidat.

## Changements

MissionLaunchPreview transporte une identité commune et une identité par départ. Chaque départ utilise son LaunchProfile effectif, avec priorité au profil propre de la tâche. Les identités font partie du payload complet déjà signé par verdict_token : un aperçu devenu différent nécessite une nouvelle confirmation. L’aperçu reste sans mutation de révision.

Les deux interfaces affichent objectif, rôle, fournisseur, niveau demandé, modèle résolu, modèle rapporté, profil du projet, skills sélectionnés, périmètre et limites. Le champ du modèle réel n’est jamais déduit du modèle configuré : « Inconnu avant l’exécution ». Un fournisseur personnalisé sans résolution connue affiche « Inconnu ». Aucun profil/skill absent n’est inventé.

## Matrice

| Critère T1 | Preuve du candidat | Résultat |
|---|---|---|
| req-4 : résumé complet avant confirmation, web et CLI | mission_status.go : missionLaunchIdentity ; mission_cli.go : printMissionLaunchIdentity ; web/mission.js : launchIdentityView. Tests Go : configuration codex standard → gpt-5.6-sol, observation vide, skills sélectionnés ; départ override distinct du fournisseur commun fixture et révision inchangée. | PASS sur les cas contrôlés |
| req-5 : donnée inconnue explicite | Test fournisseur legacy sans modèle, CLI « Inconnu » ; cockpit legacy « Unknown », observation « Unknown before execution ». Aucun modèle réel inféré. | PASS |
| req-6 : recette FR/EN, thèmes, clavier, demandé différent de résolu | Go standard distinct de gpt-5.6-sol ; navigateur natif : auto distinct de sonnet. Cockpit sur racine /tmp/swarm-qw1-visual-joy403z4, aucun agent lancé : FR/EN, etat/sombre, sélection au clavier, fermeture Échap restituant le focus au bouton de lancement. Captures launch-fr-dark.png, launch-en-dark.png, launch-en-light-unknown.png, launch-fr-light.png dans test-results/clear-launch-recovery/. | PASS sur ce périmètre ; profils non vides contrôlés par le moteur, pas une campagne fournisseurs réelle |

## Contrôles exécutés

- go test ./... -timeout 40m : exit 0, 476,065 s, premier candidat Go avant dernière correction des traductions CLI.
- go test ./... -run 'TestMissionLaunchIdentity|TestMissionLaunchPreview|TestMissionPreview|TestI18n' -count=1 -timeout 120s : exit 0, 1,579 s, après dernière correction CLI.
- npm test : exit 0, graphes, aperçu, rafraîchissement, audit et catalogue i18n.
- go vet ./... : exit 0.
- python3 tools/agent-workflows/check.py : exit 0.
- git diff --check : exit 0.
- Console capturée du navigateur de recette : zéro erreur/avertissement. Pas de capture réseau exhaustive ; aucun succès d’appel réel Claude/Codex revendiqué.

## APEX / PDCA

Analyser : manque confirmé du transport d’identité. Planifier : compléter le contrat existant sans nouveau parcours. Exécuter : moteur + rendus + catalogue. Vérifier : tests comportementaux et DOM réel. Ajuster : textes moteur anglais repérés puis corrigés pendant la recette.

## Suite

Faire examiner ce candidat et ses empreintes. Ne pas accepter les anciennes preuves FAIL ni créer une exécution fictive. L’intervention directe est distincte de l’autonomie observée ; T2–T5 ne sont pas terminés.

## Correction après la revue indépendante à cinq appels
Le réviseur a confirmé req-5 et req-6 sur les captures et les reçus du moteur, puis refusé req-4 : objectif absent. Correction réelle : champ objective alimenté depuis Work.Objective dans les identités commune et par départ, rendus CLI et web, test moteur et test de rendu DOM. Le texte de mission saisi par l’utilisateur reste tel quel ; son contenu n’est pas traduit automatiquement.

Recette reprise sur le serveur isolé : objectif « Comprendre les agents avant de lancer » effectivement visible dans FR/EN, etat/sombre. Captures v2 sous docs/screenshots/clear-launch-recovery. Échap rend le focus à mission-primary, constat DOM réel. Aucun fournisseur exécuté pendant la recette. Go ciblé, npm test et go vet PASS après cette correction ; nouvelle suite Go complète en cours. L’ancienne revue reste dans l’historique ; aucune acceptation forcée.

## Extraits du candidat remis

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
type MissionLaunchItem struct {
	ID        string                 `json:"id"`
	Title     string                 `json:"title"`
	Reason    string                 `json:"reason,omitempty"`
	Workspace string                 `json:"workspace,omitempty"`
	Identity  *MissionLaunchIdentity `json:"identity,omitempty"`
	Normal    bool                   `json:"normal,omitempty"`
}
type MissionLaunchPreview struct {
	Identity             MissionLaunchIdentity `json:"identity"`
	Organization         Organization          `json:"organization"`
	Token                string                `json:"verdict_token"`
	Revision             int                   `json:"revision"`
	RequestedSlots       int                   `json:"requested_slots"`
	Immediate            int                   `json:"immediate"`
	EffectiveConcurrency int                   `json:"effective_concurrency"`
	SharedWorkspace      bool                  `json:"shared_workspace"`
	ConcurrencyMode      string                `json:"concurrency_mode"`
	ConcurrencyDetail    string                `json:"concurrency_detail"`
	Departures           []MissionLaunchItem   `json:"departures"`
	Waiting              []MissionLaunchItem   `json:"waiting"`
	Limits               []string              `json:"limits"`
	Contract             MissionLaunchContract `json:"contract"`
}
type MissionLaunchContract struct {
	Scope      string `json:"scope"`
	Budget     string `json:"budget"`
	Recovery   string `json:"recovery"`
	Validation string `json:"validation"`
}
type MissionCoordinationPhase struct {
	Kind     string `json:"kind"`
	Label    string `json:"label"`
	Summary  string `json:"summary"`
	Actor    string `json:"actor"`
	NextStep string `json:"next_step"`
	At       string `json:"at,omitempty"`
	Relative string `json:"relative,omitempty"`
}
type MissionStatus struct {
	Changes           MissionChanges             `json:"changes_since_visit"`
	Spending          MissionSpending            `json:"spending"`
	Guidance          MissionGuidance            `json:"guidance"`
	Runtime           RuntimeHealth              `json:"runtime"`
	ProviderCooldowns []ProviderCooldown         `json:"provider_cooldowns,omitempty"`
	EvidenceStage     string                     `json:"evidence_stage"`
	Organization      Organization               `json:"organization"`
	Authorized        bool                       `json:"authorized"`
	Enabled           bool                       `json:"enabled"`
	Paused            bool                       `json:"paused"`
	Summary           string                     `json:"summary"`
	Next              string                     `json:"next"`
	Validated         int                        `json:"validated"`
	Review            int                        `json:"review"`
	Running           int                        `json:"running"`
	ActiveAgents      int                        `json:"active_agents"`
	UncertainAgents   int                        `json:"uncertain_agents"`
	Total             int                        `json:"total"`
	Supervision       MissionSupervision         `json:"supervision"`
	Understanding     MissionUnderstanding       `json:"understanding"`
	Coordination      []MissionCoordinationPhase `json:"coordination"`
	Tasks             []MissionTask              `json:"tasks"`
}

func missionLaunchContract(w Work, profile LaunchProfile, budget BudgetView) MissionLaunchContract {
	scope := strings.TrimSpace(w.Scope)
	if scope == "" {
		scope = strings.TrimSpace(w.Objective)
	}
	if scope == "" {
		scope = w.Title
	}
	contract := MissionLaunchContract{
		Scope:    fmt.Sprintf("%s
```

### mission_status.go
```
preview.Identity, err = s.missionLaunchIdentity(profile, providers)
	if err != nil {
		return preview, err
	}
	preview.Identity.Objective = w.Objective
	preview.Revision = w.Revision
	preview.Organization = organization(w)
	if preview.Organization.Ready {
		if e := s.reviewerAvailable(w); e != nil {
			preview.Organization.Ready = false
			preview.Organization.Issues = append(preview.Organization.Issues, e.Error())
			preview.Organization.Next = "Rétablir le vérificateur avant le lancement."
		}
	}
	agents, err := s.agents(work)
	if err != nil {
		return preview, err
	}
	occupiedWorkspaces, err := s.activeWorkspaces()
	if err != nil {
		return preview, err
	}
	cost, err := s.costSummary(work)
	if err != nil {
		return preview, err
	}
	budget, err := s.budget(work)
	if err != nil {
		return preview, err
	}
	preview.Contract = missionLaunchContract(w, profile, budget)
	if !preview.Organization.Ready {
		preview.Limits = append(preview.Limits, preview.Organization.Issues...)
		preview.ConcurrencyDetail = preview.Organization.Next
		return preview, nil
	}

	in := dispatchInputs{work: &w, agents: agents, profile: &profile, taskCost: cost.ByTask,
		reserve: budget.Budget.Reserve, autonomy: autonomyAuto, slots: slots, paused: false,
		depsReady: map[string]bool{}, priority: s.priorities(work), occupiedWorkspaces: occupiedWorkspaces,
		launchBlocked: map[string]string{}}
	for i := range w.Tasks {
		in.depsReady[w.Tasks[i].ID] = s.dependenciesReady(&w, &w.Tasks[i])
		task := w.Tasks[i]
		if (task.Status != "todo" && task.Status != "blocked") || !in.depsReady[task.ID] || profileFor(in, task) == nil {
			continue
		}
		candidate := *profileFor(in, task)
		launch := Launch{Schema: 1, EventID: "preview-" + hash([]byte(work + "|" + task.ID + "|" + fmt.Sprint(w.Revision)))[:20],
			Revision: w.Revision, TaskID: task.ID, Provider: candidate.Provider, Role: candidate.Role,
			Workspace: candidate.Workspace, Instruction: candidate.Instruction, Level: candidate.Level,
			Timeout: candidate.Timeout, Capture: candidate.Capture, Limits: candidate.Limits, Origin: originMissionPreview}
		if _, _, err := s.prepareLaunch(work, launch, true); err != nil {
			in.launchBlocked[task.ID] = err.Error()
		}
	}
	plan, _ := planDispatch(in)
	starts := map[string]dispatchDecision{}
	remaining := budget.Remaining
	for _, decision := range plan {
		if budget.Budget.Limit > 0 && budget.Budget.Reserve > remaining {
			in.launchBlocked[decision.TaskID] = "Budget estimatif insuffisant : nouveaux départs suspendus ; examiner le budget."
			continue
		}
		if budget.Budget.Limit > 0 {
			remaining -= budget.Budget.Reserve
		}
		starts[decision.TaskID] = decision
		task, _ := w.task(decision.TaskID)
		title := decision.TaskID
		if task != nil {
			title = task.Title
		}
		identity, identityErr := s.missionLaunchIdentity(decision.Profile, providers)
		if identityErr != nil {
			return preview, identityErr
		}
		identity.Objective = w.Objective
		preview.Departures = append(preview.Departures, MissionLaunchItem{ID: decision.TaskID, Title: title, Workspace: decision.Profile.Workspace, Identity: &identity})
	}
	workspaces := []string{}
	for _, task := range w.Tasks {
		if task.Status != "todo" && task.Status != "blocked" {
			continue
		}
		if p := profileFor(in, task); p != nil {
			for _, workspace := range workspaces {
				if workspaceOverlap(workspace, p.Workspace) {
					preview.SharedWorkspace = true
					break
				}
			}
			workspaces = append(workspaces, p.Workspace)
		}
		if _, starting := starts[task.ID]; starting {
			continue
		}
		state, reason := missionDispatchState(in, task)
		if blocked := in.launchBlocked[task.ID]; blocked != "" {
			state, reason = "intervention", blocked
		}
		normal := state == "waiting"
		if state == "ready" {
			reason = "Attend un créneau libre ou la libération de l’espace partagé"
			normal = true
		}
		preview.Waiting = append(preview.Waiting, MissionLaunchItem{ID: task.ID, Title: task.Title, Reason: reason, Normal: normal})
	}
	prev
```

### mission_cli.go
```
func printMissionLaunchIdentity(out io.Writer, identity MissionLaunchIdentity) {
	value := func(s string) string {
		if s == "" {
			return uiText("Inconnu")
		}
		return s
	}
	profile := identity.ProjectProfile
	if profile == "" {
		profile = uiText("Aucun profil de projet")
	}
	skills := strings.Join(identity.Skills, ", ")
	if skills == "" {
		skills = uiText("Aucun skill sélectionné")
	}
	role := identity.Role
	switch role {
	case "worker":
		role = uiText("Exécutant")
	case "planner":
		role = uiText("Responsable")
	case "subplanner":
		role = uiText("Sous-planificateur")
	}
	level := identity.RequestedLevel
	if level == "auto" {
		level = uiText("Automatique")
	}
	for _, row := range [][2]string{
		{uiText("Objectif"), value(identity.Objective)},
		{uiText("Rôle"), role}, {uiText("Fournisseur"), value(identity.Provider)},
		{uiText("Niveau demandé"), value(level)}, {uiText("Modèle résolu par la configuration"), value(identity.ResolvedModel)},
		{uiText("Modèle rapporté par le fournisseur"), uiText("Inconnu avant l’exécution")},
		{uiText("Profil du projet"), profile}, {uiText("Skills sélectionnés"), skills},
	} {
		fmt.Fprintf(out, "- %s : %s\n", row[0], row[1])
	}
}

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
 launchContractView(contract){const list=node('dl',undefined,'mission-launch-contract');for(const [label,value]of [[tr_web_mission_js('Portée'),contract.scope],[tr_web_mission_js('Budget'),contract.budget],[tr_web_mission_js('Reprises'),contract.recovery],[tr_web_mission_js('Validations'),contract.validation]]){const row=node('div');row.append(node('dt',label),node('dd',missionText(value)));list.append(row)}return list},
 openDetails(){const details=$('mission-results');if(details){details.open=true;details.querySelector('summary')?.focus();details.scrollIntoView({block:'nearest'})}},
 openJournal(){const journal=$('fil-bloc');if(journal){journal.open=true;journal.querySelector('summary')?.focus();journal.scrollIntoView({block:'nearest'})}},
 organizationHelp(){const o=snapshot.mission.organization;openModal(tr_web_mission_js('Organisation de la mission'),missionText(o.next),{action:'help'});$('confirm').hidden=true;$('cancel').textContent=tr_web_mission_js('Fermer');preview(o.issues.map(missionText).join('\n')+'\n'+missionText(o.verification)+tr_web_mission_js('\nCréez un travail vide depuis Gérer les missions, puis utilisez Confier ce besoin à une équipe autonome. Les anciennes missions ne sont pas converties automatiquement.'))},
 overview(d){
  if(d.guidance?.primary){const a=d.guidance.primary;return {kind:a.kind,label:missionText(a.label),effect:missionText(a.effect),tone:a.tone,summary:missionFactText(d.guidance.what),next:missionText(d.guidance.next),task:d.tasks.find(t=>t.id===a.task)}}
  if(d.runtime?.state==='blocked')return {summary:missionText(d.runtime.message),next:missionText(d.runtime.next_step),label:tr_web_mission_js('Diagnostic du stockage'),kind:'runtime',tone:'attention'};
  if(d.organization&&!d.organization.ready)return {summary:missionText(d.organization.label),next:missionText(d.organization.next),label:tr_web_mission_js('Préparer l’organisation'),kind:'organization',tone:'attention'};
  const p=typeof snapshot!=='undefined'?snapshot?.work?.planning:null;
  if(p?.paused)return {summary:tr_web_mission_js('La planification est suspendue.'),next:tr_web_mission_js('Reprenez les décisions avant de lancer la suite.'),label:tr_web_mission_js('Reprendre la planification'),kind:'planning-resume',tone:'attention'};
  if(p&&(!d.total||d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state)))&&!p.scopes.every(s=>s.state==='closed'))return {summary:p.failure||tr_web_mission_js('Les responsables préparent ou vérifient la suite.'),next:p.paused?tr_web_mission_js('La planification est suspendue.'):!d.authorized?tr_web_mission_js('Lancez la mission pour démarrer les décisions.'):tr_web_mission_js('Consultez les décisions et les retours des agents.'),label:!d.authorized?tr_web_mission_js('Lancer la mission'):tr_web_miss
```

## Tests comportementaux avec fournisseurs simulés
```go
//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMissionLaunchIdentitySeparatesResolutionFromObservation(t *testing.T) {
	modelCatalogTest(t)
	t.Setenv("SWARM_LANG", "fr")
	s := storeTest(t)
	providers := Providers{Providers: map[string]Provider{"codex": {Command: "/fixture/codex"}, "legacy": {Command: "/fixture/unknown"}}}
	got, err := s.missionLaunchIdentity(LaunchProfile{Provider: "codex", Role: "worker", Level: "standard", Skills: []ActionSkillSelection{{Path: ".claude/skills/apex/SKILL.md"}}}, providers)
	if err != nil {
		t.Fatal(err)
	}
	if got.RequestedLevel != "standard" || got.ResolvedModel != "gpt-5.6-sol" || got.ActualModel != "" || len(got.Skills) != 1 || got.ProjectProfile != "" {
		t.Fatalf("resolution misrepresented: %+v", got)
	}
	var out bytes.Buffer
	printMissionLaunchIdentity(&out, got)
	if !strings.Contains(out.String(), "gpt-5.6-sol") || !strings.Contains(out.String(), "Inconnu avant l’exécution") || !strings.Contains(out.String(), ".claude/skills/apex/SKILL.md") {
		t.Fatal(out.String())
	}
	t.Setenv("SWARM_LANG", "en")
	out.Reset()
	printMissionLaunchIdentity(&out, got)
	if !strings.Contains(out.String(), "Requested level") || !strings.Contains(out.String(), "Unknown before execution") || !strings.Contains(out.String(), "Project profile") {
		t.Fatal(out.String())
	}
	t.Setenv("SWARM_LANG", "fr")
	legacy, err := s.missionLaunchIdentity(LaunchProfile{Provider: "legacy"}, providers)
	if err != nil || legacy.ResolvedModel != "" || legacy.ActualModel != "" || legacy.Role != "worker" || legacy.RequestedLevel != "auto" {
		t.Fatalf("legacy guessed: %+v %v", legacy, err)
	}
	out.Reset()
	printMissionLaunchIdentity(&out, legacy)
	if !strings.Contains(out.String(), "Modèle résolu par la configuration : Inconnu") || !strings.Contains(out.String(), "Aucun skill sélectionné") {
		t.Fatal(out.String())
	}
}

func TestMissionLaunchPreviewIdentityUsesTaskOverrides(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	providers, _ := s.providers()
	providers.Providers["override"] = providers.Providers["fixture"]
	raw, _ := json.Marshal(providers)
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	taskProfile := profile
	taskProfile.Provider = "override"
	taskProfile.Instruction = "task specific instruction"
	if err := s.setProfile(w.ID, "t1", taskProfile, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Identity.Objective != w.Objective || preview.Departures[0].Identity.Objective != w.Objective {
		t.Fatal("objective lost from launch summary")
	}
	if len(preview.Departures) != 1 || preview.Departures[0].Identity == nil || preview.Departures[0].Identity.Provider != "override" || preview.Identity.Provider != "fixture" || preview.Departures[0].Identity.ActualModel != "" {
		t.Fatalf("missing departure identity %+v", preview)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("read-only preview mutated revision")
	}
}

```

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
