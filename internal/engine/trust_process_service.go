//go:build linux

package engine

import (
	"encoding/json"
	"sort"
	"strings"
	"time"
)

func trustProcessStateValue(st *trustState) *trustProcessState {
	if st.Processes == nil {
		st.Processes = &trustProcessState{}
	}
	p := st.Processes
	if p.Definitions == nil {
		p.Definitions = map[string][]TrustProcessDefinition{}
	}
	if p.Cases == nil {
		p.Cases = map[string]TrustProcessCase{}
	}
	if p.Replies == nil {
		p.Replies = map[string]trustProcessReply{}
	}
	return p
}
func trustProcessMember(id trustIdentity, c TrustProcessCase) bool {
	return trustSecretAdmin(id) || id.Subject == c.Owner || (containsTrust(c.Participants, id.Subject) && id.Integrity >= 2 && (trustHasRole(id, "approver") || trustHasRole(id, "effect")))
}
func trustProcessDataAllowed(st *trustState, resource string, c *TrustDataContext) error {
	p := effectiveTrustDataPolicy(st)
	if !bool(p.Enabled) || c == nil || !containsTrust([]string{"public", "internal"}, c.Classification) || c.Destination != "local-process-state" || c.Provider != "swarm" || c.Tool != "process-workflow" || !containsTrust(p.Classifications, c.Classification) || !containsTrust(p.Destinations, c.Destination) || !containsTrust(p.Providers, c.Provider) || !containsTrust(p.Tools, c.Tool) {
		return trustProcessDenied()
	}
	for _, r := range p.Resources {
		if r.Resource == resource && r.Classification == c.Classification {
			return nil
		}
	}
	return trustProcessDenied()
}
func trustProcessCaseView(st *trustState, c TrustProcessCase) any {
	if !bool(trustProcessPolicyValue(st.ProcessPolicy).Enabled) || trustProcessDataAllowed(st, c.Resource, &c.DataContext) != nil {
		return map[string]any{"schema_version": 1, "id": c.ID, "availability": "blocked", "reason": "politique de processus ou de données inactive/non concordante ; demander une correction à l’administrateur"}
	}
	return c
}
func trustProcessView(st *trustState, id trustIdentity) any {
	ps := trustProcessStateValue(st)
	cases := []any{}
	keys := []string{}
	for key := range ps.Cases {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		c := ps.Cases[key]
		if trustProcessMember(id, c) {
			cases = append(cases, trustProcessCaseView(st, c))
		}
	}
	raw, _ := json.Marshal(st)
	return map[string]any{"schema_version": 1, "policy": trustProcessPolicyValue(st.ProcessPolicy), "config_revision": st.Revision, "definitions": ps.Definitions, "cases": cases, "identity_context": id.Context, "data_policy": effectiveTrustDataPolicy(st), "simulation": true, "effects_supported": false, "command_bytes_max": 1 << 20, "shared_state_bytes_max": trustStateMaxBytes, "shared_state_bytes": len(raw), "max_version": trustPolicyBounds()["process_version"]["max"]}
}
func (s *Store) trustProcessOperation(op string, raw []byte, token string) (any, error) {
	cmd, err := trustProcessParse(op, raw)
	if err != nil {
		return nil, err
	}
	write := op != "show" && op != "case-show" && op != "definition-preview" && op != "policy-preview"
	return s.withTrustLock(write, func(st *trustState) (any, error) {
		now := time.Now().UTC()
		actor, err := findTrustIdentityAt(st, token, now)
		if err != nil {
			return nil, err
		}
		ps := trustProcessStateValue(st)
		if op == "show" {
			return trustProcessView(st, actor), nil
		}
		if op == "case-show" {
			c, ok := ps.Cases[cmd.CaseID]
			if !ok || !trustProcessMember(actor, c) || trustProcessDataAllowed(st, c.Resource, &c.DataContext) != nil {
				return nil, trustProcessDenied()
			}
			return trustProcessCaseView(st, c), nil
		}
		if op != "definition-preview" && op != "policy-preview" && op != "policy-apply" && !bool(trustProcessPolicyValue(st.ProcessPolicy).Enabled) {
			return nil, trustProcessDenied()
		}
		switch op {
		case "policy-preview", "policy-apply", "definition-preview", "definition-publish":
			if !trustSecretAdmin(actor) {
				return nil, trustProcessDenied()
			}
		case "case-create":
			if !trustHasRole(actor, "requester") || actor.Integrity < 1 {
				return nil, trustProcessDenied()
			}
			if err := trustProcessDataAllowed(st, cmd.Resource, cmd.DataContext); err != nil {
				return nil, err
			}
		case "case-transition":
			c, ok := ps.Cases[cmd.CaseID]
			if !ok || !trustProcessMember(actor, c) {
				return nil, trustProcessDenied()
			}
			if err := trustProcessDataAllowed(st, c.Resource, &c.DataContext); err != nil {
				return nil, err
			}
		}
		digest := hashMust(struct {
			Operation string
			Command   TrustProcessCommand
		}{op, cmd})
		if prior, ok := ps.Replies[cmd.EventID]; ok {
			if prior.Digest != digest {
				return nil, &CommandError{Code: "event_conflict", Message: "event_id de processus réutilisé avec un autre contenu"}
			}
			if prior.Authority != trustAuthorityDigest(actor) {
				return nil, trustProcessDenied()
			}
			var value any
			if json.Unmarshal(prior.Value, &value) != nil {
				return nil, trustProcessInvalid()
			}
			return value, nil
		}
		if err := verifyTrustEvidenceChain(TrustProofExport{Evidence: st.Evidence, Originals: st.Originals}); err != nil {
			return nil, err
		}
		var value any
		var object, content string
		switch op {
		case "policy-preview", "policy-apply":
			if !trustSecretAdmin(actor) || cmd.Policy == nil || cmd.ExpectedRevision == nil || len(strings.TrimSpace(cmd.Reason)) < 8 {
				return nil, trustProcessDenied()
			}
			if *cmd.ExpectedRevision != st.Revision {
				return nil, &CommandError{Code: "revision_conflict", Message: "configuration modifiée ; rechargez avant de confirmer"}
			}
			if cmd.DataPolicy != nil {
				if err := validateTrustDataPolicy(cmd.DataPolicy); err != nil {
					return nil, err
				}
			}
			if op == "policy-preview" {
				return map[string]any{"current_revision": st.Revision, "proposed_revision": st.Revision + 1, "policy": cmd.Policy, "data_policy": func() *TrustDataPolicy {
					if cmd.DataPolicy != nil {
						return cmd.DataPolicy
					}
					v := effectiveTrustDataPolicy(st)
					return &v
				}(), "applied": false}, nil
			}
			st.Revision++
			st.ProcessPolicy = cmd.Policy
			if cmd.DataPolicy != nil {
				st.DataPolicy = cmd.DataPolicy
			}
			st.OIDCSessions = nil
			st.OIDCPending = nil
			st.WorkloadSessions = nil
			st.History = append(st.History, trustConfigRevision{Revision: st.Revision, Policy: st.Policy, Actor: actor.Subject, Reason: cmd.Reason, EventID: cmd.EventID, At: now.Format(time.RFC3339Nano)})
			value = map[string]any{"revision": st.Revision, "policy": st.ProcessPolicy, "data_policy": effectiveTrustDataPolicy(st), "applied": true}
			object = "process-policy"
			content = hashMust(value)

		case "definition-preview", "definition-publish":
			if !trustSecretAdmin(actor) || cmd.Definition == nil || cmd.ExpectedVersion == nil || *cmd.ExpectedVersion < 0 || validateTrustProcessDocument(*cmd.Definition) != nil || len(strings.TrimSpace(cmd.Reason)) < 8 {
				return nil, trustProcessInvalid()
			}
			versions := ps.Definitions[cmd.Definition.ID]
			if len(versions) != *cmd.ExpectedVersion {
				return nil, &CommandError{Code: "revision_conflict", Message: "version de définition modifiée ; rechargez avant de publier"}
			}
			if int64(len(versions)) >= trustPolicyBounds()["process_version"]["max"] {
				return nil, trustProcessInvalid()
			}
			d := TrustProcessDefinition{Schema: 1, ID: cmd.Definition.ID, Version: len(versions) + 1, Document: *cmd.Definition, PublishedBy: actor.Subject, PublishedAt: now}
			d.Digest = processDefinitionDigest(d.Document, d.Version)
			value = map[string]any{"definition_id": d.ID, "version": d.Version, "content_digest": d.Digest, "published": op == "definition-publish"}
			if op == "definition-preview" {
				return value, nil
			}
			ps.Definitions[d.ID] = append(versions, d)
			object = d.ID
			content = d.Digest
		case "case-create":
			if !trustHasRole(actor, "requester") || actor.Integrity < 1 || cmd.Participants == nil || !validTrustResource(cmd.Resource) || strings.TrimSpace(cmd.Title) == "" || strings.TrimSpace(cmd.Reference) == "" {
				return nil, trustProcessDenied()
			}
			d, ok := processDefinitionFor(ps, cmd.DefinitionID, cmd.DefinitionVersion)
			if !ok || d.Version != len(ps.Definitions[d.ID]) || d.Digest != cmd.DefinitionDigest {
				return nil, &CommandError{Code: "revision_conflict", Message: "définition obsolète ; choisir la dernière version publiée"}
			}
			if err := trustProcessDataAllowed(st, cmd.Resource, cmd.DataContext); err != nil {
				return nil, err
			}
			participants := append([]string{}, cmd.Participants...)
			sort.Strings(participants)
			seen := map[string]bool{}
			for _, subject := range participants {
				if seen[subject] || subject == actor.Subject {
					return nil, trustProcessDenied()
				}
				seen[subject] = true
				found := false
				for _, id := range st.Identities {
					if id.Subject == subject && !bool(id.Disabled) && id.Integrity >= 2 && (trustHasRole(id, "approver") || trustHasRole(id, "effect")) {
						found = true
					}
				}
				if !found {
					return nil, trustProcessDenied()
				}
			}
			c := TrustProcessCase{Schema: 1, ID: newID("case-"), DefinitionID: d.ID, DefinitionVersion: d.Version, DefinitionDigest: d.Digest, Revision: 1, Owner: actor.Subject, Participants: participants, Resource: cmd.Resource, DataContext: *cmd.DataContext, Title: cmd.Title, Reference: cmd.Reference, Node: d.Document.Start, History: []TrustProcessStep{}, CreatedAt: now, UpdatedAt: now}
			c.InputDigest = processCaseInputDigest(c)
			ps.Cases[c.ID] = c
			value = c
			object = c.ID
			content = c.InputDigest
		case "case-transition":
			c, ok := ps.Cases[cmd.CaseID]
			if !ok || !trustProcessMember(actor, c) {
				return nil, trustProcessDenied()
			}
			if cmd.ExpectedRevision == nil || *cmd.ExpectedRevision != c.Revision || cmd.InputDigest != c.InputDigest || cmd.DefinitionDigest != c.DefinitionDigest {
				return nil, &CommandError{Code: "revision_conflict", Message: "dossier ou contenu modifié ; rechargez avant de confirmer"}
			}
			if err := trustProcessDataAllowed(st, c.Resource, &c.DataContext); err != nil {
				return nil, err
			}
			d, ok := processDefinitionFor(ps, c.DefinitionID, c.DefinitionVersion)
			if !ok {
				return nil, trustProcessInvalid()
			}
			var edge TrustProcessTransition
			for _, e := range d.Document.Transitions {
				if e.ID == cmd.TransitionID && e.From == c.Node {
					edge = e
				}
			}
			role := false
			for _, r := range edge.Roles {
				if trustHasRole(actor, r) {
					role = true
				}
			}
			if edge.ID == "" || !role || actor.Integrity < edge.MinimumIntegrity {
				return nil, trustProcessDenied()
			}
			if (edge.Kind == "approval" || edge.Kind == "exception") && (len(strings.TrimSpace(cmd.Reason)) < 8 || (st.Policy.RequireSeparation && actor.Subject == c.Owner)) {
				return nil, trustProcessDenied()
			}
			if edge.Kind == "exception" && !trustSecretAdmin(actor) {
				return nil, trustProcessDenied()
			}
			if int64(c.Revision) >= trustPolicyBounds()["process_version"]["max"] {
				return nil, trustProcessInvalid()
			}
			step := TrustProcessStep{Sequence: len(c.History) + 1, Transition: edge.ID, Kind: edge.Kind, From: c.Node, To: edge.To, Actor: actor.Subject, InputDigest: c.InputDigest, Reason: cmd.Reason, At: now}
			c.History = append(c.History, step)
			c.Node = edge.To
			c.Revision++
			c.UpdatedAt = now
			ps.Cases[c.ID] = c
			value = map[string]any{"case_id": c.ID, "revision": c.Revision, "node_id": c.Node, "input_digest": c.InputDigest, "definition_digest": c.DefinitionDigest, "transition": step}
			object = c.ID
			content = c.InputDigest
		default:
			return nil, trustProcessInvalid()
		}
		originalHash := hash(raw)
		st.Originals[originalHash] = append([]byte{}, raw...)
		appendTrustEvidence(st, "process-"+op, actor, object, content, originalHash, "applied", "transition de dossier ; aucun effet de paiement", now)
		reply, err := json.Marshal(value)
		if err != nil {
			return nil, err
		}
		ps.Replies[cmd.EventID] = trustProcessReply{Digest: digest, Authority: trustAuthorityDigest(actor), Value: reply}
		return value, nil
	})
}
