//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"reflect"
	"time"
)

const terminalMonitoring = "Terminal natif : durée et processus surveillés ; appels d’outils, progression métier et coût non mesurés."
const terminalOutputLimit = 4 << 20

type terminalEvent struct {
	Seq  int64  `json:"seq"`
	Data []byte `json:"data,omitempty"`
	Cols int    `json:"cols,omitempty"`
	Rows int    `json:"rows,omitempty"`
}
type terminalRequest struct {
	Agent  string `json:"agent"`
	Work   string `json:"work"`
	Kind   string `json:"kind"`
	Client string `json:"client"`
	Lease  string `json:"lease"`
	Seq    int64  `json:"seq"`
	Data   []byte `json:"data,omitempty"`
	Cols   int    `json:"cols,omitempty"`
	Rows   int    `json:"rows,omitempty"`
}
type terminalReply struct {
	Error    string `json:"error,omitempty"`
	Lease    string `json:"lease,omitempty"`
	Seq      int64  `json:"seq"`
	Writable bool   `json:"writable"`
	Busy     bool   `json:"busy"`
}

func terminalStructuredLimits(l RunLimits) bool {
	return l.SilenceSeconds != 0 || l.ToolSeconds != 0 || l.MaxToolCalls != 0 || l.MaxRepeatedCalls != 0 || l.MaxConsecutiveErrors != 0
}

// No generic removal of CLI flags: unknown launch policies need an explicit
// interactive_args configuration. Never add approval/sandbox bypass flags.
func terminalProvider(p Provider) (Provider, error) {
	if terminalStructuredLimits(p.Limits) {
		return p, fmt.Errorf("Ce fournisseur impose des limites d’outils non mesurables en terminal ; utiliser le mode automatisé.")
	}
	if p.InteractiveArgs != nil {
		p.Args = append([]string{}, p.InteractiveArgs...)
		return p, nil
	}
	switch {
	case filepath.Base(p.Command) == "codex" && reflect.DeepEqual(p.Args, []string{"exec", "--json", "--sandbox", "workspace-write", "-"}):
		p.Args = []string{"--sandbox", "workspace-write", "--no-alt-screen"}
	case filepath.Base(p.Command) == "claude" && reflect.DeepEqual(p.Args, []string{"-p", "--output-format", "stream-json", "--verbose"}):
		p.Args = []string{}
	default:
		return p, fmt.Errorf("Configurer interactive_args pour ce fournisseur ; sa commande automatisée ne peut pas être convertie sans modifier ses permissions.")
	}
	return p, nil
}
func migrateTerminals(db *sql.DB, root string, backup bool) error {
	if backup {
		path := filepath.Join(root, ".swarm", newID("state-pre-v10-")+".db")
		f, e := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if e != nil {
			return e
		}
		if e = f.Close(); e != nil {
			return e
		}
		if _, e = db.Exec("VACUUM INTO ?", path); e != nil {
			return e
		}
	}
	_, e := db.Exec(`BEGIN IMMEDIATE;
 CREATE TABLE IF NOT EXISTS terminal_events(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL REFERENCES agents(id), data BLOB NOT NULL, cols INTEGER NOT NULL DEFAULT 0, rows INTEGER NOT NULL DEFAULT 0);
 CREATE INDEX IF NOT EXISTS terminal_cursor ON terminal_events(agent_id,seq);
 PRAGMA user_version=10;
 COMMIT;`)
	return e
}
func (s *Store) terminalSocket(id string) string {
	return filepath.Join(s.root, ".swarm", "term-"+hash([]byte(id))[:16]+".sock")
}
func (s *Store) terminalEvents(id string, after int64) ([]terminalEvent, error) {
	rows, e := s.db.Query("SELECT seq,data,cols,rows FROM terminal_events WHERE agent_id=? AND seq>? ORDER BY seq LIMIT 16", id, after)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	events := []terminalEvent{}
	for rows.Next() {
		var v terminalEvent
		if e = rows.Scan(&v.Seq, &v.Data, &v.Cols, &v.Rows); e != nil {
			return nil, e
		}
		events = append(events, v)
	}
	return events, rows.Err()
}
func (s *Store) terminalRPC(a Agent, r terminalRequest) (terminalReply, error) {
	var reply terminalReply
	if !interactiveMode(a.Mode) || a.Status != "running" || a.Desired == "stop" {
		return reply, fmt.Errorf("Session non interactive ou terminée ; consultation seule.")
	}
	if a.Host != hostIdentity() || a.SupervisorStamp == "" || processStamp(a.Supervisor) != a.SupervisorStamp {
		return reply, fmt.Errorf("Superviseur indisponible ; aucune saisie envoyée.")
	}
	c, e := net.DialTimeout("unix", s.terminalSocket(a.ID), time.Second)
	if e != nil {
		return reply, fmt.Errorf("Terminal indisponible ; relire l’état avant de saisir.")
	}
	defer c.Close()
	_ = c.SetDeadline(time.Now().Add(2 * time.Second))
	if e = json.NewEncoder(c).Encode(r); e != nil {
		return reply, e
	}
	e = json.NewDecoder(c).Decode(&reply)
	if e == nil && reply.Error != "" {
		e = fmt.Errorf("%s", reply.Error)
	}
	return reply, e
}
