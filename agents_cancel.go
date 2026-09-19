//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

func cancellableQueuedAgent(a Agent) bool {
	return (a.Status == "queued" || (a.UnregisteredClaim && a.Desired == "stop")) && a.Supervisor == 0 && a.Child == 0 && a.Heartbeat == ""
}

// Competes with supervise's queued -> starting claim. It never stops a process
// or releases a registered launch. The legacy SQL-starting/JSON-queued gap
// is cancellable only with durable desired=stop. The old supervisor must save
// its registration BEFORE reading desired, and must read desired BEFORE spawning
// a provider. The body CAS therefore either loses to registration or ensures a
// delayed old supervisor will read stop before any provider launch.
func (s *Store) cancelQueuedAgent(a Agent) error {
	if !cancellableQueuedAgent(a) {
		return fmt.Errorf("Démarrage déjà pris en charge : vérifier son état avant annulation.")
	}
	var before []byte
	var previousStatus, desired string
	err := s.db.QueryRow("SELECT body,status,desired FROM agents WHERE id=?", a.ID).Scan(&before, &previousStatus, &desired)
	if err != nil {
		return err
	}
	current, err := decodeAgentRow(before, previousStatus, desired)
	if err != nil {
		return err
	}
	if !cancellableQueuedAgent(current) {
		return fmt.Errorf("Démarrage déjà pris en charge : actualisez son état.")
	}
	a = current
	a.Status = "interrupted"
	a.StopKind = originOperator
	a.Activity = "Démarrage annulé avant prise en charge — reprise possible"
	a.Ended = now()
	body, err := json.Marshal(a)
	if err != nil {
		return err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	result, err := tx.Exec("UPDATE agents SET status='interrupted',body=?,desired='stop' WHERE id=? AND status=? AND desired=? AND CAST(body AS BLOB)=?", body, a.ID, previousStatus, desired, before)
	if err != nil {
		return err
	}
	n, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if n != 1 {
		return fmt.Errorf("Le démarrage a changé ou a été pris en charge : actualisez son état.")
	}
	if _, err = tx.Exec("UPDATE reservations SET state='released' WHERE agent_id=? AND state='reserved'", a.ID); err != nil {
		return err
	}
	if err = tx.Commit(); err != nil {
		return err
	}
	if err = s.log(a.ID, "lifecycle", a.Activity); err != nil {
		return err
	}
	return s.settleAgentTask(a)
}
