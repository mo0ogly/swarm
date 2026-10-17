//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strings"
)

func trustProcessInvalid() error {
	return fmt.Errorf("processus : définition, version ou commande invalide")
}
func trustProcessDenied() error {
	return &CommandError{Code: "authorization_required", Message: "processus refusé : vérifier identité, participation, rôle, politique, version et contenu exact"}
}
func processLabelValid(v TrustProcessLabel) bool {
	return strings.TrimSpace(v.FR) != "" && strings.TrimSpace(v.EN) != ""
}
func validateTrustProcessDocument(d TrustProcessDocument) error {
	if d.Schema != 1 || !safeName(d.ID) || !processLabelValid(d.Label) || d.Nodes == nil || d.Transitions == nil || len(d.Nodes) == 0 {
		return trustProcessInvalid()
	}
	nodes := map[string]TrustProcessNode{}
	edges := map[string][]string{}
	reverse := map[string][]string{}
	indegree := map[string]int{}
	seen := map[string]bool{}
	terminals := []string{}
	for _, n := range d.Nodes {
		if !safeName(n.ID) || !processLabelValid(n.Label) {
			return trustProcessInvalid()
		}
		if _, ok := nodes[n.ID]; ok {
			return trustProcessInvalid()
		}
		nodes[n.ID] = n
		indegree[n.ID] = 0
		if bool(n.Terminal) {
			terminals = append(terminals, n.ID)
		}
	}
	if _, ok := nodes[d.Start]; !ok || len(terminals) == 0 {
		return trustProcessInvalid()
	}
	for _, e := range d.Transitions {
		from, fromOK := nodes[e.From]
		_, toOK := nodes[e.To]
		if !safeName(e.ID) || seen[e.ID] || !fromOK || !toOK || e.From == e.To || !processLabelValid(e.Label) || len(e.Roles) == 0 || e.MinimumIntegrity < 1 || e.MinimumIntegrity > 3 {
			return trustProcessInvalid()
		}
		seen[e.ID] = true
		roles := map[string]bool{}
		for _, role := range e.Roles {
			if !containsTrust([]string{"admin", "requester", "approver", "effect"}, role) || roles[role] {
				return trustProcessInvalid()
			}
			roles[role] = true
		}
		switch e.Kind {
		case "advance":
			if bool(from.Terminal) {
				return trustProcessInvalid()
			}
		case "approval":
			if bool(from.Terminal) || e.MinimumIntegrity < 2 || len(roles) != 1 || !roles["approver"] {
				return trustProcessInvalid()
			}
		case "exception":
			if e.MinimumIntegrity != 3 || len(roles) != 1 || !roles["admin"] {
				return trustProcessInvalid()
			}
		default:
			return trustProcessInvalid()
		}
		if e.Kind != "exception" {
			edges[e.From] = append(edges[e.From], e.To)
			reverse[e.To] = append(reverse[e.To], e.From)
			indegree[e.To]++
		}
	}
	queue := []string{}
	for id, n := range nodes {
		if !bool(n.Terminal) && len(edges[id]) == 0 {
			return trustProcessInvalid()
		}
		if indegree[id] == 0 {
			queue = append(queue, id)
		}
	}
	visited := 0
	for len(queue) > 0 {
		id := queue[0]
		queue = queue[1:]
		visited++
		for _, to := range edges[id] {
			indegree[to]--
			if indegree[to] == 0 {
				queue = append(queue, to)
			}
		}
	}
	if visited != len(nodes) {
		return trustProcessInvalid()
	}
	reachable := map[string]bool{}
	queue = []string{d.Start}
	for len(queue) > 0 {
		id := queue[0]
		queue = queue[1:]
		if reachable[id] {
			continue
		}
		reachable[id] = true
		queue = append(queue, edges[id]...)
	}
	if len(reachable) != len(nodes) {
		return trustProcessInvalid()
	}
	canFinish := map[string]bool{}
	queue = terminals
	for len(queue) > 0 {
		id := queue[0]
		queue = queue[1:]
		if canFinish[id] {
			continue
		}
		canFinish[id] = true
		queue = append(queue, reverse[id]...)
	}
	if len(canFinish) != len(nodes) {
		return trustProcessInvalid()
	}
	return nil
}

func trustProcessParse(op string, raw []byte) (TrustProcessCommand, error) {
	var c TrustProcessCommand
	if len(raw) > 1<<20 || !bytes.HasPrefix(bytes.TrimSpace(raw), []byte("{")) {
		return c, trustProcessInvalid()
	}
	if err := strict(raw, &c); err != nil {
		return c, trustProcessInvalid()
	}
	allowed := map[string]bool{}
	var fields map[string]json.RawMessage
	if json.Unmarshal(raw, &fields) != nil {
		return c, trustProcessInvalid()
	}
	names := []string{}
	switch op {
	case "show":
	case "case-show":
		names = []string{"case_id"}
	case "policy-preview", "policy-apply":
		names = []string{"schema_version", "event_id", "policy", "data_policy", "expected_revision", "reason"}
	case "definition-preview", "definition-publish":
		names = []string{"schema_version", "event_id", "definition", "expected_version", "reason"}
	case "case-create":
		names = []string{"schema_version", "event_id", "definition_id", "definition_version", "definition_digest", "participants", "resource", "data_context", "title", "reference", "reason"}
	case "case-transition":
		names = []string{"schema_version", "event_id", "case_id", "expected_revision", "definition_digest", "input_digest", "transition_id", "reason"}
	default:
		return c, trustProcessInvalid()
	}
	for _, k := range names {
		allowed[k] = true
	}
	for k, v := range fields {
		if !allowed[k] || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return c, trustProcessInvalid()
		}
	}
	if op != "show" && op != "case-show" && (c.Schema != 1 || !safeName(c.EventID)) {
		return c, trustProcessInvalid()
	}
	return c, nil
}
func processDefinitionDigest(d TrustProcessDocument, version int) string {
	return hashMust(struct {
		Document TrustProcessDocument
		Version  int
	}{d, version})
}
func processCaseInputDigest(c TrustProcessCase) string {
	return hashMust(struct {
		DefinitionID            string
		DefinitionVersion       int
		DefinitionDigest, Owner string
		Participants            []string
		Resource                string
		DataContext             TrustDataContext
		Title, Reference        string
	}{c.DefinitionID, c.DefinitionVersion, c.DefinitionDigest, c.Owner, c.Participants, c.Resource, c.DataContext, c.Title, c.Reference})
}
func processDefinitionFor(ps *trustProcessState, id string, version int) (TrustProcessDefinition, bool) {
	if ps == nil || version < 1 {
		return TrustProcessDefinition{}, false
	}
	versions := ps.Definitions[id]
	if version > len(versions) {
		return TrustProcessDefinition{}, false
	}
	d := versions[version-1]
	return d, d.Version == version
}
func validateTrustProcesses(ps *trustProcessState) error {
	if ps == nil {
		return nil
	}
	for id, versions := range ps.Definitions {
		for i, d := range versions {
			if d.Schema != 1 || d.ID != id || d.Document.ID != id || d.Version != i+1 || d.Digest != processDefinitionDigest(d.Document, d.Version) || validateTrustProcessDocument(d.Document) != nil {
				return trustProcessInvalid()
			}
		}
	}
	for id, c := range ps.Cases {
		d, ok := processDefinitionFor(ps, c.DefinitionID, c.DefinitionVersion)
		if !ok || c.Schema != 1 || c.ID != id || c.DefinitionDigest != d.Digest || c.Revision != len(c.History)+1 || c.InputDigest != processCaseInputDigest(c) {
			return trustProcessInvalid()
		}
		node := d.Document.Start
		for i, h := range c.History {
			found := false
			for _, edge := range d.Document.Transitions {
				if edge.ID == h.Transition && edge.From == node && edge.To == h.To && edge.Kind == h.Kind {
					found = true
				}
			}
			if !found || h.Sequence != i+1 || h.From != node || h.InputDigest != c.InputDigest {
				return trustProcessInvalid()
			}
			node = h.To
		}
		if node != c.Node {
			return trustProcessInvalid()
		}
	}
	return nil
}
