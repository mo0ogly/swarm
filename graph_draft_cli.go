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
