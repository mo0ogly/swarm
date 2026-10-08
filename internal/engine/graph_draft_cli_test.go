//go:build linux

package engine

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/cookiejar"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

type graphDraftCLIResult struct {
	code   int
	stdout []byte
	stderr []byte
}

func graphDraftB02Binary(t *testing.T) string {
	t.Helper()
	binary := filepath.Join(t.TempDir(), "swarm")
	command := repositoryCommand(t, "go", "build", "-o", binary, "./cmd/swarm")
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("build réel: %v\n%s", err, output)
	}
	return binary
}

func graphDraftB02Root(t *testing.T) (string, Work) {
	t.Helper()
	root := t.TempDir()
	store, err := openStore(root, true)
	if err != nil {
		t.Fatal(err)
	}
	work := graphDraftWork(t, store)
	if err = store.db.Close(); err != nil {
		t.Fatal(err)
	}
	return root, work
}

func graphDraftB02JSONFile(t *testing.T, value any) string {
	t.Helper()
	raw, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(t.TempDir(), "request.json")
	if err = os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

func runGraphDraftB02CLI(t *testing.T, binary string, args ...string) graphDraftCLIResult {
	t.Helper()
	command := exec.Command(binary, args...)
	command.Dir = t.TempDir()
	var stdout, stderr bytes.Buffer
	command.Stdout, command.Stderr = &stdout, &stderr
	err := command.Run()
	code := 0
	if err != nil {
		var exit *exec.ExitError
		if !strings.Contains(err.Error(), "exit status") || !asExitError(err, &exit) {
			t.Fatalf("processus CLI: %v", err)
		}
		code = exit.ExitCode()
	}
	return graphDraftCLIResult{code: code, stdout: stdout.Bytes(), stderr: stderr.Bytes()}
}

func asExitError(err error, target **exec.ExitError) bool {
	exit, ok := err.(*exec.ExitError)
	if ok {
		*target = exit
	}
	return ok
}

func decodeGraphDraftB02[T any](t *testing.T, raw []byte) T {
	t.Helper()
	var value T
	if err := json.Unmarshal(raw, &value); err != nil {
		t.Fatalf("JSON invalide: %v\n%s", err, raw)
	}
	return value
}

func importGraphDraftB02(t *testing.T, binary, root string, work Work, op GraphDraftOperation) GraphDraft {
	t.Helper()
	request := GraphDraftSaveRequest{Schema: 1, WorkID: work.ID, ExpectedRevision: work.Revision, Operations: []GraphDraftOperation{op}}
	result := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "import", work.ID, "--input", graphDraftB02JSONFile(t, request))
	if result.code != 0 {
		t.Fatalf("import code=%d stderr=%s", result.code, result.stderr)
	}
	return decodeGraphDraftB02[GraphDraft](t, result.stdout)
}

func previewGraphDraftB02(t *testing.T, binary, root string, work Work, draft GraphDraft) GraphDraftPreview {
	t.Helper()
	request := GraphDraftPreviewRequest{Schema: 1, WorkID: work.ID, DraftID: draft.ID, ExpectedRevision: work.Revision}
	result := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "preview", work.ID, "--input", graphDraftB02JSONFile(t, request))
	if result.code != 0 {
		t.Fatalf("preview code=%d stderr=%s", result.code, result.stderr)
	}
	return decodeGraphDraftB02[GraphDraftPreview](t, result.stdout)
}

type graphDraftB02Server struct {
	base, token string
	client      *http.Client
	command     *exec.Cmd
	stderr      bytes.Buffer
}

func startGraphDraftB02Server(t *testing.T, binary, root string) *graphDraftB02Server {
	t.Helper()
	command := exec.Command(binary, "--root", root, "web", "127.0.0.1:0")
	stdout, err := command.StdoutPipe()
	if err != nil {
		t.Fatal(err)
	}
	server := &graphDraftB02Server{command: command}
	command.Stderr = &server.stderr
	if err = command.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { server.stop() })
	line := make(chan string, 1)
	readErr := make(chan error, 1)
	go func() {
		value, read := bufio.NewReader(stdout).ReadString('\n')
		line <- value
		readErr <- read
	}()
	select {
	case value := <-line:
		if err = <-readErr; err != nil {
			t.Fatalf("sortie serveur: %v stderr=%s", err, server.stderr.String())
		}
		fields := strings.Fields(value)
		if len(fields) != 4 {
			t.Fatalf("URL serveur absente: %q stderr=%s", value, server.stderr.String())
		}
		sessionURL := fields[3]
		parsed, parseErr := url.Parse(sessionURL)
		if parseErr != nil {
			t.Fatal(parseErr)
		}
		server.base = parsed.Scheme + "://" + parsed.Host
		server.token = strings.TrimPrefix(parsed.Path, "/session/")
		jar, jarErr := cookiejar.New(nil)
		if jarErr != nil {
			t.Fatal(jarErr)
		}
		server.client = &http.Client{Jar: jar, Timeout: 5 * time.Second}
		response, getErr := server.client.Get(sessionURL)
		if getErr != nil {
			t.Fatal(getErr)
		}
		response.Body.Close()
	case <-time.After(8 * time.Second):
		t.Fatalf("serveur muet; stderr=%s", server.stderr.String())
	}
	return server
}

func (s *graphDraftB02Server) stop() {
	if s.command == nil || s.command.Process == nil || s.command.ProcessState != nil {
		return
	}
	_ = s.command.Process.Kill()
	_ = s.command.Wait()
}

func (s *graphDraftB02Server) post(t *testing.T, path string, value any) (int, []byte) {
	t.Helper()
	raw, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	request, err := http.NewRequest(http.MethodPost, s.base+path, bytes.NewReader(raw))
	if err != nil {
		t.Fatal(err)
	}
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("Origin", s.base)
	request.Header.Set("X-Swarm-CSRF", s.token)
	response, err := s.client.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	body, err := io.ReadAll(response.Body)
	if err != nil {
		t.Fatal(err)
	}
	return response.StatusCode, body
}

func TestGraphDraftB02CLIHTTPParity(t *testing.T) {
	binary := graphDraftB02Binary(t)
	cliRoot, cliWork := graphDraftB02Root(t)
	httpRoot, httpWork := graphDraftB02Root(t)
	op := GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}

	cliDraft := importGraphDraftB02(t, binary, cliRoot, cliWork, op)
	staleCLIPreview := GraphDraftPreviewRequest{Schema: 1, WorkID: cliWork.ID, DraftID: cliDraft.ID, ExpectedRevision: cliWork.Revision + 1}
	cliConflict := runGraphDraftB02CLI(t, binary, "--root", cliRoot, "--json", "plan", "draft", "preview", cliWork.ID, "--input", graphDraftB02JSONFile(t, staleCLIPreview))
	if cliConflict.code != 3 {
		t.Fatalf("conflit CLI non distingué: code=%d stderr=%s", cliConflict.code, cliConflict.stderr)
	}
	cliFailure := decodeGraphDraftB02[struct {
		Failure CommandError `json:"failure"`
	}](t, cliConflict.stderr)
	compare := runGraphDraftB02CLI(t, binary, "--root", cliRoot, "--json", "plan", "draft", "compare", cliWork.ID, cliDraft.ID)
	if compare.code != 0 {
		t.Fatalf("compare code=%d stderr=%s", compare.code, compare.stderr)
	}
	comparison := decodeGraphDraftB02[GraphDraftComparison](t, compare.stdout)
	if !comparison.NoImplicitLaunch || comparison.PreviewToken == "" || len(comparison.AffectedTasks) == 0 {
		t.Fatalf("comparaison non discriminante: %#v", comparison)
	}
	cliApply := GraphDraftApplyRequest{Schema: 1, WorkID: cliWork.ID, DraftID: cliDraft.ID, EventID: "b02-cli-apply", ExpectedRevision: cliWork.Revision, PreviewToken: comparison.PreviewToken, ContentDigest: comparison.ContentDigest}
	cliResult := runGraphDraftB02CLI(t, binary, "--root", cliRoot, "--json", "plan", "draft", "apply", cliWork.ID, "--input", graphDraftB02JSONFile(t, cliApply))
	if cliResult.code != 0 {
		t.Fatalf("apply CLI code=%d stderr=%s", cliResult.code, cliResult.stderr)
	}

	server := startGraphDraftB02Server(t, binary, httpRoot)
	status, body := server.post(t, "/api/v1/graph-drafts", GraphDraftSaveRequest{Schema: 1, WorkID: httpWork.ID, ExpectedRevision: httpWork.Revision, Operations: []GraphDraftOperation{op}})
	if status != http.StatusOK {
		t.Fatalf("save HTTP=%d body=%s", status, body)
	}
	httpDraft := decodeGraphDraftB02[GraphDraft](t, body)
	status, body = server.post(t, "/api/v1/graph-drafts/preview", GraphDraftPreviewRequest{Schema: 1, WorkID: httpWork.ID, DraftID: httpDraft.ID, ExpectedRevision: httpWork.Revision + 1})
	httpFailure := decodeGraphDraftB02[struct {
		Failure CommandError `json:"failure"`
	}](t, body)
	if status != http.StatusConflict || httpFailure.Failure.Code != "revision_conflict" || cliFailure.Failure.Code != httpFailure.Failure.Code {
		t.Fatalf("parité conflit HTTP=%d/%s CLI=%d/%s", status, httpFailure.Failure.Code, cliConflict.code, cliFailure.Failure.Code)
	}
	status, body = server.post(t, "/api/v1/graph-drafts/preview", GraphDraftPreviewRequest{Schema: 1, WorkID: httpWork.ID, DraftID: httpDraft.ID, ExpectedRevision: httpWork.Revision})
	if status != http.StatusOK {
		t.Fatalf("preview HTTP=%d body=%s", status, body)
	}
	httpPreview := decodeGraphDraftB02[GraphDraftPreview](t, body)
	status, body = server.post(t, "/api/v1/graph-drafts/apply", GraphDraftApplyRequest{Schema: 1, WorkID: httpWork.ID, DraftID: httpDraft.ID, EventID: "b02-http-apply", ExpectedRevision: httpWork.Revision, PreviewToken: httpPreview.PreviewToken, ContentDigest: httpPreview.ContentDigest})
	if status != http.StatusOK {
		t.Fatalf("apply HTTP=%d body=%s", status, body)
	}

	server.stop()
	for name, root := range map[string]string{"cli": cliRoot, "http": httpRoot} {
		store, err := openStore(root, false)
		if err != nil {
			t.Fatal(err)
		}
		workID := cliWork.ID
		if name == "http" {
			workID = httpWork.ID
		}
		current, err := store.get(workID)
		if err != nil || !taskDependsOn(t, current, "t2", "t1") {
			store.db.Close()
			t.Fatalf("effet %s absent: %v", name, err)
		}
		var attempts int
		if err = store.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", workID).Scan(&attempts); err != nil || attempts != 0 {
			store.db.Close()
			t.Fatalf("%s départ implicite: %d %v", name, attempts, err)
		}
		store.db.Close()
	}
}

func TestGraphDraftB02InvalidInputs(t *testing.T) {
	binary := graphDraftB02Binary(t)
	root, work := graphDraftB02Root(t)
	oversized := filepath.Join(t.TempDir(), "too-large.json")
	if err := os.WriteFile(oversized, bytes.Repeat([]byte{'x'}, graphDraftCLIFileLimit+1), 0600); err != nil {
		t.Fatal(err)
	}
	tooLarge := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "import", work.ID, "--input", oversized)
	if tooLarge.code != 2 || !bytes.Contains(tooLarge.stderr, []byte(`"code": "invalid_input"`)) {
		t.Fatalf("borne absente: code=%d stderr=%s", tooLarge.code, tooLarge.stderr)
	}
	invalid := graphDraftB02JSONFile(t, map[string]any{"schema_version": 1, "work_id": work.ID, "expected_revision": work.Revision, "operations": []any{}, "unexpected": true})
	bad := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "import", work.ID, "--input", invalid)
	if bad.code != 2 || !bytes.Contains(bad.stderr, []byte(`"code": "invalid_input"`)) {
		t.Fatalf("champ inconnu accepté: code=%d stderr=%s", bad.code, bad.stderr)
	}

	draft := importGraphDraftB02(t, binary, root, work, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	exported := filepath.Join(t.TempDir(), "draft.json")
	exportResult := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "export", work.ID, draft.ID, "--output", exported)
	if exportResult.code != 0 {
		t.Fatalf("export code=%d stderr=%s", exportResult.code, exportResult.stderr)
	}
	stat, err := os.Stat(exported)
	if err != nil || stat.Size() > graphDraftCLIFileLimit {
		t.Fatalf("export non borné: size=%v err=%v", stat, err)
	}
	roundTrip := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "import", work.ID, "--input", exported)
	if roundTrip.code != 0 {
		t.Fatalf("réimport code=%d stderr=%s", roundTrip.code, roundTrip.stderr)
	}
	importedAgain := decodeGraphDraftB02[GraphDraft](t, roundTrip.stdout)
	if importedAgain.ID == draft.ID || importedAgain.ContentDigest != draft.ContentDigest {
		t.Fatalf("aller-retour export/import incorrect: %#v", importedAgain)
	}
	preview := previewGraphDraftB02(t, binary, root, work, draft)
	apply := GraphDraftApplyRequest{Schema: 1, WorkID: work.ID, DraftID: draft.ID, EventID: "b02-replay", ExpectedRevision: work.Revision, PreviewToken: preview.PreviewToken, ContentDigest: preview.ContentDigest}
	requestPath := graphDraftB02JSONFile(t, apply)
	first := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "apply", work.ID, "--input", requestPath)
	second := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "apply", work.ID, "--input", requestPath)
	if first.code != 0 || second.code != 0 || !bytes.Equal(first.stdout, second.stdout) {
		t.Fatalf("rejeu non idempotent: first=%d second=%d", first.code, second.code)
	}
	apply.ContentDigest = strings.Repeat("0", len(apply.ContentDigest))
	conflict := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "apply", work.ID, "--input", graphDraftB02JSONFile(t, apply))
	if conflict.code != 3 || !bytes.Contains(conflict.stderr, []byte(`"code": "event_conflict"`)) {
		t.Fatalf("conflit de rejeu absent: code=%d stderr=%s", conflict.code, conflict.stderr)
	}
}

func TestGraphDraftB02UndoNoAttempts(t *testing.T) {
	binary := graphDraftB02Binary(t)
	root, work := graphDraftB02Root(t)
	draft := importGraphDraftB02(t, binary, root, work, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	undo := graphDraftEditRequest{Schema: 1, ExpectedRevision: work.Revision, ExpectedDraftRevision: draft.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t3"}}}
	result := runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "undo", work.ID, draft.ID, "--input", graphDraftB02JSONFile(t, undo))
	if result.code != 0 {
		t.Fatalf("undo code=%d stderr=%s", result.code, result.stderr)
	}
	undone := decodeGraphDraftB02[graphDraftEditResult](t, result.stdout)
	if undone.Action != "undo" || undone.Draft.Revision != 2 || undone.Draft.PreviewToken != "" {
		t.Fatalf("undo non durable: %#v", undone)
	}
	redo := graphDraftEditRequest{Schema: 1, ExpectedRevision: work.Revision, ExpectedDraftRevision: undone.Draft.Revision, Operations: draft.Operations}
	result = runGraphDraftB02CLI(t, binary, "--root", root, "--json", "plan", "draft", "redo", work.ID, draft.ID, "--input", graphDraftB02JSONFile(t, redo))
	if result.code != 0 {
		t.Fatalf("redo code=%d stderr=%s", result.code, result.stderr)
	}
	redone := decodeGraphDraftB02[graphDraftEditResult](t, result.stdout)
	if redone.Action != "redo" || redone.Draft.Revision != 3 || redone.Draft.ContentDigest != draft.ContentDigest {
		t.Fatalf("redo incorrect: %#v", redone)
	}
	store, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer store.db.Close()
	current, err := store.get(work.ID)
	if err != nil || current.Revision != work.Revision || taskDependsOn(t, current, "t2", "t1") {
		t.Fatalf("édition a modifié le plan: rev=%d err=%v", current.Revision, err)
	}
	var attempts int
	if err = store.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", work.ID).Scan(&attempts); err != nil || attempts != 0 {
		t.Fatalf("undo/redo a modifié les tentatives: %d %v", attempts, err)
	}
}

func TestGraphDraftB02BilingualHelp(t *testing.T) {
	binary := graphDraftB02Binary(t)
	fr := runGraphDraftB02CLI(t, binary, "--lang", "fr", "aide", "brouillons")
	en := runGraphDraftB02CLI(t, binary, "--lang", "en", "help", "drafts")
	if fr.code != 0 || !bytes.Contains(fr.stdout, []byte("PRÉPARER LES DÉPENDANCES")) || !bytes.Contains(fr.stdout, []byte("plan draft undo|redo")) {
		t.Fatalf("aide FR absente: code=%d out=%s err=%s", fr.code, fr.stdout, fr.stderr)
	}
	if en.code != 0 || !bytes.Contains(en.stdout, []byte("PREPARE PLAN DEPENDENCIES")) || !bytes.Contains(en.stdout, []byte("plan draft undo|redo")) || bytes.Contains(en.stdout, []byte("Appliquer explicitement")) {
		t.Fatalf("English help missing: code=%d out=%s err=%s", en.code, en.stdout, en.stderr)
	}
	for _, language := range []string{"fr", "en"} {
		result := runGraphDraftB02CLI(t, binary, "--lang", language, "plan", "draft", "unknown", "work")
		if result.code != 2 {
			t.Fatalf("%s code erreur=%d stderr=%s", language, result.code, result.stderr)
		}
		if language == "en" && (!bytes.Contains(result.stderr, []byte("Error:")) || !bytes.Contains(result.stderr, []byte("usage: plan draft"))) {
			t.Fatalf("erreur EN non traduite: %s", result.stderr)
		}
		if language == "fr" && (!bytes.Contains(result.stderr, []byte("Erreur :")) || !bytes.Contains(result.stderr, []byte("usage : plan draft"))) {
			t.Fatalf("erreur FR absente: %s", result.stderr)
		}
	}
}

func TestGraphDraftB02InvalidCommandDoesNotOpenStore(t *testing.T) {
	binary := graphDraftB02Binary(t)
	for _, pos := range [][]string{
		{"plan", "draft", "unknown", "work"},
		{"plan", "draft", "show", "work"},
		{"plan", "draft", "import", "work", "extra"},
		{"plan", "draft"},
	} {
		root := t.TempDir()
		args := append([]string{"--root", root, "--json"}, pos...)
		result := runGraphDraftB02CLI(t, binary, args...)
		if result.code != 2 || !bytes.Contains(result.stderr, []byte(`"code": "invalid_input"`)) {
			t.Fatalf("invalid command opened storage or returned wrong error: %v code=%d stderr=%s", pos, result.code, result.stderr)
		}
		if _, err := os.Lstat(filepath.Join(root, ".swarm")); !os.IsNotExist(err) {
			t.Fatalf("invalid command created storage: %v err=%v", pos, err)
		}
	}
}

func Example_graphDraftCLI() {
	fmt.Println("swarm plan draft compare WORK DRAFT --json")
	// Output: swarm plan draft compare WORK DRAFT --json
}
