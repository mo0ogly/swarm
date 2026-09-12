package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const schemaVersion = 4

type ManualOverride struct {
	Reason         string `json:"reason"`
	Actor          string `json:"actor"`
	At             string `json:"at"`
	PreviousStatus string `json:"previous_status"`
	Revision       int    `json:"revision"`
}

type BrainstormAnswer struct {
	Attempt string `json:"attempt"`
	Text    string `json:"text"`
	At      string `json:"at"`
}

type Revalidation struct {
	PreviousArtifacts map[string]string `json:"previous_artifacts"`
	Config            string            `json:"config_digest"`
	Report            string            `json:"report,omitempty"`
	ReportHash        string            `json:"report_hash,omitempty"`
}

type Task struct {
	Revalidation    *Revalidation      `json:"revalidation,omitempty"`
	PlanChecks      map[string]string  `json:"plan_checks,omitempty"`
	PlanBriefHash   string             `json:"plan_brief_hash,omitempty"`
	PlanRole        string             `json:"plan_role,omitempty"`
	PlanMaxAttempts int                `json:"plan_max_attempts,omitempty"`
	PlanToolLimit   int                `json:"plan_tool_limit,omitempty"`
	Contexts        []SavedContext     `json:"contexts,omitempty"`
	Answers         []BrainstormAnswer `json:"answers,omitempty"`
	Question        string             `json:"question,omitempty"`
	Response        string             `json:"response,omitempty"`
	ResponseError   string             `json:"response_error,omitempty"`
	Brainstorm      bool               `json:"brainstorm,omitempty"`
	Override        *ManualOverride    `json:"manual_override,omitempty"`
	ID              string             `json:"id"`
	Title           string             `json:"title"`
	Owner           string             `json:"owner"`
	Deliverable     string             `json:"deliverable"`
	Criteria        []string           `json:"criteria"`
	Depends         []string           `json:"depends"`
	Status          string             `json:"status"`
	Blocker         string             `json:"blocker"`
	Next            string             `json:"next"`
	Attempts        []Attempt          `json:"attempts"`
	Gate            *GateRecord        `json:"gate,omitempty"`
}
type Attempt struct {
	ID      string `json:"id"`
	Status  string `json:"status"`
	Started string `json:"started"`
	Ended   string `json:"ended,omitempty"`
}
type GitState struct {
	Head    string `json:"head"`
	Branch  string `json:"branch"`
	Changes string `json:"changes"`
}
type Work struct {
	Plans         []ApprovedPlan `json:"plans,omitempty"`
	Retex         []Retex        `json:"retex,omitempty"`
	PlanningBrief *PlanningBrief `json:"planning_brief,omitempty"`
	Schema        int            `json:"schema_version"`
	ID            string         `json:"id"`
	Revision      int            `json:"revision"`
	Title         string         `json:"title"`
	Objective     string         `json:"objective"`
	Scope         string         `json:"scope"`
	Criteria      []string       `json:"criteria"`
	Next          string         `json:"next"`
	Summary       string         `json:"summary"`
	Memory        []string       `json:"memory"`
	Created       string         `json:"created"`
	Updated       string         `json:"updated"`
	Git           GitState       `json:"git"`
	Tasks         []Task         `json:"tasks"`
}
type Event struct {
	ID       string          `json:"id"`
	WorkID   string          `json:"work_id"`
	Revision int             `json:"revision"`
	Kind     string          `json:"kind"`
	At       string          `json:"at"`
	Payload  json.RawMessage `json:"payload"`
}
type Request struct {
	Schema      int      `json:"schema_version"`
	EventID     string   `json:"event_id"`
	Revision    int      `json:"expected_revision"`
	ID          string   `json:"id,omitempty"`
	Title       string   `json:"title,omitempty"`
	Objective   string   `json:"objective,omitempty"`
	Scope       string   `json:"scope,omitempty"`
	Criteria    []string `json:"criteria,omitempty"`
	Owner       string   `json:"owner,omitempty"`
	Deliverable string   `json:"deliverable,omitempty"`
	Depends     []string `json:"depends,omitempty"`
	Status      string   `json:"status,omitempty"`
	Blocker     string   `json:"blocker,omitempty"`
	Next        string   `json:"next,omitempty"`
	Summary     string   `json:"summary,omitempty"`
	Memory      []string `json:"memory,omitempty"`
	Outcome     string   `json:"outcome,omitempty"`
	Observation string   `json:"observation,omitempty"`
	Orientation string   `json:"orientation,omitempty"`
	Decision    string   `json:"decision,omitempty"`
	Result      string   `json:"result,omitempty"`
}

func now() string { return time.Now().UTC().Format(time.RFC3339Nano) }
func newID(prefix string) string {
	b := make([]byte, 12)
	if _, e := rand.Read(b); e != nil {
		panic(e)
	}
	return prefix + hex.EncodeToString(b)
}
func nonempty(s string) bool { return strings.TrimSpace(s) != "" }
func require(ok bool, msg string) error {
	if !ok {
		return fmt.Errorf("%s", msg)
	}
	return nil
}
func (w *Work) task(id string) (*Task, error) {
	for i := range w.Tasks {
		if w.Tasks[i].ID == id {
			return &w.Tasks[i], nil
		}
	}
	return nil, fmt.Errorf("tâche inconnue : %s", id)
}
func safeName(s string) bool {
	if s == "" || len(s) > 100 {
		return false
	}
	for _, r := range s {
		if !(r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z' || r >= '0' && r <= '9' || r == '-' || r == '_') {
			return false
		}
	}
	return true
}
func localFile(root, name string) (string, error) {
	if !nonempty(name) || filepath.IsAbs(name) {
		return "", fmt.Errorf("chemin relatif requis : %s", name)
	}
	resolved, e := filepath.EvalSymlinks(filepath.Join(root, name))
	if e != nil {
		return "", e
	}
	rel, e := filepath.Rel(root, resolved)
	if e != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("chemin hors projet : %s", name)
	}
	st, e := os.Stat(resolved)
	if e != nil {
		return "", e
	}
	if !st.Mode().IsRegular() {
		return "", fmt.Errorf("fichier régulier requis : %s", name)
	}
	return resolved, nil
}
func atomicWrite(path string, data []byte) error {
	f, e := os.CreateTemp(filepath.Dir(path), ".swarm-write-")
	if e != nil {
		return e
	}
	name := f.Name()
	defer os.Remove(name)
	if _, e = f.Write(data); e != nil {
		f.Close()
		return e
	}
	if e = f.Sync(); e != nil {
		f.Close()
		return e
	}
	if e = f.Close(); e != nil {
		return e
	}
	return os.Rename(name, path)
}
