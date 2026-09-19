package main

type Retex struct {
	ID            string      `json:"id"`
	Source        DialogueRef `json:"source"`
	Problem       string      `json:"problem"`
	Nature        string      `json:"nature"`
	Proposal      string      `json:"proposal"`
	Effort        string      `json:"effort"`
	Risk          string      `json:"risk"`
	Criteria      string      `json:"criteria"`
	Proof         string      `json:"proof"`
	Status        string      `json:"status"`
	Task          string      `json:"task"`
	At            string      `json:"at"`
	ExportPending bool        `json:"export_pending,omitempty"`
	ExportHash    string      `json:"export_hash,omitempty"`
	ExportError   string      `json:"export_error,omitempty"`
	Lesson        string      `json:"lesson,omitempty"`
}
