# Dossier de recette D01

## Candidat et frontières

- HEAD : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`; candidat = HEAD + diff sale partagé.
- Sources D01 : `graph_delivery_e2e_test.go`, `tests/graph_automation_e2e.cjs`.
- Rapport : `docs/D01.md`. Ce dossier et le rapport restent chacun sous 48 Ko.
- Contrôle protégé non modifié : `docs/plans/graphe-automatisation-20261005/execution/d/verify.py`.
- Fixtures : `t.TempDir()` / store de test côté Go; `mkdtemp` et roots dédiées prévus côté produit. Aucun store de mission vivante.
- Fournisseur : fixture seulement; aucune autonomie fournisseur réelle démontrée.

## Six scénarios moteur — preuve fraîche

Le reçu courant `execution/d/results/e81280bcfb7f457da33dbbd0975c5323/1.json` consigne `go test -race ./... -run '^TestGraphDeliveryD01' -count=1 -json -timeout=120s`, code 0, log SHA-256 `d843b100669aa6db336b2d381116e0943e439e9599c863de29b6eff4fb1505a8`. Le log contient un événement PASS pour chacun des six tests, sans skip/fail.

| Test | Déclencheur | Assertion sensible à la régression | Raccords |
| --- | --- | --- | --- |
| `TestGraphDeliveryD01PublicLifecycle` | brouillon → preview/apply → cycle → programme | dépendance persistée, cycle refusé sans mutation, programme désactivé, zéro agent implicite | R01, R02, R08, R17 |
| `TestGraphDeliveryD01RequestReplay` | demande rejouée puis même clé/contenu différent | identités/révision stables; `idempotency_conflict`; effet unique | R09, R12 |
| `TestGraphDeliveryD01Conflict` | deux apply concurrents depuis la même révision | exactement un succès et un `revision_conflict` | R03, R08 |
| `TestGraphDeliveryD01WorkspaceWait` | réservation active puis demande | `waiting/workspace_wait`, aucune tentative ni effet | R10, R13 |
| `TestGraphDeliveryD01RestartRetention` | demande + programme, store rouvert, tick après occurrences manquées | identités conservées, règle skip, aucune soumission cachée | R11, R13 |
| `TestGraphDeliveryD01ProofFreshness` | candidat modifié, refus puis preuve causale nouvelle/répétée | digest changé, reprise unique, rejeu refusé, budgets/coût inchangés | R13, R14, R18 |

## Correction du reçu bloqué

Le reçu de tentative précédente échouait sur `plan draft preview` avec `invalid_input`. Le contrat Go est `GraphDraft.ID string json:"draft_id"`; la recette JS lisait `cycle.id` et `draft.id`. Les trois usages envoyés aux opérations preview/apply utilisent maintenant `cycle.draft_id` / `draft.draft_id`.

Le superviseur exécute déjà tous les tests D01 sous race avant de démarrer Node. La recette tentait pourtant de relancer trois tests Go depuis Node, opération rejetée par le sandbox (`spawnSync go EPERM`). Elle recherche désormais le reçu JSON race dans le répertoire parent, exige code 0, vérifie SHA-256 du log et exige les trois événements PASS `WorkspaceWait`, `RestartRetention`, `ProofFreshness`. L’absence ou l’altération d’une preuve échoue fermée.

La seconde reprise a ensuite rencontré `spawnSync <binary>/swarm EPERM` au premier `init`. Aucun CLI, serveur ou navigateur n’a été exercé dans ce sandbox après la correction; les attentes ci-dessous restent donc NOT TESTED.

## Recette produit attendue

`tests/graph_automation_e2e.cjs` compose les recettes graphe/journal et programmes sur roots distinctes. Les variantes contractuelles sont `fr-sombre`, `fr-etat`, `en-sombre`, `en-etat`; chacune doit produire un PNG relatif, zéro erreur console/réseau inattendue et toutes les assertions suivantes.

| Assertion | Observable attendu | État courant |
| --- | --- | --- |
| `prepare_roles` | owners planner/worker via CLI publique, zéro agent | NOT TESTED sandbox |
| `dependency_apply` | preview/apply puis dépendance visible | NOT TESTED sandbox |
| `cycle_rejected` | `dependency_cycle`, révision inchangée | PASS moteur; UX NOT TESTED |
| `agent_journal` | ligne visible acteur/action/raison | NOT TESTED sandbox |
| `program_replay` | programme retrouvé après reload | NOT TESTED sandbox |
| `content_conflict` | même événement/contenu différent → conflit, effet unique | PASS moteur; UX NOT TESTED |
| `workspace_wait` | reçu race authentifié + restitution combinée | PASS moteur; produit NOT TESTED |
| `restart_retention` | reçu race authentifié + reload programme | PASS moteur; produit NOT TESTED |
| `causal_recovery` | reçu race authentifié, pas une action UI directe | PASS moteur; produit NOT TESTED |
| `unknown_cost` | état inconnu visible graphe et programme | PASS moteur partiel; UX NOT TESTED |
| `focus_restore` | focus rendu après fermeture aides/panneaux | NOT TESTED sandbox |
| `graph_preserved` | arêtes conservées après administration programmes | NOT TESTED sandbox |

## Matrice R01–R18

| ID | Couverture D01 observée | Résultat | Limite / prochain propriétaire |
| --- | --- | --- | --- |
| R01 | préparation et absence de départ implicite | PASS moteur | rôles UX à contrôler hôte |
| R02 | apply + cycle refusé sans mutation | PASS moteur | UI/capture manquante |
| R03 | concurrence : un gagnant, un conflit | PASS moteur | message UX manquant |
| R04 | conservation viewport prévue par recette graphe | NOT TESTED | navigateur hôte |
| R05 | groupe/compte/liens masqués prévu par recette graphe | NOT TESTED | navigateur hôte |
| R06 | sélection/journal prévue dans la recette D01 | NOT TESTED | navigateur hôte |
| R07 | undo/redo couvert par recette composée | NOT TESTED | navigateur hôte |
| R08 | opérations/services publics et tests moteur | PASS moteur | probe CLI Node bloqué |
| R09 | rejeu identique + conflit de contenu | PASS moteur | restitution produit absente |
| R10 | espace occupé → attente sans tentative | PASS moteur | affichage UX absent |
| R11 | restart/skip sans rafale | PASS moteur | programme UX absent |
| R12 | déduplication/conflit de demande | PASS moteur partiel | signature/redaction C03 non rejouées |
| R13 | attente/restart/reprise causale | PASS moteur | affichage attribution absent |
| R14 | candidat changé → preuve périmée | PASS moteur | revue actuelle absente |
| R15 | quatre variantes, clavier/focus | NOT TESTED | navigateur hôte requis |
| R16 | migration/rollback | NOT APPLICABLE D01 | propriétaire D02 |
| R17 | absence d’agent/acceptation implicite | PASS moteur partiel | clôture publique réelle non exercée |
| R18 | coût fournisseur reste inconnu | PASS moteur | libellé UI absent |

## Reçus courants et reprise

- `e81280.../0.json` : inventaire exact des six tests, code 0, SHA-256 `6fbdc0...58b0`.
- `e81280.../1.json` : race ciblée, six PASS, code 0, SHA-256 `d843b1...05a8`.
- `e81280.../2.json` : build isolé, code 0.
- `e81280.../3.json` : Node code 1, `spawnSync .../swarm EPERM`, log SHA-256 `043fb7...1147`; aucune preuve navigateur.
- Contrôle précédent après correction `draft_id` : `2d549.../3.json`, Node code 1 sur `spawnSync go EPERM`; cette relance Go redondante a été supprimée.

Empreintes avant finalisation du rapport :

- `graph_delivery_e2e_test.go` : `82bf6ac3434af24cd40472dd82a5375e1708819785ed3f190ecea6f2fdd9c731`.
- `tests/graph_automation_e2e.cjs` : `9281bce7c5918474ef6cef9d1979d6c8fa00e31f8572431d9338c4363a2c2b38`.

## Gardes et remise reviewer

- Les protections du harness sont exécutées avant le contrôle; celui-ci a atteint les étapes inventaire/race/build, donc `protect()` initial n’a pas échoué.
- Aucun rapport A/B/C, manifeste ou harness superviseur n’a été modifié par D01.
- Le conducteur hôte doit relancer une fois le contrôle exact avec processus enfants Node autorisés, puis joindre `results.json`, quatre PNG et diagnostics.
- Le reviewer indépendant doit recevoir les deux sources D01, ce dossier, le rapport, les reçus courants et le futur reçu navigateur. Il doit identifier HEAD + diff sale. Ce dossier n’est ni une autosignature ni une acceptation.

## Complément vérifié supervision — 6 octobre 2026

Après fin des deux producteurs et pause publique avant modification inputs : correction recette uniquement des codes de sortie (cycle2, conflit événement3) et du champ failure.code de la CLI ; les observations structurées internes restent code. Les échecs précédents sont conservés et attribués recette/agent/supervision, restrictions EPERM attribuées sandbox. Aucun moteur produit modifié, aucune troisième production ni hausse de budget.

Contrôle hôte execution/d/verify.py D01 code0 : six tests Go ciblés/race sans skip, build canonique, vrais parcours navigateur de produit isolé FR/EN sombre/État, douze assertions par variante, diagnostics console/réseau sans erreur inattendue. Reçus/logs authentifiables execution/d/results/712d5153817846a389db0ec0b587beba ; browser/results.json. Capture des programmes et du graphe fermé dans docs/screenshots/graph-delivery-d01. Inspection IA des captures, aucune inspection humaine ni autonomie fournisseur démontrée. Une capture d'inspecteur en-sombre présente un contenu transitoirement vide et n'est pas retenue comme preuve visuelle de cet état ; assertions DOM sélection/tentative sont passées, ne pas prétendre inspection complète de toutes transitions.

Limites : deux fixtures produit distinctes graphe/programmes ; préparation des rôles exercée via CLI (pas assistant IA de préparation), attente/restart/reprise causale prouvés par vrais tests moteur avec reçu race SHA vérifié par recette, pas déclenchés depuis navigateur. Journal affiche les événements fixture ; aucun agent fournisseur réel lancé. R16 migration relève de D02. Verdict indépendant et acceptation publique restent requis : résultat hôte PASS n'est pas acceptation.

## Source courante intégrale graph_delivery_e2e_test.go

SHA256 82bf6ac3434af24cd40472dd82a5375e1708819785ed3f190ecea6f2fdd9c731

```go
//go:build linux

package main

import (
        "errors"
        "reflect"
        "sync"
        "testing"
        "time"
)

// D01 deliberately composes the public service boundaries used by the CLI and
// HTTP handlers.  The stores are all backed by t.TempDir through storeTest; no
// running mission or provider is touched.
func TestGraphDeliveryD01PublicLifecycle(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)

        draft, preview := createGraphDraftTest(t, s, w,
                GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        applied := applyGraphDraftTest(t, s, w, draft, preview, "d01-public-apply")
        current, err := s.get(w.ID)
        if err != nil || applied.Revision != current.Revision || !taskDependsOn(t, current, "t2", "t1") {
                t.Fatalf("public graph apply was not observable: result=%+v work=%+v err=%v", applied, current, err)
        }

        cycle, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{
                Schema: 1, WorkID: current.ID, ExpectedRevision: current.Revision,
                Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"}},
        })
        if err != nil {
                t.Fatal(err)
        }
        if _, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: current.ID, DraftID: cycle.ID, ExpectedRevision: current.Revision}); graphCode(err) != "dependency_cycle" {
                t.Fatalf("cycle was not rejected at preview: %v", err)
        }
        afterCycle, _ := s.get(w.ID)
        if afterCycle.Revision != current.Revision || !taskDependsOn(t, afterCycle, "t2", "t1") {
                t.Fatal("cycle rejection changed the accepted graph")
        }

        request := automationC02Create(current, "d01-program", "once", "2099-01-01T09:00:00", "", "UTC", 1)
        service := s.automationService()
        programPreview, err := service.Preview(request, 1, func() time.Time { return automationC02Time(t, "2098-01-01T00:00:00Z") })
        if err != nil || programPreview.PreviewToken == "" || programPreview.CreatesEnabled {
                t.Fatalf("program preview did not remain inert: %+v err=%v", programPreview, err)
        }
        program, err := service.Create(AutomationCreateCommand{Schedule: request, PreviewToken: programPreview.PreviewToken}, func() time.Time { return automationC02Time(t, "2098-01-01T00:00:00Z") })
        if err != nil || program.Schedule.State != "disabled" || len(program.Occurrences) != 0 {
                t.Fatalf("program was not created disabled: %+v err=%v", program, err)
        }
        listed, err := service.List()
        if err != nil || len(listed) != 1 || listed[0].Schedule.ScheduleID != program.Schedule.ScheduleID {
                t.Fatalf("program journal/list did not expose the created program: %+v err=%v", listed, err)
        }
        var agents int
        if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
                t.Fatalf("draft/program preparation started an agent: count=%d err=%v", agents, err)
        }
}

func TestGraphDeliveryD01RequestReplay(t *testing.T) {
        s, w := automationC01Fixture(t)
        request := automationC01Request("d01-request-replay", w)
        first, fresh, err := s.submitAutomationResume(request)
        if err != nil || !fresh {
                t.Fatalf("initial request missing: fresh=%v err=%v", fresh, err)
        }
        replayed, fresh, err := s.submitAutomationResume(request)
        if err != nil || fresh || replayed.RequestID != first.RequestID || replayed.OccurrenceID != first.OccurrenceID || replayed.Revision != first.Revision {
                t.Fatalf("same request was not replayed: first=%+v replay=%+v fresh=%v err=%v", first, replayed, fresh, err)
        }
        request.Source = "different-content"
        request.ContentDigest = automationResumeDigest(request)
        _, _, err = s.submitAutomationResume(request)
        var command *CommandError
        if !errors.As(err, &command) || command.Code != "idempotency_conflict" {
                t.Fatalf("same key with different content was not rejected: %v", err)
        }
        var count int
        if err = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE idempotency_key=?", request.IdempotencyKey).Scan(&count); err != nil || count != 1 {
                t.Fatalf("request replay produced duplicate durable rows: count=%d err=%v", count, err)
        }
}

func TestGraphDeliveryD01Conflict(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        draftA, previewA := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        draftB, previewB := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t3"})
        requests := []GraphDraftApplyRequest{
                {Schema: 1, WorkID: w.ID, DraftID: draftA.ID, EventID: "d01-conflict-a", ExpectedRevision: w.Revision, PreviewToken: previewA.PreviewToken, ContentDigest: previewA.ContentDigest},
                {Schema: 1, WorkID: w.ID, DraftID: draftB.ID, EventID: "d01-conflict-b", ExpectedRevision: w.Revision, PreviewToken: previewB.PreviewToken, ContentDigest: previewB.ContentDigest},
        }
        errs := make([]error, len(requests))
        start := make(chan struct{})
        var group sync.WaitGroup
        for index := range requests {
                group.Add(1)
                go func(index int) {
                        defer group.Done()
                        <-start
                        _, errs[index] = s.applyGraphDraft(operatorIdentity(), requests[index])
                }(index)
        }
        close(start)
        group.Wait()
        passes, conflicts := 0, 0
        for _, applyErr := range errs {
                if applyErr == nil {
                        passes++
                } else if graphCode(applyErr) == "revision_conflict" {
                        conflicts++
                } else {
                        t.Fatalf("unexpected concurrent result: %v", applyErr)
                }
        }
        current, err := s.get(w.ID)
        if err != nil || passes != 1 || conflicts != 1 || current.Revision != w.Revision+1 {
                t.Fatalf("conflict did not preserve one winner: passes=%d conflicts=%d revision=%d err=%v", passes, conflicts, current.Revision, err)
        }
}

func TestGraphDeliveryD01WorkspaceWait(t *testing.T) {
        s, w := automationC01Fixture(t)
        holder, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "d01-workspace-holder", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: s.root, Capture: true})
        if err != nil || !created || holder.ID == "" {
                t.Fatalf("workspace holder missing: %+v created=%v err=%v", holder, created, err)
        }
        before, _ := s.get(w.ID)
        request, _, err := s.submitAutomationResume(automationC01Request("d01-workspace-wait", before))
        if err != nil {
                t.Fatal(err)
        }
        waiting, err := s.processAutomationRequest(request.RequestID, "d01-wait-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || waiting.State != "waiting" || waiting.Reason != "workspace_wait" || waiting.NextAction != "retry_after_resource_release" {
                t.Fatalf("occupied workspace did not yield durable wait: %+v err=%v", waiting, err)
        }
        after, _ := s.get(w.ID)
        if len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) {
                t.Fatalf("workspace wait consumed an attempt: before=%d after=%d", len(before.Tasks[0].Attempts), len(after.Tasks[0].Attempts))
        }
        var effects int
        if err = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", request.RequestID).Scan(&effects); err != nil || effects != 0 {
                t.Fatalf("workspace wait produced an effect: count=%d err=%v", effects, err)
        }
}

func TestGraphDeliveryD01RestartRetention(t *testing.T) {
        s, w := automationC01Fixture(t)
        request, _, err := s.submitAutomationResume(automationC01Request("d01-restart-request", w))
        if err != nil {
                t.Fatal(err)
        }
        base := automationC02Time(t, "2026-01-01T07:00:00Z")
        schedule, err := s.createAutomationSchedule(automationC02Create(w, "d01-restart-program", "daily", "2026-01-01T09:00:00", "2026-01-03T09:00:00", "UTC", 3), func() time.Time { return base })
        if err != nil {
                t.Fatal(err)
        }
        schedule, err = s.setAutomationScheduleState(schedule.ScheduleID, "enabled", schedule.Revision, func() time.Time { return base })
        if err != nil {
                t.Fatal(err)
        }
        reopened, err := openStore(s.root, false)
        if err != nil {
                t.Fatal(err)
        }
        defer reopened.db.Close()
        retained, err := reopened.automationRequest(request.RequestID)
        if err != nil || retained.State != "received" || retained.OccurrenceID != request.OccurrenceID {
                t.Fatalf("request identity lost on reopen: %+v err=%v", retained, err)
        }
        if err = reopened.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2026-01-02T10:00:00Z") }); err != nil {
                t.Fatal(err)
        }
        program, err := reopened.automationSchedule(schedule.ScheduleID)
        if err != nil || program.NextAt != "2026-01-03T09:00:00Z" || program.EmittedCount != 2 {
                t.Fatalf("restart skip policy was not retained: %+v err=%v", program, err)
        }
        var submitted int
        if err = reopened.db.QueryRow("SELECT count(*) FROM automation_schedule_journal WHERE schedule_id=? AND state='submitted'", schedule.ScheduleID).Scan(&submitted); err != nil || submitted != 0 {
                t.Fatalf("restart emitted a hidden catch-up request: count=%d err=%v", submitted, err)
        }
}

func TestGraphDeliveryD01ProofFreshness(t *testing.T) {
        before := Work{Planning: &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-before"}}, Tasks: []Task{{ID: "source", Status: "accepted"}, {ID: "checked", Status: "accepted", Depends: []string{"source"}}}}
        after := before
        after.Tasks = append([]Task(nil), before.Tasks...)
        after.Planning = &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-after"}}
        projection := graphDraftProofProjection(&before, &after, []string{"checked"})
        if !projection.Relevant || projection.Kind != "relevant" || projection.BeforeDigest == projection.AfterDigest {
                t.Fatalf("changed candidate did not stale the proof projection: %+v", projection)
        }

        s, w := automationC01Fixture(t)
        request, _, err := s.submitAutomationResume(automationC01Request("d01-causal-recovery", w))
        if err != nil {
                t.Fatal(err)
        }
        budgetBefore, err := s.budget(w.ID)
        if err != nil {
                t.Fatal(err)
        }
        if err = s.stopMission(w.ID); err != nil {
                t.Fatal(err)
        }
        rejected, err := s.processAutomationRequest(request.RequestID, "d01-causal-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || rejected.State != "rejected" || rejected.Reason != "authorization_required" {
                t.Fatalf("causal prerequisite was not observable: %+v err=%v", rejected, err)
        }
        if err = s.setMission(w.ID, true); err != nil {
                t.Fatal(err)
        }
        if err = s.pause(w.ID, false); err != nil {
                t.Fatal(err)
        }
        evidence := AutomationCausalRecovery{Schema: 1, RequestID: request.RequestID, PreviousReason: rejected.Reason, EvidenceKind: "authorization_revision", EvidenceDigest: automationEvidenceDigest("d01-mission-policy-reenabled")}
        recovered, err := s.recoverAutomationRequestCausally(evidence, func() time.Time { return time.Now() })
        if err != nil || recovered.State != "received" || recovered.NextAction != "claim" {
                t.Fatalf("changed causal evidence did not recover the request: %+v err=%v", recovered, err)
        }
        if _, err = s.recoverAutomationRequestCausally(evidence, func() time.Time { return time.Now() }); commandFailure(err).Code != "causal_change_required" {
                t.Fatalf("same causal evidence was reusable: %v", err)
        }
        budgetAfter, err := s.budget(w.ID)
        if err != nil || !reflect.DeepEqual(budgetBefore, budgetAfter) || budgetAfter.ActualCost != nil {
                t.Fatalf("recovery changed limits or invented provider cost: before=%+v after=%+v err=%v", budgetBefore, budgetAfter, err)
        }
}

```

## Source courante intégrale tests/graph_automation_e2e.cjs

SHA256 c30c9e7d8fc998da75428bffd71175172decd47f61ee3827e399b294a68ef85f

```javascript
'use strict';
// D01 product receipt.  It composes the native graph and Programs journeys on
// isolated roots, adds a public-CLI rejection/replay probe, and records which
// engine D01 test supplies each non-browser guarantee.  No provider is started.
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,spawnSync}=require('node:child_process');
if(!process.argv[2]||!process.argv[3])throw Error('usage: node tests/graph_automation_e2e.cjs BINARY OUTPUT');
const binary=path.resolve(process.argv[2]),output=path.resolve(process.argv[3]);
fs.mkdirSync(output,{recursive:true});
const scratch=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-d01-'));
const graphOutput=path.join(output,'graph'),automationOutput=path.join(output,'programs');

function executeAsync(command,args,options={}){
 return new Promise((resolve,reject)=>{
  const child=spawn(command,args,{env:options.env,stdio:['ignore','pipe','pipe']});let stdout='',stderr='',settled=false;
  const timer=setTimeout(()=>{if(!settled){child.kill('SIGTERM');reject(Error(`${command} ${args.join(' ')} timed out`))}},options.timeout||110000);
  child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);
  child.on('error',error=>{if(!settled){settled=true;clearTimeout(timer);reject(error)}});
  child.on('close',status=>{if(settled)return;settled=true;clearTimeout(timer);if(status!==0)reject(Error(`${command} ${args.join(' ')} failed (${status}):\n${stdout}\n${stderr}`));else resolve({stdout,stderr,status})});
 });
}
function publicCLI(root,args,input,expected=0){
 const result=spawnSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined,maxBuffer:16*1024*1024,timeout:15000});
 if(result.error)throw Error(`CLI ${args.join(' ')} did not complete: ${result.error.message}`);
 assert.equal(result.status,expected,`CLI ${args.join(' ')}: ${result.stderr}`);
 const text=(expected===0?result.stdout:result.stderr).trim();
 return text?JSON.parse(text):{};
}
function mutation(root,args,revision,body){
 return publicCLI(root,args,{schema_version:1,event_id:crypto.randomUUID(),expected_revision:revision,...body}).work;
}
function engineProbe(){
 const pattern='^(TestGraphDeliveryD01WorkspaceWait|TestGraphDeliveryD01RestartRetention|TestGraphDeliveryD01ProofFreshness)$';
 const parent=path.dirname(output);
 const receipt=fs.readdirSync(parent).filter(name=>name.endsWith('.json')).map(name=>({name,value:JSON.parse(fs.readFileSync(path.join(parent,name),'utf8'))})).find(row=>row.value.command?.[0]==='go'&&row.value.command.includes('-race')&&row.value.command.some(arg=>arg.startsWith('^TestGraphDeliveryD01')));
 assert.ok(receipt,'supervisor D01 race receipt missing');assert.equal(receipt.value.exit_code,0,'supervisor D01 race failed');
 const log=fs.readFileSync(receipt.value.log,'utf8'),digest=crypto.createHash('sha256').update(log).digest('hex');assert.equal(digest,receipt.value.sha256,'supervisor D01 race log digest mismatch');
 const tests=log.split('\n').filter(line=>line.startsWith('{')).map(line=>JSON.parse(line)).filter(event=>event.Action==='pass'&&event.Test&&new RegExp(pattern).test(event.Test)).map(event=>event.Test);
 const expected=['TestGraphDeliveryD01WorkspaceWait','TestGraphDeliveryD01RestartRetention','TestGraphDeliveryD01ProofFreshness'];
 assert.deepEqual(tests.sort(),expected.sort());
 return {tests,command:receipt.value.command,receipt:receipt.name,log_sha256:digest};
}
function cliProbe(){
 const root=path.join(scratch,'cli');fs.mkdirSync(root,{recursive:true});publicCLI(root,['init']);
 let work=mutation(root,['work','create'],0,{title:'D01 public probe',objective:'Traverse product boundaries',scope:'isolated fixture',criteria:['observable effects']});
 work=mutation(root,['task','add',work.id],work.revision,{id:'prepare',title:'Prepare',owner:'planner',deliverable:'docs/prepare.md',criteria:['roles visible'],next:'prepare'});
 work=mutation(root,['task','add',work.id],work.revision,{id:'execute',title:'Execute',owner:'worker',deliverable:'docs/execute.md',criteria:['effect visible'],depends:['prepare'],next:'wait'});
 const shown=publicCLI(root,['work','show',work.id]).work;
 assert.deepEqual(shown.tasks.map(task=>task.owner),['planner','worker']);
 const cycle=publicCLI(root,['plan','draft','import',work.id],{schema_version:1,work_id:work.id,expected_revision:work.revision,operations:[{kind:'add_dependency',prerequisite:'execute',dependent:'prepare'}]});
 const rejected=publicCLI(root,['plan','draft','preview',work.id],{schema_version:1,work_id:work.id,draft_id:cycle.draft_id,expected_revision:work.revision},2);
 assert.equal(rejected.failure.code,'dependency_cycle');
 const afterCycle=publicCLI(root,['work','show',work.id]).work;assert.equal(afterCycle.revision,work.revision);assert.deepEqual(afterCycle.tasks.find(task=>task.id==='execute').depends,['prepare']);
 work=mutation(root,['task','add',work.id],work.revision,{id:'review',title:'Review',owner:'reviewer',deliverable:'docs/review.md',criteria:['independent'],next:'review'});
 const draft=publicCLI(root,['plan','draft','import',work.id],{schema_version:1,work_id:work.id,expected_revision:work.revision,operations:[{kind:'add_dependency',prerequisite:'execute',dependent:'review'}]});
 const preview=publicCLI(root,['plan','draft','preview',work.id],{schema_version:1,work_id:work.id,draft_id:draft.draft_id,expected_revision:work.revision});
 const apply={schema_version:1,work_id:work.id,draft_id:draft.draft_id,event_id:'d01-public-replay',expected_revision:work.revision,preview_token:preview.preview_token,content_digest:preview.content_digest};
 const first=publicCLI(root,['plan','draft','apply',work.id],apply),again=publicCLI(root,['plan','draft','apply',work.id],apply);
 assert.equal(again.revision,first.revision);assert.equal(again.applied_at,first.applied_at);
 const conflicted=publicCLI(root,['plan','draft','apply',work.id],{...apply,content_digest:'different-content'},3);assert.equal(conflicted.failure.code,'event_conflict');
 const finalWork=publicCLI(root,['work','show',work.id]).work;assert.equal(finalWork.revision,first.revision);assert.ok(finalWork.tasks.find(task=>task.id==='review').depends.includes('execute'));
 return {root_kind:'temporary',work_id:work.id,prepare_roles:{owners:shown.tasks.map(task=>task.owner),agent_count:publicCLI(root,['agent','list',work.id]).agents.length},cycle_rejected:{code:rejected.failure.code,revision_preserved:afterCycle.revision},dependency_apply:{revision:first.revision,dependency:'execute -> review'},content_conflict:{code:conflicted.failure.code,replay_revision:again.revision,rows_preserved:true}};
}

async function graphJourney(){
 let source=fs.readFileSync(path.join('tests','graph_draft_ui.cjs'),'utf8');
 const loads='const load_measurements=[];for(const cards of [50,200,500])load_measurements.push(await loadMeasurement(browser,cards));';
 assert.equal(source.split(loads).length,2,'graph recipe load anchor changed');source=source.replace(loads,'const load_measurements=[];');
 const anchor="  const activityScreenshot=name+'-attempt.png';";assert.equal(source.split(anchor).length,2,'graph recipe journal anchor changed');
 const journal=String.raw`
  await page.locator('[data-mission-action="journal"]').click();
  await page.waitForFunction(()=>document.querySelector('#fil-bloc')?.open&&document.querySelectorAll('#fil-entrees .fil-entree').length);
  const d01Journal=await page.evaluate(()=>[...document.querySelectorAll('#fil-entrees .fil-entree')].map(row=>({origin:row.dataset.origine,actor:row.querySelector('.fil-origine')?.textContent,action:row.querySelector('.fil-label')?.textContent,reason:row.querySelector('.fil-message')?.textContent,visible:!!row.getClientRects().length})));
  assert.ok(d01Journal.some(row=>row.visible&&row.actor&&row.action&&row.reason&&/t000|Tâche 0/.test(row.reason)),'selected agent activity missing from journal');
  fs.writeFileSync(path.join(outDir,name+'-d01-journal.json'),JSON.stringify(d01Journal,null,2));
`;
 source=source.replace(anchor,journal+'\n'+anchor);
 const generated=path.join(scratch,'graph-d01.cjs');fs.writeFileSync(generated,source);
 const result=await executeAsync(process.execPath,[generated,binary,graphOutput],{env:{...process.env,PUPPETEER_MODULE:require.resolve('puppeteer')},timeout:110000});
 fs.writeFileSync(path.join(output,'graph-run.log'),result.stdout+result.stderr);
 return JSON.parse(fs.readFileSync(path.join(graphOutput,'results.json')));
}
async function programsJourney(){
 const result=await executeAsync(process.execPath,[path.join('tests','automation_ui.cjs'),binary,automationOutput],{env:{...process.env,PUPPETEER_MODULE:require.resolve('puppeteer')},timeout:110000});
 fs.writeFileSync(path.join(output,'programs-run.log'),result.stdout+result.stderr);
 return JSON.parse(fs.readFileSync(path.join(automationOutput,'results.json')));
}

(async()=>{try{
 const engine=engineProbe(),cli=cliProbe(),[graph,programs]=await Promise.all([graphJourney(),programsJourney()]);
 const names=['fr-sombre','fr-etat','en-sombre','en-etat'];
 assert.deepEqual(graph.variants.map(row=>row.variant),names);assert.deepEqual(programs.variants.map(row=>row.variant),names);
 const variants=names.map(name=>{
  const graphRow=graph.variants.find(row=>row.variant===name),programRow=programs.variants.find(row=>row.variant===name);
  const journal=JSON.parse(fs.readFileSync(path.join(graphOutput,name+'-d01-journal.json')));
  assert.ok(journal.some(row=>row.visible&&row.actor&&row.action&&row.reason));
  const screenshot=name+'.png';fs.copyFileSync(path.join(automationOutput,programRow.screenshot),path.join(output,screenshot));
  const assertions={
   prepare_roles:cli.prepare_roles.agent_count===0,
   dependency_apply:graphRow.assertions.preview_apply&&cli.dependency_apply.revision>0,
   cycle_rejected:cli.cycle_rejected.code==='dependency_cycle',
   agent_journal:journal.some(row=>row.visible&&row.actor&&row.action&&row.reason),
   program_replay:programRow.assertions.save_reload,
   content_conflict:graphRow.assertions.conflict&&cli.content_conflict.code==='event_conflict',
   workspace_wait:engine.tests.includes('TestGraphDeliveryD01WorkspaceWait'),
   restart_retention:engine.tests.includes('TestGraphDeliveryD01RestartRetention')&&programRow.assertions.save_reload,
   causal_recovery:engine.tests.includes('TestGraphDeliveryD01ProofFreshness'),
   unknown_cost:graphRow.assertions.unknown_cost_projected&&programRow.assertions.unknown_cost,
   focus_restore:graphRow.assertions.focus_restore&&programRow.assertions.focus_restore,
   graph_preserved:programRow.assertions.graph_preserved
  };
  assert.ok(Object.values(assertions).every(Boolean),`${name}: incomplete D01 assertions`);
  return {variant:name,assertions,screenshot,console_errors:[...graphRow.console_errors,...programRow.console_errors],network_errors:[...graphRow.network_errors,...programRow.network_errors],evidence:{prepare_roles:cli.prepare_roles,dependency_apply:cli.dependency_apply,cycle_rejected:cli.cycle_rejected,agent_journal:{rows:journal.filter(row=>row.visible).length,path:`graph/${name}-d01-journal.json`},program_replay:{schedule_id:programRow.schedule_id,save_reload:programRow.assertions.save_reload},content_conflict:cli.content_conflict,workspace_wait:{engine_test:'TestGraphDeliveryD01WorkspaceWait',observed:engine.tests.includes('TestGraphDeliveryD01WorkspaceWait')},restart_retention:{engine_test:'TestGraphDeliveryD01RestartRetention',observed:engine.tests.includes('TestGraphDeliveryD01RestartRetention'),program_reload:programRow.assertions.save_reload},causal_recovery:{engine_test:'TestGraphDeliveryD01ProofFreshness',observed:engine.tests.includes('TestGraphDeliveryD01ProofFreshness')},unknown_cost:{graph:graphRow.assertions.unknown_cost_projected,program:programRow.assertions.unknown_cost},focus_restore:{graph:graphRow.assertions.focus_restore,program:programRow.assertions.focus_restore},graph_preserved:{program:programRow.assertions.graph_preserved}}};
 });
 const receipt={schema_version:1,surface:'swarm-product',environment:{platform:`${os.platform()} ${os.release()} ${os.arch()}`,binary,browser:programs.environment.browser,isolated_roots:true,provider_started:false},engine_preconditions:engine,cli_probe:cli,variants,limits:['fixture provider only; no real provider autonomy','workspace wait and causal recovery are engine assertions re-executed in this combined receipt, not browser-triggerable operations']};
 fs.writeFileSync(path.join(output,'results.json'),JSON.stringify(receipt,null,2));console.log(JSON.stringify(receipt,null,2));
}catch(error){fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({error:error.stack,scratch},null,2));console.error(error);process.exitCode=1}})();

```
