//go:build linux

package main

import (
	"crypto/subtle"
	"embed"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

//go:embed web/*
var cockpitWeb embed.FS

type webRequest struct {
	Level           string        `json:"level,omitempty"`
	ModelPolicyHash string        `json:"model_policy_hash,omitempty"`
	PlanBriefHash   string        `json:"plan_brief_hash,omitempty"`
	Plan            PlanReview    `json:"plan,omitempty"`
	References      []DialogueRef `json:"references,omitempty"`
	ContextHash     string        `json:"context_hash,omitempty"`
	Retex           Retex         `json:"retex,omitempty"`
	Offset          int           `json:"offset,omitempty"`
	Capture         bool          `json:"capture_output"`
	Kind            string        `json:"kind"`
	Work            string        `json:"work"`
	Task            string        `json:"task"`
	Agent           string        `json:"agent"`
	Event           string        `json:"event_id"`
	Revision        int           `json:"expected_revision"`
	Provider        string        `json:"provider"`
	Role            string        `json:"role"`
	Workspace       string        `json:"workspace"`
	Instruction     string        `json:"instruction"`
	Path            string        `json:"path"`
	Name            string        `json:"name"`
	Note            string        `json:"note"`
	Author          string        `json:"author"`
	Decision        string        `json:"decision"`
	Request         Request       `json:"request"`
	Budget          Budget        `json:"budget"`
	Autonomy        string        `json:"autonomy,omitempty"`
	Slots           int           `json:"slots,omitempty"`
}

func (s *Store) webAction(r webRequest) (any, error) {
	if r.Kind == "plan-read" {
		return s.readPlan(r.Work, r.Task)
	}
	if r.Kind == "plan-commit" {
		return s.commitPlan(r.Work, r.Event, r.Revision, r.Plan)
	}
	if r.Kind == "task" {
		r.Request.Schema = 1
		r.Request.EventID = r.Event
		r.Request.Revision = r.Revision
		r.Request.ID = r.Task
		// Personne ne se declare moteur depuis l'exterieur : ce qui arrive par
		// le reseau est un geste humain, quel que soit le champ envoye.
		r.Request.Origin = ""
		return s.executeRequest(r.Work, "task.update", r.Request)
	}
	if strings.HasPrefix(r.Kind, "retex-") {
		return s.retexAction(r)
	}
	if r.Kind == "dialogue-search" {
		w, e := s.get(r.Work)
		if e != nil {
			return nil, e
		}
		return searchDialogue(w, r.Note, r.Offset), nil
	}
	if r.Kind == "context-preview" {
		a, _, e := s.prepareLaunch(r.Work, Launch{Level: r.Level, ModelPolicyHash: r.ModelPolicyHash, PlanBriefHash: r.PlanBriefHash, Brainstorm: true, Schema: 1, EventID: r.Event, Revision: r.Revision, Provider: r.Provider, Workspace: r.Workspace, Instruction: r.Instruction, Capture: r.Capture, References: r.References}, true)
		return a, e
	}
	if r.Kind == "start" || r.Kind == "brainstorm" {
		if r.Kind == "brainstorm" && r.ContextHash == "" {
			return nil, fmt.Errorf("Examiner le contexte avant envoi.")
		}
		a, created, e := s.prepare(r.Work, Launch{Level: r.Level, ModelPolicyHash: r.ModelPolicyHash, PlanBriefHash: r.PlanBriefHash, References: r.References, ContextHash: r.ContextHash, Brainstorm: r.Kind == "brainstorm", Schema: 1, EventID: r.Event, Revision: r.Revision, TaskID: r.Task, Provider: r.Provider, Role: r.Role, Workspace: r.Workspace, Instruction: r.Instruction, Capture: r.Capture})
		if e != nil {
			return nil, e
		}
		if created {
			e = s.spawnAgent(a)
		}
		return a, e
	}
	w, e := s.get(r.Work)
	if e != nil {
		return nil, e
	}
	if r.Revision != w.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	switch r.Kind {
	case "stop", "reconcile":
		a, e := s.agent(r.Agent)
		if e != nil {
			return nil, e
		}
		if a.WorkID != r.Work {
			return nil, fmt.Errorf("agent hors travail")
		}
		if r.Kind == "stop" {
			e = s.stopAgent(a.ID)
		} else {
			e = s.reconcile(a.ID)
		}
		return map[string]string{"message": "Demande enregistrée ; vérifier l’état observé."}, e
	case "retry":
		a, e := s.agent(r.Agent)
		if e != nil {
			return nil, e
		}
		if a.WorkID != r.Work || activeAgent(a) {
			return nil, fmt.Errorf("tentative incompatible")
		}
		if r.Level == "" && a.ModelRoute != nil {
			r.Level = a.ModelRoute.Level
		}
		next, created, e := s.prepare(r.Work, Launch{Level: r.Level, ModelPolicyHash: r.ModelPolicyHash, Schema: 1, EventID: r.Event, Revision: r.Revision, TaskID: a.TaskID, Provider: a.Provider, Role: a.Role, Workspace: a.CWD, Instruction: r.Instruction, Previous: a.ID, Parent: a.Parent, Capture: r.Capture})
		if e == nil && created {
			e = s.spawnAgent(next)
		}
		return next, e
	case "adopt-brief":
		return s.adoptBrief(r.Work, r.Task, r.Path, r.Note, r.Event, r.Revision)
	case "submit":
		e = s.submitReportAt(r.Work, r.Task, r.Path, r.Revision, "")
	case "gate-preview", "gate":
		t, err := w.task(r.Task)
		if err != nil {
			return nil, err
		}
		if r.Kind == "gate" && strings.TrimSpace(r.Name) == "" {
			return nil, fmt.Errorf("Nom de la gate requis : nom humain de cette évaluation.")
		}
		d := &taskDialog{task: *t, reportPath: r.Path, gateName: strings.TrimSpace(r.Name)}
		if e = s.previewGate(r.Work, d); e != nil {
			return nil, e
		}
		if r.Kind == "gate-preview" {
			return map[string]string{"preview": d.review}, nil
		}
		d.gateRevision = r.Revision
		e = s.recordDialogGate(r.Work, d)
	case "override":
		e = s.overrideReviewedTaskAt(r.Work, r.Task, r.Note, r.Revision)
	case "decision":
		e = s.resolveDecision(r.Work, r.Decision, operatorIdentity(), r.Note)
	case "budget":
		e = s.setBudget(r.Work, r.Budget)
	case "pause":
		e = s.pause(r.Work, true)
	case "unpause":
		if e = s.pause(r.Work, false); e == nil {
			// Reprendre les départs relance immédiatement l'ordonnanceur.
			_, e = s.dispatch(r.Work)
		}
	case "profile":
		e = s.setProfile(r.Work, r.Task, LaunchProfile{Provider: r.Provider, Role: r.Role, Workspace: r.Workspace,
			Instruction: r.Instruction, Level: r.Level, Capture: r.Capture}, r.Revision)
	case "autonomy":
		if e = s.setAutonomy(r.Work, r.Autonomy, r.Slots); e == nil {
			_, e = s.dispatch(r.Work)
		}
	case "dispatch":
		launched, err := s.dispatch(r.Work)
		if err != nil {
			return nil, err
		}
		return map[string]any{"launched": dispatchedIDs(launched),
			"message": fmt.Sprintf("%d départ(s) automatique(s) ; consulter le journal du travail pour les refus.", len(launched))}, nil
	case "ooda":
		r.Request.Schema = 1
		r.Request.EventID = r.Event
		r.Request.Revision = r.Revision
		r.Request.Owner = operatorIdentity()
		return s.executeRequest(r.Work, "ooda", r.Request)
	default:
		return nil, fmt.Errorf("action inconnue")
	}
	return map[string]string{"message": "Action enregistrée."}, e
}
func newWebHandler(s *Store, host, token string) http.Handler {
	mux := http.NewServeMux()
	same := func(a, b string) bool { return subtle.ConstantTimeCompare([]byte(a), []byte(b)) == 1 }
	send := func(w http.ResponseWriter, value any) {
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		_ = json.NewEncoder(w).Encode(value)
	}
	fail := func(w http.ResponseWriter, e error) {
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		status := 400
		if commandFailure(e).Code == "revision_conflict" {
			status = 409
		}
		w.WriteHeader(status)
		send(w, map[string]any{"error": e.Error(), "failure": commandFailure(e)})
	}
	s.registerProviderAdmin(mux, send, fail)
	mux.HandleFunc("/api/v1/works", func(w http.ResponseWriter, r *http.Request) {
		v, e := s.list()
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
	mux.HandleFunc("/api/v1/snapshot", func(w http.ResponseWriter, r *http.Request) {
		work := r.URL.Query().Get("work")
		snapshot, e := s.cockpitSnapshot(work)
		if e != nil {
			fail(w, e)
			return
		}
		ds, e := s.decisions(work)
		if e != nil {
			fail(w, e)
			return
		}
		snapshot["decisions"] = ds
		b, e := s.budget(work)
		if e != nil {
			fail(w, e)
			return
		}
		snapshot["budget"] = b
		snapshot["hierarchy"] = s.hierarchyText(work)
		v, e := s.visit(work, operatorIdentity())
		if e != nil {
			fail(w, e)
			return
		}
		snapshot["resume"] = s.resumeSinceText(work, v)
		// Le cockpit doit pouvoir situer ce qui a changé depuis la visite
		// précédente ; jusqu'ici cette date ne sortait que sous forme de texte.
		snapshot["visit"] = v
		providers, e := s.providers()
		if e == nil {
			snapshot["providers"] = providers
		} else {
			snapshot["providers_error"] = e.Error()
		}
		snapshot["root"] = s.root
		send(w, snapshot)
	})
	mux.HandleFunc("/api/v1/task", func(w http.ResponseWriter, r *http.Request) {
		work := r.URL.Query().Get("work")
		id := r.URL.Query().Get("task")
		ww, e := s.get(work)
		if e != nil {
			fail(w, e)
			return
		}
		t, e := ww.task(id)
		if e != nil {
			fail(w, e)
			return
		}
		d := &taskDialog{task: *t}
		agents, e := s.agents(work)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"revision": ww.Revision, "task": t, "reports": s.taskReports(id), "gates": s.gateFiles(id), "review": s.reviewText(work, d), "actions": s.taskActions(&ww, t, agents)})
	})
	mux.HandleFunc("/api/v1/report", func(w http.ResponseWriter, r *http.Request) {
		p, e := safeReport(s.root, r.URL.Query().Get("path"))
		if e != nil {
			fail(w, e)
			return
		}
		// Only project documentation is readable through this endpoint.
		rel, err := filepath.Rel(filepath.Join(s.root, "docs"), p)
		if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
			http.Error(w, "Rapport hors documentation", 403)
			return
		}
		f, e := os.Open(p)
		if e != nil {
			fail(w, e)
			return
		}
		defer f.Close()
		b, e := io.ReadAll(io.LimitReader(f, 262145))
		if e != nil {
			fail(w, e)
			return
		}
		if len(b) > 262144 {
			b = b[:262144]
			b = append(b, []byte("\n[Affichage limité à 256 Kio]")...)
		}
		send(w, map[string]string{"text": string(b)})
	})
	mux.HandleFunc("/api/v1/logs", func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		after, _ := strconv.ParseInt(q.Get("after"), 10, 64)
		p, e := s.queryLogs(q.Get("work"), q.Get("agent"), q.Get("q"), q.Get("kind"), after, 100)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, p)
	})
	mux.HandleFunc("/api/v1/activity", func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		limit, _ := strconv.Atoi(q.Get("limit"))
		p, e := s.activity(q.Get("work"), activityQuery{
			Before:        q.Get("before"),
			Limit:         limit,
			DecisionsOnly: q.Get("decisions") == "1",
		})
		if e != nil {
			fail(w, e)
			return
		}
		send(w, p)
	})
	mux.HandleFunc("/api/v1/action", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		var request webRequest
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if e != nil {
			fail(w, e)
			return
		}
		if e = strict(raw, &request); e != nil {
			fail(w, e)
			return
		}
		value, e := s.webAction(request)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/assist/meta", func(w http.ResponseWriter, r *http.Request) {
		pages := []map[string]string{}
		for _, p := range assistPages {
			pages = append(pages, map[string]string{"id": p.ID, "title": p.Title, "purpose": p.Purpose})
		}
		templates := []map[string]string{}
		for _, t := range assistTemplates {
			templates = append(templates, map[string]string{"id": t.ID, "label": t.Label, "question": t.Question})
		}
		support := map[string]string{}
		if ps, err := s.providers(); err == nil {
			for name, p := range ps.Providers {
				if _, err := assistantProvider(p); err != nil {
					support[name] = err.Error()
				} else {
					support[name] = ""
				}
			}
		}
		send(w, map[string]any{"contract": pageContextContract, "answer_contract": assistAnswerName, "prompt_version": assistPromptVersion, "provider_support": support, "pages": pages, "templates": templates})
	})
	mux.HandleFunc("/api/v1/assist/turns", func(w http.ResponseWriter, r *http.Request) {
		work := r.URL.Query().Get("work")
		ww, e := s.get(work)
		if e != nil {
			fail(w, e)
			return
		}
		turns, e := s.assistCurrentTurns(work)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"revision": ww.Revision, "turns": turns})
	})

	mux.HandleFunc("/api/v1/assist/context", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		raw := r.URL.Query().Get("coordinates")
		if len(raw) > 4096 {
			http.Error(w, "Coordonnées trop longues", 413)
			return
		}
		var c PageCoordinates
		if e := strict([]byte(raw), &c); e != nil {
			fail(w, e)
			return
		}
		v, e := s.pageContext(r.URL.Query().Get("work"), c)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
	mux.HandleFunc("/api/v1/assist/cancel", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
		if e != nil {
			fail(w, e)
			return
		}
		var q struct {
			Work string `json:"work"`
			ID   string `json:"id"`
		}
		if e = strict(raw, &q); e != nil {
			fail(w, e)
			return
		}
		if e = s.cancelAssist(q.Work, q.ID); e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]bool{"cancelled": true})
	})
	mux.HandleFunc("/api/v1/assist/export", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		work := r.URL.Query().Get("work")
		if _, e := s.get(work); e != nil {
			fail(w, e)
			return
		}
		turns, e := allAssistTurns(s.db, work)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"schema_version": 1, "work_id": work, "turns": turns, "exported_at": now()})
	})
	mux.HandleFunc("/api/v1/assist/ask", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if e != nil {
			fail(w, e)
			return
		}
		var request struct {
			Work    string        `json:"work"`
			Preview bool          `json:"preview"`
			Request AssistRequest `json:"request"`
		}
		if e = strict(raw, &request); e != nil {
			fail(w, e)
			return
		}
		if request.Preview {
			preview, e := s.assistPreview(request.Work, request.Request)
			if e != nil {
				fail(w, e)
				return
			}
			send(w, preview)
			return
		}

		ps, e := s.providers()
		if e != nil {
			fail(w, e)
			return
		}
		if _, e = assistantProvider(ps.Providers[request.Request.Provider]); e != nil {
			fail(w, e)
			return
		}
		turn, e := s.assistAsk(request.Work, request.Request)
		if e != nil {
			fail(w, e)
			return
		}
		if turn.Status == "pending" {
			if e = s.spawnAssistTurn(turn); e != nil {
				fail(w, e)
				return
			}
		}
		send(w, turn)
	})
	mux.HandleFunc("/api/v1/visit", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		ww, e := s.get(r.URL.Query().Get("work"))
		if e != nil {
			fail(w, e)
			return
		}
		if e = s.markVisit(ww.ID, operatorIdentity(), ww.Revision); e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]bool{"saved": true})
	})
	mux.HandleFunc("/api/v1/events", func(w http.ResponseWriter, r *http.Request) {
		work := r.URL.Query().Get("work")
		after, _ := strconv.Atoi(r.Header.Get("Last-Event-ID"))
		if after == 0 {
			after, _ = strconv.Atoi(r.URL.Query().Get("after"))
		}
		if _, e := s.get(work); e != nil {
			fail(w, e)
			return
		}
		w.Header().Set("Content-Type", "text/event-stream")
		flusher, ok := w.(http.Flusher)
		if !ok {
			return
		}
		tick := time.NewTicker(time.Second)
		defer tick.Stop()
		for {
			ww, e := s.get(work)
			if e != nil {
				return
			}
			if ww.Revision > after {
				rows, e := s.db.Query("SELECT revision,kind FROM events WHERE work_id=? AND revision>? ORDER BY revision LIMIT 100", work, after)
				if e != nil {
					return
				}
				for rows.Next() {
					var rev int
					var kind string
					if rows.Scan(&rev, &kind) != nil {
						break
					}
					b, _ := json.Marshal(map[string]any{"revision": rev, "kind": kind})
					fmt.Fprintf(w, "id: %d\nevent: change\ndata: %s\n\n", rev, b)
					after = rev
				}
				rows.Close()
			}
			fmt.Fprintf(w, "event: refresh\ndata: {}\n\n")
			flusher.Flush()
			select {
			case <-r.Context().Done():
				return
			case <-tick.C:
			}
		}
	})
	files, _ := fs.Sub(cockpitWeb, "web")
	mux.Handle("/", http.FileServer(http.FS(files)))
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("Referrer-Policy", "no-referrer")
		w.Header().Set("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")
		if r.Host != host {
			http.Error(w, "Hôte refusé", 403)
			return
		}
		if strings.HasPrefix(r.URL.Path, "/session/") {
			if r.Method != "GET" || !same(strings.TrimPrefix(r.URL.Path, "/session/"), token) {
				http.Error(w, "Session refusée", 403)
				return
			}
			http.SetCookie(w, &http.Cookie{Name: "swarm_session", Value: token, Path: "/", HttpOnly: true, SameSite: http.SameSiteStrictMode})
			http.Redirect(w, r, "/", http.StatusSeeOther)
			return
		}
		cookie, e := r.Cookie("swarm_session")
		if e != nil || !same(cookie.Value, token) {
			http.Error(w, "Ouvrir le lien de session affiché au lancement de swarm web.", 403)
			return
		}
		if r.Method != "GET" && r.Method != "HEAD" {
			if r.Header.Get("Origin") != "http://"+host || !same(r.Header.Get("X-Swarm-CSRF"), token) {
				http.Error(w, "Origine ou confirmation de session refusée", 403)
				return
			}
		}
		if r.URL.Path == "/api/v1/session" {
			send(w, map[string]string{"csrf": token})
			return
		}
		mux.ServeHTTP(w, r)
	})
}
func (s *Store) serveWeb(address string, out io.Writer) error {
	if _, err := os.Stat(filepath.Join(s.root, ".swarm", "providers.json")); os.IsNotExist(err) {
		if err = s.initProviders(); err != nil && !os.IsExist(err) {
			return err
		}
	}
	if address == "" {
		address = "127.0.0.1:0"
	}
	host, _, e := net.SplitHostPort(address)
	if e != nil {
		return e
	}
	if host != "127.0.0.1" && host != "::1" {
		return fmt.Errorf("écoute locale loopback uniquement")
	}
	listener, e := net.Listen("tcp", address)
	if e != nil {
		return e
	}
	defer listener.Close()
	if err := s.assistReconcile(); err != nil {
		return err
	}
	token := newID("session-")
	fmt.Fprintf(out, "Cockpit local : http://%s/session/%s\nArrêter ce serveur ne coupe pas les agents.\n", listener.Addr(), token)
	server := &http.Server{Handler: newWebHandler(s, listener.Addr().String(), token), ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 10 * time.Second, IdleTimeout: 30 * time.Second}
	return server.Serve(listener)
}
