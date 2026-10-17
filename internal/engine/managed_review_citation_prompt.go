//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

func managedCitationContextDigest(c managedReviewContext) string {
	raw, _ := json.Marshal(c)
	return hash(raw)
}

func managedCitationCatalogForContext(c managedReviewContext, contextDigest string) (managedCitationCatalog, error) {
	base := c
	base.Tasks = nil
	originals, err := managedFragmentResolutionOriginals(base)
	if err != nil {
		return managedCitationCatalog{}, err
	}
	for _, task := range c.Tasks {
		scoped, err := managedFragmentResolutionOriginals(managedReviewContext{Tasks: []managedReviewTaskContext{task}})
		if err != nil {
			return managedCitationCatalog{}, err
		}
		for _, original := range scoped {
			original.Name = "tasks/" + task.Task + "/" + original.Name
			originals = append(originals, original)
		}
	}
	if c.Diff != "" {
		originals = append(originals, managedReviewFragmentArtifact{Name: "current-changes.diff", Kind: "diff", Content: c.Diff})
	}
	return buildManagedCitationCatalog(c.Candidate, contextDigest, originals)
}

func managedCitationMetadata(c managedReviewContext) managedReviewContext {
	c.Diff, c.Receipt = "", nil
	c.Sources = append([]ReviewSource(nil), c.Sources...)
	for i := range c.Sources {
		c.Sources[i].Content = ""
	}
	c.Tasks = append([]managedReviewTaskContext(nil), c.Tasks...)
	for i := range c.Tasks {
		c.Tasks[i].Report = ""
		c.Tasks[i].Controls = append([]ValidationControlResult(nil), c.Tasks[i].Controls...)
		for j := range c.Tasks[i].Controls {
			c.Tasks[i].Controls[j].ReviewOutput = ""
		}
	}
	return c
}

func managedCitationTransport(c managedCitationCatalog) any {
	originals := make([][]any, 0, len(c.Originals))
	for _, original := range c.Originals {
		refs := make([]int, 0, len(original.Spans))
		for _, span := range original.Spans {
			id := span.Reference
			if span.JSON {
				id = -1 - id
			}
			refs = append(refs, id)
		}
		originals = append(originals, []any{original.Name, original.Kind, original.Digest, original.Bytes, refs})
	}
	quotes := make([][]any, 0, len(c.Quotes))
	literal := c.literalReferences()
	for i, quote := range c.Quotes {
		encoding := "verbatim"
		if !literal[i] {
			encoding = "json_string"
		}
		quotes = append(quotes, []any{quote.ID, quote.Text, encoding})
	}
	return struct {
		Version         int      `json:"version"`
		Candidate       string   `json:"candidate_commit"`
		ContextDigest   string   `json:"context_sha256"`
		OriginalColumns []string `json:"original_columns"`
		Originals       [][]any  `json:"originals"`
		SpanEncoding    string   `json:"span_encoding"`
		QuoteColumns    []string `json:"quote_columns"`
		Quotes          [][]any  `json:"quotes"`
	}{c.Version, c.Candidate, c.ContextDigest, []string{"name", "kind", "sha256", "bytes", "ordered_spans"}, originals, "Nonnegative span n inserts quotes[n].text verbatim. Negative span n inserts quotes[-1-n].text using JSON string escaping, without surrounding quotes. Concatenate spans to recover the exact original and its SHA-256. evidence_ref uses a nonnegative id permitted by the schema. evidence_encoding specifies whether the engine uses that quote verbatim or JSON-escaped, without surrounding quotes, to recover an exact contiguous original span.", []string{"id", "text", "evidence_encoding"}, quotes}
}

func managedReferencedReviewPrompt(prefix string, c managedReviewContext) (string, string, error) {
	catalog, err := managedCitationCatalogForContext(c, managedCitationContextDigest(c))
	if err != nil {
		return "", "", err
	}
	payload := struct {
		Context managedReviewContext `json:"task_contracts_reports_controls"`
		Digest  string               `json:"catalog_sha256"`
		Catalog any                  `json:"citation_catalog"`
	}{managedCitationMetadata(c), catalog.digest(), managedCitationTransport(catalog)}
	raw, err := json.Marshal(payload)
	if err != nil {
		return "", "", err
	}
	return incrementalReviewPrefix(prefix, c) + fmt.Sprintf("\nRéponse complète limitée à %d octets UTF-8.\n", structuredProviderReplyLimit) + "\nSWARM_MANAGED_REVIEW_CONTEXT\n" + string(raw), managedReferencedDecisionSchema(c, catalog, nil), nil
}

func managedReviewSchemaForContext(c managedReviewContext) (string, error) {
	if c.CitationContractVersion != 2 {
		return managedReviewSchema, nil
	}
	catalog, err := managedCitationCatalogForContext(c, managedCitationContextDigest(c))
	if err != nil {
		return "", err
	}
	return managedReferencedDecisionSchema(c, catalog, nil), nil
}

func managedReviewSchemaForBatch(b managedReviewBatch) (string, error) {
	if b.Context.CitationContractVersion != 2 {
		return managedReviewSchema, nil
	}
	catalog, err := managedCitationCatalogForContext(b.Context, managedCitationContextDigest(b.Context))
	if err != nil {
		return "", err
	}
	subset := b.Context
	subset.Tasks = nil
	seen := map[string]bool{}
	for _, id := range b.Tasks {
		found := false
		for _, task := range b.Context.Tasks {
			if task.Task == id && !seen[id] {
				subset.Tasks = append(subset.Tasks, task)
				seen[id] = true
				found = true
				break
			}
		}
		if !found {
			return "", fmt.Errorf("affectation de schéma inconnue ou dupliquée")
		}
	}
	if len(subset.Tasks) == 0 {
		return "", fmt.Errorf("lot de schéma sans tâche")
	}
	return managedReferencedDecisionSchema(subset, catalog, nil), nil
}

func managedReviewBatchInputBytes(b managedReviewBatch) (int, error) {
	if b.Context.CitationContractVersion != 2 {
		return len(b.Prompt), nil
	}
	schema, err := managedReviewSchemaForBatch(b)
	return len(b.Prompt) + len(schema), err
}

func parseManagedBatchReply(reply string, b managedReviewBatch) (string, []ManagedTaskReview, error) {
	if b.Context.CitationContractVersion == 2 {
		catalog, err := managedCitationCatalogForContext(b.Context, managedCitationContextDigest(b.Context))
		if err != nil {
			return "", nil, err
		}
		reply, err = decodeManagedReferencedDecision(reply, catalog, b.Context.Candidate, catalog.ContextDigest, false)
		if err != nil {
			return "", nil, reviewResponseProtocolError(err)
		}
	}
	subset := b.Context
	subset.Tasks = nil
	for _, id := range b.Tasks {
		for _, task := range b.Context.Tasks {
			if task.Task == id {
				subset.Tasks = append(subset.Tasks, task)
			}
		}
	}
	return parseManagedLiteralReview(reply, subset)
}

func managedFragmentDecisionSchema(c managedReviewContext, b managedFragmentFinalEvidence, visible managedReviewContext) (string, error) {
	if c.CitationContractVersion != 2 {
		return fragmentDecisionSchema(b), nil
	}
	catalog, err := managedCitationCatalogForContext(visible, b.ContextDigest)
	if err != nil {
		return "", err
	}
	return managedReferencedDecisionSchema(c, catalog, b.Questions), nil
}

func managedReferencedFragmentMessage(prefix string, c managedReviewContext, b managedFragmentFinalEvidence, selection *managedFragmentEvidenceSelection) (string, managedReviewContext, error) {
	visible := managedFragmentVisibleOriginals(c, b, selection)
	catalog, err := managedCitationCatalogForContext(visible, b.ContextDigest)
	if err != nil {
		return "", managedReviewContext{}, err
	}
	payload := struct {
		Context     managedReviewContext          `json:"task_contracts_reports_controls"`
		Inspections managedFragmentFinalTransport `json:"partial_inspections"`
		Digest      string                        `json:"catalog_sha256"`
		Catalog     any                           `json:"citation_catalog"`
	}{managedCitationMetadata(c), compactManagedFragmentFinalEvidence(b), catalog.digest(), managedCitationTransport(catalog)}
	raw, err := json.Marshal(payload)
	if err != nil {
		return "", managedReviewContext{}, err
	}
	instructions := fmt.Sprintf("Réponse complète limitée à %d octets UTF-8.\n", managedFragmentReplyLimit) + "DECISION FINALE INDEPENDANTE. Les inspections partielles et historical_observations sont des observations attribuées, jamais des verdicts de tâche. Réexaminer chaque critère, chaque partial_defects et chaque unresolved_questions, ainsi que les interactions et régressions. Les extraits ne remplacent pas un original entier ; les originaux effectivement fournis sont attribués dans citation_catalog. Un inventaire inspecté ne prouve aucune conformité. Les preuves absentes conduisent à unknown ; un défaut démontré à fail. Les réserves historiques exigent une résolution appuyée par les changements actuels. Aucun outil ni modification.\n"
	if len(b.Questions) > 0 {
		instructions += "Renvoyer review et resolutions ; chaque réserve doit conserver packet, artifact et need_index. Une réserve unknown ou fail empêche l’acceptation.\n"
	}
	prompt := incrementalReviewPrefix(prefix, c) + "\n" + instructions + "\nSWARM_FRAGMENT_FINAL_EVIDENCE\n" + string(raw)
	return prompt, visible, nil
}

func renderManagedReferencedFragmentDecision(prefix string, c managedReviewContext, b managedFragmentFinalEvidence, selection *managedFragmentEvidenceSelection) (string, managedReviewContext, error) {
	prompt, visible, err := managedReferencedFragmentMessage(prefix, c, b, selection)
	if err != nil {
		return "", managedReviewContext{}, err
	}
	schema, err := managedFragmentDecisionSchema(c, b, visible)
	if err != nil {
		return "", managedReviewContext{}, err
	}
	if err := fragmentInputFits(b.InputBudget, prompt, schema); err != nil {
		return "", managedReviewContext{}, fmt.Errorf("décision finale trop grande ; aucune preuve tronquée : %w", err)
	}
	return prompt, visible, nil
}

func managedReferencedArtifactCost(prefix string, c managedReviewContext, b managedFragmentFinalEvidence, artifact managedReviewFragmentArtifact, basePrompt, baseSchema string) (int, int, error) {
	selection := managedFragmentEvidenceSelection{Artifacts: []managedReviewFragmentArtifact{artifact}}
	prompt, visible, err := managedReferencedFragmentMessage(prefix, c, b, &selection)
	if err != nil {
		return 0, 0, err
	}
	schema, err := managedFragmentDecisionSchema(c, b, visible)
	if err != nil {
		return 0, 0, err
	}
	byteCost := max(0, len(prompt)+len(schema)-len(basePrompt)-len(baseSchema))
	if b.InputBudget == nil {
		return byteCost, 0, nil
	}
	before, err := b.InputBudget.measure(basePrompt, baseSchema)
	if err != nil {
		return 0, 0, err
	}
	after, err := b.InputBudget.measure(prompt, schema)
	if err != nil && after.TokenRoom >= 0 {
		return 0, 0, err
	}
	return byteCost, max(0, before.TokenRoom-after.TokenRoom), nil
}
