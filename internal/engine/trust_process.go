//go:build linux

package engine

import "time"

type TrustProcessPolicy struct {
	Enabled trustOIDCEnabled `json:"enabled"`
}

func (p *TrustProcessPolicy) UnmarshalJSON(raw []byte) error {
	if err := trustExplicitFields(raw, "enabled"); err != nil {
		return err
	}
	type plain TrustProcessPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustProcessPolicy(v)
	return nil
}
func trustProcessPolicyValue(p *TrustProcessPolicy) TrustProcessPolicy {
	if p == nil {
		return TrustProcessPolicy{}
	}
	return *p
}

type TrustProcessLabel struct {
	FR string `json:"fr"`
	EN string `json:"en"`
}
type TrustProcessNode struct {
	ID       string            `json:"id"`
	Label    TrustProcessLabel `json:"label"`
	Terminal trustOIDCEnabled  `json:"terminal"`
}
type TrustProcessTransition struct {
	ID               string            `json:"id"`
	From             string            `json:"from"`
	To               string            `json:"to"`
	Label            TrustProcessLabel `json:"label"`
	Kind             string            `json:"kind"`
	Roles            []string          `json:"roles"`
	MinimumIntegrity int               `json:"minimum_integrity"`
}
type TrustProcessDocument struct {
	Schema      int                      `json:"schema_version"`
	ID          string                   `json:"id"`
	Label       TrustProcessLabel        `json:"label"`
	Start       string                   `json:"start"`
	Nodes       []TrustProcessNode       `json:"nodes"`
	Transitions []TrustProcessTransition `json:"transitions"`
}
type TrustProcessDefinition struct {
	Schema      int                  `json:"schema_version"`
	ID          string               `json:"id"`
	Version     int                  `json:"version"`
	Digest      string               `json:"content_digest"`
	Document    TrustProcessDocument `json:"document"`
	PublishedBy string               `json:"published_by"`
	PublishedAt time.Time            `json:"published_at"`
}
type TrustProcessStep struct {
	Sequence    int       `json:"sequence"`
	Transition  string    `json:"transition_id"`
	Kind        string    `json:"kind"`
	From        string    `json:"from"`
	To          string    `json:"to"`
	Actor       string    `json:"actor"`
	InputDigest string    `json:"input_digest"`
	Reason      string    `json:"reason"`
	At          time.Time `json:"at"`
}
type TrustProcessCase struct {
	Schema            int                `json:"schema_version"`
	ID                string             `json:"id"`
	DefinitionID      string             `json:"definition_id"`
	DefinitionVersion int                `json:"definition_version"`
	DefinitionDigest  string             `json:"definition_digest"`
	Revision          int                `json:"revision"`
	InputDigest       string             `json:"input_digest"`
	Owner             string             `json:"owner"`
	Participants      []string           `json:"participants"`
	Resource          string             `json:"resource"`
	DataContext       TrustDataContext   `json:"data_context"`
	Title             string             `json:"title"`
	Reference         string             `json:"reference"`
	Node              string             `json:"node_id"`
	History           []TrustProcessStep `json:"history"`
	CreatedAt         time.Time          `json:"created_at"`
	UpdatedAt         time.Time          `json:"updated_at"`
}
type trustProcessReply struct {
	Digest    string `json:"digest"`
	Authority string `json:"authority"`
	Value     []byte `json:"value"`
}
type trustProcessState struct {
	Definitions map[string][]TrustProcessDefinition `json:"definitions"`
	Cases       map[string]TrustProcessCase         `json:"cases"`
	Replies     map[string]trustProcessReply        `json:"replies"`
}
type TrustProcessCommand struct {
	Policy            *TrustProcessPolicy   `json:"policy,omitempty"`
	DataPolicy        *TrustDataPolicy      `json:"data_policy,omitempty"`
	Schema            int                   `json:"schema_version,omitempty"`
	EventID           string                `json:"event_id,omitempty"`
	Definition        *TrustProcessDocument `json:"definition,omitempty"`
	ExpectedVersion   *int                  `json:"expected_version,omitempty"`
	DefinitionID      string                `json:"definition_id,omitempty"`
	DefinitionVersion int                   `json:"definition_version,omitempty"`
	DefinitionDigest  string                `json:"definition_digest,omitempty"`
	CaseID            string                `json:"case_id,omitempty"`
	ExpectedRevision  *int                  `json:"expected_revision,omitempty"`
	InputDigest       string                `json:"input_digest,omitempty"`
	Participants      []string              `json:"participants,omitempty"`
	Resource          string                `json:"resource,omitempty"`
	DataContext       *TrustDataContext     `json:"data_context,omitempty"`
	Title             string                `json:"title,omitempty"`
	Reference         string                `json:"reference,omitempty"`
	TransitionID      string                `json:"transition_id,omitempty"`
	Reason            string                `json:"reason,omitempty"`
}
