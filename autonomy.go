//go:build linux

package main

import "fmt"

// Niveau d'autonomie d'un travail. Il décide de ce que le moteur fait sans
// demander. Une acceptation automatique exige en plus une politique de
// validation propre à la tâche, explicitement enregistrée par l'opérateur ;
// aucun niveau d'autonomie ne suffit à lui seul.
//
//	manuel   — l'opérateur lance et relaie
//	assisté  — le handoff prouvé est relayé, les départs restent humains
//	autonome — les départs prêts partent seuls dans les créneaux disponibles
const (
	autonomyManual   = "manuel"
	autonomyAssisted = "assiste"
	autonomyAuto     = "autonome"
)

// Valeur par défaut d'un travail sans réglage enregistré.
const autonomyDefault = autonomyAuto

// Créneaux simultanés par défaut. Deux tentatives parallèles supposent deux
// espaces de travail distincts ; sinon le moteur refuse le recouvrement.
const slotsDefault = 2

const slotsMax = 16

func validAutonomy(level string) bool {
	return level == autonomyManual || level == autonomyAssisted || level == autonomyAuto
}

func autonomyLabel(level string) string {
	switch level {
	case autonomyManual:
		return "Manuel — départs et relais humains"
	case autonomyAssisted:
		return "Assisté — relais automatique du handoff prouvé"
	case autonomyAuto:
		return "Autonome — départs automatiques dans les créneaux"
	}
	return level
}

// Le réglage est local au poste, comme la suspension des départs : une archive
// importée ne démarre rien par elle-même.
func (s *Store) setAutonomy(work, level string, slots int) error {
	if !validAutonomy(level) {
		return fmt.Errorf("niveau d'autonomie inconnu : %s (manuel, assiste ou autonome)", level)
	}
	if slots < 1 || slots > slotsMax {
		return fmt.Errorf("créneaux hors bornes : 1..%d", slotsMax)
	}
	if _, e := s.get(work); e != nil {
		return e
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	if level == autonomyAuto {
		if e := organizationGuardTx(tx, work); e != nil {
			return e
		}
	}
	if _, e = tx.Exec("INSERT INTO cockpit_controls(work_id,autonomy,slots) VALUES(?,?,?) ON CONFLICT(work_id) DO UPDATE SET autonomy=excluded.autonomy, slots=excluded.slots", work, level, slots); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "autonomy",
		fmt.Sprintf("opérateur local : %s · %d créneau(x) ; sans effet sur les tentatives déjà lancées", autonomyLabel(level), slots)); e != nil {
		return e
	}
	return tx.Commit()
}

func (s *Store) autonomy(work string) string {
	var level string
	if e := s.db.QueryRow("SELECT autonomy FROM cockpit_controls WHERE work_id=?", work).Scan(&level); e != nil || !validAutonomy(level) {
		return autonomyDefault
	}
	return level
}

func (s *Store) slots(work string) int {
	var slots int
	if e := s.db.QueryRow("SELECT slots FROM cockpit_controls WHERE work_id=?", work).Scan(&slots); e != nil || slots < 1 {
		return slotsDefault
	}
	return min(slots, slotsMax)
}
