# Dossier de revue — B02

## Identité et périmètre

- Mission/tâche/tentative/producteur : `w-768ed45d1845fbec90bea04d` / `B02` / `a-004ced44ad642c134969524b` / `auto-a0cb585aa2672d49a728`.
- Base Git : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed` ; checkout sale préexistant conservé.
- Candidat : fichiers ci-dessous non commités. Ce dossier n'est ni une revue indépendante ni une acceptation.
- Choix : `plan draft` appelle directement les méthodes B01. Compare réutilise `previewGraphDraft` et `graphState`; undo/redo réécrivent seulement une proposition via `saveGraphDraft`, jamais un effet appliqué.

## Empreintes SHA-256

| Fichier | SHA-256 |
| --- | --- |
| graph_draft_cli.go | `3b3e3de387c052f6150a1ad1858deb25614c7363da2e724fcb19a79c3d5593d7` |
| graph_draft_cli_test.go | `45f1946e8152290697e697db91349cc7edf5ac02f5c721dd9f12545a934fa91d` |
| graph_draft.go (B01 inchangé) | `b290e64625c496c763432aaf83af9cf0288dab11ff2296c6df5a2980cd7edb8b` |
| graph_draft_http.go (B01 inchangé, reproduit ci-dessous) | `88e40ee9dea2525a9051e1f190871469de314233951937381633cbdd5477ac35` |
| main.go | `39f33922b50e17edff317115c6362867028732953c30546dbad2fd73f90dde68` |
| console_cli.go | `fbfbb60584b772669888b1cfb0f72cc51d5e2988b143a72c434b26913d569f25` |
| cli_help_topics.go | `b60b1055b46f122480deb8640e19c5d4f96500c4b64e178b55b3de6c75fa6696` |
| locales/en.json | `924057902a22e48bfc077e9a29f31aed5f59e2df39da350ab201587f09863d1c` |
| GUIDE-UTILISATEUR.md | `b8075bddc16723fb7412a664bb78528edf6828aa790fc04dac7cd06ba0d7c87e` |
| docs/en/USER-GUIDE.md | `551f929df31d7053de5d94dc705f79fdbc4a4241959c723874b32188acf02720` |

## Contrôles observés

- `go test ./... -run '^TestGraphDraftB02(InvalidInputs|UndoNoAttempts|BilingualHelp)$' -count=1 -json -timeout=120s` : exit 0 ; trois tests PASS.
- `go test ./... -run '^$' -count=1 -timeout=120s` : exit 0 ; compilation de tous les tests.
- `git diff --check` : exit 0.
- `python3 docs/plans/graphe-automatisation-20261005/execution/b/verify.py B02` : exit 1. Le vrai serveur échoue à `listen tcp 127.0.0.1:0: socket: operation not permitted`; les trois autres tests passent. La parité HTTP reste NOT TESTED dans ce sandbox et doit être rejouée une fois par le conducteur hôte.
- Aucune suite complète, aucun navigateur, aucun fournisseur réel et aucune revue indépendante exécutés par ce worker.

## CLI

```go
//go:build linux

package main

import (
        "bytes"
        "encoding/json"
        "fmt"
        "io"
        "os"
        "sort"
)

const graphDraftCLIFileLimit = 64 << 10

type GraphDraftComparison struct {
        Schema             int                   `json:"schema_version"`
        WorkID             string                `json:"work_id"`
        DraftID            string                `json:"draft_id"`
        BaseRevision       int                   `json:"base_revision"`
        CurrentRevision    int                   `json:"current_revision"`
        ContentDigest      string                `json:"content_digest"`
        PreviewToken       string                `json:"preview_token"`
        Operations         []GraphDraftOperation `json:"operations"`
        AffectedTasks      []string              `json:"affected_tasks"`
        DependenciesBefore map[string][]string   `json:"dependencies_before"`
        DependenciesAfter  map[string][]string   `json:"dependencies_after"`
        NoImplicitLaunch   bool                  `json:"no_implicit_launch"`
}

type graphDraftEditRequest struct {
        Schema                int                   `json:"schema_version"`
        ExpectedRevision      int                   `json:"expected_revision"`
        ExpectedDraftRevision int                   `json:"expected_draft_revision"`
        Operations            []GraphDraftOperation `json:"operations"`
}

type graphDraftEditResult struct {
        Schema int        `json:"schema_version"`
        Action string     `json:"action"`
        Draft  GraphDraft `json:"draft"`
}

func readGraphDraftCLIInput(path string) ([]byte, error) {
        if path == "" {
                return nil, graphDraftError("invalid_input", "--input requis")
        }
        var reader io.Reader = os.Stdin
        if path != "-" {
                file, err := os.Open(path)
                if err != nil {
                        return nil, graphDraftError("invalid_input", err.Error())
                }
                defer file.Close()
                reader = file
        }
        raw, err := io.ReadAll(io.LimitReader(reader, graphDraftCLIFileLimit+1))
        if err != nil {
                return nil, graphDraftError("invalid_input", err.Error())
        }
        if len(raw) > graphDraftCLIFileLimit {
                return nil, graphDraftError("invalid_input", "entrée de brouillon supérieure à 64 Kio")
        }
        return raw, nil
}

func writeGraphDraftCLIOutput(path string, value any) error {
        if path == "" || path == "-" {
                return graphDraftError("invalid_input", "--output doit désigner un fichier")
        }
        raw, err := json.MarshalIndent(value, "", "  ")
        if err != nil {
                return err
        }
        raw = append(raw, '\n')
        if len(raw) > graphDraftCLIFileLimit {
                return graphDraftError("invalid_input", "export de brouillon supérieur à 64 Kio")
        }
        if err = atomicWrite(path, raw); err != nil {
                return graphDraftError("invalid_input", err.Error())
        }
        return nil
}

func graphDraftDependencies(work Work) map[string][]string {
        result := make(map[string][]string, len(work.Tasks))
        for _, task := range work.Tasks {
                values := append([]string(nil), task.Depends...)
                sort.Strings(values)
                result[task.ID] = values
        }
        return result
}

func (s *Store) compareGraphDraft(workID, draftID string) (GraphDraftComparison, error) {
        var result GraphDraftComparison
        work, err := s.get(workID)
        if err != nil {
                return result, err
        }
        draft, err := s.getGraphDraft(operatorIdentity(), workID, draftID)
        if err != nil {
                return result, err
        }
        preview, err := s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{
                Schema: 1, WorkID: workID, DraftID: draftID, ExpectedRevision: work.Revision,
        })
        if err != nil {
                return result, err
        }
        after, _, err := graphState(&work, draft.Operations)
        if err != nil {
                return result, err
        }
        for id := range after {
                sort.Strings(after[id])
        }
        return GraphDraftComparison{
                Schema: 1, WorkID: workID, DraftID: draftID, BaseRevision: draft.BaseRevision,
                CurrentRevision: work.Revision, ContentDigest: preview.ContentDigest,
                PreviewToken: preview.PreviewToken, Operations: preview.Operations,
                AffectedTasks: preview.AffectedTasks, DependenciesBefore: graphDraftDependencies(work),
                DependenciesAfter: after, NoImplicitLaunch: preview.NoImplicitLaunch,
        }, nil
}

func decodeGraphDraftCLI(raw []byte, value any) error {
        if len(bytes.TrimSpace(raw)) == 0 {
                return graphDraftError("invalid_input", "document JSON requis")
        }
        if err := strict(raw, value); err != nil {
                return graphDraftError("invalid_input", err.Error())
        }
        return nil
}

func graphDraftCLIUsage() error {
        return graphDraftError("invalid_input", "usage : plan draft show|export|import|compare|preview|apply|undo|redo WORK [DRAFT] [--input fichier.json] [--output fichier.json]")
}

func validateGraphDraftCLISyntax(pos []string) error {
        if len(pos) < 4 || len(pos) > 5 || pos[0] != "plan" || pos[1] != "draft" || !safeName(pos[3]) {
                return graphDraftCLIUsage()
        }
        switch pos[2] {
        case "show", "export", "compare", "undo", "redo":
                if len(pos) != 5 || !safeName(pos[4]) {
                        return graphDraftCLIUsage()
                }
        case "import", "preview", "apply":
                if len(pos) != 4 {
                        return graphDraftCLIUsage()
                }
        default:
                return graphDraftCLIUsage()
        }
        return nil
}

func (s *Store) graphDraftCLI(pos []string, input, output string, out io.Writer) error {
        if err := validateGraphDraftCLISyntax(pos); err != nil {
                return err
        }
        action, workID := pos[2], pos[3]
        draftID := ""
        if len(pos) == 5 {
                draftID = pos[4]
        } else if len(pos) != 4 {
                return graphDraftCLIUsage()
        }
        switch action {
        case "show":
                if draftID == "" {
                        return graphDraftCLIUsage()
                }
                value, err := s.getGraphDraft(operatorIdentity(), workID, draftID)
                if err != nil {
                        return err
                }
                return printJSON(out, value)
        case "export":
                if draftID == "" {
                        return graphDraftCLIUsage()
                }
                value, err := s.getGraphDraft(operatorIdentity(), workID, draftID)
                if err != nil {
                        return err
                }
                work, err := s.get(workID)
                if err != nil {
                        return err
                }
                exported := GraphDraftSaveRequest{Schema: 1, WorkID: workID, ExpectedRevision: work.Revision, Operations: value.Operations}
                if err = writeGraphDraftCLIOutput(output, exported); err != nil {
                        return err
                }
                return printJSON(out, map[string]any{"schema_version": 1, "draft_id": value.ID, "output": output})
        case "import":
                if draftID != "" {
                        return graphDraftCLIUsage()
                }
                raw, err := readGraphDraftCLIInput(input)
                if err != nil {
                        return err
                }
                var request GraphDraftSaveRequest
                if err = decodeGraphDraftCLI(raw, &request); err != nil {
                        return err
                }
                if request.WorkID == "" {
                        request.WorkID = workID
                }
                if request.WorkID != workID {
                        return graphDraftError("invalid_input", "work_id ne correspond pas à WORK")
                }
                value, err := s.saveGraphDraft(operatorIdentity(), request)
                if err != nil {
                        return err
                }
                return printJSON(out, value)
        case "compare":
                if draftID == "" {
                        return graphDraftCLIUsage()
                }
                value, err := s.compareGraphDraft(workID, draftID)
                if err != nil {
                        return err
                }
                return printJSON(out, value)
        case "preview":
                if draftID != "" {
                        return graphDraftCLIUsage()
                }
                raw, err := readGraphDraftCLIInput(input)
                if err != nil {
                        return err
                }
                var request GraphDraftPreviewRequest
                if err = decodeGraphDraftCLI(raw, &request); err != nil {
                        return err
                }
                if request.WorkID == "" {
                        request.WorkID = workID
                }
                if request.WorkID != workID {
                        return graphDraftError("invalid_input", "work_id ne correspond pas à WORK")
                }
                value, err := s.previewGraphDraft(operatorIdentity(), request)
                if err != nil {
                        return err
                }
                return printJSON(out, value)
        case "apply":
                if draftID != "" {
                        return graphDraftCLIUsage()
                }
                raw, err := readGraphDraftCLIInput(input)
                if err != nil {
                        return err
                }
                var request GraphDraftApplyRequest
                if err = decodeGraphDraftCLI(raw, &request); err != nil {
                        return err
                }
                if request.WorkID == "" {
                        request.WorkID = workID
                }
                if request.WorkID != workID {
                        return graphDraftError("invalid_input", "work_id ne correspond pas à WORK")
                }
                value, err := s.applyGraphDraft(operatorIdentity(), request)
                if err != nil {
                        return err
                }
                return printJSON(out, value)
        case "undo", "redo":
                if draftID == "" {
                        return graphDraftCLIUsage()
                }
                raw, err := readGraphDraftCLIInput(input)
                if err != nil {
                        return err
                }
                var edit graphDraftEditRequest
                if err = decodeGraphDraftCLI(raw, &edit); err != nil {
                        return err
                }
                value, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{
                        Schema: edit.Schema, WorkID: workID, DraftID: draftID,
                        ExpectedRevision:      edit.ExpectedRevision,
                        ExpectedDraftRevision: edit.ExpectedDraftRevision,
                        Operations:            edit.Operations,
                })
                if err != nil {
                        return err
                }
                return printJSON(out, graphDraftEditResult{Schema: 1, Action: action, Draft: value})
        default:
                return graphDraftCLIUsage()
        }
}

const graphDraftCLIHelpSource = `PRÉPARER LES DÉPENDANCES DU PLAN
Lire : swarm plan draft show TRAVAIL BROUILLON --json
Importer une proposition bornée à %d Kio : swarm plan draft import TRAVAIL --input proposition.json --json
Exporter : swarm plan draft export TRAVAIL BROUILLON --output brouillon.json --json
Comparer et prévisualiser : swarm plan draft compare TRAVAIL BROUILLON --json
ou swarm plan draft preview TRAVAIL --input preview.json --json
Appliquer explicitement : swarm plan draft apply TRAVAIL --input apply.json --json
Annuler/rétablir une édition : swarm plan draft undo|redo TRAVAIL BROUILLON --input edition.json --json
undo et redo enregistrent une nouvelle révision du brouillon. Ils ne retirent jamais
une révision déjà appliquée et ne modifient aucune tentative. Pour inverser un effet
appliqué, importer une nouvelle proposition inverse, puis la prévisualiser et l’appliquer.
Les erreurs JSON conservent un code métier stable. Codes de sortie : 0 succès ;
2 entrée invalide ; 3 conflit ; 4 autorisation ; 5 indisponibilité ; 6 effet incertain ; 7 attente.
`

func graphDraftCLIHelp() string {
        return fmt.Sprintf(uiText(graphDraftCLIHelpSource), graphDraftCLIFileLimit>>10)
}
```

## TEST

```go
//go:build linux

package main

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
        command := exec.Command("go", "build", "-o", binary, ".")
        command.Dir = "."
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
```

## HTTP

```go
//go:build linux

package main

import (
        "io"
        "net/http"
)

func registerGraphDraftHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
        decode := func(w http.ResponseWriter, r *http.Request, value any) error {
                raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
                if err != nil {
                        return err
                }
                return strict(raw, value)
        }
        mux.HandleFunc("/api/v1/graph-drafts", func(w http.ResponseWriter, r *http.Request) {
                if r.Method == http.MethodGet {
                        value, err := s.getGraphDraft(operatorIdentity(), r.URL.Query().Get("work"), r.URL.Query().Get("draft"))
                        if err != nil {
                                fail(w, err)
                                return
                        }
                        send(w, value)
                        return
                }
                if r.Method != http.MethodPost {
                        http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftSaveRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.saveGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/graph-drafts/preview", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftPreviewRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.previewGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/graph-drafts/apply", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftApplyRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.applyGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
}
```

## Raccord exact dans main.go

```go
Préparer : 2 contrat ; 3 conflit ; 4 autorisation ; 5 fournisseur ou méthode indisponible ; 6 arrêt non confirmé.
`

func mainHelpText() string {
        return uiText(help) + uiText("Brouillons du plan : swarm plan draft show|export|import|compare|preview|apply|undo|redo TRAVAIL [BROUILLON]\n")
}

func readInput(path string) ([]byte, error) {
        var r io.Reader = os.Stdin
        if path != "-" {
        fail := func(e error) int {
                if asJSON {
                        _ = printJSON(errOut, map[string]any{"schema_version": 1, "error": e.Error(), "failure": commandFailure(e)})
                } else {
                        fmt.Fprintln(errOut, uiText("Erreur :"), uiEngineText(commandFailure(e).Message))
                }
                switch commandFailure(e).Code {
                case "conflict", "revision_conflict", "preview_stale", "active_scope_conflict", "event_conflict", "idempotency_conflict", "terminal_target", "already_applied", "effect_started", "draft_conflict":
                        return 3
                case "source_refused", "preparation_disabled", "authorization_required":
                        return 4
                case "provider_unavailable", "method_unavailable", "workspace_wait":
                        return 5
                case "interrupted", "uncertain_effect":
                        return 6
                case "operation_pending":
                        return 7
                default:
                        return 2
                }
        }
        if len(pos) == 0 {
                fmt.Fprint(out, mainHelpText())
                return 0
        }
                if asJSON {
                        _ = printJSON(out, map[string]any{"work": updated, "applied": true, "task_id": task})
                } else {
                        fmt.Fprintf(out, uiText("Politique enregistrée pour %s à la révision %d.\n"), task, updated.Revision)
                }
                return 0
        }
        if pos[0] == "plan" || pos[0] == "console" || pos[0] == "agent" || pos[0] == "exchange" || pos[0] == "providers" || (pos[0] == "_supervise" || pos[0] == "_assist" || pos[0] == "_prepare_turn" || pos[0] == "_dialogue_agent") || pos[0] == "control" || pos[0] == "web" || pos[0] == "dispatch" || pos[0] == "autonomy" || pos[0] == "profile" {
                if e := agentCLI(s, pos, input, output, asJSON, out); e != nil {
                        return fail(e)
                }
                return 0
        }
        if pos[0] == "init" {
                _ = printJSON(out, map[string]any{"schema_version": 1, "root": s.root, "state": ".swarm/state.db"})
```

## Raccord exact dans console_cli.go

```go
                } else {
                        filtered = append(filtered, part)
                }
        }
        pos = filtered
        arg := func(n int) string {
                if len(pos) > n {
                        return pos[n]
                }
                return ""
        }
        switch pos[0] {
        case "plan":
                return s.graphDraftCLI(pos, input, output, out)
        case "workspace":
                switch arg(1) {
                case "status":
                        turns, e := s.workspaceTurns()
                        if e != nil {
```

## Raccord exact de l'aide

```go
package main

import (
        "fmt"
        "sort"
        "strings"
)

var cliHelpTopics = map[string]string{
        "brouillons": graphDraftCLIHelpSource,
        "planification": `CONFIER UN BESOIN À UNE ÉQUIPE
Sur une mission vide : swarm planning enable TRAVAIL --input activation.json.
L’activation fixe le fournisseur, les limites, le dépôt Git et les contrôles par
exigence. Les tâches créées héritent de ces contrôles autorisés.
Le terminal propose ses propres dialogues et raccourcis : la parité UX n’est pas totale.`,
}

func cliTopicHelp(topic string) (string, error) {
        topic = strings.ToLower(strings.TrimSpace(topic))
        aliases := map[string]string{"planning": "planification", "draft": "brouillons", "drafts": "brouillons", "management": "pilotage", "tasks": "taches", "validation": "validations", "lifecycle": "cycle-vie", "logs": "journaux", "context": "contexte", "parity": "parite"}
        if target, ok := aliases[topic]; ok {
                topic = target
        }
        if topic == "" {
                keys := make([]string, 0, len(cliHelpTopics))
                for k := range cliHelpTopics {
                        keys = append(keys, k)
                }
                sort.Strings(keys)
                return uiText("AIDE SWARM — choisissez un sujet\n\nswarm aide SUJET\nSujets : ") + strings.Join(keys, ", ") + uiText("\n\nConsole : help SUJET ; F1 ouvre l’aide de la fenêtre courante.\nPréparation : /aide SUJET ; /aide affiche les commandes.\nL’aide ne lance aucune action.\n"), nil
        }
        text, ok := cliHelpTopics[topic]
        if !ok {
                return "", fmt.Errorf("sujet d’aide inconnu : %s ; swarm aide liste les sujets", topic)
        }
        if topic == "brouillons" {
                return graphDraftCLIHelp(), nil
        }
        return uiText(text) + "\n", nil
}


```

## Service B01 réutilisé sans modification

Le code B02 appelle `getGraphDraft`, `saveGraphDraft`, `previewGraphDraft`, `applyGraphDraft` et `graphState` de `graph_draft.go` dont l'empreinte figure ci-dessus. Les gardes de normalisation, droit, révision, cycle, périmètre actif, jeton et idempotence restent donc l'autorité unique. Leur source complète et leurs tests sont déjà dans `docs/B01-dossier.md`; aucun fichier B01 n'a été modifié pendant B02.

## Limites de revue

Le reviewer doit recevoir ce dossier, `docs/B02.md`, le diff sale courant et `docs/B01-dossier.md` comme entrée liée. L'empreinte d'un fichier partagé peut changer après remise ; les contrôles et la revue devront alors viser le même candidat actualisé. Le test HTTP est conçu comme vrai processus binaire + vrai serveur authentifié ; son exécution n'est pas démontrée ici à cause du refus de socket.


## Correction de supervision après fin du producteur

Le helper CLI reçoit command.Dir = t.TempDir() : aucun sous-processus de recette sans --root ne travaille dans le dépôt. Un test précédent avait invoqué plan draft unknown sans root et migré le stockage courant v24 : incident réel de recette/CLI, pas restriction réseau. Archives originales conservées.

Le parseur partagé validateGraphDraftCLISyntax (source complète ci-dessus) est aussi appelé dans main.go AVANT filepath.Abs/openStore :

```go
if pos[0] == "plan" {
    if err := validateGraphDraftCLISyntax(pos); err != nil {
        return fail(err)
    }
}
```

TestGraphDraftB02InvalidCommandDoesNotOpenStore exécute quatre erreurs de syntaxe en processus réels sur répertoire temporaire vide ; exige invalid_input/exit2 et absence de .swarm. La correction ne change pas les sources B01 ni ses rapports acceptés. Commande d’erreur invalide : aucun accès stockage/migration. Nouveau CLI canonique compatible v24 nécessaire pour lire la mission migrée ; ce remplacement ne prouve ni livraison de l’éditeur ni recette finale.
