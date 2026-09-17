package main

import (
	"fmt"
	"io"
	"os"
	"strings"
)

// The browser supplies a report coordinate, never a report body. Only reports
// discovered for the selected task are admitted, using the existing path guard.
func (s *Store) reportContext(b *contextBuilder, c PageCoordinates) error {
	if c.PageID != "tasks" || len(c.Selected) != 1 {
		return fmt.Errorf("Sélectionner une tâche pour analyser son rapport.")
	}
	allowed := false
	for _, path := range s.taskReports(c.Selected[0]) {
		if path == c.Report {
			allowed = true
			break
		}
	}
	if !allowed {
		return fmt.Errorf("Rapport non associé à la tâche sélectionnée.")
	}
	path, err := safeReport(s.root, c.Report)
	if err != nil {
		return err
	}
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	raw, err := io.ReadAll(io.LimitReader(f, 12001))
	if err != nil {
		return err
	}
	if len(raw) > 12000 {
		raw = raw[:12000]
		b.ctx.Truncated = true
		b.omit("Rapport limité aux 12 000 premiers octets ; les conclusions situées après peuvent manquer.")
	}
	if strings.TrimSpace(string(raw)) == "" {
		return fmt.Errorf("Rapport vide : aucune analyse possible.")
	}
	b.seq++
	b.ctx.Facts = append(b.ctx.Facts, PageFact{ID: fmt.Sprintf("f%d", b.seq), Name: "contenu_du_rapport", Kind: "texte_non_fiable", Source: "report/" + c.Report, Value: guardBlock(string(raw), 12000)})
	b.ctx.Untrusted = true
	// Replace generic omissions that would incorrectly deny the supplied excerpt.
	b.ctx.Omissions = filterReportOmissions(b.ctx.Omissions)
	b.ctx.Missing = filterReportOmissions(b.ctx.Missing)
	b.ctx.Limits[1] = "Seul le rapport sélectionné est lu, avec masquage des secrets reconnaissables. Les affirmations de son auteur ne constituent pas une validation indépendante."
	b.omit("Les autres rapports, fichiers de preuve et diffs ne sont pas transmis.")
	return nil
}

func filterReportOmissions(items []string) []string {
	out := []string{}
	for _, item := range items {
		if !strings.HasPrefix(item, "Contenu des fichiers de preuve et des rapports") && !strings.HasPrefix(item, "Contenu des rapports et des preuves") {
			out = append(out, item)
		}
	}
	return out
}
