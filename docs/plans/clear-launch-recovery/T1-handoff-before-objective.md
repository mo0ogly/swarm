# T1 — correctif du superviseur, 3 octobre 2026

Le résumé avant lancement est maintenant implémenté dans le moteur, le CLI et le cockpit. Il sépare la configuration demandée/résolue de l’observation réelle, toujours inconnue avant le départ ; l’acceptation indépendante de T1 reste à réaliser.

## Identité

- Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T1.
- Auteur du correctif : superviseur Codex dans cette conversation ; aucune tentative Swarm fictive créée pour ces modifications.
- Base : 1d9570bd4ef617130c6be96b7ec88844fdbcd00e ; candidat non commité. Empreintes exactes : T1-candidate.json.
- Ancien rapport conservé dans T1-first-diagnostic.md comme diagnostic de la première tentative ; ses constats d’absence ne décrivent plus ce candidat.

## Changements

MissionLaunchPreview transporte une identité commune et une identité par départ. Chaque départ utilise son LaunchProfile effectif, avec priorité au profil propre de la tâche. Les identités font partie du payload complet déjà signé par verdict_token : un aperçu devenu différent nécessite une nouvelle confirmation. L’aperçu reste sans mutation de révision.

Les deux interfaces affichent rôle, fournisseur, niveau demandé, modèle résolu, modèle rapporté, profil du projet, skills sélectionnés, périmètre et limites. Le champ du modèle réel n’est jamais déduit du modèle configuré : « Inconnu avant l’exécution ». Un fournisseur personnalisé sans résolution connue affiche « Inconnu ». Aucun profil/skill absent n’est inventé.

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

## Éléments du code réellement remis au vérificateur
Ces extraits décrivent le candidat T1 vérifié ; la réparation de reprise de revue est un correctif moteur distinct.

### mission_status.go
```
type MissionLaunchIdentity struct {
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
		Scope:    fmt.Sprintf("%s · %d tâche(s) · dossier %s", scope, len(w.Tasks), profile.Workspace),
		Recovery: fmt.Sprintf("Au plus %d tentatives automatiques par chaîne de reprise, premier départ compris ; un environnement inchangé attend une nouvelle vérification et les plafonds propres aux tâches restent prioritaires.", maxAutomaticAttempts),
	}
	if budget.Budget.Limit > 0 {
		contract.Budget = fmt.Sprintf("Plafond estimatif %.2f USD · %.2f USD disponibles · %.2f USD réservés par nouveau départ.", budget.Budget.Limit, budget.Remaining, budget.Budget.Reserve)
	} else {
		contract.Budget = "Aucun plafond financier configuré ; les limites d’outils et de tentatives de chaque tâche restent appliquées."
	}
	automatic, controls := 0, 0
	for i := range w.Tasks {
		if w.Tasks[i].ValidationPolicy != nil && w.Tasks[i].ValidationPolicy.Mode == "automatic" {
			automatic++
			controls += len(w.Tasks[i].ValidationPolicy.Controls)
		}
	}
	contract.Validation = fmt.Sprintf("%d tâche(s) en revue humaine · %d tâche(s) avec %d contrôle(s) automatique(s) préautorisés.", len(w.Tasks)-automatic, automatic, controls)
	return contract
}

// missionLaunchPreview asks the dispatcher for the first wave using the
// proposed common profile. It is read-only: confirmation must evaluate the
// current revision again before persisting the authorization.
func (s *Store) missionLaunchPreview(work string, profile LaunchProfile, slots int) (MissionLaunchPreview, error) {
	preview := MissionLaunchPreview{RequestedSlots: slots, Departures: []MissionLaunchItem{}, Waiting: []Mission
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
  if(p&&(!d.total||d.tasks.every(t=>['validated','waived','abandoned'].includes(t.state)))&&!p.scopes.every(s=>s.state==='closed'))return {summary:p.failure||tr_web_mission_js('Les responsables préparent ou vérifient la suite.'),next:p.paused?tr_web_mission_js('La planification est suspendue.'):!d.authorized?tr_web_mission_js('Lancez la mission pour démarrer les décisions.'):tr_web_mission_js('Consultez les décisions et les retours des agents.'),label:!d.authorized?tr_web_mission_js('Lancer la mission'):tr_web_mission_js('Voir les décisions'),kind:!d.authorized?'start':'planning',tone:p.failure?'attention':'info'};
  const configurations=d.tasks.filter(t=>t.state==='configure');
  const needs=d.tasks.filter(t=>['review','intervention'].includes(t.state)).sort((a,b)=>b.impact-a.impact);
  const counts=d.validated+'/'+d.total+tr_web_mission_js(' résultats validés · ')+d.running+tr_web_mission_js(' en cours · ')+needs.length+tr_web_mission_js(' décision(s) attendue(s).');
  if(!d.total)return {summary:tr_web_mission_js('Ce travail ne contient encore aucune tâche.'),next:tr_web_mission_js('Préparez le besoin avec l’IA pour construire un plan.'),label:tr_web_mission_js('Préparer les tâches'),kind:'prepare',tone:'info'};
  if(configurations.length)return {summary:counts,next:configurations.length+' '+(configurations.length>1?tr_web_mission_js('tâches utiliseront'):tr_web_mission_js('tâche utilisera'))+tr_web_mission_js(' la même configuration de lancement.'),label:tr_web_mission_js('Préparer le lancement'),kind:'configure',task:configurations[0],tone:'info'};
  if(needs.length){const t=needs[0];return {summary:counts,next:t.title+' : '+missionText(t.reason),label:t.attempt_limit_reached?tr_web_mission_js('Examiner les tentatives et les refus'):t.state==='review'?tr_web_mission_js('Examiner le résultat'):tr_web_mission_js('Résoudre le blocage'),kind:'decision',task:t,tone:'attention'}}
  const running=d.tasks.find(t=>t.state==='running');
  if(running){const supervision=d.authorized&&!d.enabled?tr_web_mission_js(' Le conducteur des prochains dépa
```

### Tests comportementaux (doublures de fournisseurs explicitement utilisées)
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
	if len(preview.Departures) != 1 || preview.Departures[0].Identity == nil || preview.Departures[0].Identity.Provider != "override" || preview.Identity.Provider != "fixture" || preview.Departures[0].Identity.ActualModel != "" {
		t.Fatalf("missing departure identity %+v", preview)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("read-only preview mutated revision")
	}
}

```

### Empreintes du candidat T1
```json
{
  "base": "1d9570bd4ef617130c6be96b7ec88844fdbcd00e",
  "files_sha256": {
    "mission_status.go": "5f9711c758d674a3249f96833f4f0e6f13beefa92b7116f1419ff4edc4fb7fd0",
    "mission_cli.go": "09ff5a8f955661caef0a70242f03abc10e438ce4a9e3fad89579ab767e9731fc",
    "web/mission.js": "bfeeba57a581fc6c0fc7d2db84fc176338e039556956dcbe55b36f3264d481fe",
    "locales/en.json": "405d74e7e5b462b79586ed0ee892926fad4c37f1daee7a40adfdb7d29c41dbaa",
    "web/i18n-en.js": "d2e95248b7e93146ebb99e9ecaf1a2698050f02295a1aee733547b820ed7a3cb",
    "mission_launch_identity_test.go": "c021f19b4c7a87a903ae9cc9f97f25c912b80b515c7f10ea428e0d84a6c9c736"
  },
  "dirty": true
}

```

La recette navigateur était exécutée par le superviseur, sur un serveur isolé sans appel payant. Les captures sont des preuves visuelles de ce contrôle manuel, pas des preuves de fonctionnement des fournisseurs réels. Le rapport du troisième worker prouve uniquement son contrôle d’intégrité et ses tests. Les états inconnus sont présents dans les captures et les rendus CLI, jamais remplacés par une supposition.
